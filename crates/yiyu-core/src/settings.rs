use crate::paths::AppPaths;
use crate::provider::{ProviderConfig, built_in_providers};
use crate::{YiyuError, YiyuResult};
use atomic_write_file::AtomicWriteFile;
use serde::{Deserialize, Deserializer, Serialize};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use tokio::sync::{Mutex, RwLock};

pub const SETTINGS_VERSION: u32 = 1;
pub const DEFAULT_HISTORY_LIMIT: u32 = 500;
pub const DEFAULT_MAX_INPUT_CHARS: u32 = 20_000;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ThemePreference {
    #[default]
    System,
    Light,
    Dark,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MotionPreference {
    #[default]
    System,
    Reduce,
    Normal,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum UiLanguage {
    #[default]
    System,
    Zh,
    En,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ProxyMode {
    #[default]
    System,
    Direct,
    Manual,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ProxyConfig {
    pub mode: ProxyMode,
    pub url: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ShortcutConfig {
    pub selection: Option<String>,
    pub clipboard: Option<String>,
    pub open_main: Option<String>,
}

impl Default for ShortcutConfig {
    fn default() -> Self {
        Self {
            selection: Some("Alt+Shift+Y".into()),
            clipboard: Some("Alt+Shift+V".into()),
            open_main: Some("Alt+Shift+Space".into()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    pub version: u32,
    pub onboarding_completed: bool,
    pub providers: Vec<ProviderConfig>,
    pub active_provider_id: Option<String>,
    pub source_language: String,
    pub target_language: String,
    pub fallback_target_language: String,
    pub custom_target_language: Option<String>,
    pub system_prompt: Option<String>,
    pub shortcuts: ShortcutConfig,
    pub capture_clipboard_fallback: bool,
    pub auto_save_history: bool,
    pub history_limit: u32,
    pub max_input_chars: u32,
    pub provider_proxy: ProxyConfig,
    pub update_proxy: ProxyConfig,
    pub check_updates: bool,
    pub theme: ThemePreference,
    pub motion: MotionPreference,
    pub ui_language: UiLanguage,
    pub window_glass: bool,
    pub silent_start: bool,
    pub background_id: Option<String>,
    // None represents a legacy configuration: retain its custom image if present.
    pub background_choice: Option<String>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            version: SETTINGS_VERSION,
            onboarding_completed: false,
            providers: built_in_providers(),
            active_provider_id: None,
            source_language: "auto".into(),
            target_language: "zh-Hans".into(),
            fallback_target_language: "en".into(),
            custom_target_language: None,
            system_prompt: None,
            shortcuts: ShortcutConfig::default(),
            capture_clipboard_fallback: true,
            auto_save_history: true,
            history_limit: DEFAULT_HISTORY_LIMIT,
            max_input_chars: DEFAULT_MAX_INPUT_CHARS,
            provider_proxy: ProxyConfig::default(),
            update_proxy: ProxyConfig::default(),
            check_updates: true,
            theme: ThemePreference::default(),
            motion: MotionPreference::default(),
            ui_language: UiLanguage::default(),
            window_glass: true,
            silent_start: false,
            background_id: None,
            background_choice: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ValidationIssue {
    pub field: String,
    pub message: String,
}

impl Settings {
    pub fn active_provider(&self) -> Option<&ProviderConfig> {
        self.active_provider_id
            .as_deref()
            .and_then(|id| self.providers.iter().find(|provider| provider.id == id))
    }

    pub fn effective_target_language(&self) -> String {
        self.custom_target_language
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .unwrap_or(&self.target_language)
            .to_string()
    }

    pub fn validate(&self) -> Result<(), Vec<ValidationIssue>> {
        let mut issues = Vec::new();
        if self.background_choice.as_deref().is_some_and(|choice| {
            !matches!(
                choice,
                "none" | "alpine-lake" | "spring-valley" | "coast" | "custom"
            )
        }) {
            issues.push(ValidationIssue {
                field: "backgroundChoice".into(),
                message: "invalid background choice".into(),
            });
        }
        if self
            .background_id
            .as_deref()
            .is_some_and(|id| uuid::Uuid::parse_str(id).is_err())
        {
            issues.push(ValidationIssue {
                field: "backgroundId".into(),
                message: "invalid background identifier".into(),
            });
        }
        if self.version != SETTINGS_VERSION {
            issues.push(ValidationIssue {
                field: "version".into(),
                message: format!("unsupported settings version: {}", self.version),
            });
        }
        if !(10..=10_000).contains(&self.history_limit) {
            issues.push(ValidationIssue {
                field: "historyLimit".into(),
                message: "must be between 10 and 10000".into(),
            });
        }
        if !(1_000..=200_000).contains(&self.max_input_chars) {
            issues.push(ValidationIssue {
                field: "maxInputChars".into(),
                message: "must be between 1000 and 200000".into(),
            });
        }
        if self.target_language.trim().is_empty() {
            issues.push(ValidationIssue {
                field: "targetLanguage".into(),
                message: "target language is required".into(),
            });
        }
        if self.fallback_target_language.trim().is_empty() {
            issues.push(ValidationIssue {
                field: "fallbackTargetLanguage".into(),
                message: "fallback target language is required".into(),
            });
        }
        if self.active_provider_id.is_none() {
            issues.push(ValidationIssue {
                field: "activeProviderId".into(),
                message: "an active provider is required".into(),
            });
        } else if self.active_provider().is_none() {
            issues.push(ValidationIssue {
                field: "activeProviderId".into(),
                message: "active provider does not exist".into(),
            });
        }

        let mut ids = std::collections::HashSet::new();
        for (index, provider) in self.providers.iter().enumerate() {
            if provider.id.trim().is_empty() || !ids.insert(provider.id.clone()) {
                issues.push(ValidationIssue {
                    field: format!("providers.{index}.id"),
                    message: "provider id must be non-empty and unique".into(),
                });
            }
            if provider.label.trim().is_empty() {
                issues.push(ValidationIssue {
                    field: format!("providers.{index}.label"),
                    message: "provider label is required".into(),
                });
            }
            let valid_url = url::Url::parse(provider.base_url.trim())
                .map(|url| matches!(url.scheme(), "http" | "https"))
                .unwrap_or(false);
            if !valid_url {
                issues.push(ValidationIssue {
                    field: format!("providers.{index}.baseUrl"),
                    message: "base URL must be http(s)".into(),
                });
            }
            if provider.model.trim().is_empty() {
                issues.push(ValidationIssue {
                    field: format!("providers.{index}.model"),
                    message: "model is required".into(),
                });
            }
        }

        validate_proxy("providerProxy", &self.provider_proxy, &mut issues);
        validate_proxy("updateProxy", &self.update_proxy, &mut issues);

        let shortcuts = [
            ("shortcuts.selection", self.shortcuts.selection.as_deref()),
            ("shortcuts.clipboard", self.shortcuts.clipboard.as_deref()),
            ("shortcuts.openMain", self.shortcuts.open_main.as_deref()),
        ];
        let mut seen_shortcuts = std::collections::HashSet::new();
        for (field, shortcut) in shortcuts {
            if let Some(value) = shortcut.filter(|value| !value.trim().is_empty()) {
                let normalized = value.trim().to_ascii_lowercase();
                if !seen_shortcuts.insert(normalized) {
                    issues.push(ValidationIssue {
                        field: field.into(),
                        message: "shortcut is assigned more than once".into(),
                    });
                }
            }
        }

        if issues.is_empty() {
            Ok(())
        } else {
            Err(issues)
        }
    }
}

fn validate_proxy(field: &str, proxy: &ProxyConfig, issues: &mut Vec<ValidationIssue>) {
    match proxy.mode {
        ProxyMode::Manual => {
            let Some(value) = proxy
                .url
                .as_deref()
                .map(str::trim)
                .filter(|v| !v.is_empty())
            else {
                issues.push(ValidationIssue {
                    field: format!("{field}.url"),
                    message: "manual proxy URL is required".into(),
                });
                return;
            };
            if !(value.starts_with("http://")
                || value.starts_with("https://")
                || value.starts_with("socks5://")
                || value.starts_with("socks5h://"))
            {
                issues.push(ValidationIssue {
                    field: format!("{field}.url"),
                    message: "proxy must use http, https, socks5 or socks5h".into(),
                });
            }
        }
        ProxyMode::System | ProxyMode::Direct => {}
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SettingsSnapshot {
    pub revision: u64,
    pub settings: Settings,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct SettingsPatch {
    pub onboarding_completed: Option<bool>,
    pub providers: Option<Vec<ProviderConfig>>,
    #[serde(
        default,
        deserialize_with = "deserialize_nullable_patch",
        skip_serializing_if = "Option::is_none"
    )]
    pub active_provider_id: Option<Option<String>>,
    pub source_language: Option<String>,
    pub target_language: Option<String>,
    pub fallback_target_language: Option<String>,
    #[serde(
        default,
        deserialize_with = "deserialize_nullable_patch",
        skip_serializing_if = "Option::is_none"
    )]
    pub custom_target_language: Option<Option<String>>,
    #[serde(
        default,
        deserialize_with = "deserialize_nullable_patch",
        skip_serializing_if = "Option::is_none"
    )]
    pub system_prompt: Option<Option<String>>,
    pub shortcuts: Option<ShortcutConfig>,
    pub capture_clipboard_fallback: Option<bool>,
    pub auto_save_history: Option<bool>,
    pub history_limit: Option<u32>,
    pub max_input_chars: Option<u32>,
    pub provider_proxy: Option<ProxyConfig>,
    pub update_proxy: Option<ProxyConfig>,
    pub check_updates: Option<bool>,
    pub theme: Option<ThemePreference>,
    pub motion: Option<MotionPreference>,
    pub ui_language: Option<UiLanguage>,
    pub window_glass: Option<bool>,
    pub silent_start: Option<bool>,
    pub background_choice: Option<String>,
    // Background changes are only accepted through the managed-resource commands.
    #[serde(skip_deserializing)]
    pub background_id: Option<Option<String>>,
}

fn deserialize_nullable_patch<'de, D, T>(deserializer: D) -> Result<Option<Option<T>>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    Option::<T>::deserialize(deserializer).map(Some)
}

impl Settings {
    pub fn apply_patch(&self, patch: &SettingsPatch) -> Settings {
        let mut next = self.clone();
        macro_rules! replace {
            ($field:ident) => {
                if let Some(value) = patch.$field.clone() {
                    next.$field = value;
                }
            };
        }
        replace!(onboarding_completed);
        replace!(providers);
        if let Some(value) = patch.active_provider_id.clone() {
            next.active_provider_id = value;
        }
        replace!(source_language);
        replace!(target_language);
        replace!(fallback_target_language);
        if let Some(value) = patch.custom_target_language.clone() {
            next.custom_target_language = value;
        }
        if let Some(value) = patch.system_prompt.clone() {
            next.system_prompt = value;
        }
        replace!(shortcuts);
        replace!(capture_clipboard_fallback);
        replace!(auto_save_history);
        replace!(history_limit);
        replace!(max_input_chars);
        replace!(provider_proxy);
        replace!(update_proxy);
        replace!(check_updates);
        replace!(theme);
        replace!(motion);
        replace!(ui_language);
        replace!(window_glass);
        replace!(silent_start);
        if let Some(value) = patch.background_choice.clone() {
            next.background_choice = Some(value);
        }
        if let Some(value) = patch.background_id.clone() {
            next.background_id = value;
        }
        next
    }
}

pub struct SettingsStore {
    path: PathBuf,
}

impl SettingsStore {
    pub fn new(paths: &AppPaths) -> Self {
        Self {
            path: paths.settings_file(),
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
        let settings: Settings = serde_json::from_slice(&bytes).map_err(|error| {
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
        let bytes = serde_json::to_vec_pretty(settings)?;
        let mut file = AtomicWriteFile::options().open(&self.path)?;
        file.write_all(&bytes)?;
        file.commit()?;
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

#[cfg(test)]
mod tests {
    use super::*;

    fn valid_settings() -> Settings {
        Settings {
            active_provider_id: Some("openai".into()),
            ..Default::default()
        }
    }

    #[test]
    fn defaults_are_valid_once_a_provider_is_selected() {
        assert!(valid_settings().validate().is_ok());
    }

    #[test]
    fn nullable_patch_fields_distinguish_null_from_omission() {
        let settings = Settings {
            custom_target_language: Some("Esperanto".into()),
            system_prompt: Some("Translate faithfully".into()),
            ..valid_settings()
        };
        let patch: SettingsPatch = serde_json::from_value(serde_json::json!({
            "customTargetLanguage": null,
            "systemPrompt": null
        }))
        .unwrap();
        assert_eq!(patch.custom_target_language, Some(None));
        assert_eq!(patch.system_prompt, Some(None));
        assert_eq!(patch.active_provider_id, None);
        let updated = settings.apply_patch(&patch);
        assert_eq!(updated.custom_target_language, None);
        assert_eq!(updated.system_prompt, None);
        assert_eq!(updated.active_provider_id, settings.active_provider_id);
    }

    #[tokio::test]
    async fn corrupt_settings_are_backed_up_before_new_settings_are_saved() {
        let temp = tempfile::tempdir().unwrap();
        let paths = AppPaths::new(temp.path().join("app"));
        paths.ensure().unwrap();
        let original = b"{broken, preserve this file";
        fs::write(paths.settings_file(), original).unwrap();
        let service = SettingsService::new(&paths).unwrap();
        service
            .patch(
                0,
                SettingsPatch {
                    active_provider_id: Some(Some("openai".into())),
                    ..Default::default()
                },
            )
            .await
            .unwrap();
        let backups = fs::read_dir(paths.root())
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .filter(|path| {
                path.file_name()
                    .unwrap()
                    .to_string_lossy()
                    .starts_with("settings.invalid-")
            })
            .collect::<Vec<_>>();
        assert_eq!(backups.len(), 1);
        assert_eq!(fs::read(&backups[0]).unwrap(), original);
    }

    #[test]
    fn old_settings_default_to_no_background_and_external_patch_cannot_set_it() {
        let mut value = serde_json::to_value(valid_settings()).unwrap();
        value.as_object_mut().unwrap().remove("backgroundId");
        let old: Settings = serde_json::from_value(value).unwrap();
        assert_eq!(old.background_id, None);
        let id = uuid::Uuid::new_v4().to_string();
        let patch: SettingsPatch =
            serde_json::from_value(serde_json::json!({"backgroundId": id})).unwrap();
        assert_eq!(old.apply_patch(&patch).background_id, None);
        let next = old.apply_patch(&SettingsPatch {
            background_id: Some(Some(id.clone())),
            ..Default::default()
        });
        assert_eq!(serde_json::to_value(next).unwrap()["backgroundId"], id);
    }

    #[test]
    fn background_choice_preserves_legacy_image_and_can_be_disabled() {
        let id = uuid::Uuid::new_v4().to_string();
        let mut encoded = serde_json::to_value(valid_settings()).unwrap();
        encoded["backgroundId"] = serde_json::json!(id);
        encoded.as_object_mut().unwrap().remove("backgroundChoice");
        let old: Settings = serde_json::from_value(encoded).unwrap();
        assert_eq!(old.background_choice, None);
        assert_eq!(old.background_id.as_deref(), Some(id.as_str()));

        let none = old.apply_patch(&SettingsPatch {
            background_choice: Some("none".into()),
            ..Default::default()
        });
        assert_eq!(none.background_id.as_deref(), Some(id.as_str()));
        assert_eq!(none.background_choice.as_deref(), Some("none"));
        assert!(none.validate().is_ok());
        let mut invalid = none;
        invalid.background_choice = Some("../../other-file".into());
        assert!(
            invalid
                .validate()
                .unwrap_err()
                .iter()
                .any(|issue| issue.field == "backgroundChoice")
        );
    }

    #[test]
    fn existing_settings_default_to_visible_start_and_silent_start_persists() {
        let mut value = serde_json::to_value(valid_settings()).unwrap();
        value.as_object_mut().unwrap().remove("silentStart");
        let old: Settings = serde_json::from_value(value).unwrap();
        assert!(!old.silent_start);
        let next = old.apply_patch(&SettingsPatch {
            silent_start: Some(true),
            ..Default::default()
        });
        let reloaded: Settings =
            serde_json::from_value(serde_json::to_value(next).unwrap()).unwrap();
        assert!(reloaded.silent_start);
    }

    #[test]
    fn invalid_ranges_are_reported_by_field() {
        let mut settings = valid_settings();
        settings.history_limit = 3;
        settings.max_input_chars = 300_000;
        let issues = settings.validate().unwrap_err();
        assert!(issues.iter().any(|issue| issue.field == "historyLimit"));
        assert!(issues.iter().any(|issue| issue.field == "maxInputChars"));
    }

    #[test]
    fn duplicate_shortcuts_are_rejected() {
        let mut settings = valid_settings();
        settings.shortcuts.clipboard = settings.shortcuts.selection.clone();
        let issues = settings.validate().unwrap_err();
        assert!(
            issues
                .iter()
                .any(|issue| issue.field == "shortcuts.clipboard")
        );
    }

    #[test]
    fn atomic_store_round_trips() {
        let temp = tempfile::tempdir().unwrap();
        let paths = AppPaths::new(temp.path().join("app"));
        paths.ensure().unwrap();
        let store = SettingsStore::new(&paths);
        let settings = valid_settings();
        store.save(&settings).unwrap();
        assert_eq!(store.load().unwrap(), settings);
    }

    #[test]
    fn corrupt_file_is_not_reported_as_valid() {
        let temp = tempfile::tempdir().unwrap();
        let paths = AppPaths::new(temp.path().join("app"));
        paths.ensure().unwrap();
        fs::write(paths.settings_file(), b"{broken").unwrap();
        let store = SettingsStore::new(&paths);
        assert!(store.load().is_err());
        assert!(paths.settings_file().exists());
    }

    #[tokio::test]
    async fn stale_revision_cannot_overwrite_new_settings() {
        let temp = tempfile::tempdir().unwrap();
        let paths = AppPaths::new(temp.path().join("app"));
        paths.ensure().unwrap();
        let service = SettingsService::new(&paths).unwrap();
        service
            .patch(
                0,
                SettingsPatch {
                    active_provider_id: Some(Some("openai".into())),
                    ..Default::default()
                },
            )
            .await
            .unwrap();
        let result = service
            .patch(
                0,
                SettingsPatch {
                    target_language: Some("ja".into()),
                    ..Default::default()
                },
            )
            .await;
        assert!(result.is_err());
        assert_eq!(service.snapshot().await.settings.target_language, "zh-Hans");
    }

    #[tokio::test]
    async fn future_revision_cannot_bypass_concurrency_checks() {
        let temp = tempfile::tempdir().unwrap();
        let paths = AppPaths::new(temp.path().join("app"));
        paths.ensure().unwrap();
        let service = SettingsService::new(&paths).unwrap();
        let before = service.snapshot().await;
        let result = service
            .patch(
                99,
                SettingsPatch {
                    active_provider_id: Some(Some("openai".into())),
                    ..Default::default()
                },
            )
            .await;
        assert!(result.is_err());
        assert_eq!(service.snapshot().await, before);
    }

    #[tokio::test]
    async fn persistence_failure_keeps_the_last_valid_runtime_settings() {
        let temp = tempfile::tempdir().unwrap();
        let paths = AppPaths::new(temp.path().join("app"));
        paths.ensure().unwrap();
        let service = SettingsService::new(&paths).unwrap();
        let before = service.snapshot().await;
        fs::create_dir(paths.settings_file()).unwrap();
        let result = service
            .patch(
                0,
                SettingsPatch {
                    active_provider_id: Some(Some("openai".into())),
                    ..Default::default()
                },
            )
            .await;
        assert!(result.is_err());
        assert_eq!(service.snapshot().await, before);
    }
}
