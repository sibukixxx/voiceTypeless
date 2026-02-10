# STT Engine Interface Contract

STTエンジンの共通インタフェース定義。すべてのSTT実装（Apple Speech, Whisper.cpp, Cloud）はこのトレイトを実装する。

## Rust Trait

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
pub struct AudioSegment {
    /// PCM audio data (f32, mono)
    pub samples: Vec<f32>,
    /// Sample rate in Hz
    pub sample_rate: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SttContext {
    /// Language hint (e.g., "ja-JP", "en-US")
    pub language: String,
    /// Domain dictionary for recognition hints
    pub dictionary: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscriptResult {
    /// Transcribed text
    pub text: String,
    /// Confidence score (0.0 - 1.0)
    pub confidence: f32,
    /// Whether this is a partial (interim) result
    pub is_partial: bool,
}

#[async_trait::async_trait]
pub trait SttEngine: Send + Sync {
    /// Transcribe an audio segment
    async fn transcribe(
        &self,
        audio: AudioSegment,
        ctx: SttContext,
    ) -> Result<TranscriptResult, SttError>;

    /// Whether this engine supports partial (streaming) results
    fn supports_partial(&self) -> bool;
}
```

## Implementations

| Engine | Crate | Partial Support | Notes |
|--------|-------|----------------|-------|
| Apple Speech | `stt-apple-bridge` | Yes | macOS only, Swift bridge |
| Whisper.cpp | `crates/core/infra/stt` | No | Local, cross-platform |
| Cloud STT | `crates/core/infra/stt` | Yes | Optional, requires API key |

## Error Types

```rust
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
```
