use serde::{Deserialize, Serialize};

/// アプリケーション設定
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    /// STTエンジン選択
    pub stt_engine: SttEngineChoice,
    /// デフォルトモード
    pub default_mode: String,
    /// デフォルト出力先
    pub default_deliver_target: String,
    /// リライト有効/無効（デフォルト）
    pub rewrite_enabled: bool,
    /// 貼り付けallowlist（bundle id）
    pub paste_allowlist: Vec<String>,
    /// 貼り付け前に確認するか
    pub paste_confirm: bool,
    /// 音声保存ポリシー
    pub audio_retention: AudioRetention,
    /// セグメント自動削除（日数、0=無期限）
    pub segment_ttl_days: u32,
    /// グローバルホットキー（toggle_recording）
    pub hotkey_toggle: String,
    /// Claude API キー（ローカル SQLite に保存）
    pub claude_api_key: Option<String>,
    /// STT 言語設定（デフォルト "ja-JP"）
    pub language: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SttEngineChoice {
    Apple,
    Whisper,
    Cloud,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AudioRetention {
    /// 音声を保存しない（デフォルト）
    None,
    /// TTL日数だけ保持
    Ttl,
    /// 永続保存
    Permanent,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            stt_engine: SttEngineChoice::Apple,
            default_mode: "raw".to_string(),
            default_deliver_target: "clipboard".to_string(),
            rewrite_enabled: false,
            paste_allowlist: vec![],
            paste_confirm: true,
            audio_retention: AudioRetention::None,
            segment_ttl_days: 0,
            hotkey_toggle: "CmdOrCtrl+Shift+R".to_string(),
            claude_api_key: None,
            language: "ja-JP".to_string(),
        }
    }
}
