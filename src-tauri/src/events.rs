use tauri::{AppHandle, Emitter};

pub const TRANSLATION_EVENT: &str = "translation://event";
pub const SETTINGS_CHANGED: &str = "settings://changed";
pub const SETTINGS_NAVIGATE: &str = "settings://navigate";
pub const HISTORY_CHANGED: &str = "history://changed";
pub const PERMISSION_CHANGED: &str = "capture://permission";
pub const UPDATE_STATE: &str = "update://state";

pub fn emit<R: tauri::Runtime, T: serde::Serialize + Clone>(
    app: &AppHandle<R>,
    event: &str,
    payload: T,
) {
    if let Err(error) = app.emit(event, payload) {
        tracing::warn!(target: "yiyu::events", event, %error, "event emit failed");
    }
}
