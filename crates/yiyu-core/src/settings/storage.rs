use super::{SETTINGS_VERSION, Settings, SettingsPatch, SettingsSnapshot};
use crate::paths::AppPaths;
use crate::{YiyuError, YiyuResult};
use atomic_write_file::AtomicWriteFile;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex as StdMutex};
use tokio::sync::{Mutex, RwLock};

pub struct SettingsStore {
    path: PathBuf,
    preserved: StdMutex<Option<PreservedDocument>>,
}

struct PreservedDocument {
    original: serde_json::Value,
    known: serde_json::Value,
}

// Retain fields this version doesn't understand without exposing them to the renderer.
// Known maps (such as extraHeaders) may still have entries deliberately removed.
fn preserve_unknown_fields(
    original: &serde_json::Value,
    known: &serde_json::Value,
    next: &mut serde_json::Value,
) {
    match (original, known, next) {
        (
            serde_json::Value::Object(original),
            serde_json::Value::Object(known),
            serde_json::Value::Object(next),
        ) => {
            for (key, value) in original {
                if let Some(known_value) = known.get(key) {
                    if let Some(next_value) = next.get_mut(key) {
                        preserve_unknown_fields(value, known_value, next_value);
                    }
                } else {
                    next.entry(key.clone()).or_insert_with(|| value.clone());
                }
            }
        }
        (
            serde_json::Value::Array(original),
            serde_json::Value::Array(known),
            serde_json::Value::Array(next),
        ) => {
            for (index, next_value) in next.iter_mut().enumerate() {
                let source_index = next_value
                    .get("id")
                    .and_then(serde_json::Value::as_str)
                    .and_then(|id| {
                        original.iter().position(|value| {
                            value.get("id").and_then(serde_json::Value::as_str) == Some(id)
                        })
                    })
                    .unwrap_or(index);
                if let (Some(original_value), Some(known_value)) =
                    (original.get(source_index), known.get(source_index))
                {
                    // A newly added identified item must not inherit another item's fields.
                    if next_value.get("id").is_some()
                        && original_value.get("id") != next_value.get("id")
                    {
                        continue;
                    }
                    preserve_unknown_fields(original_value, known_value, next_value);
                }
            }
        }
        _ => {}
    }
}

impl SettingsStore {
    pub fn new(paths: &AppPaths) -> Self {
        Self {
            path: paths.settings_file(),
            preserved: StdMutex::new(None),
        }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn load(&self) -> YiyuResult<Settings> {
        if !self.path.exists() {
            return Ok(Settings::default());
        }
        let bytes = fs::read(&self.path)?;
        let original: serde_json::Value = serde_json::from_slice(&bytes).map_err(|error| {
            YiyuError::new(
                crate::ErrorKind::Config,
                format!("settings file is invalid: {error}"),
            )
            .field("settings")
        })?;
        if original
            .get("version")
            .and_then(serde_json::Value::as_u64)
            .is_some_and(|version| version > u64::from(SETTINGS_VERSION))
        {
            return Err(YiyuError::new(crate::ErrorKind::Unsupported, "Settings were created by a newer version of Yiyu. Update the application to use them.").field("version"));
        }
        let settings: Settings = serde_json::from_value(original.clone()).map_err(|error| {
            YiyuError::new(
                crate::ErrorKind::Config,
                format!("settings file is invalid: {error}"),
            )
            .field("settings")
        })?;
        settings.validate().map_err(|issues| {
            YiyuError::new(
                crate::ErrorKind::Validation,
                issues
                    .iter()
                    .map(|issue| format!("{}: {}", issue.field, issue.message))
                    .collect::<Vec<_>>()
                    .join("; "),
            )
        })?;
        let known = serde_json::to_value(&settings)?;
        *self
            .preserved
            .lock()
            .map_err(|_| YiyuError::storage("settings state is unavailable"))? =
            Some(PreservedDocument { original, known });
        Ok(settings)
    }

    pub fn save(&self, settings: &Settings) -> YiyuResult<()> {
        settings.validate().map_err(|issues| {
            YiyuError::new(
                crate::ErrorKind::Validation,
                issues
                    .iter()
                    .map(|issue| format!("{}: {}", issue.field, issue.message))
                    .collect::<Vec<_>>()
                    .join("; "),
            )
        })?;
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)?;
        }
        let known = serde_json::to_value(settings)?;
        let mut original = known.clone();
        let mut preserved = self
            .preserved
            .lock()
            .map_err(|_| YiyuError::storage("settings state is unavailable"))?;
        if let Some(previous) = preserved.as_ref() {
            preserve_unknown_fields(&previous.original, &previous.known, &mut original);
        }
        let bytes = serde_json::to_vec_pretty(&original)?;
        let mut file = AtomicWriteFile::options().open(&self.path)?;
        file.write_all(&bytes)?;
        file.commit()?;
        *preserved = Some(PreservedDocument { original, known });
        Ok(())
    }

    fn backup_invalid(&self) -> YiyuResult<PathBuf> {
        let bytes = fs::read(&self.path)?;
        let backup = self
            .path
            .with_file_name(format!("settings.invalid-{}.json", uuid::Uuid::new_v4()));
        let mut options = fs::OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let mut file = options.open(&backup)?;
        file.write_all(&bytes)?;
        file.sync_all()?;
        Ok(backup)
    }
}

pub struct SettingsService {
    store: SettingsStore,
    recovery_backup: Option<PathBuf>,
    current: RwLock<SettingsSnapshot>,
    revision: AtomicU64,
    commit: Mutex<()>,
}

impl SettingsService {
    pub fn new(paths: &AppPaths) -> YiyuResult<Arc<Self>> {
        let store = SettingsStore::new(paths);
        let (settings, recovery_backup) = match store.load() {
            Ok(settings) => (settings, None),
            Err(error) if error.kind == crate::ErrorKind::Unsupported => return Err(error),
            Err(_) => (Settings::default(), Some(store.backup_invalid()?)),
        };
        Ok(Arc::new(Self {
            store,
            recovery_backup,
            current: RwLock::new(SettingsSnapshot {
                revision: 0,
                settings,
            }),
            revision: AtomicU64::new(0),
            commit: Mutex::new(()),
        }))
    }

    pub fn recovery_backup(&self) -> Option<&Path> {
        self.recovery_backup.as_deref()
    }

    pub async fn snapshot(&self) -> SettingsSnapshot {
        self.current.read().await.clone()
    }

    pub async fn patch(
        &self,
        expected_revision: u64,
        patch: SettingsPatch,
    ) -> YiyuResult<SettingsSnapshot> {
        let _guard = self.commit.lock().await;
        let current_revision = self.revision.load(Ordering::SeqCst);
        if expected_revision != current_revision {
            return Err(YiyuError::new(
                crate::ErrorKind::Validation,
                format!(
                    "settings revision mismatch: current {current_revision}, received {expected_revision}"
                ),
            )
            .field("revision"));
        }
        let current = self.current.read().await.clone();
        let next = current.settings.apply_patch(&patch);
        next.validate().map_err(|issues| {
            YiyuError::new(
                crate::ErrorKind::Validation,
                issues
                    .iter()
                    .map(|issue| format!("{}: {}", issue.field, issue.message))
                    .collect::<Vec<_>>()
                    .join("; "),
            )
        })?;
        self.store.save(&next)?;
        let revision = current_revision + 1;
        self.revision.store(revision, Ordering::SeqCst);
        let snapshot = SettingsSnapshot {
            revision,
            settings: next,
        };
        *self.current.write().await = snapshot.clone();
        Ok(snapshot)
    }
}
