use rusqlite::{Connection, params};

use crate::domain::error::AppError;
use crate::domain::settings::AppSettings;
use crate::domain::types::{
    DictionaryEntry, DictionaryScope, HistoryPage, Mode, Segment, SessionDetail, SessionSummary,
};

/// SQLiteストレージ（sessions + segments）
pub struct Storage {
    conn: Connection,
}

impl Storage {
    /// 新規接続（ファイルパス指定）
    pub fn open(path: &str) -> Result<Self, AppError> {
        let conn = Connection::open(path)
            .map_err(|e| AppError::storage(format!("DB接続に失敗: {e}")))?;
        let storage = Self { conn };
        storage.migrate()?;
        Ok(storage)
    }

    /// in-memory DB（テスト用）
    pub fn open_in_memory() -> Result<Self, AppError> {
        let conn = Connection::open_in_memory()
            .map_err(|e| AppError::storage(format!("in-memory DB作成に失敗: {e}")))?;
        let storage = Self { conn };
        storage.migrate()?;
        Ok(storage)
    }

    /// スキーママイグレーション
    fn migrate(&self) -> Result<(), AppError> {
        self.conn
            .execute_batch(
                "
                CREATE TABLE IF NOT EXISTS sessions (
                    session_id TEXT PRIMARY KEY,
                    state      TEXT NOT NULL DEFAULT 'idle',
                    mode       TEXT NOT NULL,
                    created_at TEXT NOT NULL,
                    updated_at TEXT NOT NULL
                );

                CREATE TABLE IF NOT EXISTS segments (
                    segment_id     TEXT PRIMARY KEY,
                    session_id     TEXT NOT NULL,
                    raw_text       TEXT NOT NULL DEFAULT '',
                    rewritten_text TEXT,
                    confidence     REAL NOT NULL DEFAULT 0.0,
                    audio_path     TEXT,
                    created_at     TEXT NOT NULL,
                    FOREIGN KEY (session_id) REFERENCES sessions(session_id)
                );

                CREATE INDEX IF NOT EXISTS idx_segments_session
                    ON segments(session_id);
                CREATE INDEX IF NOT EXISTS idx_sessions_created
                    ON sessions(created_at DESC);

                CREATE TABLE IF NOT EXISTS dictionary_entries (
                    id          TEXT PRIMARY KEY,
                    scope       TEXT NOT NULL DEFAULT 'global',
                    mode        TEXT,
                    pattern     TEXT NOT NULL,
                    replacement TEXT NOT NULL,
                    priority    INTEGER NOT NULL DEFAULT 0,
                    enabled     INTEGER NOT NULL DEFAULT 1
                );

                CREATE INDEX IF NOT EXISTS idx_dict_scope
                    ON dictionary_entries(scope, enabled);

                CREATE TABLE IF NOT EXISTS settings (
                    key   TEXT PRIMARY KEY,
                    value TEXT NOT NULL
                );
                ",
            )
            .map_err(|e| AppError::storage(format!("マイグレーション失敗: {e}")))?;
        Ok(())
    }

    // --- Sessions ---

    pub fn insert_session(
        &self,
        session_id: &str,
        mode: Mode,
        now: &str,
    ) -> Result<(), AppError> {
        let mode_str = serde_json::to_value(mode)
            .map_err(|e| AppError::internal(format!("mode serialize: {e}")))?;
        self.conn
            .execute(
                "INSERT INTO sessions (session_id, state, mode, created_at, updated_at) VALUES (?1, 'idle', ?2, ?3, ?3)",
                params![session_id, mode_str.as_str().unwrap_or("raw"), now],
            )
            .map_err(|e| AppError::storage(format!("セッション挿入失敗: {e}")))?;
        Ok(())
    }

    pub fn update_session_state(
        &self,
        session_id: &str,
        state: &str,
        now: &str,
    ) -> Result<(), AppError> {
        self.conn
            .execute(
                "UPDATE sessions SET state = ?1, updated_at = ?2 WHERE session_id = ?3",
                params![state, now, session_id],
            )
            .map_err(|e| AppError::storage(format!("セッション状態更新失敗: {e}")))?;
        Ok(())
    }

    // --- Segments ---

    pub fn insert_segment(
        &self,
        segment_id: &str,
        session_id: &str,
        now: &str,
    ) -> Result<(), AppError> {
        self.conn
            .execute(
                "INSERT INTO segments (segment_id, session_id, created_at) VALUES (?1, ?2, ?3)",
                params![segment_id, session_id, now],
            )
            .map_err(|e| AppError::storage(format!("セグメント挿入失敗: {e}")))?;
        Ok(())
    }

    pub fn update_segment_text(
        &self,
        segment_id: &str,
        raw_text: &str,
        confidence: f32,
    ) -> Result<(), AppError> {
        self.conn
            .execute(
                "UPDATE segments SET raw_text = ?1, confidence = ?2 WHERE segment_id = ?3",
                params![raw_text, confidence, segment_id],
            )
            .map_err(|e| AppError::storage(format!("セグメントテキスト更新失敗: {e}")))?;
        Ok(())
    }

    pub fn update_segment_rewritten(
        &self,
        segment_id: &str,
        rewritten_text: &str,
    ) -> Result<(), AppError> {
        self.conn
            .execute(
                "UPDATE segments SET rewritten_text = ?1 WHERE segment_id = ?2",
                params![rewritten_text, segment_id],
            )
            .map_err(|e| AppError::storage(format!("セグメントリライト更新失敗: {e}")))?;
        Ok(())
    }

    // --- Dictionary ---

    pub fn upsert_dictionary_entry(&self, entry: &DictionaryEntry) -> Result<String, AppError> {
        let id = entry
            .id
            .clone()
            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
        let scope_str = match entry.scope {
            DictionaryScope::Global => "global",
            DictionaryScope::Mode => "mode",
        };
        let mode_str = entry.mode.map(|m| {
            serde_json::to_value(m)
                .ok()
                .and_then(|v| v.as_str().map(|s| s.to_string()))
                .unwrap_or_default()
        });

        self.conn
            .execute(
                "INSERT INTO dictionary_entries (id, scope, mode, pattern, replacement, priority, enabled)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
                 ON CONFLICT(id) DO UPDATE SET
                    scope = excluded.scope,
                    mode = excluded.mode,
                    pattern = excluded.pattern,
                    replacement = excluded.replacement,
                    priority = excluded.priority,
                    enabled = excluded.enabled",
                params![
                    id,
                    scope_str,
                    mode_str,
                    entry.pattern,
                    entry.replacement,
                    entry.priority,
                    entry.enabled as i32,
                ],
            )
            .map_err(|e| AppError::storage(format!("辞書エントリ保存失敗: {e}")))?;

        Ok(id)
    }

    pub fn list_dictionary_entries(
        &self,
        scope: Option<&str>,
    ) -> Result<Vec<DictionaryEntry>, AppError> {
        let mut stmt;
        let entries: Vec<DictionaryEntry>;

        if let Some(scope_filter) = scope {
            stmt = self
                .conn
                .prepare(
                    "SELECT id, scope, mode, pattern, replacement, priority, enabled
                     FROM dictionary_entries
                     WHERE scope = ?1
                     ORDER BY priority DESC",
                )
                .map_err(|e| AppError::storage(format!("クエリ準備失敗: {e}")))?;
            entries = stmt
                .query_map(params![scope_filter], |row| Self::map_dict_row(row))
                .map_err(|e| AppError::storage(format!("クエリ実行失敗: {e}")))?
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| AppError::storage(format!("行読み取り失敗: {e}")))?;
        } else {
            stmt = self
                .conn
                .prepare(
                    "SELECT id, scope, mode, pattern, replacement, priority, enabled
                     FROM dictionary_entries
                     ORDER BY priority DESC",
                )
                .map_err(|e| AppError::storage(format!("クエリ準備失敗: {e}")))?;
            entries = stmt
                .query_map([], |row| Self::map_dict_row(row))
                .map_err(|e| AppError::storage(format!("クエリ実行失敗: {e}")))?
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| AppError::storage(format!("行読み取り失敗: {e}")))?;
        }

        Ok(entries)
    }

    pub fn get_enabled_dictionary_entries(
        &self,
        scope: &str,
        mode: Option<&str>,
    ) -> Result<Vec<DictionaryEntry>, AppError> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT id, scope, mode, pattern, replacement, priority, enabled
                 FROM dictionary_entries
                 WHERE enabled = 1
                   AND (scope = 'global' OR (scope = ?1 AND (mode IS NULL OR mode = ?2)))
                 ORDER BY priority DESC",
            )
            .map_err(|e| AppError::storage(format!("クエリ準備失敗: {e}")))?;

        let entries = stmt
            .query_map(params![scope, mode.unwrap_or("")], |row| {
                Self::map_dict_row(row)
            })
            .map_err(|e| AppError::storage(format!("クエリ実行失敗: {e}")))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| AppError::storage(format!("行読み取り失敗: {e}")))?;

        Ok(entries)
    }

    pub fn delete_dictionary_entry(&self, id: &str) -> Result<bool, AppError> {
        let affected = self
            .conn
            .execute(
                "DELETE FROM dictionary_entries WHERE id = ?1",
                params![id],
            )
            .map_err(|e| AppError::storage(format!("辞書エントリ削除失敗: {e}")))?;
        Ok(affected > 0)
    }

    fn map_dict_row(row: &rusqlite::Row) -> rusqlite::Result<DictionaryEntry> {
        let scope_str: String = row.get(1)?;
        let mode_str: Option<String> = row.get(2)?;
        let enabled_int: i32 = row.get(6)?;

        Ok(DictionaryEntry {
            id: Some(row.get(0)?),
            scope: match scope_str.as_str() {
                "mode" => DictionaryScope::Mode,
                _ => DictionaryScope::Global,
            },
            mode: mode_str.as_deref().map(parse_mode),
            pattern: row.get(3)?,
            replacement: row.get(4)?,
            priority: row.get(5)?,
            enabled: enabled_int != 0,
        })
    }

    // --- Queries ---

    pub fn list_history(
        &self,
        limit: u32,
        cursor: Option<&str>,
    ) -> Result<HistoryPage, AppError> {
        let mut stmt;
        let rows: Vec<SessionSummary>;

        if let Some(cursor_ts) = cursor {
            stmt = self
                .conn
                .prepare(
                    "SELECT s.session_id, s.state, s.mode, s.created_at, s.updated_at,
                            (SELECT COUNT(*) FROM segments seg WHERE seg.session_id = s.session_id) as seg_count
                     FROM sessions s
                     WHERE s.created_at < ?1
                     ORDER BY s.created_at DESC
                     LIMIT ?2",
                )
                .map_err(|e| AppError::storage(format!("クエリ準備失敗: {e}")))?;
            rows = stmt
                .query_map(params![cursor_ts, limit + 1], |row| {
                    Ok(SessionSummary {
                        session_id: row.get(0)?,
                        state: row.get(1)?,
                        mode: parse_mode(row.get::<_, String>(2)?.as_str()),
                        created_at: row.get(3)?,
                        updated_at: row.get(4)?,
                        segment_count: row.get(5)?,
                    })
                })
                .map_err(|e| AppError::storage(format!("クエリ実行失敗: {e}")))?
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| AppError::storage(format!("行読み取り失敗: {e}")))?;
        } else {
            stmt = self
                .conn
                .prepare(
                    "SELECT s.session_id, s.state, s.mode, s.created_at, s.updated_at,
                            (SELECT COUNT(*) FROM segments seg WHERE seg.session_id = s.session_id) as seg_count
                     FROM sessions s
                     ORDER BY s.created_at DESC
                     LIMIT ?1",
                )
                .map_err(|e| AppError::storage(format!("クエリ準備失敗: {e}")))?;
            rows = stmt
                .query_map(params![limit + 1], |row| {
                    Ok(SessionSummary {
                        session_id: row.get(0)?,
                        state: row.get(1)?,
                        mode: parse_mode(row.get::<_, String>(2)?.as_str()),
                        created_at: row.get(3)?,
                        updated_at: row.get(4)?,
                        segment_count: row.get(5)?,
                    })
                })
                .map_err(|e| AppError::storage(format!("クエリ実行失敗: {e}")))?
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| AppError::storage(format!("行読み取り失敗: {e}")))?;
        }

        let has_next = rows.len() > limit as usize;
        let items: Vec<SessionSummary> = rows.into_iter().take(limit as usize).collect();
        let next_cursor = if has_next {
            items.last().map(|s| s.created_at.clone())
        } else {
            None
        };

        Ok(HistoryPage { items, next_cursor })
    }

    pub fn get_session_detail(&self, session_id: &str) -> Result<Option<SessionDetail>, AppError> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT session_id, state, mode, created_at FROM sessions WHERE session_id = ?1",
            )
            .map_err(|e| AppError::storage(format!("クエリ準備失敗: {e}")))?;

        let session = stmt
            .query_row(params![session_id], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                ))
            })
            .ok();

        let Some((sid, state, mode_str, created_at)) = session else {
            return Ok(None);
        };

        let mut seg_stmt = self
            .conn
            .prepare(
                "SELECT segment_id, session_id, raw_text, rewritten_text, confidence, created_at
                 FROM segments WHERE session_id = ?1 ORDER BY created_at",
            )
            .map_err(|e| AppError::storage(format!("クエリ準備失敗: {e}")))?;

        let segments = seg_stmt
            .query_map(params![session_id], |row| {
                Ok(Segment {
                    segment_id: row.get(0)?,
                    session_id: row.get(1)?,
                    raw_text: row.get(2)?,
                    rewritten_text: row.get(3)?,
                    confidence: row.get(4)?,
                    created_at: row.get(5)?,
                })
            })
            .map_err(|e| AppError::storage(format!("クエリ実行失敗: {e}")))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| AppError::storage(format!("行読み取り失敗: {e}")))?;

        Ok(Some(SessionDetail {
            session_id: sid,
            state,
            mode: parse_mode(&mode_str),
            created_at,
            segments,
        }))
    }

    // --- Settings ---

    pub fn get_settings(&self) -> Result<AppSettings, AppError> {
        let mut stmt = self
            .conn
            .prepare("SELECT key, value FROM settings")
            .map_err(|e| AppError::storage(format!("クエリ準備失敗: {e}")))?;

        let rows: Vec<(String, String)> = stmt
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
            .map_err(|e| AppError::storage(format!("クエリ実行失敗: {e}")))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| AppError::storage(format!("行読み取り失敗: {e}")))?;

        if rows.is_empty() {
            return Ok(AppSettings::default());
        }

        // key-value をJSONに組み立ててデシリアライズ
        let mut map = serde_json::Map::new();
        for (key, value) in &rows {
            // JSONとして解析可能ならそのまま、そうでなければ文字列として
            if let Ok(v) = serde_json::from_str(value) {
                map.insert(key.clone(), v);
            } else {
                map.insert(key.clone(), serde_json::Value::String(value.clone()));
            }
        }

        let json = serde_json::Value::Object(map);
        let mut settings = AppSettings::default();

        // 各フィールドを上書き（存在するキーだけ）
        if let Ok(merged) = serde_json::from_value::<AppSettings>(json) {
            settings = merged;
        }

        Ok(settings)
    }

    pub fn save_settings(&self, settings: &AppSettings) -> Result<(), AppError> {
        let json = serde_json::to_value(settings)
            .map_err(|e| AppError::internal(format!("settings serialize: {e}")))?;

        if let Some(obj) = json.as_object() {
            for (key, value) in obj {
                let value_str = value.to_string();
                self.conn
                    .execute(
                        "INSERT INTO settings (key, value) VALUES (?1, ?2)
                         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
                        params![key, value_str],
                    )
                    .map_err(|e| AppError::storage(format!("設定保存失敗: {e}")))?;
            }
        }

        Ok(())
    }

    // --- Data cleanup ---

    pub fn delete_old_segments(&self, before_date: &str) -> Result<u32, AppError> {
        let affected = self
            .conn
            .execute(
                "DELETE FROM segments WHERE created_at < ?1",
                params![before_date],
            )
            .map_err(|e| AppError::storage(format!("セグメント削除失敗: {e}")))?;
        Ok(affected as u32)
    }

    pub fn delete_old_sessions(&self, before_date: &str) -> Result<u32, AppError> {
        // セグメントが全て削除されたセッションを削除
        let affected = self
            .conn
            .execute(
                "DELETE FROM sessions WHERE created_at < ?1
                 AND session_id NOT IN (SELECT DISTINCT session_id FROM segments)",
                params![before_date],
            )
            .map_err(|e| AppError::storage(format!("セッション削除失敗: {e}")))?;
        Ok(affected as u32)
    }
}

fn parse_mode(s: &str) -> Mode {
    match s {
        "raw" => Mode::Raw,
        "memo" => Mode::Memo,
        "tech" => Mode::Tech,
        "email_jp" => Mode::EmailJp,
        "minutes" => Mode::Minutes,
        _ => Mode::Raw,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn now() -> String {
        "2025-01-15T10:30:00Z".to_string()
    }

    #[test]
    fn test_insert_and_get_session() {
        let storage = Storage::open_in_memory().unwrap();
        storage
            .insert_session("s1", Mode::Memo, &now())
            .unwrap();

        let detail = storage.get_session_detail("s1").unwrap().unwrap();
        assert_eq!(detail.session_id, "s1");
        assert_eq!(detail.state, "idle");
        assert_eq!(detail.mode, Mode::Memo);
        assert!(detail.segments.is_empty());
    }

    #[test]
    fn test_insert_segment_and_update() {
        let storage = Storage::open_in_memory().unwrap();
        storage
            .insert_session("s1", Mode::Raw, &now())
            .unwrap();
        storage
            .insert_segment("seg1", "s1", &now())
            .unwrap();
        storage
            .update_segment_text("seg1", "テスト書き起こし", 0.95)
            .unwrap();
        storage
            .update_segment_rewritten("seg1", "テスト整形済み")
            .unwrap();

        let detail = storage.get_session_detail("s1").unwrap().unwrap();
        assert_eq!(detail.segments.len(), 1);
        assert_eq!(detail.segments[0].raw_text, "テスト書き起こし");
        assert_eq!(
            detail.segments[0].rewritten_text.as_deref(),
            Some("テスト整形済み")
        );
        assert!((detail.segments[0].confidence - 0.95).abs() < f32::EPSILON);
    }

    #[test]
    fn test_list_history_pagination() {
        let storage = Storage::open_in_memory().unwrap();
        for i in 0..5 {
            storage
                .insert_session(
                    &format!("s{i}"),
                    Mode::Memo,
                    &format!("2025-01-15T10:3{i}:00Z"),
                )
                .unwrap();
        }

        // Page 1: limit=2
        let page1 = storage.list_history(2, None).unwrap();
        assert_eq!(page1.items.len(), 2);
        assert!(page1.next_cursor.is_some());
        // 最新が先頭
        assert_eq!(page1.items[0].session_id, "s4");
        assert_eq!(page1.items[1].session_id, "s3");

        // Page 2
        let page2 = storage
            .list_history(2, page1.next_cursor.as_deref())
            .unwrap();
        assert_eq!(page2.items.len(), 2);
        assert_eq!(page2.items[0].session_id, "s2");

        // Page 3
        let page3 = storage
            .list_history(2, page2.next_cursor.as_deref())
            .unwrap();
        assert_eq!(page3.items.len(), 1);
        assert!(page3.next_cursor.is_none());
    }

    #[test]
    fn test_update_session_state() {
        let storage = Storage::open_in_memory().unwrap();
        storage
            .insert_session("s1", Mode::Memo, &now())
            .unwrap();
        storage
            .update_session_state("s1", "recording", &now())
            .unwrap();

        let detail = storage.get_session_detail("s1").unwrap().unwrap();
        assert_eq!(detail.state, "recording");
    }

    #[test]
    fn test_segment_count_in_history() {
        let storage = Storage::open_in_memory().unwrap();
        storage
            .insert_session("s1", Mode::Memo, &now())
            .unwrap();
        storage.insert_segment("seg1", "s1", &now()).unwrap();
        storage.insert_segment("seg2", "s1", &now()).unwrap();

        let page = storage.list_history(10, None).unwrap();
        assert_eq!(page.items[0].segment_count, 2);
    }

    #[test]
    fn test_get_nonexistent_session() {
        let storage = Storage::open_in_memory().unwrap();
        let result = storage.get_session_detail("nonexistent").unwrap();
        assert!(result.is_none());
    }

    // --- Dictionary tests ---

    #[test]
    fn test_upsert_and_list_dictionary() {
        let storage = Storage::open_in_memory().unwrap();
        let entry = DictionaryEntry {
            id: None,
            scope: DictionaryScope::Global,
            mode: None,
            pattern: "くろーど".into(),
            replacement: "Claude".into(),
            priority: 10,
            enabled: true,
        };
        let id = storage.upsert_dictionary_entry(&entry).unwrap();
        assert!(!id.is_empty());

        let entries = storage.list_dictionary_entries(None).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].pattern, "くろーど");
        assert_eq!(entries[0].replacement, "Claude");
        assert_eq!(entries[0].priority, 10);
        assert!(entries[0].enabled);
    }

    #[test]
    fn test_upsert_update_existing() {
        let storage = Storage::open_in_memory().unwrap();
        let entry = DictionaryEntry {
            id: Some("dict1".into()),
            scope: DictionaryScope::Global,
            mode: None,
            pattern: "foo".into(),
            replacement: "bar".into(),
            priority: 5,
            enabled: true,
        };
        storage.upsert_dictionary_entry(&entry).unwrap();

        // 同じIDで更新
        let updated = DictionaryEntry {
            id: Some("dict1".into()),
            scope: DictionaryScope::Global,
            mode: None,
            pattern: "foo".into(),
            replacement: "baz".into(),
            priority: 10,
            enabled: true,
        };
        storage.upsert_dictionary_entry(&updated).unwrap();

        let entries = storage.list_dictionary_entries(None).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].replacement, "baz");
        assert_eq!(entries[0].priority, 10);
    }

    #[test]
    fn test_list_dictionary_by_scope() {
        let storage = Storage::open_in_memory().unwrap();
        storage
            .upsert_dictionary_entry(&DictionaryEntry {
                id: Some("g1".into()),
                scope: DictionaryScope::Global,
                mode: None,
                pattern: "a".into(),
                replacement: "b".into(),
                priority: 1,
                enabled: true,
            })
            .unwrap();
        storage
            .upsert_dictionary_entry(&DictionaryEntry {
                id: Some("m1".into()),
                scope: DictionaryScope::Mode,
                mode: Some(Mode::Tech),
                pattern: "c".into(),
                replacement: "d".into(),
                priority: 1,
                enabled: true,
            })
            .unwrap();

        let global = storage.list_dictionary_entries(Some("global")).unwrap();
        assert_eq!(global.len(), 1);
        assert_eq!(global[0].id.as_deref(), Some("g1"));

        let mode = storage.list_dictionary_entries(Some("mode")).unwrap();
        assert_eq!(mode.len(), 1);
        assert_eq!(mode[0].id.as_deref(), Some("m1"));
    }

    #[test]
    fn test_get_enabled_dictionary_entries() {
        let storage = Storage::open_in_memory().unwrap();
        storage
            .upsert_dictionary_entry(&DictionaryEntry {
                id: Some("e1".into()),
                scope: DictionaryScope::Global,
                mode: None,
                pattern: "a".into(),
                replacement: "b".into(),
                priority: 10,
                enabled: true,
            })
            .unwrap();
        storage
            .upsert_dictionary_entry(&DictionaryEntry {
                id: Some("e2".into()),
                scope: DictionaryScope::Global,
                mode: None,
                pattern: "c".into(),
                replacement: "d".into(),
                priority: 5,
                enabled: false, // disabled
            })
            .unwrap();

        let entries = storage
            .get_enabled_dictionary_entries("global", None)
            .unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].id.as_deref(), Some("e1"));
    }

    #[test]
    fn test_delete_dictionary_entry() {
        let storage = Storage::open_in_memory().unwrap();
        storage
            .upsert_dictionary_entry(&DictionaryEntry {
                id: Some("del1".into()),
                scope: DictionaryScope::Global,
                mode: None,
                pattern: "x".into(),
                replacement: "y".into(),
                priority: 1,
                enabled: true,
            })
            .unwrap();

        assert!(storage.delete_dictionary_entry("del1").unwrap());
        assert!(!storage.delete_dictionary_entry("del1").unwrap()); // already deleted

        let entries = storage.list_dictionary_entries(None).unwrap();
        assert!(entries.is_empty());
    }

    // --- Settings tests ---

    #[test]
    fn test_settings_default_when_empty() {
        let storage = Storage::open_in_memory().unwrap();
        let settings = storage.get_settings().unwrap();
        assert_eq!(settings.segment_ttl_days, 0);
        assert!(settings.paste_confirm);
        assert!(settings.paste_allowlist.is_empty());
    }

    #[test]
    fn test_save_and_get_settings() {
        let storage = Storage::open_in_memory().unwrap();
        let mut settings = AppSettings::default();
        settings.segment_ttl_days = 30;
        settings.rewrite_enabled = true;
        settings.paste_allowlist = vec!["com.apple.Terminal".to_string()];

        storage.save_settings(&settings).unwrap();

        let loaded = storage.get_settings().unwrap();
        assert_eq!(loaded.segment_ttl_days, 30);
        assert!(loaded.rewrite_enabled);
        assert_eq!(loaded.paste_allowlist, vec!["com.apple.Terminal"]);
    }

    // --- Data cleanup tests ---

    #[test]
    fn test_delete_old_segments() {
        let storage = Storage::open_in_memory().unwrap();
        storage
            .insert_session("s1", Mode::Raw, "2025-01-01T00:00:00Z")
            .unwrap();
        storage
            .insert_segment("seg_old", "s1", "2025-01-01T00:00:00Z")
            .unwrap();
        storage
            .insert_segment("seg_new", "s1", "2025-06-01T00:00:00Z")
            .unwrap();

        let deleted = storage
            .delete_old_segments("2025-03-01T00:00:00Z")
            .unwrap();
        assert_eq!(deleted, 1);

        let detail = storage.get_session_detail("s1").unwrap().unwrap();
        assert_eq!(detail.segments.len(), 1);
        assert_eq!(detail.segments[0].segment_id, "seg_new");
    }

    #[test]
    fn test_delete_old_sessions() {
        let storage = Storage::open_in_memory().unwrap();
        storage
            .insert_session("s_old", Mode::Raw, "2025-01-01T00:00:00Z")
            .unwrap();
        storage
            .insert_session("s_new", Mode::Raw, "2025-06-01T00:00:00Z")
            .unwrap();
        // s_new has a segment, so it shouldn't be deleted even if old
        storage
            .insert_segment("seg1", "s_new", "2025-06-01T00:00:00Z")
            .unwrap();

        let deleted = storage
            .delete_old_sessions("2025-12-01T00:00:00Z")
            .unwrap();
        // s_old has no segments and is old → deleted
        // s_new has segments → not deleted
        assert_eq!(deleted, 1);

        assert!(storage.get_session_detail("s_old").unwrap().is_none());
        assert!(storage.get_session_detail("s_new").unwrap().is_some());
    }

    #[test]
    fn test_dictionary_priority_ordering() {
        let storage = Storage::open_in_memory().unwrap();
        for (id, pri) in [("low", 1), ("high", 100), ("mid", 50)] {
            storage
                .upsert_dictionary_entry(&DictionaryEntry {
                    id: Some(id.into()),
                    scope: DictionaryScope::Global,
                    mode: None,
                    pattern: id.into(),
                    replacement: id.into(),
                    priority: pri,
                    enabled: true,
                })
                .unwrap();
        }

        let entries = storage.list_dictionary_entries(None).unwrap();
        assert_eq!(entries[0].id.as_deref(), Some("high"));
        assert_eq!(entries[1].id.as_deref(), Some("mid"));
        assert_eq!(entries[2].id.as_deref(), Some("low"));
    }
}
