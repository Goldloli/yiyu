use yiyu_core::YiyuResult;
use yiyu_core::settings::{Settings, SettingsPatch, SettingsService, SettingsSnapshot};

pub fn pending_after_provider_change(previous: &Settings, next: &Settings) -> Vec<String> {
    let exists = |id: &str| next.providers.iter().any(|provider| provider.id == id);
    let mut pending: Vec<String> = previous
        .pending_credential_cleanup
        .iter()
        .filter(|id| !exists(id))
        .cloned()
        .collect();
    for provider in &previous.providers {
        if !exists(&provider.id) && !pending.contains(&provider.id) {
            pending.push(provider.id.clone());
        }
    }
    pending
}

/// Save the provider change and cleanup intent together before touching credentials.
pub async fn persist_and_cleanup<F>(
    settings: &SettingsService,
    expected_revision: u64,
    patch: SettingsPatch,
    removed: bool,
    delete: F,
) -> YiyuResult<SettingsSnapshot>
where
    F: Fn(&str) -> YiyuResult<()>,
{
    let saved = settings.patch(expected_revision, patch).await?;
    if !removed {
        return Ok(saved);
    }
    match retry_pending_cleanup(settings, saved.clone(), delete).await {
        Ok(updated) => Ok(updated),
        Err(error) => {
            tracing::warn!(target: "yiyu_app::credentials", kind = ?error.kind, "credential cleanup state could not be saved");
            Ok(saved)
        }
    }
}

pub async fn retry_pending_cleanup<F>(
    settings: &SettingsService,
    snapshot: SettingsSnapshot,
    delete: F,
) -> YiyuResult<SettingsSnapshot>
where
    F: Fn(&str) -> YiyuResult<()>,
{
    let mut pending = Vec::new();
    for id in &snapshot.settings.pending_credential_cleanup {
        let provider_exists = snapshot
            .settings
            .providers
            .iter()
            .any(|provider| provider.id == *id);
        if !provider_exists && delete(id).is_err() {
            pending.push(id.clone());
        }
    }
    if pending == snapshot.settings.pending_credential_cleanup {
        return Ok(snapshot);
    }
    settings
        .patch(
            snapshot.revision,
            SettingsPatch {
                pending_credential_cleanup: Some(pending),
                ..Default::default()
            },
        )
        .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use yiyu_core::YiyuError;
    use yiyu_core::paths::AppPaths;

    #[tokio::test]
    async fn failed_cleanup_is_persisted_and_can_be_retried_after_restart() {
        let temp = tempfile::tempdir().unwrap();
        let paths = AppPaths::new(temp.path());
        let settings = SettingsService::new(&paths).unwrap();
        let original = settings
            .patch(
                0,
                SettingsPatch {
                    active_provider_id: Some(Some("openai".into())),
                    ..Default::default()
                },
            )
            .await
            .unwrap();
        let providers: Vec<_> = original
            .settings
            .providers
            .iter()
            .filter(|p| p.id != "deepseek")
            .cloned()
            .collect();
        let mut patch = SettingsPatch {
            providers: Some(providers),
            ..Default::default()
        };
        let next = original.settings.apply_patch(&patch);
        patch.pending_credential_cleanup =
            Some(pending_after_provider_change(&original.settings, &next));
        let saved = settings.patch(original.revision, patch).await.unwrap();
        let failed =
            retry_pending_cleanup(&settings, saved, |_| Err(YiyuError::storage("unavailable")))
                .await
                .unwrap();
        assert_eq!(failed.settings.pending_credential_cleanup, ["deepseek"]);

        let restarted = SettingsService::new(&paths).unwrap();
        let loaded = restarted.snapshot().await;
        assert_eq!(loaded.settings.pending_credential_cleanup, ["deepseek"]);
        let retried = retry_pending_cleanup(&restarted, loaded, |_| Ok(()))
            .await
            .unwrap();
        assert!(retried.settings.pending_credential_cleanup.is_empty());
        assert!(
            SettingsService::new(&paths)
                .unwrap()
                .snapshot()
                .await
                .settings
                .pending_credential_cleanup
                .is_empty()
        );
    }

    #[tokio::test]
    async fn readded_provider_is_never_deleted_by_an_old_cleanup_request() {
        let temp = tempfile::tempdir().unwrap();
        let paths = AppPaths::new(temp.path());
        let settings = SettingsService::new(&paths).unwrap();
        let saved = settings
            .patch(
                0,
                SettingsPatch {
                    active_provider_id: Some(Some("openai".into())),
                    pending_credential_cleanup: Some(vec!["openai".into()]),
                    ..Default::default()
                },
            )
            .await
            .unwrap();
        let calls = AtomicUsize::new(0);
        let clean = retry_pending_cleanup(&settings, saved, |_| {
            calls.fetch_add(1, Ordering::SeqCst);
            Ok(())
        })
        .await
        .unwrap();
        assert_eq!(calls.load(Ordering::SeqCst), 0);
        assert!(clean.settings.pending_credential_cleanup.is_empty());
    }

    #[tokio::test]
    async fn failed_settings_commit_never_deletes_credentials() {
        let temp = tempfile::tempdir().unwrap();
        let paths = AppPaths::new(temp.path());
        let settings = SettingsService::new(&paths).unwrap();
        let original = settings.snapshot().await;
        let mut patch = SettingsPatch {
            active_provider_id: Some(Some("openai".into())),
            providers: Some(
                original
                    .settings
                    .providers
                    .iter()
                    .filter(|p| p.id != "deepseek")
                    .cloned()
                    .collect(),
            ),
            ..Default::default()
        };
        let next = original.settings.apply_patch(&patch);
        patch.pending_credential_cleanup =
            Some(pending_after_provider_change(&original.settings, &next));
        std::fs::create_dir(paths.settings_file()).unwrap();
        let calls = AtomicUsize::new(0);
        assert!(
            persist_and_cleanup(&settings, original.revision, patch, true, |_| {
                calls.fetch_add(1, Ordering::SeqCst);
                Ok(())
            })
            .await
            .is_err()
        );
        assert_eq!(calls.load(Ordering::SeqCst), 0);
        assert_eq!(
            settings.snapshot().await.settings.providers.len(),
            original.settings.providers.len()
        );
    }

    #[tokio::test]
    async fn failed_queue_update_keeps_a_retryable_cleanup_intent() {
        let temp = tempfile::tempdir().unwrap();
        let paths = AppPaths::new(temp.path());
        let settings = SettingsService::new(&paths).unwrap();
        let original = settings.snapshot().await;
        let mut patch = SettingsPatch {
            active_provider_id: Some(Some("openai".into())),
            providers: Some(
                original
                    .settings
                    .providers
                    .iter()
                    .filter(|p| p.id != "deepseek")
                    .cloned()
                    .collect(),
            ),
            ..Default::default()
        };
        let next = original.settings.apply_patch(&patch);
        patch.pending_credential_cleanup =
            Some(pending_after_provider_change(&original.settings, &next));
        let saved = settings.patch(original.revision, patch).await.unwrap();
        let backup = temp.path().join("saved-settings.json");
        std::fs::rename(paths.settings_file(), &backup).unwrap();
        std::fs::create_dir(paths.settings_file()).unwrap();
        let calls = AtomicUsize::new(0);
        assert!(
            retry_pending_cleanup(&settings, saved, |_| {
                calls.fetch_add(1, Ordering::SeqCst);
                Ok(())
            })
            .await
            .is_err()
        );
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        assert_eq!(
            settings
                .snapshot()
                .await
                .settings
                .pending_credential_cleanup,
            ["deepseek"]
        );
        let on_disk = std::fs::read_to_string(backup).unwrap();
        assert!(on_disk.contains("deepseek"));
    }
}
