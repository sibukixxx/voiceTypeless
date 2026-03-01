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
    /// Soniox API キー
    pub soniox_api_key: Option<String>,
    /// STT 言語設定（デフォルト "ja-JP"）
    pub language: String,
    /// VAD セグメント最大長のオーバーライド（ms, None=エンジンデフォルト）
    pub vad_max_segment_ms: Option<u64>,
    /// Whisper モデルサイズ
    pub whisper_model_size: WhisperModelSize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SttEngineChoice {
    Apple,
    Whisper,
    Cloud,
    Soniox,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WhisperModelSize {
    Base,
    Small,
    Medium,
    Large,
}

impl Default for WhisperModelSize {
    fn default() -> Self {
        Self::Base
    }
}

impl WhisperModelSize {
    /// モデルファイル名を返す
    pub fn filename(&self) -> &str {
        match self {
            Self::Base => "ggml-base.bin",
            Self::Small => "ggml-small.bin",
            Self::Medium => "ggml-medium.bin",
            Self::Large => "ggml-large-v3.bin",
        }
    }

    /// ダウンロード URL を返す
    pub fn download_url(&self) -> String {
        let file = self.filename();
        format!("https://huggingface.co/ggerganov/whisper.cpp/resolve/main/{file}")
    }
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            stt_engine: SttEngineChoice::Soniox,
            default_mode: "raw".to_string(),
            default_deliver_target: "clipboard".to_string(),
            rewrite_enabled: false,
            paste_allowlist: vec![],
            paste_confirm: true,
            audio_retention: AudioRetention::None,
            segment_ttl_days: 0,
            hotkey_toggle: "CmdOrCtrl+Shift+R".to_string(),
            claude_api_key: None,
            soniox_api_key: None,
            language: "ja-JP".to_string(),
            vad_max_segment_ms: None,
            whisper_model_size: WhisperModelSize::Base,
        }
    }
}
