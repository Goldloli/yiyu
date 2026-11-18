use crate::capture::{self, PermissionStatus};
use crate::dto::{
    AppInfoDTO, BootstrapDTO, DiagnosticsDTO, HistoryMutationDTO, ProviderViewDTO,
    SettingsEnvelopeDTO, StartTranslationDTO, TranslationEventDTO, TranslationStartResultDTO,
};
use crate::events;
use crate::state::AppState;
use crate::translation_session::TranslationIntent;
use crate::update::{self, UpdateSnapshotDTO};
use crate::windows;
use std::sync::Arc;
use std::time::Instant;
use tauri::{AppHandle, State};
use tauri_plugin_autostart::ManagerExt;
use yiyu_core::history::{HistoryEntry, HistoryFilter, NewHistoryEntry};
use yiyu_core::language::LANGUAGES;
use yiyu_core::settings::{SettingsPatch, SettingsSnapshot};
use yiyu_core::translation::{TranslationRequest, fetch_models, test_provider, translate_stream};
use yiyu_core::{ErrorKind, YiyuError, YiyuResult};

fn provider_views(
    settings: &yiyu_core::settings::Settings,
    credentials: &crate::credentials::CredentialStore,
) -> Vec<ProviderViewDTO> {
    settings
        .providers
        .iter()
        .cloned()
        .map(|config| {
            let configured = !config.requires_api_key
                || credentials
                    .get(&config.id)
                    .ok()
                    .flatten()
                    .is_some_and(|value| !value.trim().is_empty());
            ProviderViewDTO { config, configured }
        })
        .collect()
}

fn settings_envelope(
    snapshot: SettingsSnapshot,
    credentials: &crate::credentials::CredentialStore,
) -> SettingsEnvelopeDTO {
    let providers = provider_views(&snapshot.settings, credentials);
    SettingsEnvelopeDTO {
        revision: snapshot.revision,
        settings: snapshot.settings,
        providers,
    }
}

#[tauri::command]
pub async fn get_bootstrap(state: State<'_, Arc<AppState>>) -> YiyuResult<BootstrapDTO> {
    let state = state.inner().clone();
    let settings = state.settings.snapshot().await;
    let providers = provider_views(&settings.settings, &state.credentials);
    let history = state.history.list(&HistoryFilter {
        limit: Some(80),
        ..Default::default()
    })?;
    Ok(BootstrapDTO {
        app: update::app_info(&state.paths),
        settings_recovery_backup: state
            .settings
            .recovery_backup()
            .map(|path| path.to_string_lossy().into_owned()),
        shortcut_registration_failed: state.shortcut_registration_failed(),
        permissions: capture::permission_status(),
        update: state.update.snapshot(),
        settings,
        providers,
        languages: LANGUAGES.to_vec(),
        history,
    })
}

#[tauri::command]
pub fn get_app_info(state: State<'_, Arc<AppState>>) -> AppInfoDTO {
    update::app_info(&state.paths)
}

#[tauri::command]
pub async fn get_settings(state: State<'_, Arc<AppState>>) -> YiyuResult<SettingsEnvelopeDTO> {
    let snapshot = state.settings.snapshot().await;
    Ok(settings_envelope(snapshot, &state.credentials))
}

#[tauri::command]
pub async fn patch_settings(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
    expected_revision: u64,
    patch: SettingsPatch,
) -> YiyuResult<SettingsEnvelopeDTO> {
    let _settings_guard = state.settings_update.lock().await;
    let previous = state.settings.snapshot().await;
    if expected_revision != previous.revision {
        return Err(YiyuError::new(
            ErrorKind::Validation,
            format!(
                "settings revision mismatch: current {}, received {expected_revision}",
                previous.revision
            ),
        )
        .field("revision"));
    }
    let next = previous.settings.apply_patch(&patch);
    next.validate().map_err(|issues| {
        YiyuError::new(
            ErrorKind::Validation,
            issues
                .iter()
                .map(|issue| format!("{}: {}", issue.field, issue.message))
                .collect::<Vec<_>>()
                .join("; "),
        )
    })?;
    crate::shortcuts::validate_unique(&next)?;

    let shortcuts_changed = previous.settings.shortcuts != next.shortcuts;
    if shortcuts_changed && let Err(error) = crate::shortcuts::reload(&app, &next) {
        state.set_shortcut_registration_failed(
            crate::shortcuts::reload(&app, &previous.settings).is_err(),
        );
        return Err(error);
    }
    let snapshot = match state.settings.patch(expected_revision, patch).await {
        Ok(snapshot) => snapshot,
        Err(error) => {
            if shortcuts_changed {
                state.set_shortcut_registration_failed(
                    crate::shortcuts::reload(&app, &previous.settings).is_err(),
                );
            }
            return Err(error);
        }
    };
    if shortcuts_changed {
        state.set_shortcut_registration_failed(false);
    }
    for provider in &previous.settings.providers {
        if !snapshot
            .settings
            .providers
            .iter()
            .any(|current| current.id == provider.id)
        {
            let _ = state.credentials.delete(&provider.id);
        }
    }
    let envelope = settings_envelope(snapshot.clone(), &state.credentials);
    events::emit(&app, events::SETTINGS_CHANGED, envelope.clone());
    if previous.settings.shortcuts != snapshot.settings.shortcuts
        && let Err(error) = crate::tray::sync_shortcuts(&app, &snapshot.settings.shortcuts)
    {
        tracing::warn!(target: "yiyu_app::tray", %error, "cannot update menu shortcuts");
    }
    if snapshot.settings.onboarding_completed {
        windows::hide_onboarding(&app);
    }
    Ok(envelope)
}

#[tauri::command]
pub async fn get_background(state: State<'_, Arc<AppState>>) -> YiyuResult<Option<String>> {
    state.backgrounds.current(&state.settings).await
}

#[tauri::command]
pub async fn set_background(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
    expected_revision: u64,
    data: String,
) -> YiyuResult<SettingsEnvelopeDTO> {
    let snapshot = state
        .backgrounds
        .replace(&state.settings, expected_revision, Some(&data))
        .await?;
    let envelope = settings_envelope(snapshot, &state.credentials);
    events::emit(&app, events::SETTINGS_CHANGED, envelope.clone());
    Ok(envelope)
}

#[tauri::command]
pub async fn clear_background(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
    expected_revision: u64,
) -> YiyuResult<SettingsEnvelopeDTO> {
    let snapshot = state
        .backgrounds
        .replace(&state.settings, expected_revision, None)
        .await?;
    let envelope = settings_envelope(snapshot, &state.credentials);
    events::emit(&app, events::SETTINGS_CHANGED, envelope.clone());
    Ok(envelope)
}

#[tauri::command]
pub async fn save_provider_secret(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
    provider_id: String,
    secret: String,
) -> YiyuResult<()> {
    if secret.trim().is_empty() {
        state.credentials.delete(&provider_id)?;
    } else {
        state.credentials.set(&provider_id, secret.trim())?;
    }
    let settings = state.settings.snapshot().await;
    events::emit(
        &app,
        events::SETTINGS_CHANGED,
        settings_envelope(settings, &state.credentials),
    );
    Ok(())
}

#[tauri::command]
pub async fn delete_provider_secret(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
    provider_id: String,
) -> YiyuResult<()> {
    state.credentials.delete(&provider_id)?;
    let settings = state.settings.snapshot().await;
    events::emit(
        &app,
        events::SETTINGS_CHANGED,
        settings_envelope(settings, &state.credentials),
    );
    Ok(())
}

#[tauri::command]
pub async fn test_provider_connection(
    state: State<'_, Arc<AppState>>,
    provider: yiyu_core::provider::ProviderConfig,
    api_key: Option<String>,
) -> YiyuResult<yiyu_core::translation::ProviderTestResult> {
    let settings = state.settings.snapshot().await;
    let key = api_key
        .filter(|value| !value.trim().is_empty())
        .or_else(|| state.credentials.get(&provider.id).ok().flatten());
    Ok(test_provider(&provider, key.as_deref(), &settings.settings.provider_proxy).await)
}

#[tauri::command]
pub async fn fetch_provider_models(
    state: State<'_, Arc<AppState>>,
    provider: yiyu_core::provider::ProviderConfig,
    api_key: Option<String>,
) -> YiyuResult<Vec<String>> {
    let settings = state.settings.snapshot().await;
    let key = api_key
        .filter(|value| !value.trim().is_empty())
        .or_else(|| state.credentials.get(&provider.id).ok().flatten());
    fetch_models(&provider, key.as_deref(), &settings.settings.provider_proxy).await
}

#[tauri::command]
pub async fn translate_start(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
    input: StartTranslationDTO,
) -> YiyuResult<TranslationStartResultDTO> {
    let surface = input.surface.clone();
    let intent = state.translations.begin_request(&surface);
    let result = begin_translation(app, state.inner().clone(), input, &intent).await;
    if state.translations.finish_request(&surface, &intent) {
        result
    } else {
        Err(YiyuError::cancelled())
    }
}

async fn begin_translation(
    app: AppHandle,
    state: Arc<AppState>,
    input: StartTranslationDTO,
    intent: &TranslationIntent,
) -> YiyuResult<TranslationStartResultDTO> {
    let snapshot = state.settings.snapshot().await;
    let provider_id = input
        .provider_id
        .or(snapshot.settings.active_provider_id.clone())
        .ok_or_else(|| {
            YiyuError::new(ErrorKind::Config, "configure a provider before translating")
                .field("activeProviderId")
                .action("open-settings")
        })?;
    let provider = snapshot
        .settings
        .providers
        .iter()
        .find(|provider| provider.id == provider_id)
        .cloned()
        .ok_or_else(|| {
            YiyuError::new(ErrorKind::Config, "active provider does not exist")
                .field("activeProviderId")
        })?;
    let api_key = state.credentials.get(&provider.id)?;
    if provider.requires_api_key
        && api_key
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .is_none()
    {
        return Err(
            YiyuError::new(ErrorKind::Authentication, "API key is not configured")
                .field("apiKey")
                .action("open-settings"),
        );
    }
    let text = input.text.trim().to_string();
    if text.is_empty() {
        return Err(YiyuError::validation("empty text", "text"));
    }
    if text.chars().count() > snapshot.settings.max_input_chars as usize {
        return Err(YiyuError::validation(
            format!(
                "input exceeds the current limit of {} characters",
                snapshot.settings.max_input_chars
            ),
            "maxInputChars",
        ));
    }
    let surface = input.surface;
    if surface.is_empty() {
        return Err(YiyuError::validation(
            "translation surface is missing",
            "surface",
        ));
    }
    let session_id = uuid::Uuid::new_v4().to_string();
    let session_for_task = session_id.clone();
    let cancel = state
        .translations
        .begin_translation(&surface, session_id.clone(), intent)
        .ok_or_else(YiyuError::cancelled)?;
    let request = TranslationRequest {
        provider,
        api_key,
        text: text.clone(),
        manual_source_language: input.manual_source_language,
        target_language: input
            .target_language
            .unwrap_or(snapshot.settings.effective_target_language()),
        fallback_target_language: input
            .fallback_target_language
            .unwrap_or(snapshot.settings.fallback_target_language.clone()),
        proxy: snapshot.settings.provider_proxy.clone(),
        max_input_chars: snapshot.settings.max_input_chars,
        ui_language: match snapshot.settings.ui_language {
            yiyu_core::settings::UiLanguage::En => "en".into(),
            _ => "zh".into(),
        },
    };
    let request_text = request.text.clone();
    let mut preparing = TranslationEventDTO::state(&session_id, &surface, "preparing");
    preparing.source_text = Some(text.clone());
    events::emit(&app, events::TRANSLATION_EVENT, preparing);
    let task_state = state.clone();
    let task_app = app.clone();
    tauri::async_runtime::spawn(async move {
        let started = Instant::now();
        let delayed_state = task_state.clone();
        let delayed_app = task_app.clone();
        let session_for_delta = session_for_task.clone();
        let surface_for_delta = surface.clone();
        let result = translate_stream(&request, &cancel, move |delta| {
            if delayed_state
                .translations
                .is_active(&surface_for_delta, &session_for_delta)
            {
                let mut event =
                    TranslationEventDTO::state(&session_for_delta, &surface_for_delta, "streaming");
                event.source_text = Some(request_text.clone());
                event.text = Some(delta.to_string());
                events::emit(&delayed_app, events::TRANSLATION_EVENT, event);
            }
        })
        .await;

        match result {
            Ok(outcome) => {
                if !task_state
                    .translations
                    .claim_translation_completion(&surface, &session_for_task)
                {
                    return;
                }
                let mut event =
                    TranslationEventDTO::state(&session_for_task, &surface, "completed");
                event.source_text = Some(text.clone());
                event.text = Some(outcome.text.clone());
                event.direction = Some(outcome.direction.clone());
                event.duration_ms = Some(outcome.duration_ms);
                let mut persistence_error = None;
                let mut history_saved = false;
                if snapshot.settings.auto_save_history {
                    let entry = NewHistoryEntry {
                        source_text: text,
                        translated_text: outcome.text,
                        source_language: outcome.direction.source_label,
                        target_language: outcome.direction.target_label,
                        provider_id: request.provider.id.clone(),
                        provider_label: request.provider.label.clone(),
                        model: request.provider.model.clone(),
                        duration_ms: outcome.duration_ms,
                    };
                    match task_state
                        .history
                        .insert(&entry, snapshot.settings.history_limit)
                    {
                        Ok(_) => {
                            history_saved = true;
                            events::emit(&task_app, events::HISTORY_CHANGED, true);
                        }
                        Err(error) => {
                            task_state.remember_pending_history(&session_for_task, entry);
                            persistence_error = Some(error);
                        }
                    }
                }
                event.history_saved = Some(history_saved);
                event.persistence_error = persistence_error;
                events::emit(&task_app, events::TRANSLATION_EVENT, event);
                task_state
                    .translations
                    .finish_translation(&surface, &session_for_task);
            }
            Err(error) => {
                let state_name = if error.kind == ErrorKind::Cancelled {
                    "cancelled"
                } else {
                    crate::logging::record_request_failure(
                        &request.provider.id,
                        &request.provider.model,
                        None,
                        started.elapsed().as_millis() as u64,
                        &error,
                    );
                    "failed"
                };
                let mut event = TranslationEventDTO::state(&session_for_task, &surface, state_name);
                if error.kind != ErrorKind::Cancelled {
                    event.error = Some(error);
                }
                if task_state
                    .translations
                    .is_active(&surface, &session_for_task)
                {
                    events::emit(&task_app, events::TRANSLATION_EVENT, event);
                }
                task_state
                    .translations
                    .finish_translation(&surface, &session_for_task);
            }
        }
    });

    Ok(TranslationStartResultDTO { session_id })
}

#[tauri::command]
pub async fn translate_cancel(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
    surface: String,
) -> YiyuResult<()> {
    if let Some(session_id) = state.translations.cancel_translation(&surface) {
        events::emit(
            &app,
            events::TRANSLATION_EVENT,
            TranslationEventDTO::state(&session_id, &surface, "cancelled"),
        );
    }
    Ok(())
}

#[tauri::command]
pub async fn capture_selection(state: State<'_, Arc<AppState>>) -> YiyuResult<String> {
    let settings = state.settings.snapshot().await;
    capture::capture_selection(settings.settings.capture_clipboard_fallback, None)
        .await
        .map_err(Into::into)
}

#[tauri::command]
pub async fn translate_clipboard(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
) -> YiyuResult<TranslationStartResultDTO> {
    let intent = state.translations.begin_request(windows::POPUP);
    let text = tokio::select! {
        biased;
        _ = intent.cancel.cancelled() => Err(YiyuError::cancelled()),
        text = capture::read_clipboard_text() => text,
    };
    let result = match text {
        Ok(text) => {
            begin_translation(
                app,
                state.inner().clone(),
                popup_translation_input(windows::POPUP, text),
                &intent,
            )
            .await
        }
        Err(error) => Err(error),
    };
    if state.translations.finish_request(windows::POPUP, &intent) {
        result
    } else {
        Err(YiyuError::cancelled())
    }
}

#[tauri::command]
pub async fn list_history(
    state: State<'_, Arc<AppState>>,
    filter: HistoryFilter,
) -> YiyuResult<Vec<HistoryEntry>> {
    state.history.list(&filter)
}

#[tauri::command]
pub async fn delete_history(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
    id: String,
) -> YiyuResult<bool> {
    let changed = state.history.delete(&id)?;
    events::emit(&app, events::HISTORY_CHANGED, true);
    Ok(changed)
}

#[tauri::command]
pub async fn clear_history(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
) -> YiyuResult<HistoryMutationDTO> {
    let changed = state.history.clear_all()?;
    events::emit(&app, events::HISTORY_CHANGED, true);
    Ok(HistoryMutationDTO { changed })
}

#[tauri::command]
pub async fn retry_save_history(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
    session_id: String,
) -> YiyuResult<HistoryEntry> {
    let settings = state.settings.snapshot().await;
    let saved = state.retry_pending_history(&session_id, settings.settings.history_limit)?;
    events::emit(&app, events::HISTORY_CHANGED, true);
    Ok(saved)
}

#[tauri::command]
pub async fn copy_text(text: String) -> YiyuResult<()> {
    capture::copy_text(&text).await
}

#[tauri::command]
pub fn open_external(url: String) -> YiyuResult<()> {
    let parsed = url::Url::parse(&url)?;
    if !matches!(parsed.scheme(), "http" | "https" | "mailto") {
        return Err(YiyuError::validation(
            "only http, https and mailto links can be opened",
            "url",
        ));
    }
    tauri_plugin_opener::open_url(url, None::<&str>)
        .map_err(|error| YiyuError::new(ErrorKind::Other, format!("cannot open link: {error}")))
}

#[tauri::command]
pub async fn refresh_permissions(app: AppHandle) -> PermissionStatus {
    let status = capture::permission_status();
    events::emit(&app, events::PERMISSION_CHANGED, status);
    status
}

#[tauri::command]
pub fn request_accessibility_permission() {
    capture::request_accessibility_permission()
}

#[tauri::command]
pub fn open_accessibility_settings() -> YiyuResult<()> {
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg("x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility")
            .spawn()
            .map(|_| ())
            .map_err(|error| {
                YiyuError::new(
                    ErrorKind::Other,
                    format!("cannot open system settings: {error}"),
                )
            })
    }
    #[cfg(not(target_os = "macos"))]
    {
        Err(YiyuError::new(
            ErrorKind::Unsupported,
            "accessibility settings are not available on this platform",
        ))
    }
}

#[tauri::command]
pub fn get_autostart(app: AppHandle) -> YiyuResult<bool> {
    app.autolaunch().is_enabled().map_err(|error| {
        YiyuError::new(ErrorKind::Other, format!("cannot read login item: {error}"))
    })
}

#[tauri::command]
pub fn set_autostart(app: AppHandle, enabled: bool) -> YiyuResult<bool> {
    let manager = app.autolaunch();
    let result = if enabled {
        manager.enable()
    } else {
        manager.disable()
    };
    result.map_err(|error| {
        YiyuError::new(
            ErrorKind::Other,
            format!("cannot update login item: {error}"),
        )
    })?;
    manager.is_enabled().map_err(|error| {
        YiyuError::new(ErrorKind::Other, format!("cannot read login item: {error}"))
    })
}

#[tauri::command]
pub fn open_main_window(app: AppHandle) {
    windows::show_main(&app);
}

#[tauri::command]
pub fn open_settings_window(app: AppHandle, section: Option<String>) {
    match section.as_deref() {
        Some(
            "general" | "appearance" | "models" | "shortcuts" | "network" | "updates" | "about",
        ) => {
            windows::show_settings_section(&app, section.as_deref().unwrap_or("models"));
        }
        _ => windows::show_settings(&app),
    }
}

#[tauri::command]
pub fn open_onboarding_window(app: AppHandle) {
    windows::show_onboarding(&app);
}

#[tauri::command]
pub fn close_popup(app: AppHandle, state: State<'_, Arc<AppState>>) {
    if state.popup_pinned() {
        state.set_popup_pinned(false);
    }
    let _ = state.translations.cancel_translation(windows::POPUP);
    windows::hide_popup(&app);
}

#[tauri::command]
pub fn set_popup_pinned(state: State<'_, Arc<AppState>>, pinned: bool) {
    state.set_popup_pinned(pinned);
}

#[tauri::command]
pub fn resize_popup_window(app: AppHandle, width: f64, height: f64) -> YiyuResult<()> {
    windows::resize_popup(&app, width, height)
}

#[tauri::command]
pub fn quit_app(app: AppHandle, state: State<'_, Arc<AppState>>) {
    state.translations.cancel_all();
    windows::hide_all(&app);
    app.exit(0);
}

#[tauri::command]
pub async fn check_update(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
    force: bool,
) -> YiyuResult<UpdateSnapshotDTO> {
    let settings = state.settings.snapshot().await;
    let result = state
        .update
        .check(&settings.settings.update_proxy, force)
        .await;
    let snapshot = state.update.snapshot();
    events::emit(&app, events::UPDATE_STATE, snapshot.clone());
    result.map(|_| snapshot)
}

#[tauri::command]
pub async fn download_update(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
) -> YiyuResult<UpdateSnapshotDTO> {
    let settings = state.settings.snapshot().await;
    let progress_app = app.clone();
    let result = state
        .update
        .download(&settings.settings.update_proxy, move |snapshot| {
            events::emit(&progress_app, events::UPDATE_STATE, snapshot);
        })
        .await;
    let snapshot = state.update.snapshot();
    events::emit(&app, events::UPDATE_STATE, snapshot.clone());
    result.map(|_| snapshot)
}

#[tauri::command]
pub fn cancel_update_download(state: State<'_, Arc<AppState>>) {
    state.update.cancel();
}

#[tauri::command]
pub fn install_update(
    _app: AppHandle,
    state: State<'_, Arc<AppState>>,
    path: String,
) -> YiyuResult<()> {
    let path = state.update.verify_ready_file(&path)?;
    state.translations.cancel_all();
    update::install_ready_update(&path)?;
    #[cfg(target_os = "windows")]
    _app.exit(0);
    Ok(())
}

#[tauri::command]
pub fn get_update_snapshot(state: State<'_, Arc<AppState>>) -> UpdateSnapshotDTO {
    state.update.snapshot()
}

#[tauri::command]
pub fn get_diagnostics(state: State<'_, Arc<AppState>>) -> DiagnosticsDTO {
    DiagnosticsDTO {
        data_dir: state.paths.root().display().to_string(),
        logs_dir: state.paths.logs_dir().display().to_string(),
        version: env!("CARGO_PKG_VERSION").into(),
        platform: std::env::consts::OS.into(),
    }
}

fn show_popup_failure(
    app: &AppHandle,
    state: &AppState,
    surface: &str,
    intent: &TranslationIntent,
    source: Option<String>,
    error: YiyuError,
) {
    if error.kind == ErrorKind::Cancelled {
        state.translations.finish_request(surface, intent);
        return;
    }
    if !state.translations.fail_request(surface, intent) {
        return;
    }
    let session_id = uuid::Uuid::new_v4().to_string();
    let mut preparing = TranslationEventDTO::state(&session_id, surface, "preparing");
    preparing.source_text = Some(source.unwrap_or_default());
    events::emit(app, events::TRANSLATION_EVENT, preparing);
    let mut failed = TranslationEventDTO::state(&session_id, surface, "failed");
    failed.error = Some(error);
    events::emit(app, events::TRANSLATION_EVENT, failed);
    windows::show_popup_at_cursor(app, 380.0, 240.0);
}

fn popup_translation_input(surface: &str, text: String) -> StartTranslationDTO {
    StartTranslationDTO {
        surface: surface.into(),
        text,
        provider_id: None,
        manual_source_language: None,
        target_language: None,
        fallback_target_language: None,
    }
}

async fn translate_popup_text(
    app: AppHandle,
    state: Arc<AppState>,
    surface: &str,
    intent: &TranslationIntent,
    text: String,
) {
    match begin_translation(
        app.clone(),
        state.clone(),
        popup_translation_input(surface, text.clone()),
        intent,
    )
    .await
    {
        Ok(_) => {
            if state.translations.finish_request(surface, intent) {
                windows::show_popup_at_cursor(&app, 380.0, 260.0);
            }
        }
        Err(error) => show_popup_failure(&app, &state, surface, intent, Some(text), error),
    }
}

pub async fn capture_and_translate(
    app: AppHandle,
    state: Arc<AppState>,
    surface: &str,
    intent: TranslationIntent,
) {
    let snapshot = tokio::select! {
        biased;
        _ = intent.cancel.cancelled() => return,
        snapshot = state.settings.snapshot() => snapshot,
    };
    tracing::info!(
        target: "yiyu_app::capture",
        surface,
        fallback = snapshot.settings.capture_clipboard_fallback,
        "selection capture started"
    );
    let captured = capture::capture_selection(
        snapshot.settings.capture_clipboard_fallback,
        Some(&intent.cancel),
    )
    .await;
    match captured {
        Ok(text) => {
            tracing::info!(
                target: "yiyu_app::capture",
                surface,
                characters = text.chars().count(),
                "selection capture succeeded"
            );
            translate_popup_text(app, state, surface, &intent, text).await;
        }
        Err(error) => {
            let error: YiyuError = error.into();
            tracing::warn!(
                target: "yiyu_app::capture",
                surface,
                kind = ?error.kind,
                "selection capture failed"
            );
            show_popup_failure(&app, &state, surface, &intent, None, error);
        }
    }
}

pub async fn translate_clipboard_and_show(
    app: AppHandle,
    state: Arc<AppState>,
    intent: TranslationIntent,
) {
    let text = tokio::select! {
        biased;
        _ = intent.cancel.cancelled() => return,
        text = capture::read_clipboard_text() => text,
    };
    match text {
        Ok(text) => {
            translate_popup_text(app, state, windows::POPUP, &intent, text).await;
        }
        Err(error) => {
            show_popup_failure(&app, &state, windows::POPUP, &intent, None, error);
        }
    }
}

pub fn begin_daily_update_check(app: AppHandle, state: Arc<AppState>) {
    tauri::async_runtime::spawn(async move {
        let snapshot = state.settings.snapshot().await;
        if snapshot.settings.check_updates && state.update.automatic_check_due() {
            let _ = state
                .update
                .check(&snapshot.settings.update_proxy, false)
                .await;
            events::emit(&app, events::UPDATE_STATE, state.update.snapshot());
        }
    });
}
