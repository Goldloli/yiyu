use crate::events;
use crate::state::AppState;
use crate::update::{self, UpdateSnapshotDTO};
use std::sync::Arc;
use tauri::{AppHandle, State};
use yiyu_core::YiyuResult;

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
