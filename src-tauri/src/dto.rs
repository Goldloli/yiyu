use serde::{Deserialize, Serialize};
use yiyu_core::YiyuError;
use yiyu_core::history::HistoryEntry;
use yiyu_core::language::{Language, LanguageDirection};
use yiyu_core::provider::ProviderConfig;
use yiyu_core::settings::{Settings, SettingsSnapshot};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppInfoDTO {
    pub name: String,
    pub display_name: String,
    pub version: String,
    pub identifier: String,
    pub platform: String,
    pub arch: String,
    pub data_dir: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderViewDTO {
    #[serde(flatten)]
    pub config: ProviderConfig,
    pub configured: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BootstrapDTO {
    pub app: AppInfoDTO,
    pub settings: SettingsSnapshot,
    pub settings_recovery_backup: Option<String>,
    pub shortcut_registration_failed: bool,
    pub providers: Vec<ProviderViewDTO>,
    pub languages: Vec<Language>,
    pub history: Vec<HistoryEntry>,
    pub permissions: crate::capture::PermissionStatus,
    pub update: crate::update::UpdateSnapshotDTO,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SettingsEnvelopeDTO {
    pub revision: u64,
    pub settings: Settings,
    pub providers: Vec<ProviderViewDTO>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StartTranslationDTO {
    pub surface: String,
    pub text: String,
    pub provider_id: Option<String>,
    pub manual_source_language: Option<String>,
    pub target_language: Option<String>,
    pub fallback_target_language: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TranslationEventDTO {
    pub session_id: String,
    pub surface: String,
    pub state: String,
    pub source_text: Option<String>,
    pub text: Option<String>,
    pub direction: Option<LanguageDirection>,
    pub error: Option<YiyuError>,
    pub duration_ms: Option<u64>,
    pub history_saved: Option<bool>,
    pub persistence_error: Option<YiyuError>,
}

impl TranslationEventDTO {
    pub fn state(session_id: &str, surface: &str, state: &str) -> Self {
        Self {
            session_id: session_id.into(),
            surface: surface.into(),
            state: state.into(),
            source_text: None,
            text: None,
            direction: None,
            error: None,
            duration_ms: None,
            history_saved: None,
            persistence_error: None,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TranslationStartResultDTO {
    pub session_id: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryMutationDTO {
    pub changed: usize,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiagnosticsDTO {
    pub data_dir: String,
    pub logs_dir: String,
    pub version: String,
    pub platform: String,
}
