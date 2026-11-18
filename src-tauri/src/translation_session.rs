use std::collections::HashMap;
use std::sync::Mutex;
use tokio_util::sync::CancellationToken;

struct ActiveTranslation {
    session_id: String,
    cancel: CancellationToken,
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

#[derive(Default)]
pub struct TranslationCoordinator {
    inner: Mutex<TranslationState>,
}

impl TranslationCoordinator {
    pub fn begin_request(&self, surface: &str) -> TranslationIntent {
        let intent = TranslationIntent {
            id: uuid::Uuid::new_v4().to_string(),
            cancel: CancellationToken::new(),
        };
        let mut state = self.inner.lock().expect("translation state poisoned");
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
        let mut state = self.inner.lock().expect("translation state poisoned");
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
        let mut state = self.inner.lock().expect("translation state poisoned");
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
        let mut state = self.inner.lock().expect("translation state poisoned");
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
        let mut state = self.inner.lock().expect("translation state poisoned");
        remove_if_current(&mut state.active, surface, session_id);
    }

    pub fn claim_translation_completion(&self, surface: &str, session_id: &str) -> bool {
        let mut state = self.inner.lock().expect("translation state poisoned");
        remove_if_current(&mut state.active, surface, session_id)
    }

    pub fn cancel_translation(&self, surface: &str) -> Option<String> {
        let mut state = self.inner.lock().expect("translation state poisoned");
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
        let mut state = self.inner.lock().expect("translation state poisoned");
        for (_, entry) in state.pending.drain() {
            entry.cancel.cancel();
        }
        for (_, entry) in state.active.drain() {
            entry.cancel.cancel();
        }
    }

    pub fn is_active(&self, surface: &str, session_id: &str) -> bool {
        self.inner
            .lock()
            .expect("translation state poisoned")
            .active
            .get(surface)
            .is_some_and(|entry| entry.session_id == session_id)
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
        let state = TranslationCoordinator::default();
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
        let state = TranslationCoordinator::default();
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
        let state = TranslationCoordinator::default();
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
        let state = TranslationCoordinator::default();
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
    fn shutdown_cancels_pending_and_active_work_on_all_surfaces() {
        let state = TranslationCoordinator::default();
        let popup = state.begin_request("popup");
        let popup_active = state
            .begin_translation("popup", "popup-1".into(), &popup)
            .unwrap();
        assert!(state.finish_request("popup", &popup));
        let main = state.begin_request("main");
        let main_active = state
            .begin_translation("main", "main-1".into(), &main)
            .unwrap();
        let next_popup = state.begin_request("popup");

        state.cancel_all();

        assert!(next_popup.cancel.is_cancelled());
        assert!(main.cancel.is_cancelled());
        assert!(popup_active.is_cancelled());
        assert!(main_active.is_cancelled());
        assert!(!state.claim_translation_completion("popup", "popup-1"));
        assert!(!state.is_active("main", "main-1"));
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
}
