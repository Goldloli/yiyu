use crate::YiyuResult;
use crate::paths::AppPaths;
use rusqlite::{Connection, OptionalExtension, named_params, params};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

const SCHEMA_VERSION: i32 = 1;
const MIGRATION_001: &str = r#"
CREATE TABLE IF NOT EXISTS translation_history (
    id TEXT PRIMARY KEY,
    source_text TEXT NOT NULL,
    translated_text TEXT NOT NULL,
    source_language TEXT NOT NULL,
    target_language TEXT NOT NULL,
    provider_id TEXT NOT NULL,
    provider_label TEXT NOT NULL,
    model TEXT NOT NULL,
    favorite INTEGER NOT NULL DEFAULT 0,
    duration_ms INTEGER NOT NULL,
    created_at INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_history_created_at
    ON translation_history(created_at DESC);
CREATE INDEX IF NOT EXISTS idx_history_favorite_created_at
    ON translation_history(favorite DESC, created_at DESC);
"#;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryEntry {
    pub id: String,
    pub source_text: String,
    pub translated_text: String,
    pub source_language: String,
    pub target_language: String,
    pub provider_id: String,
    pub provider_label: String,
    pub model: String,
    pub favorite: bool,
    pub duration_ms: u64,
    pub created_at: u64,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NewHistoryEntry {
    pub source_text: String,
    pub translated_text: String,
    pub source_language: String,
    pub target_language: String,
    pub provider_id: String,
    pub provider_label: String,
    pub model: String,
    pub duration_ms: u64,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct HistoryFilter {
    pub query: Option<String>,
    pub favorite_only: bool,
    pub limit: Option<u32>,
    pub offset: Option<u32>,
}

pub struct HistoryStore {
    connection: Mutex<Connection>,
    path: PathBuf,
}

impl std::fmt::Debug for HistoryStore {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("HistoryStore")
            .field("path", &self.path)
            .finish()
    }
}

impl HistoryStore {
    pub fn open(paths: &AppPaths) -> YiyuResult<Self> {
        paths.ensure()?;
        Self::from_path(paths.history_file())
    }

    pub fn from_path(path: impl Into<PathBuf>) -> YiyuResult<Self> {
        let path = path.into();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let connection = Connection::open(&path)?;
        let store = Self {
            connection: Mutex::new(connection),
            path,
        };
        store.configure()?;
        store.migrate()?;
        Ok(store)
    }

    pub fn in_memory() -> YiyuResult<Self> {
        let connection = Connection::open_in_memory()?;
        let store = Self {
            connection: Mutex::new(connection),
            path: PathBuf::from(":memory:"),
        };
        store.configure()?;
        store.migrate()?;
        Ok(store)
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    fn configure(&self) -> YiyuResult<()> {
        let connection = self.connection.lock().expect("history lock poisoned");
        connection.pragma_update(None, "journal_mode", "WAL")?;
        connection.pragma_update(None, "busy_timeout", 5_000)?;
        connection.pragma_update(None, "synchronous", "NORMAL")?;
        Ok(())
    }

    fn migrate(&self) -> YiyuResult<i32> {
        let mut connection = self.connection.lock().expect("history lock poisoned");
        let mut version =
            connection.pragma_query_value(None, "user_version", |row| row.get::<_, i32>(0))?;
        while version < SCHEMA_VERSION {
            let transaction = connection.transaction()?;
            if version == 0 {
                transaction.execute_batch(MIGRATION_001)?;
            }
            version += 1;
            transaction.pragma_update(None, "user_version", version)?;
            transaction.commit()?;
        }
        Ok(version)
    }

    pub fn insert(&self, entry: &NewHistoryEntry, limit: u32) -> YiyuResult<HistoryEntry> {
        let id = uuid::Uuid::new_v4().to_string();
        let created_at = now_millis();
        let mut connection = self.connection.lock().expect("history lock poisoned");
        let transaction = connection.transaction()?;
        let changed = transaction.execute(
            "INSERT INTO translation_history
             (id, source_text, translated_text, source_language, target_language,
              provider_id, provider_label, model, favorite, duration_ms, created_at)
             VALUES (:id, :source, :translated, :source_lang, :target_lang,
                     :provider_id, :provider_label, :model, 0, :duration, :created_at)",
            named_params![
                ":id": id,
                ":source": entry.source_text,
                ":translated": entry.translated_text,
                ":source_lang": entry.source_language,
                ":target_lang": entry.target_language,
                ":provider_id": entry.provider_id,
                ":provider_label": entry.provider_label,
                ":model": entry.model,
                ":duration": entry.duration_ms as i64,
                ":created_at": created_at as i64,
            ],
        )?;
        if changed != 1 {
            return Err(crate::YiyuError::storage(
                "history insert did not affect exactly one row",
            ));
        }
        prune_with_connection(&transaction, limit)?;
        transaction.commit()?;
        Ok(HistoryEntry {
            id,
            source_text: entry.source_text.clone(),
            translated_text: entry.translated_text.clone(),
            source_language: entry.source_language.clone(),
            target_language: entry.target_language.clone(),
            provider_id: entry.provider_id.clone(),
            provider_label: entry.provider_label.clone(),
            model: entry.model.clone(),
            favorite: false,
            duration_ms: entry.duration_ms,
            created_at,
        })
    }

    pub fn get(&self, id: &str) -> YiyuResult<Option<HistoryEntry>> {
        let connection = self.connection.lock().expect("history lock poisoned");
        connection
            .query_row(
                "SELECT id, source_text, translated_text, source_language, target_language,
                        provider_id, provider_label, model, favorite, duration_ms, created_at
                 FROM translation_history WHERE id = ?1",
                [id],
                map_entry,
            )
            .optional()
            .map_err(Into::into)
    }

    pub fn list(&self, filter: &HistoryFilter) -> YiyuResult<Vec<HistoryEntry>> {
        let connection = self.connection.lock().expect("history lock poisoned");
        let query = filter
            .query
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty());
        let limit = filter.limit.unwrap_or(200).clamp(1, 1_000);
        let offset = filter.offset.unwrap_or(0);
        let mut statement = connection.prepare(
            "SELECT id, source_text, translated_text, source_language, target_language,
                    provider_id, provider_label, model, favorite, duration_ms, created_at
             FROM translation_history
             WHERE (?1 IS NULL OR source_text LIKE '%' || ?1 || '%'
                    OR translated_text LIKE '%' || ?1 || '%'
                    OR provider_label LIKE '%' || ?1 || '%'
                    OR model LIKE '%' || ?1 || '%')
               AND (?2 = 0 OR favorite = 1)
             ORDER BY created_at DESC
             LIMIT ?3 OFFSET ?4",
        )?;
        let rows = statement.query_map(
            params![query, filter.favorite_only, limit, offset],
            map_entry,
        )?;
        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

    pub fn set_favorite(&self, id: &str, favorite: bool, limit: u32) -> YiyuResult<bool> {
        let mut connection = self.connection.lock().expect("history lock poisoned");
        let transaction = connection.transaction()?;
        let changed = transaction.execute(
            "UPDATE translation_history SET favorite = ?1 WHERE id = ?2",
            params![favorite, id],
        )?;
        prune_with_connection(&transaction, limit)?;
        transaction.commit()?;
        Ok(changed > 0)
    }

    pub fn delete(&self, id: &str) -> YiyuResult<bool> {
        let connection = self.connection.lock().expect("history lock poisoned");
        Ok(connection.execute("DELETE FROM translation_history WHERE id = ?1", [id])? > 0)
    }

    pub fn clear_regular(&self) -> YiyuResult<usize> {
        let connection = self.connection.lock().expect("history lock poisoned");
        Ok(connection.execute("DELETE FROM translation_history WHERE favorite = 0", [])?)
    }

    pub fn clear_all(&self) -> YiyuResult<usize> {
        let connection = self.connection.lock().expect("history lock poisoned");
        Ok(connection.execute("DELETE FROM translation_history", [])?)
    }

    pub fn prune(&self, limit: u32) -> YiyuResult<usize> {
        let connection = self.connection.lock().expect("history lock poisoned");
        prune_with_connection(&connection, limit)
    }
}

fn prune_with_connection(connection: &Connection, limit: u32) -> YiyuResult<usize> {
    let regular_count: i64 = connection.query_row(
        "SELECT COUNT(*) FROM translation_history WHERE favorite = 0",
        [],
        |row| row.get(0),
    )?;
    let excess = regular_count.saturating_sub(limit as i64);
    if excess <= 0 {
        return Ok(0);
    }
    Ok(connection.execute(
        "DELETE FROM translation_history
             WHERE id IN (
               SELECT id FROM translation_history
               WHERE favorite = 0
               ORDER BY created_at ASC, rowid ASC
               LIMIT ?1
             )",
        [excess],
    )?)
}

fn map_entry(row: &rusqlite::Row<'_>) -> rusqlite::Result<HistoryEntry> {
    Ok(HistoryEntry {
        id: row.get(0)?,
        source_text: row.get(1)?,
        translated_text: row.get(2)?,
        source_language: row.get(3)?,
        target_language: row.get(4)?,
        provider_id: row.get(5)?,
        provider_label: row.get(6)?,
        model: row.get(7)?,
        favorite: row.get::<_, i64>(8)? != 0,
        duration_ms: row.get::<_, i64>(9)? as u64,
        created_at: row.get::<_, i64>(10)? as u64,
    })
}

fn now_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(source: &str, translated: &str, provider: &str) -> NewHistoryEntry {
        NewHistoryEntry {
            source_text: source.into(),
            translated_text: translated.into(),
            source_language: "English".into(),
            target_language: "简体中文".into(),
            provider_id: provider.into(),
            provider_label: provider.into(),
            model: "test-model".into(),
            duration_ms: 42,
        }
    }

    #[test]
    fn insert_prune_and_favorite_are_independent() {
        let store = HistoryStore::in_memory().unwrap();
        let first = store.insert(&entry("one", "一", "openai"), 1).unwrap();
        store.set_favorite(&first.id, true, 1).unwrap();
        let second = store.insert(&entry("two", "二", "openai"), 1).unwrap();
        let third = store.insert(&entry("three", "三", "openai"), 1).unwrap();
        let items = store.list(&HistoryFilter::default()).unwrap();
        assert!(items.iter().any(|item| item.id == first.id));
        assert_eq!(
            items.iter().filter(|item| item.favorite).count(),
            1,
            "favorite must survive pruning"
        );
        assert!(store.get(&second.id).unwrap().is_none());
        assert!(store.get(&third.id).unwrap().is_some());
    }

    #[test]
    fn failed_pruning_rolls_back_the_new_history_record() {
        let store = HistoryStore::in_memory().unwrap();
        let first = store.insert(&entry("one", "一", "local"), 1).unwrap();
        store
            .connection
            .lock()
            .unwrap()
            .execute_batch(
                "CREATE TRIGGER fail_prune BEFORE DELETE ON translation_history
             BEGIN SELECT RAISE(FAIL, 'simulated pruning failure'); END;",
            )
            .unwrap();
        assert!(store.insert(&entry("two", "二", "local"), 1).is_err());
        let items = store.list(&HistoryFilter::default()).unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].id, first.id);
    }

    #[test]
    fn search_and_clear_regular_preserve_favorites() {
        let store = HistoryStore::in_memory().unwrap();
        let first = store
            .insert(&entry("hello world", "你好", "deepseek"), 100)
            .unwrap();
        store.set_favorite(&first.id, true, 100).unwrap();
        store
            .insert(&entry("apple", "苹果", "openai"), 100)
            .unwrap();
        let filter = HistoryFilter {
            query: Some("world".into()),
            ..Default::default()
        };
        assert_eq!(store.list(&filter).unwrap().len(), 1);
        assert_eq!(store.clear_regular().unwrap(), 1);
        assert_eq!(store.list(&HistoryFilter::default()).unwrap().len(), 1);
        assert!(store.get(&first.id).unwrap().unwrap().favorite);
    }

    #[test]
    fn legacy_favorites_are_visible_in_the_regular_history_and_clearable() {
        let store = HistoryStore::in_memory().unwrap();
        let first = store
            .insert(&entry("legacy", "旧记录", "local"), 100)
            .unwrap();
        store.set_favorite(&first.id, true, 100).unwrap();
        let filter: HistoryFilter = serde_json::from_str(r#"{"query":null,"limit":200}"#).unwrap();
        assert_eq!(store.list(&filter).unwrap().len(), 1);
        assert_eq!(store.clear_all().unwrap(), 1);
        assert!(store.list(&filter).unwrap().is_empty());
    }

    #[test]
    fn migration_is_idempotent() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("history.sqlite");
        let store = HistoryStore::from_path(&path).unwrap();
        drop(store);
        let reopened = HistoryStore::from_path(&path).unwrap();
        assert_eq!(reopened.list(&HistoryFilter::default()).unwrap().len(), 0);
    }
}
