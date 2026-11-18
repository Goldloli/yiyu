use crate::dto::{StartTranslationDTO, TranslationStartResultDTO};
use crate::state::AppState;
use crate::translation_flow;
use std::sync::Arc;
use tauri::{AppHandle, State};
use yiyu_core::YiyuResult;

#[tauri::command]
pub async fn translate_start(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
    input: StartTranslationDTO,
) -> YiyuResult<TranslationStartResultDTO> {
    translation_flow::translate_start(app, state.inner().clone(), input).await
}

#[tauri::command]
pub async fn translate_cancel(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
    surface: String,
) -> YiyuResult<()> {
    translation_flow::translate_cancel(app, state.inner().clone(), surface).await
}

#[tauri::command]
pub async fn capture_selection(state: State<'_, Arc<AppState>>) -> YiyuResult<String> {
    translation_flow::capture_selection(state.inner().clone()).await
}

#[tauri::command]
pub async fn translate_clipboard(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
) -> YiyuResult<TranslationStartResultDTO> {
    translation_flow::translate_clipboard(app, state.inner().clone()).await
}
