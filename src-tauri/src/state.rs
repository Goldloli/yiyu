use crate::credentials::CredentialStore;
use crate::translation_session::TranslationCoordinator;
use crate::update::UpdateManager;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use yiyu_core::history::{HistoryEntry, HistoryStore, NewHistoryEntry};
use yiyu_core::paths::AppPaths;
use yiyu_core::settings::SettingsService;
use yiyu_core::{YiyuError, YiyuResult};

pub struct AppState {
    pub paths: AppPaths,
    pub settings: Arc<SettingsService>,
    pub history: Arc<HistoryStore>,
    pub credentials: CredentialStore,
    pub update: Arc<UpdateManager>,
    pub backgrounds: crate::background::BackgroundManager,
    pub settings_update: tokio::sync::Mutex<()>,
    pub translations: TranslationCoordinator,
    pending_history: Mutex<HashMap<String, NewHistoryEntry>>,
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
            pending_history: Mutex::new(HashMap::new()),
            popup_pinned: Mutex::new(false),
            shortcut_registration_failed: Mutex::new(false),
        }))
    }

    pub fn remember_pending_history(&self, session_id: &str, entry: NewHistoryEntry) {
        self.pending_history
            .lock()
            .expect("history state poisoned")
            .insert(session_id.to_string(), entry);
    }

    pub fn retry_pending_history(&self, session_id: &str, limit: u32) -> YiyuResult<HistoryEntry> {
        let mut pending = self.pending_history.lock().expect("history state poisoned");
        let entry = pending.get(session_id).ok_or_else(|| {
            YiyuError::storage("there is no pending history write for this translation")
        })?;
        let saved = self.history.insert(entry, limit)?;
        pending.remove(session_id);
        Ok(saved)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pending_history_retries_are_isolated_by_session() {
        let temp = tempfile::tempdir().unwrap();
        let state = AppState::new(AppPaths::new(temp.path())).unwrap();
        state.remember_pending_history(
            "main-1",
            NewHistoryEntry {
                source_text: "main text".into(),
                ..Default::default()
            },
        );
        state.remember_pending_history(
            "popup-1",
            NewHistoryEntry {
                source_text: "popup text".into(),
                ..Default::default()
            },
        );
        assert_eq!(
            state
                .retry_pending_history("main-1", 10)
                .unwrap()
                .source_text,
            "main text"
        );
        assert_eq!(
            state
                .retry_pending_history("popup-1", 10)
                .unwrap()
                .source_text,
            "popup text"
        );
        assert!(state.retry_pending_history("main-1", 10).is_err());
    }

    #[test]
    fn concurrent_history_retries_insert_only_once() {
        let temp = tempfile::tempdir().unwrap();
        let state = AppState::new(AppPaths::new(temp.path())).unwrap();
        state.remember_pending_history(
            "same-session",
            NewHistoryEntry {
                source_text: "once".into(),
                ..Default::default()
            },
        );
        let first = state.clone();
        let second = state.clone();
        let a = std::thread::spawn(move || first.retry_pending_history("same-session", 10));
        let b = std::thread::spawn(move || second.retry_pending_history("same-session", 10));
        assert_ne!(a.join().unwrap().is_ok(), b.join().unwrap().is_ok());
        assert_eq!(state.history.list(&Default::default()).unwrap().len(), 1);
    }
}
