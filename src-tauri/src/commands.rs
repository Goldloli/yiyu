#[cfg(test)]
mod contract_tests;
pub mod history;
pub mod settings;
pub mod system;
pub mod translation;
pub mod updates;

use self::settings::provider_views;
use crate::capture;
use crate::dto::{AppInfoDTO, BootstrapDTO};
use crate::state::AppState;
use crate::update;
use std::sync::Arc;
use tauri::State;
use yiyu_core::YiyuResult;
use yiyu_core::history::HistoryFilter;
use yiyu_core::language::LANGUAGES;

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
