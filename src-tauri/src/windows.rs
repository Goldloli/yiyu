use tauri::{
    AppHandle, Emitter, Manager, PhysicalPosition, PhysicalSize, WebviewUrl, WebviewWindowBuilder,
};
use yiyu_core::{ErrorKind, YiyuError, YiyuResult};

#[cfg(target_os = "macos")]
thread_local! {
    static DISMISS_MONITOR: std::cell::RefCell<Option<objc2::rc::Retained<objc2::runtime::AnyObject>>> =
        const { std::cell::RefCell::new(None) };
}

pub const MAIN: &str = "main";
pub const SETTINGS: &str = "settings";
pub const ONBOARDING: &str = "onboarding";
pub const POPUP: &str = "popup";

const POPUP_MIN_WIDTH: f64 = 320.0;
const POPUP_MAX_WIDTH: f64 = 560.0;
const POPUP_MIN_HEIGHT: f64 = 180.0;
const POPUP_MAX_HEIGHT: f64 = 680.0;

#[cfg(target_os = "macos")]
tauri_nspanel::tauri_panel! {
    panel!(YiyuPopupPanel {
        config: {
            can_become_key_window: true,
            is_floating_panel: true
        }
    })
}

pub fn install_all(app: &AppHandle) -> tauri::Result<()> {
    create_main(app)?;
    create_settings(app)?;
    create_onboarding(app)?;
    create_popup(app)?;
    Ok(())
}

pub fn startup_window(onboarding_completed: bool, silent_start: bool) -> Option<&'static str> {
    if !onboarding_completed {
        Some(ONBOARDING)
    } else if silent_start {
        None
    } else {
        Some(MAIN)
    }
}

fn create_main(app: &AppHandle) -> tauri::Result<()> {
    if app.get_webview_window(MAIN).is_some() {
        return Ok(());
    }
    let builder = WebviewWindowBuilder::new(app, MAIN, WebviewUrl::App("index.html#/main".into()))
        .title("译羽")
        .inner_size(980.0, 660.0)
        .min_inner_size(760.0, 500.0)
        .visible(false)
        .center()
        .resizable(true);
    #[cfg(target_os = "macos")]
    let builder = builder
        .title_bar_style(tauri::TitleBarStyle::Overlay)
        .hidden_title(true)
        .transparent(true);
    builder.build()?;
    Ok(())
}

fn create_settings(app: &AppHandle) -> tauri::Result<()> {
    if app.get_webview_window(SETTINGS).is_some() {
        return Ok(());
    }
    let builder = WebviewWindowBuilder::new(
        app,
        SETTINGS,
        WebviewUrl::App("index.html#/settings".into()),
    )
    .title("译羽设置")
    .inner_size(860.0, 640.0)
    .min_inner_size(720.0, 520.0)
    .visible(false)
    .center()
    .resizable(true);
    #[cfg(target_os = "macos")]
    let builder = builder
        .title_bar_style(tauri::TitleBarStyle::Overlay)
        .hidden_title(true);
    builder.build()?;
    Ok(())
}

fn create_onboarding(app: &AppHandle) -> tauri::Result<()> {
    if app.get_webview_window(ONBOARDING).is_some() {
        return Ok(());
    }
    let builder = WebviewWindowBuilder::new(
        app,
        ONBOARDING,
        WebviewUrl::App("index.html#/onboarding".into()),
    )
    .title("欢迎使用译羽")
    .inner_size(780.0, 640.0)
    .min_inner_size(700.0, 560.0)
    .visible(false)
    .center()
    .resizable(true);
    #[cfg(target_os = "macos")]
    let builder = builder
        .title_bar_style(tauri::TitleBarStyle::Overlay)
        .hidden_title(true);
    builder.build()?;
    Ok(())
}

#[cfg(target_os = "macos")]
fn create_popup(app: &AppHandle) -> tauri::Result<()> {
    use tauri_nspanel::{CollectionBehavior, PanelBuilder, PanelLevel, StyleMask};

    if app.get_webview_window(POPUP).is_some() {
        return Ok(());
    }
    PanelBuilder::<_, YiyuPopupPanel>::new(app, POPUP)
        .url(WebviewUrl::App("index.html#/popup".into()))
        .title("译羽快速翻译")
        .size(tauri::Size::Logical(tauri::LogicalSize::new(380.0, 260.0)))
        .content_size(tauri::Size::Logical(tauri::LogicalSize::new(380.0, 260.0)))
        .min_size(tauri::Size::Logical(tauri::LogicalSize::new(
            POPUP_MIN_WIDTH,
            POPUP_MIN_HEIGHT,
        )))
        .max_size(tauri::Size::Logical(tauri::LogicalSize::new(
            POPUP_MAX_WIDTH,
            POPUP_MAX_HEIGHT,
        )))
        .style_mask(
            StyleMask::empty()
                .borderless()
                .resizable()
                .nonactivating_panel()
                .full_size_content_view(),
        )
        .collection_behavior(
            CollectionBehavior::new()
                .can_join_all_spaces()
                .full_screen_auxiliary(),
        )
        .no_activate(true)
        .level(PanelLevel::Floating)
        .transparent(true)
        .opaque(false)
        .has_shadow(true)
        .hides_on_deactivate(false)
        .movable_by_window_background(true)
        .with_window(|window| {
            window
                .visible(false)
                .decorations(false)
                .always_on_top(true)
                .skip_taskbar(true)
                .resizable(true)
                .transparent(true)
        })
        .build()?;
    Ok(())
}

#[cfg(target_os = "windows")]
fn create_popup(app: &AppHandle) -> tauri::Result<()> {
    if app.get_webview_window(POPUP).is_some() {
        return Ok(());
    }
    let window = WebviewWindowBuilder::new(app, POPUP, WebviewUrl::App("index.html#/popup".into()))
        .title("译羽快速翻译")
        .inner_size(380.0, 260.0)
        .min_inner_size(POPUP_MIN_WIDTH, POPUP_MIN_HEIGHT)
        .max_inner_size(POPUP_MAX_WIDTH, POPUP_MAX_HEIGHT)
        .visible(false)
        .decorations(false)
        .always_on_top(true)
        .skip_taskbar(true)
        .resizable(true)
        .shadow(true)
        .build()?;
    crate::windows_chrome::make_popup_nonactivating(&window)?;
    Ok(())
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
fn create_popup(_app: &AppHandle) -> tauri::Result<()> {
    Ok(())
}

pub fn show_main(app: &AppHandle) {
    if let Some(window) = app.get_webview_window(MAIN) {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
    }
}

pub fn show_settings(app: &AppHandle) {
    if let Some(window) = app.get_webview_window(SETTINGS) {
        let _ = window.center();
        let _ = window.show();
        let _ = window.set_focus();
    }
}

pub fn show_settings_section(app: &AppHandle, section: &str) {
    show_settings(app);
    if let Some(window) = app.get_webview_window(SETTINGS) {
        let _ = window.emit(crate::events::SETTINGS_NAVIGATE, section);
    }
}

pub fn show_onboarding(app: &AppHandle) {
    if let Some(window) = app.get_webview_window(ONBOARDING) {
        let _ = window.center();
        let _ = window.show();
        let _ = window.set_focus();
    }
}

pub fn hide_onboarding(app: &AppHandle) {
    if let Some(window) = app.get_webview_window(ONBOARDING) {
        let _ = window.hide();
    }
}

pub fn show_popup_at_cursor(app: &AppHandle, width: f64, height: f64) {
    let scheduled_app = app.clone();
    if let Err(error) = app.clone().run_on_main_thread(move || {
        show_popup_at_cursor_on_main(&scheduled_app, width, height);
    }) {
        tracing::warn!(
            target: "yiyu_app::windows",
            %error,
            "cannot schedule popup display on the main thread"
        );
    }
}

fn show_popup_at_cursor_on_main(app: &AppHandle, width: f64, height: f64) {
    let Some(window) = app.get_webview_window(POPUP) else {
        return;
    };
    if let Ok(cursor) = window.cursor_position() {
        let monitor = app
            .monitor_from_point(cursor.x, cursor.y)
            .ok()
            .flatten()
            .or_else(|| window.current_monitor().ok().flatten());
        if let Some(monitor) = monitor {
            let scale = monitor.scale_factor();
            let monitor_position = monitor.position();
            let monitor_size = monitor.size();
            let placement = calculate_popup_placement(
                cursor.x,
                cursor.y,
                monitor_position.x,
                monitor_position.y,
                monitor_size.width,
                monitor_size.height,
                width,
                height,
                scale,
            );
            let _ = window.set_size(PhysicalSize::new(placement.width, placement.height));
            let _ = window.set_position(PhysicalPosition::new(placement.x, placement.y));
        }
    }

    #[cfg(target_os = "macos")]
    {
        use tauri_nspanel::ManagerExt;
        if let Ok(panel) = app.get_webview_panel(POPUP) {
            install_popup_dismiss_monitor(app);
            panel.order_front_regardless();
            return;
        }
    }
    let _ = window.show();
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct PopupPlacement {
    x: i32,
    y: i32,
    width: u32,
    height: u32,
}

#[allow(clippy::too_many_arguments)]
fn calculate_popup_placement(
    cursor_x: f64,
    cursor_y: f64,
    monitor_x: i32,
    monitor_y: i32,
    monitor_width: u32,
    monitor_height: u32,
    logical_width: f64,
    logical_height: f64,
    scale: f64,
) -> PopupPlacement {
    let logical_width = logical_width.clamp(POPUP_MIN_WIDTH, POPUP_MAX_WIDTH);
    let logical_height = logical_height.clamp(POPUP_MIN_HEIGHT, POPUP_MAX_HEIGHT);
    let width = (logical_width * scale).round() as i32;
    let height = (logical_height * scale).round() as i32;
    let offset = (18.0 * scale).round() as i32;
    let safe = (8.0 * scale).round() as i32;
    let mut x = cursor_x.round() as i32 + offset;
    let mut y = cursor_y.round() as i32 - height - offset;
    let left = monitor_x + safe;
    let top = monitor_y + safe;
    let right = monitor_x + monitor_width as i32 - safe;
    let bottom = monitor_y + monitor_height as i32 - safe;
    if x + width > right {
        x = cursor_x.round() as i32 - width - offset;
    }
    if y < top {
        y = cursor_y.round() as i32 + offset;
    }
    PopupPlacement {
        x: x.clamp(left, (right - width).max(left)),
        y: y.clamp(top, (bottom - height).max(top)),
        width: width as u32,
        height: height as u32,
    }
}

pub fn hide_popup(app: &AppHandle) {
    remove_popup_dismiss_monitor();
    if let Some(window) = app.get_webview_window(POPUP) {
        let _ = window.hide();
    }
}

#[cfg(target_os = "macos")]
fn install_popup_dismiss_monitor(app: &AppHandle) {
    use crate::state::AppState;
    use block2::RcBlock;
    use objc2_app_kit::{NSEvent, NSEventMask};
    use std::ptr::NonNull;
    use std::sync::Arc;
    use tauri::Manager;

    remove_popup_dismiss_monitor();
    let app = app.clone();
    let block = RcBlock::new(move |_event: NonNull<NSEvent>| {
        let Some(state) = app.try_state::<Arc<AppState>>() else {
            return;
        };
        if !state.popup_pinned() {
            let _ = state.translations.cancel_translation(POPUP);
            hide_popup(&app);
        }
    });
    let monitor = NSEvent::addGlobalMonitorForEventsMatchingMask_handler(
        NSEventMask::LeftMouseDown | NSEventMask::RightMouseDown,
        &block,
    );
    DISMISS_MONITOR.with(|slot| *slot.borrow_mut() = monitor);
}

#[cfg(target_os = "macos")]
fn remove_popup_dismiss_monitor() {
    use objc2_app_kit::NSEvent;

    DISMISS_MONITOR.with(|slot| {
        if let Some(monitor) = slot.borrow_mut().take() {
            unsafe { NSEvent::removeMonitor(&monitor) };
        }
    });
}

#[cfg(not(target_os = "macos"))]
fn remove_popup_dismiss_monitor() {}

pub fn resize_popup(app: &AppHandle, width: f64, height: f64) -> YiyuResult<()> {
    let window = app
        .get_webview_window(POPUP)
        .ok_or_else(|| YiyuError::new(ErrorKind::Other, "popup window is unavailable"))?;
    let width = width.clamp(POPUP_MIN_WIDTH, POPUP_MAX_WIDTH);
    let height = height.clamp(POPUP_MIN_HEIGHT, POPUP_MAX_HEIGHT);
    let scale = window.scale_factor().unwrap_or(1.0);
    let position = window
        .outer_position()
        .map_err(|error| YiyuError::new(ErrorKind::Other, error.to_string()))?;
    let size = window
        .outer_size()
        .map_err(|error| YiyuError::new(ErrorKind::Other, error.to_string()))?;
    let new_height = (height * scale).round() as i32;
    let y = position.y + size.height as i32 - new_height;
    window
        .set_size(PhysicalSize::new(
            (width * scale).round() as u32,
            new_height.max(1) as u32,
        ))
        .map_err(|error| YiyuError::new(ErrorKind::Other, error.to_string()))?;
    window
        .set_position(PhysicalPosition::new(position.x, y))
        .map_err(|error| YiyuError::new(ErrorKind::Other, error.to_string()))
}

pub fn hide_all(app: &AppHandle) {
    for label in [MAIN, SETTINGS, ONBOARDING, POPUP] {
        if let Some(window) = app.get_webview_window(label) {
            let _ = window.hide();
        }
    }
}

#[cfg(target_os = "macos")]
pub fn apply_main_vibrancy(app: &AppHandle) {
    use tauri::window::{Effect, EffectState, EffectsBuilder};
    if let Some(window) = app.get_webview_window(MAIN) {
        let effects = EffectsBuilder::new()
            .effect(Effect::UnderWindowBackground)
            .effect(Effect::HudWindow)
            .state(EffectState::Active)
            .build();
        let _ = window.set_effects(effects);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn popup_flips_and_clamps_at_the_right_bottom_edge() {
        let placement =
            calculate_popup_placement(1_430.0, 860.0, 0, 0, 1_440, 900, 380.0, 260.0, 1.0);
        assert_eq!(placement.width, 380);
        assert_eq!(placement.height, 260);
        assert!(placement.x >= 8);
        assert!(placement.x + placement.width as i32 <= 1_432);
        assert!(placement.y >= 8);
        assert!(placement.y + placement.height as i32 <= 892);
    }

    #[test]
    fn popup_respects_negative_monitor_coordinates_and_scale() {
        let placement =
            calculate_popup_placement(-1_900.0, 100.0, -1_920, 0, 1_920, 1_080, 320.0, 180.0, 2.0);
        assert_eq!(placement.width, 640);
        assert_eq!(placement.height, 360);
        assert!(placement.x >= -1_904);
        assert!(placement.x + placement.width as i32 <= -16);
        assert!(placement.y >= 16);
        assert!(placement.y + placement.height as i32 <= 1_064);
    }

    #[test]
    fn popup_clamps_oversized_requests_to_its_design_bounds() {
        let placement =
            calculate_popup_placement(100.0, 100.0, 0, 0, 1_440, 900, 1_200.0, 1_000.0, 1.0);
        assert_eq!(placement.width, POPUP_MAX_WIDTH as u32);
        assert_eq!(placement.height, POPUP_MAX_HEIGHT as u32);
    }

    #[test]
    fn startup_respects_onboarding_and_silent_preference() {
        assert_eq!(startup_window(false, false), Some(ONBOARDING));
        assert_eq!(startup_window(false, true), Some(ONBOARDING));
        assert_eq!(startup_window(true, false), Some(MAIN));
        assert_eq!(startup_window(true, true), None);
    }
}
