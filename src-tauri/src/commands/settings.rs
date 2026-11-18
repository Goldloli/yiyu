use crate::dto::{ProviderViewDTO, SettingsEnvelopeDTO};
use crate::events;
use crate::state::AppState;
use crate::windows;
use std::sync::Arc;
use tauri::{AppHandle, State};
use yiyu_core::settings::{SettingsPatch, SettingsSnapshot};
use yiyu_core::translation::{fetch_models, test_provider};
use yiyu_core::{ErrorKind, YiyuError, YiyuResult};

pub(super) fn provider_views(
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
