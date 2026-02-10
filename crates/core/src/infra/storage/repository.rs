use rusqlite::{Connection, params};

use crate::domain::error::AppError;
use crate::domain::types::{HistoryPage, Mode, Segment, SessionDetail, SessionSummary};

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
}
