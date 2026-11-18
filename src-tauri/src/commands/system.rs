use crate::capture::{self, PermissionStatus};
use crate::dto::DiagnosticsDTO;
use crate::events;
use crate::state::AppState;
use crate::windows;
use std::sync::Arc;
use tauri::{AppHandle, State};
use tauri_plugin_autostart::ManagerExt;
use yiyu_core::{ErrorKind, YiyuError, YiyuResult};

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
pub fn get_diagnostics(state: State<'_, Arc<AppState>>) -> DiagnosticsDTO {
    DiagnosticsDTO {
        data_dir: state.paths.root().display().to_string(),
        logs_dir: state.paths.logs_dir().display().to_string(),
        version: env!("CARGO_PKG_VERSION").into(),
        platform: std::env::consts::OS.into(),
    }
}
