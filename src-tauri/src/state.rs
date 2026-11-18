use crate::credentials::CredentialStore;
use crate::update::UpdateManager;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tokio_util::sync::CancellationToken;
use yiyu_core::history::{HistoryEntry, HistoryStore, NewHistoryEntry};
use yiyu_core::paths::AppPaths;
use yiyu_core::settings::SettingsService;
use yiyu_core::{YiyuError, YiyuResult};

pub struct ActiveTranslation {
    pub session_id: String,
    pub cancel: CancellationToken,
}

pub struct AppState {
    pub paths: AppPaths,
    pub settings: Arc<SettingsService>,
    pub history: Arc<HistoryStore>,
    pub credentials: CredentialStore,
    pub update: Arc<UpdateManager>,
    pub backgrounds: crate::background::BackgroundManager,
    pub settings_update: tokio::sync::Mutex<()>,
    active: Mutex<HashMap<String, ActiveTranslation>>,
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
            active: Mutex::new(HashMap::new()),
            pending_history: Mutex::new(HashMap::new()),
            popup_pinned: Mutex::new(false),
            shortcut_registration_failed: Mutex::new(false),
        }))
    }

    pub fn begin_translation(&self, surface: &str, session_id: String) -> CancellationToken {
        let token = CancellationToken::new();
        let mut active = self.active.lock().expect("translation state poisoned");
        if let Some(previous) = active.insert(
            surface.to_string(),
            ActiveTranslation {
                session_id,
                cancel: token.clone(),
            },
        ) {
            previous.cancel.cancel();
        }
        token
    }

    pub fn finish_translation(&self, surface: &str, session_id: &str) {
        let mut active = self.active.lock().expect("translation state poisoned");
        remove_if_current(&mut active, surface, session_id);
    }

    pub fn claim_translation_completion(&self, surface: &str, session_id: &str) -> bool {
        let mut active = self.active.lock().expect("translation state poisoned");
        remove_if_current(&mut active, surface, session_id)
    }

    pub fn cancel_translation(&self, surface: &str) {
        let mut active = self.active.lock().expect("translation state poisoned");
        if let Some(entry) = active.remove(surface) {
            entry.cancel.cancel();
        }
    }

    pub fn cancel_all(&self) {
        let mut active = self.active.lock().expect("translation state poisoned");
        for (_, entry) in active.drain() {
            entry.cancel.cancel();
        }
    }

    pub fn is_active(&self, surface: &str, session_id: &str) -> bool {
        self.active
            .lock()
            .expect("translation state poisoned")
            .get(surface)
            .is_some_and(|entry| entry.session_id == session_id)
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

fn remove_if_current(
    active: &mut HashMap<String, ActiveTranslation>,
    surface: &str,
    session_id: &str,
) -> bool {
    if active
        .get(surface)
        .is_some_and(|entry| entry.session_id == session_id)
    {
        active.remove(surface);
        true
    } else {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn completion_claim_rejects_cancelled_or_superseded_sessions() {
        let mut active = HashMap::new();
        active.insert(
            "popup".into(),
            ActiveTranslation {
                session_id: "current".into(),
                cancel: CancellationToken::new(),
            },
        );
        assert!(remove_if_current(&mut active, "popup", "current"));
        assert!(!remove_if_current(&mut active, "popup", "stale"));
        assert!(active.is_empty());
    }

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
