use crate::capture;
use crate::dto::{StartTranslationDTO, TranslationEventDTO, TranslationStartResultDTO};
use crate::events;
use crate::state::AppState;
use crate::translation_session::TranslationIntent;
use crate::windows;
use std::sync::Arc;
use std::time::Instant;
use tauri::AppHandle;
use yiyu_core::history::NewHistoryEntry;
use yiyu_core::translation::{TranslationRequest, translate_stream};
use yiyu_core::{ErrorKind, YiyuError, YiyuResult};

pub async fn translate_start(
    app: AppHandle,
    state: Arc<AppState>,
    input: StartTranslationDTO,
) -> YiyuResult<TranslationStartResultDTO> {
    let surface = input.surface.clone();
    let intent = state.translations.begin_request(&surface);
    let result = begin_translation(app, state.clone(), input, &intent).await;
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

pub async fn translate_cancel(
    app: AppHandle,
    state: Arc<AppState>,
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

pub async fn capture_selection(state: Arc<AppState>) -> YiyuResult<String> {
    let settings = state.settings.snapshot().await;
    capture::capture_selection(settings.settings.capture_clipboard_fallback, None)
        .await
        .map_err(Into::into)
}

pub async fn translate_clipboard(
    app: AppHandle,
    state: Arc<AppState>,
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
                state.clone(),
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
