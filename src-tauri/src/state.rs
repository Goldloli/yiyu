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

#[derive(Clone)]
pub struct TranslationIntent {
    id: String,
    pub cancel: CancellationToken,
}

#[derive(Default)]
struct TranslationState {
    pending: HashMap<String, TranslationIntent>,
    active: HashMap<String, ActiveTranslation>,
}

pub struct AppState {
    pub paths: AppPaths,
    pub settings: Arc<SettingsService>,
    pub history: Arc<HistoryStore>,
    pub credentials: CredentialStore,
    pub update: Arc<UpdateManager>,
    pub backgrounds: crate::background::BackgroundManager,
    pub settings_update: tokio::sync::Mutex<()>,
    translations: Mutex<TranslationState>,
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
            translations: Mutex::new(TranslationState::default()),
            pending_history: Mutex::new(HashMap::new()),
            popup_pinned: Mutex::new(false),
            shortcut_registration_failed: Mutex::new(false),
        }))
    }

    pub fn begin_request(&self, surface: &str) -> TranslationIntent {
        let intent = TranslationIntent {
            id: uuid::Uuid::new_v4().to_string(),
            cancel: CancellationToken::new(),
        };
        let mut state = self
            .translations
            .lock()
            .expect("translation state poisoned");
        if let Some(previous) = state.pending.insert(surface.to_owned(), intent.clone()) {
            previous.cancel.cancel();
        }
        intent
    }

    pub fn begin_translation(
        &self,
        surface: &str,
        session_id: String,
        intent: &TranslationIntent,
    ) -> Option<CancellationToken> {
        let mut state = self
            .translations
            .lock()
            .expect("translation state poisoned");
        if intent.cancel.is_cancelled()
            || !state
                .pending
                .get(surface)
                .is_some_and(|entry| entry.id == intent.id)
        {
            return None;
        }
        let token = CancellationToken::new();
        if let Some(previous) = state.active.insert(
            surface.to_string(),
            ActiveTranslation {
                session_id,
                cancel: token.clone(),
            },
        ) {
            previous.cancel.cancel();
        }
        Some(token)
    }

    pub fn finish_request(&self, surface: &str, intent: &TranslationIntent) -> bool {
        let mut state = self
            .translations
            .lock()
            .expect("translation state poisoned");
        if state
            .pending
            .get(surface)
            .is_some_and(|entry| entry.id == intent.id)
        {
            state.pending.remove(surface);
            true
        } else {
            false
        }
    }

    pub fn fail_request(&self, surface: &str, intent: &TranslationIntent) -> bool {
        let mut state = self
            .translations
            .lock()
            .expect("translation state poisoned");
        if !state
            .pending
            .get(surface)
            .is_some_and(|entry| entry.id == intent.id)
        {
            return false;
        }
        state.pending.remove(surface);
        if let Some(active) = state.active.remove(surface) {
            active.cancel.cancel();
        }
        true
    }

    pub fn finish_translation(&self, surface: &str, session_id: &str) {
        let mut state = self
            .translations
            .lock()
            .expect("translation state poisoned");
        remove_if_current(&mut state.active, surface, session_id);
    }

    pub fn claim_translation_completion(&self, surface: &str, session_id: &str) -> bool {
        let mut state = self
            .translations
            .lock()
            .expect("translation state poisoned");
        remove_if_current(&mut state.active, surface, session_id)
    }

    pub fn cancel_translation(&self, surface: &str) -> Option<String> {
        let mut state = self
            .translations
            .lock()
            .expect("translation state poisoned");
        if let Some(entry) = state.pending.remove(surface) {
            entry.cancel.cancel();
        }
        if let Some(entry) = state.active.remove(surface) {
            entry.cancel.cancel();
            return Some(entry.session_id);
        }
        None
    }

    pub fn cancel_all(&self) {
        let mut state = self
            .translations
            .lock()
            .expect("translation state poisoned");
        for (_, entry) in state.pending.drain() {
            entry.cancel.cancel();
        }
        for (_, entry) in state.active.drain() {
            entry.cancel.cancel();
        }
    }

    pub fn is_active(&self, surface: &str, session_id: &str) -> bool {
        self.translations
            .lock()
            .expect("translation state poisoned")
            .active
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
    fn newer_request_cancels_pending_and_prevents_stale_translation() {
        let temp = tempfile::tempdir().unwrap();
        let state = AppState::new(AppPaths::new(temp.path())).unwrap();
        let old = state.begin_request("popup");
        let new = state.begin_request("popup");

        assert!(old.cancel.is_cancelled());
        assert!(
            state
                .begin_translation("popup", "old".into(), &old)
                .is_none()
        );
        let active = state
            .begin_translation("popup", "new".into(), &new)
            .unwrap();
        assert!(!active.is_cancelled());
        assert!(!state.finish_request("popup", &old));
        assert!(state.is_active("popup", "new"));
    }

    #[test]
    fn closing_popup_cancels_pending_capture_without_affecting_main() {
        let temp = tempfile::tempdir().unwrap();
        let state = AppState::new(AppPaths::new(temp.path())).unwrap();
        let popup = state.begin_request("popup");
        let main = state.begin_request("main");

        let _ = state.cancel_translation("popup");
        assert!(popup.cancel.is_cancelled());
        assert!(!main.cancel.is_cancelled());
        assert!(
            state
                .begin_translation("popup", "late".into(), &popup)
                .is_none()
        );
        assert!(
            state
                .begin_translation("main", "main".into(), &main)
                .is_some()
        );
    }

    #[test]
    fn starting_a_new_translation_cancels_the_previous_session() {
        let temp = tempfile::tempdir().unwrap();
        let state = AppState::new(AppPaths::new(temp.path())).unwrap();
        let first = state.begin_request("popup");
        let active = state
            .begin_translation("popup", "first".into(), &first)
            .unwrap();

        let second = state.begin_request("popup");
        assert!(!active.is_cancelled());
        assert!(
            state
                .begin_translation("popup", "second".into(), &second)
                .is_some()
        );
        assert!(active.is_cancelled());
        assert!(!state.is_active("popup", "first"));
    }

    #[test]
    fn stale_failure_cannot_cancel_a_newer_translation() {
        let temp = tempfile::tempdir().unwrap();
        let state = AppState::new(AppPaths::new(temp.path())).unwrap();
        let old = state.begin_request("popup");
        let current = state.begin_request("popup");
        let active = state
            .begin_translation("popup", "current".into(), &current)
            .unwrap();
        assert!(!state.fail_request("popup", &old));
        assert!(!active.is_cancelled());
        assert!(state.is_active("popup", "current"));
    }

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
