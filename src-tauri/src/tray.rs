use crate::{AppState, commands, windows};
use tauri::menu::{MenuBuilder, MenuItem, MenuItemBuilder};
use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, Manager};
use yiyu_core::settings::{Settings, ShortcutConfig};

const SHOW_MAIN: &str = "tray-show-main";
const TRANSLATE_SELECTION: &str = "tray-translate-selection";
const TRANSLATE_CLIPBOARD: &str = "tray-translate-clipboard";
const SHOW_SETTINGS: &str = "tray-settings";
const SHOW_ABOUT: &str = "tray-about";
const QUIT: &str = "tray-quit";

struct TrayShortcuts {
    selection: MenuItem<tauri::Wry>,
    clipboard: MenuItem<tauri::Wry>,
    main: MenuItem<tauri::Wry>,
}

pub fn sync_shortcuts(app: &AppHandle, shortcuts: &ShortcutConfig) -> tauri::Result<()> {
    if let Some(items) = app.try_state::<TrayShortcuts>() {
        items
            .selection
            .set_accelerator(shortcuts.selection.as_deref())?;
        items
            .clipboard
            .set_accelerator(shortcuts.clipboard.as_deref())?;
        items.main.set_accelerator(shortcuts.open_main.as_deref())?;
    }
    Ok(())
}

pub fn install(app: &AppHandle, settings: &Settings) -> tauri::Result<()> {
    let title = MenuItemBuilder::new("译羽").enabled(false).build(app)?;
    let show = MenuItemBuilder::with_id(SHOW_MAIN, "打开主窗口").build(app)?;
    let selection = MenuItemBuilder::with_id(TRANSLATE_SELECTION, "翻译选中文本").build(app)?;
    let clipboard = MenuItemBuilder::with_id(TRANSLATE_CLIPBOARD, "翻译剪贴板").build(app)?;
    let preferences = MenuItemBuilder::with_id(SHOW_SETTINGS, "偏好设置…")
        .accelerator("CmdOrCtrl+Comma")
        .build(app)?;
    let about = MenuItemBuilder::with_id(SHOW_ABOUT, "关于译羽").build(app)?;
    let quit = MenuItemBuilder::with_id(QUIT, "退出译羽")
        .accelerator("CmdOrCtrl+Q")
        .build(app)?;
    let menu = MenuBuilder::new(app)
        .item(&title)
        .separator()
        .items(&[&selection, &clipboard])
        .separator()
        .items(&[&show, &preferences, &about])
        .separator()
        .item(&quit)
        .build()?;
    app.manage(TrayShortcuts {
        selection,
        clipboard,
        main: show,
    });
    sync_shortcuts(app, &settings.shortcuts)?;

    let mut builder = TrayIconBuilder::with_id("yiyu-tray")
        .tooltip("译羽")
        .menu(&menu)
        .show_menu_on_left_click(true)
        .on_menu_event(|app, event| match event.id.as_ref() {
            SHOW_MAIN => windows::show_main(app),
            SHOW_SETTINGS => windows::show_settings(app),
            SHOW_ABOUT => windows::show_settings_section(app, "about"),
            TRANSLATE_SELECTION | TRANSLATE_CLIPBOARD => {
                if let Some(state) = app.try_state::<std::sync::Arc<AppState>>() {
                    let state = state.inner().clone();
                    let app = app.clone();
                    let intent = state.begin_request(windows::POPUP);
                    if event.id.as_ref() == TRANSLATE_SELECTION {
                        tauri::async_runtime::spawn(commands::capture_and_translate(
                            app,
                            state,
                            windows::POPUP,
                            intent,
                        ));
                    } else {
                        tauri::async_runtime::spawn(commands::translate_clipboard_and_show(
                            app, state, intent,
                        ));
                    }
                }
            }
            QUIT => {
                if let Some(state) = app.try_state::<std::sync::Arc<AppState>>() {
                    state.cancel_all();
                }
                app.exit(0);
            }
            _ => {}
        });
    if let Some(icon) = app.default_window_icon() {
        builder = builder.icon(icon.clone());
    }
    builder.build(app)?;
    Ok(())
}
