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

impl std::fmt::Display for Mode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Mode::Raw => write!(f, "raw"),
            Mode::Memo => write!(f, "memo"),
            Mode::Tech => write!(f, "tech"),
            Mode::EmailJp => write!(f, "email_jp"),
            Mode::Minutes => write!(f, "minutes"),
        }
    }
}

/// 出力ポリシー
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "target", rename_all = "snake_case")]
pub enum DeliverPolicy {
    Clipboard,
    Paste,
    FileAppend,
    Webhook,
}

/// 配信先ターゲット
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeliverTarget {
    Clipboard,
    Paste,
    FileAppend,
    Webhook,
}

impl DeliverTarget {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Clipboard => "clipboard",
            Self::Paste => "paste",
            Self::FileAppend => "file_append",
            Self::Webhook => "webhook",
        }
    }
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
    pub preview_text: Option<String>,
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

/// セットアップ不備の個別項目
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetupIssue {
    /// カテゴリ: "stt", "rewriter", "delivery", "permission"
    pub category: String,
    /// 深刻度: "error" (機能不全), "warning" (一部制限)
    pub severity: String,
    /// ユーザー向けメッセージ
    pub message: String,
    /// 推奨アクション
    pub action: String,
    /// 遷移先ページ（例: "settings"）
    pub navigate_to: Option<String>,
}

/// セットアップ状態の全体像
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetupStatus {
    /// error が 0 件なら true
    pub ready: bool,
    /// 検出された不備リスト
    pub issues: Vec<SetupIssue>,
    /// 現在アクティブな STT エンジン名
    pub active_stt_engine: String,
    /// 現在アクティブなリライター名
    pub active_rewriter: String,
}
