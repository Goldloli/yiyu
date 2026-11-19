use std::collections::HashMap;
use std::sync::Mutex;
use yiyu_core::history::{HistoryEntry, NewHistoryEntry};
use yiyu_core::{YiyuError, YiyuResult};

/// Holds only failed writes; successful retries are removed exactly once.
#[derive(Default)]
pub struct PendingHistory {
    entries: Mutex<HashMap<String, NewHistoryEntry>>,
}

impl PendingHistory {
    pub fn remember(&self, session_id: &str, entry: NewHistoryEntry) {
        self.entries
            .lock()
            .expect("history state poisoned")
            .insert(session_id.to_string(), entry);
    }

    pub fn retry<F>(&self, session_id: &str, save: F) -> YiyuResult<HistoryEntry>
    where
        F: FnOnce(&NewHistoryEntry) -> YiyuResult<HistoryEntry>,
    {
        let mut pending = self.entries.lock().expect("history state poisoned");
        let entry = pending.get(session_id).ok_or_else(|| {
            YiyuError::storage("there is no pending history write for this translation")
        })?;
        let saved = save(entry)?;
        pending.remove(session_id);
        Ok(saved)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use yiyu_core::history::HistoryStore;

    #[test]
    fn pending_history_retries_are_isolated_by_session() {
        let pending = PendingHistory::default();
        let history = HistoryStore::in_memory().unwrap();
        pending.remember(
            "main-1",
            NewHistoryEntry {
                source_text: "main text".into(),
                ..Default::default()
            },
        );
        pending.remember(
            "popup-1",
            NewHistoryEntry {
                source_text: "popup text".into(),
                ..Default::default()
            },
        );
        assert_eq!(
            pending
                .retry("main-1", |entry| history.insert(entry, 10))
                .unwrap()
                .source_text,
            "main text"
        );
        assert_eq!(
            pending
                .retry("popup-1", |entry| history.insert(entry, 10))
                .unwrap()
                .source_text,
            "popup text"
        );
        assert!(
            pending
                .retry("main-1", |entry| history.insert(entry, 10))
                .is_err()
        );
    }

    #[test]
    fn failed_retry_keeps_the_entry_for_a_later_attempt() {
        let pending = PendingHistory::default();
        let history = HistoryStore::in_memory().unwrap();
        pending.remember(
            "main-1",
            NewHistoryEntry {
                source_text: "keep me".into(),
                ..Default::default()
            },
        );
        assert!(
            pending
                .retry("main-1", |_| Err(YiyuError::storage(
                    "temporarily unavailable"
                )))
                .is_err()
        );
        assert_eq!(
            pending
                .retry("main-1", |entry| history.insert(entry, 10))
                .unwrap()
                .source_text,
            "keep me"
        );
    }

    #[test]
    fn concurrent_history_retries_insert_only_once() {
        let pending = Arc::new(PendingHistory::default());
        let history = Arc::new(HistoryStore::in_memory().unwrap());
        pending.remember(
            "same-session",
            NewHistoryEntry {
                source_text: "once".into(),
                ..Default::default()
            },
        );
        let a_pending = pending.clone();
        let b_pending = pending.clone();
        let a_history = history.clone();
        let b_history = history.clone();
        let a = std::thread::spawn(move || {
            a_pending.retry("same-session", |entry| a_history.insert(entry, 10))
        });
        let b = std::thread::spawn(move || {
            b_pending.retry("same-session", |entry| b_history.insert(entry, 10))
        });
        assert_ne!(a.join().unwrap().is_ok(), b.join().unwrap().is_ok());
        assert_eq!(history.list(&Default::default()).unwrap().len(), 1);
    }
}
