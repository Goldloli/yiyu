use crate::credentials::CredentialStore;
use crate::pending_history::PendingHistory;
use crate::translation_session::TranslationCoordinator;
use crate::update::UpdateManager;
use std::sync::{Arc, Mutex};
use yiyu_core::YiyuResult;
use yiyu_core::history::HistoryStore;
use yiyu_core::paths::AppPaths;
use yiyu_core::settings::SettingsService;

pub struct AppState {
    pub paths: AppPaths,
    pub settings: Arc<SettingsService>,
    pub history: Arc<HistoryStore>,
    pub credentials: CredentialStore,
    pub update: Arc<UpdateManager>,
    pub backgrounds: crate::background::BackgroundManager,
    pub settings_update: tokio::sync::Mutex<()>,
    pub translations: TranslationCoordinator,
    pub pending_history: PendingHistory,
    popup_pinned: Mutex<bool>,
    shortcut_registration_failed: Mutex<bool>,
}

impl AppState {
    pub fn new(paths: AppPaths) -> YiyuResult<Arc<Self>> {
        paths.ensure()?;
        Ok(Arc::new(Self {
            settings: SettingsService::new(&paths)?,
            history: Arc::new(HistoryStore::open(&paths)?),
            credentials: CredentialStore,
            update: Arc::new(UpdateManager::new(paths.clone())),
            backgrounds: crate::background::BackgroundManager::new(paths.root()),
            settings_update: tokio::sync::Mutex::new(()),
            paths,
            translations: TranslationCoordinator::default(),
            pending_history: PendingHistory::default(),
            popup_pinned: Mutex::new(false),
            shortcut_registration_failed: Mutex::new(false),
        }))
    }

    pub fn set_popup_pinned(&self, pinned: bool) {
        *self.popup_pinned.lock().expect("popup state poisoned") = pinned;
    }

    pub fn popup_pinned(&self) -> bool {
        *self.popup_pinned.lock().expect("popup state poisoned")
    }

    pub fn set_shortcut_registration_failed(&self, failed: bool) {
        *self
            .shortcut_registration_failed
            .lock()
            .expect("shortcut state poisoned") = failed;
    }

    pub fn shortcut_registration_failed(&self) -> bool {
        *self
            .shortcut_registration_failed
            .lock()
            .expect("shortcut state poisoned")
    }
}
