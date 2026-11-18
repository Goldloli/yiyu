use crate::{commands, state::AppState};
use std::str::FromStr;
use std::sync::Arc;
use tauri::{AppHandle, Manager};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};
use yiyu_core::settings::Settings;
use yiyu_core::{ErrorKind, YiyuError, YiyuResult};

const SELECTION: &str = "selection";
const CLIPBOARD: &str = "clipboard";
const OPEN_MAIN: &str = "open-main";

pub fn reload(app: &AppHandle, settings: &Settings) -> YiyuResult<()> {
    validate_unique(settings)?;
    app.global_shortcut()
        .unregister_all()
        .map_err(shortcut_error)?;
    for (action, shortcut) in [
        (SELECTION, settings.shortcuts.selection.as_deref()),
        (CLIPBOARD, settings.shortcuts.clipboard.as_deref()),
        (OPEN_MAIN, settings.shortcuts.open_main.as_deref()),
    ] {
        let Some(shortcut) = shortcut.map(str::trim).filter(|value| !value.is_empty()) else {
            continue;
        };
        let action_owned = action.to_string();
        let registration =
            app.global_shortcut()
                .on_shortcut(shortcut, move |app, _shortcut, event| {
                    if event.state != ShortcutState::Pressed {
                        return;
                    }
                    tracing::info!(
                        target: "yiyu_app::shortcuts",
                        action = action_owned.as_str(),
                        "global shortcut pressed"
                    );
                    let state = app.state::<Arc<AppState>>().inner().clone();
                    match action_owned.as_str() {
                        SELECTION => {
                            tauri::async_runtime::spawn(commands::capture_and_translate(
                                app.clone(),
                                state,
                                crate::windows::POPUP,
                            ));
                        }
                        CLIPBOARD => {
                            tauri::async_runtime::spawn(commands::translate_clipboard_and_show(
                                app.clone(),
                                state,
                            ));
                        }
                        OPEN_MAIN => crate::windows::show_main(app),
                        _ => {}
                    }
                });
        if let Err(error) = registration {
            if let Err(cleanup_error) = app.global_shortcut().unregister_all() {
                tracing::warn!(target: "yiyu_app::shortcuts", %cleanup_error, "failed to clear partially registered shortcuts");
            }
            return Err(shortcut_error(error));
        }
        tracing::info!(
            target: "yiyu_app::shortcuts",
            action,
            shortcut,
            "registered global shortcut"
        );
    }
    Ok(())
}

pub fn validate_unique(settings: &Settings) -> YiyuResult<()> {
    let shortcuts = [
        (
            "shortcuts.selection",
            settings.shortcuts.selection.as_deref(),
        ),
        (
            "shortcuts.clipboard",
            settings.shortcuts.clipboard.as_deref(),
        ),
        (
            "shortcuts.openMain",
            settings.shortcuts.open_main.as_deref(),
        ),
    ];
    let mut seen = std::collections::HashSet::new();
    for (field, shortcut) in shortcuts
        .into_iter()
        .filter_map(|(field, value)| value.map(|value| (field, value.trim())))
        .filter(|(_, value)| !value.is_empty())
    {
        let parsed = Shortcut::from_str(shortcut).map_err(|error| {
            YiyuError::new(
                ErrorKind::Validation,
                format!("invalid global shortcut: {error}"),
            )
            .field(field)
        })?;
        if !seen.insert(parsed) {
            return Err(YiyuError::new(
                ErrorKind::Validation,
                "shortcut is assigned more than once",
            )
            .field(field));
        }
    }
    Ok(())
}

fn shortcut_error(error: impl std::fmt::Display) -> YiyuError {
    YiyuError::new(
        ErrorKind::Validation,
        format!("global shortcut error: {error}"),
    )
    .field("shortcuts")
}

#[cfg(test)]
mod tests {
    use super::*;
    use yiyu_core::settings::ShortcutConfig;

    #[test]
    fn duplicate_shortcuts_are_rejected() {
        let settings = Settings {
            shortcuts: ShortcutConfig {
                selection: Some("Alt+Shift+Y".into()),
                clipboard: Some("Alt+Shift+Y".into()),
                open_main: None,
            },
            ..Default::default()
        };
        assert!(validate_unique(&settings).is_err());
    }

    #[test]
    fn equivalent_shortcuts_with_different_modifier_order_are_rejected() {
        let settings = Settings {
            shortcuts: ShortcutConfig {
                selection: Some("Alt+Shift+Y".into()),
                clipboard: Some("Shift+Alt+Y".into()),
                open_main: None,
            },
            ..Default::default()
        };
        assert!(validate_unique(&settings).is_err());
    }

    #[test]
    fn invalid_shortcut_syntax_is_rejected_before_registration() {
        let settings = Settings {
            shortcuts: ShortcutConfig {
                selection: Some("Alt+Shift+NotAKey".into()),
                clipboard: None,
                open_main: None,
            },
            ..Default::default()
        };
        let error = validate_unique(&settings).unwrap_err();
        assert_eq!(error.field.as_deref(), Some("shortcuts.selection"));
    }
}
