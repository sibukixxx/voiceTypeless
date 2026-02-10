use serde::{Deserialize, Serialize};

/// 書き起こし/リライトモード
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Mode {
    Raw,
    Memo,
    Tech,
    EmailJp,
    Minutes,
}

/// 出力ポリシー（Phase1は clipboard のみ）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "target", rename_all = "snake_case")]
pub enum DeliverPolicy {
    Clipboard,
}

/// セグメント情報
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Segment {
    pub segment_id: String,
    pub session_id: String,
    pub raw_text: String,
    pub rewritten_text: Option<String>,
    pub confidence: f32,
    pub created_at: String,
}

/// セッションサマリー（履歴一覧用）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionSummary {
    pub session_id: String,
    pub state: String,
    pub mode: Mode,
    pub created_at: String,
    pub updated_at: String,
    pub segment_count: u32,
}

/// セッション詳細（セグメント付き）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionDetail {
    pub session_id: String,
    pub state: String,
    pub mode: Mode,
    pub created_at: String,
    pub segments: Vec<Segment>,
}

/// 履歴ページ（カーソルベースページネーション）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryPage {
    pub items: Vec<SessionSummary>,
    pub next_cursor: Option<String>,
}

/// 辞書エントリ
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DictionaryEntry {
    pub id: Option<String>,
    pub scope: DictionaryScope,
    pub mode: Option<Mode>,
    pub pattern: String,
    pub replacement: String,
    pub priority: i32,
    pub enabled: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DictionaryScope {
    Global,
    Mode,
}
