#[cfg(target_os = "macos")]
pub mod apple_speech;
mod noop;

pub use noop::NoopSttService;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// 音声セグメント（STTへの入力）
#[derive(Debug, Clone)]
pub struct AudioSegment {
    /// PCM audio data (f32, mono)
    pub samples: Vec<f32>,
    /// Sample rate in Hz
    pub sample_rate: u32,
}

/// STTコンテキスト（認識ヒント）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SttContext {
    /// Language hint (e.g., "ja-JP", "en-US")
    pub language: String,
    /// Domain dictionary for recognition hints
    pub dictionary: Vec<String>,
}

/// 書き起こし結果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscriptResult {
    pub text: String,
    pub confidence: f32,
    pub is_partial: bool,
}

/// STTエラー
#[derive(Debug, thiserror::Error)]
pub enum SttError {
    #[error("Audio format error: {0}")]
    AudioFormat(String),
    #[error("Engine not available: {0}")]
    EngineNotAvailable(String),
    #[error("Transcription failed: {0}")]
    TranscriptionFailed(String),
    #[error("Timeout")]
    Timeout,
}

/// STTエンジン trait（Agent B が実装する）
#[async_trait]
pub trait SttEngine: Send + Sync {
    async fn transcribe(
        &self,
        audio: AudioSegment,
        ctx: SttContext,
    ) -> Result<TranscriptResult, SttError>;

    fn supports_partial(&self) -> bool;
}
