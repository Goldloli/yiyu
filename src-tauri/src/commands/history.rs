use crate::dto::HistoryMutationDTO;
use crate::events;
use crate::state::AppState;
use std::sync::Arc;
use tauri::{AppHandle, State};
use yiyu_core::YiyuResult;
use yiyu_core::history::{HistoryEntry, HistoryFilter};

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
    let saved = state.pending_history.retry(&session_id, |entry| {
        state.history.insert(entry, settings.settings.history_limit)
    })?;
    events::emit(&app, events::HISTORY_CHANGED, true);
    Ok(saved)
}
