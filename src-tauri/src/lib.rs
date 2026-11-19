mod background;
mod capture;
mod commands;
mod credential_cleanup;
mod credentials;
mod dto;
mod events;
mod logging;
#[cfg(target_os = "macos")]
mod macos_clipboard;
mod shortcuts;
mod startup_error;
mod state;
mod translation_flow;
mod translation_session;
mod tray;
mod update;
mod windows;
#[cfg(target_os = "windows")]
mod windows_chrome;

use state::AppState;
use std::sync::Arc;
use tauri::Manager;
use tauri_plugin_window_state::StateFlags;
use yiyu_core::paths::AppPaths;

fn show_primary_window(app: &tauri::AppHandle) {
    let state = app
        .try_state::<Arc<AppState>>()
        .map(|state| state.inner().clone());
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let onboarding_completed = match state {
            Some(state) => {
                state
                    .settings
                    .snapshot()
                    .await
                    .settings
                    .onboarding_completed
            }
            None => true,
        };
        if onboarding_completed {
            windows::show_main(&app);
        } else {
            windows::show_onboarding(&app);
        }
    });
}

pub fn run() {
    let paths = AppPaths::standard().expect("cannot resolve Yiyu data directory");
    paths
        .ensure()
        .expect("cannot initialize Yiyu data directory");
    logging::init(&paths.logs_dir()).expect("cannot initialize Yiyu logging");
    let state = match AppState::new(paths) {
        Ok(state) => state,
        Err(error)
            if error.kind == yiyu_core::ErrorKind::Unsupported
                && error.field.as_deref() == Some("version") =>
        {
            startup_error::show_upgrade_required();
            return;
        }
        Err(error) => panic!("cannot initialize Yiyu application state: {error}"),
    };

    let builder = tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            tracing::info!(target: "yiyu_app::lifecycle", "received another launch");
            show_primary_window(app);
        }))
        .plugin(
            tauri_plugin_window_state::Builder::default()
                .with_state_flags(StateFlags::POSITION | StateFlags::SIZE | StateFlags::MAXIMIZED)
                .build(),
        )
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .manage(state.clone());

    #[cfg(target_os = "macos")]
    let builder = builder.plugin(tauri_nspanel::init());

    builder
        .setup(move |app| {
            app.handle().plugin(
                tauri_plugin_autostart::Builder::new()
                    .app_name(app.config().identifier.clone())
                    .build(),
            )?;
            #[cfg(target_os = "macos")]
            {
                app.set_activation_policy(tauri::ActivationPolicy::Regular);
            }
            windows::install_all(app.handle())?;
            let snapshot = tauri::async_runtime::block_on(state.settings.snapshot());
            tray::install(app.handle(), &snapshot.settings)?;
            #[cfg(target_os = "macos")]
            windows::apply_main_vibrancy(app.handle());

            if let Err(error) = shortcuts::reload(app.handle(), &snapshot.settings) {
                state.set_shortcut_registration_failed(true);
                tracing::warn!(target: "yiyu_app::shortcuts", kind = ?error.kind, "global shortcut registration failed");
            }
            let startup_window = windows::startup_window(
                snapshot.settings.onboarding_completed,
                snapshot.settings.silent_start,
            );
            tracing::info!(target: "yiyu_app::lifecycle", window = ?startup_window, "startup window selected");
            match startup_window {
                Some(windows::ONBOARDING) => windows::show_onboarding(app.handle()),
                Some(windows::MAIN) => windows::show_main(app.handle()),
                _ => {}
            }
            if snapshot.settings.onboarding_completed {
                commands::updates::begin_daily_update_check(app.handle().clone(), state.clone());
            }
            Ok(())
        })
        .on_window_event(|window, event| {
            if matches!(event, tauri::WindowEvent::Focused(true))
                && matches!(window.label(), windows::SETTINGS | windows::ONBOARDING)
            {
                events::emit(
                    window.app_handle(),
                    events::PERMISSION_CHANGED,
                    capture::permission_status(),
                );
            }
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
                if window.label() == windows::POPUP
                    && let Some(state) = window.app_handle().try_state::<Arc<AppState>>()
                {
                    let _ = state.translations.cancel_translation(windows::POPUP);
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_bootstrap,
            commands::get_app_info,
            commands::settings::get_settings,
            commands::settings::patch_settings,
            commands::settings::get_background,
            commands::settings::set_background,
            commands::settings::clear_background,
            commands::settings::save_provider_secret,
            commands::settings::delete_provider_secret,
            commands::settings::retry_credential_cleanup,
            commands::settings::test_provider_connection,
            commands::settings::fetch_provider_models,
            commands::translation::translate_start,
            commands::translation::translate_cancel,
            commands::translation::capture_selection,
            commands::translation::translate_clipboard,
            commands::history::list_history,
            commands::history::delete_history,
            commands::history::clear_history,
            commands::history::retry_save_history,
            commands::system::copy_text,
            commands::system::open_external,
            commands::system::refresh_permissions,
            commands::system::request_accessibility_permission,
            commands::system::open_accessibility_settings,
            commands::system::get_autostart,
            commands::system::set_autostart,
            commands::system::open_main_window,
            commands::system::open_settings_window,
            commands::system::open_onboarding_window,
            commands::system::close_popup,
            commands::system::set_popup_pinned,
            commands::system::resize_popup_window,
            commands::system::quit_app,
            commands::updates::check_update,
            commands::updates::download_update,
            commands::updates::cancel_update_download,
            commands::updates::install_update,
            commands::updates::get_update_snapshot,
            commands::system::get_diagnostics
        ])
        .build(tauri::generate_context!())
        .expect("error while building Yiyu")
        .run(|_app, _event| {
            #[cfg(target_os = "macos")]
            if let tauri::RunEvent::Reopen {
                has_visible_windows: false,
                ..
            } = _event
            {
                tracing::info!(target: "yiyu_app::lifecycle", "received macOS reopen");
                show_primary_window(_app);
            }
        });
}
