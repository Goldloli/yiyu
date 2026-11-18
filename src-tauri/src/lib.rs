mod background;
mod capture;
mod commands;
mod credentials;
mod dto;
mod events;
mod logging;
mod shortcuts;
mod state;
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
    let state = AppState::new(paths).expect("cannot initialize Yiyu application state");

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
                commands::begin_daily_update_check(app.handle().clone(), state.clone());
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
                    state.cancel_translation(windows::POPUP);
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_bootstrap,
            commands::get_app_info,
            commands::get_settings,
            commands::patch_settings,
            commands::get_background,
            commands::set_background,
            commands::clear_background,
            commands::save_provider_secret,
            commands::delete_provider_secret,
            commands::test_provider_connection,
            commands::fetch_provider_models,
            commands::translate_start,
            commands::translate_cancel,
            commands::capture_selection,
            commands::translate_clipboard,
            commands::list_history,
            commands::delete_history,
            commands::clear_history,
            commands::retry_save_history,
            commands::copy_text,
            commands::open_external,
            commands::refresh_permissions,
            commands::request_accessibility_permission,
            commands::open_accessibility_settings,
            commands::get_autostart,
            commands::set_autostart,
            commands::open_main_window,
            commands::open_settings_window,
            commands::open_onboarding_window,
            commands::close_popup,
            commands::set_popup_pinned,
            commands::resize_popup_window,
            commands::quit_app,
            commands::check_update,
            commands::download_update,
            commands::cancel_update_download,
            commands::install_update,
            commands::get_update_snapshot,
            commands::get_diagnostics
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
