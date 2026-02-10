use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use uuid::Uuid;

// ─── AudioSegment ────────────────────────────────────────────────

/// 文字起こし対象の音声セグメント。
#[derive(Debug, Clone)]
pub struct AudioSegment {
    /// セグメント固有ID (UUID v4)
    pub id: Uuid,
    /// 録音開始時刻 (UTC, ISO 8601)
    pub started_at: chrono::DateTime<chrono::Utc>,
    /// セグメント長 (ミリ秒)
    pub duration_ms: u32,
    /// サンプルレート (Hz)。Whisper用に16000を標準とする。
    pub sample_rate: u32,
    /// チャンネル数 (1 = mono)
    pub channels: u16,
    /// PCMフォーマット
    pub pcm_format: PcmFormat,
    /// ディスク上のWAVファイルパス (sidecar方式で使用)
    pub wav_path: Option<PathBuf>,
    /// インメモリPCMサンプル (f32, mono)。インメモリエンジン用。
    pub samples: Option<Vec<f32>>,
    /// 言語ヒント (例: "ja", "en")
    pub language: Option<String>,
    /// 認識ヒント / ドメイン辞書用語
    pub hints: Vec<String>,
}

/// PCMサンプルフォーマット。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PcmFormat {
    F32Le,
    I16Le,
}

impl AudioSegment {
    /// 必須フィールドから AudioSegment を生成する。duration_ms は自動算出。
    pub fn new(
        samples: Vec<f32>,
        sample_rate: u32,
        started_at: chrono::DateTime<chrono::Utc>,
    ) -> Self {
        let duration_ms = if sample_rate > 0 {
            ((samples.len() as f64 / sample_rate as f64) * 1000.0) as u32
        } else {
            0
        };
        Self {
            id: Uuid::new_v4(),
            started_at,
            duration_ms,
            sample_rate,
            channels: 1,
            pcm_format: PcmFormat::F32Le,
            wav_path: None,
            samples: Some(samples),
            language: None,
            hints: Vec::new(),
        }
    }
}

// ─── Transcript ──────────────────────────────────────────────────

/// 文字起こし結果。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transcript {
    /// 文字起こしテキスト
    pub text: String,
    /// 信頼度スコア (0.0–1.0)。エンジンが提供しない場合は None。
    pub confidence: Option<f32>,
    /// 部分結果かどうか
    pub is_partial: bool,
    /// トークン別の確率情報 (任意)
    pub tokens: Option<Vec<TokenInfo>>,
    /// 単語/セグメント別のタイミング情報 (任意)
    pub timings: Option<Vec<TimingInfo>>,
}

/// トークン確率情報。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenInfo {
    pub token: String,
    pub probability: f32,
}

/// 単語レベルのタイミング情報。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimingInfo {
    pub word: String,
    pub start_ms: u32,
    pub end_ms: u32,
}

// ─── SttError ────────────────────────────────────────────────────

/// STT処理で発生するエラー。
#[derive(Debug, Clone)]
pub struct SttError {
    /// エラー種別
    pub kind: SttErrorKind,
    /// 人間が読める詳細メッセージ
    pub detail: String,
    /// リトライで回復可能かどうか
    pub recoverable: bool,
}

impl std::fmt::Display for SttError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "SttError::{:?}: {}", self.kind, self.detail)
    }
}

impl std::error::Error for SttError {}

/// STTエラー種別。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SttErrorKind {
    /// 音声フォーマットが不正/非対応
    AudioFormat,
    /// STTエンジンが利用不可 (未インストール、モデル未検出等)
    EngineNotAvailable,
    /// 文字起こし処理中のエラー
    TranscriptionFailed,
    /// タイムアウト
    Timeout,
    /// 音声中に発話が検出されなかった
    NoSpeech,
    /// マイク権限が拒否された
    PermissionDenied,
}

impl SttError {
    pub fn audio_format(detail: impl Into<String>) -> Self {
        Self { kind: SttErrorKind::AudioFormat, detail: detail.into(), recoverable: false }
    }

    pub fn engine_not_available(detail: impl Into<String>) -> Self {
        Self { kind: SttErrorKind::EngineNotAvailable, detail: detail.into(), recoverable: false }
    }

    pub fn transcription_failed(detail: impl Into<String>) -> Self {
        Self { kind: SttErrorKind::TranscriptionFailed, detail: detail.into(), recoverable: true }
    }

    pub fn timeout(detail: impl Into<String>) -> Self {
        Self { kind: SttErrorKind::Timeout, detail: detail.into(), recoverable: true }
    }

    pub fn no_speech() -> Self {
        Self { kind: SttErrorKind::NoSpeech, detail: "No speech detected".into(), recoverable: true }
    }

    pub fn permission_denied(detail: impl Into<String>) -> Self {
        Self { kind: SttErrorKind::PermissionDenied, detail: detail.into(), recoverable: false }
    }
}

// ─── PipelineState / PipelineEvent ───────────────────────────────

/// 音声パイプラインの状態。UI表示に利用。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PipelineState {
    /// 待機中 (マイク停止)
    Idle,
    /// 録音中 (マイクアクティブ、音声検出待ち)
    Listening,
    /// 音声検出中 (VADがspeech検出、セグメント蓄積中)
    Capturing,
    /// 文字起こし処理中 (Whisper実行中)
    Processing,
}

/// パイプラインから発行されるイベント。
#[derive(Debug, Clone)]
pub enum PipelineEvent {
    /// パイプライン状態遷移
    StateChanged(PipelineState),
    /// 部分結果 (ストリーミングエンジン用、is_partial = true)
    PartialTranscript(Transcript),
    /// 最終結果
    FinalTranscript(Transcript),
    /// エラー発生 (recoverable = true の場合はパイプライン継続)
    Error(SttError),
    /// VU メータレベル (RMS値, 0.0〜1.0)
    AudioLevel(f32),
}

// ─── SttContext ──────────────────────────────────────────────────

/// STTエンジンに渡すセッションレベルのコンテキスト。
/// AudioSegment上の language/hints を上書きする場合に使用。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SttContext {
    /// 言語指定 (AudioSegment.language より優先)
    pub language: Option<String>,
    /// セッション辞書 (AudioSegment.hints に追加される)
    pub dictionary: Vec<String>,
}

// ─── SttEngine trait ─────────────────────────────────────────────

/// STTエンジンのコアトレイト。全STT実装がこれを満たす。
///
/// `async_trait` を使用して dyn SttEngine (トレイトオブジェクト) として利用可能にする。
#[async_trait::async_trait]
pub trait SttEngine: Send + Sync {
    /// 音声セグメントを文字起こしする。
    async fn transcribe(
        &self,
        audio: &AudioSegment,
        ctx: &SttContext,
    ) -> Result<Transcript, SttError>;

    /// ストリーミング部分結果をサポートするかどうか。
    fn supports_partial(&self) -> bool;

    /// エンジン名 (例: "whisper.cpp", "apple-speech")。
    fn name(&self) -> &str;
}

// ─── Tests ───────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn audio_segment_new_computes_duration() {
        let samples = vec![0.0f32; 16000]; // 1 second at 16kHz
        let seg = AudioSegment::new(samples, 16000, chrono::Utc::now());
        assert_eq!(seg.duration_ms, 1000);
        assert_eq!(seg.channels, 1);
        assert_eq!(seg.pcm_format, PcmFormat::F32Le);
        assert!(seg.wav_path.is_none());
        assert!(seg.samples.is_some());
    }

    #[test]
    fn audio_segment_new_half_second() {
        let samples = vec![0.0f32; 8000]; // 0.5 second at 16kHz
        let seg = AudioSegment::new(samples, 16000, chrono::Utc::now());
        assert_eq!(seg.duration_ms, 500);
    }

    #[test]
    fn audio_segment_new_zero_rate() {
        let samples = vec![0.0f32; 100];
        let seg = AudioSegment::new(samples, 0, chrono::Utc::now());
        assert_eq!(seg.duration_ms, 0);
    }

    #[test]
    fn stt_error_constructors() {
        let e = SttError::audio_format("bad format");
        assert_eq!(e.kind, SttErrorKind::AudioFormat);
        assert!(!e.recoverable);

        let e = SttError::timeout("too slow");
        assert_eq!(e.kind, SttErrorKind::Timeout);
        assert!(e.recoverable);

        let e = SttError::no_speech();
        assert_eq!(e.kind, SttErrorKind::NoSpeech);
        assert!(e.recoverable);

        let e = SttError::permission_denied("denied");
        assert_eq!(e.kind, SttErrorKind::PermissionDenied);
        assert!(!e.recoverable);
    }

    #[test]
    fn stt_error_display() {
        let e = SttError::transcription_failed("engine crashed");
        let msg = format!("{}", e);
        assert!(msg.contains("TranscriptionFailed"));
        assert!(msg.contains("engine crashed"));
    }

    #[test]
    fn transcript_serialization() {
        let t = Transcript {
            text: "hello".into(),
            confidence: Some(0.95),
            is_partial: false,
            tokens: None,
            timings: None,
        };
        let json = serde_json::to_string(&t).unwrap();
        let t2: Transcript = serde_json::from_str(&json).unwrap();
        assert_eq!(t2.text, "hello");
        assert_eq!(t2.confidence, Some(0.95));
    }
}
