use super::{history, settings, system, updates};
use crate::state::AppState;
use tauri::test::{mock_builder, mock_context, noop_assets};
use yiyu_core::paths::AppPaths;
use yiyu_core::settings::{Settings, SettingsStore};

#[test]
fn moved_commands_keep_their_original_ipc_names_and_responses() {
    let temp = tempfile::tempdir().unwrap();
    let paths = AppPaths::new(temp.path());
    let provider = yiyu_core::provider::built_in_providers()
        .into_iter()
        .find(|provider| !provider.requires_api_key)
        .unwrap();
    let settings = Settings {
        active_provider_id: Some(provider.id.clone()),
        providers: vec![provider],
        ..Settings::default()
    };
    SettingsStore::new(&paths).save(&settings).unwrap();
    let state = AppState::new(paths).unwrap();
    let app = mock_builder()
        .manage(state)
        .invoke_handler(tauri::generate_handler![
            settings::get_settings,
            settings::retry_credential_cleanup,
            history::list_history,
            system::get_diagnostics,
            updates::get_update_snapshot
        ])
        .build(mock_context(noop_assets()))
        .unwrap();
    let webview = tauri::WebviewWindowBuilder::new(&app, "main", Default::default())
        .build()
        .unwrap();
    let invoke = |cmd: &str, body: serde_json::Value| {
        tauri::test::get_ipc_response(
            &webview,
            tauri::webview::InvokeRequest {
                cmd: cmd.into(),
                callback: tauri::ipc::CallbackFn(0),
                error: tauri::ipc::CallbackFn(1),
                url: "tauri://localhost".parse().unwrap(),
                body: tauri::ipc::InvokeBody::Json(body),
                headers: Default::default(),
                invoke_key: tauri::test::INVOKE_KEY.into(),
            },
        )
        .unwrap()
        .deserialize::<serde_json::Value>()
        .unwrap()
    };
    let settings = invoke("get_settings", serde_json::json!({}));
    assert_eq!(
        settings["settings"]["providers"].as_array().unwrap().len(),
        1
    );
    assert!(settings["revision"].is_number());
    assert_eq!(
        invoke("retry_credential_cleanup", serde_json::json!({}))["settings"]["pendingCredentialCleanup"],
        serde_json::json!([])
    );
    assert_eq!(
        invoke("list_history", serde_json::json!({ "filter": {} })),
        serde_json::json!([])
    );
    assert!(invoke("get_diagnostics", serde_json::json!({}))["version"].is_string());
    assert!(invoke("get_update_snapshot", serde_json::json!({})).is_object());
}
