//! STT パイプライン統合テスト。
//!
//! 前提条件:
//! - whisper-cli が PATH 上にあるか、WHISPER_BIN 環境変数で指定
//! - GGML モデルが WHISPER_MODEL 環境変数で指定
//!
//! 実行: cargo test --test stt_integration -- --ignored

use std::path::PathBuf;

use vt_core::domain::stt::{AudioSegment, PcmFormat, SttContext, SttEngine, SttErrorKind};
use vt_core::infra::stt::whisper::{WhisperConfig, WhisperSidecar};

fn whisper_config() -> WhisperConfig {
    WhisperConfig {
        binary_path: std::env::var("WHISPER_BIN")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("whisper-cli")),
        model_path: std::env::var("WHISPER_MODEL")
            .map(PathBuf::from)
            .expect("WHISPER_MODEL env var required for integration tests"),
        language: "ja".into(),
        ..Default::default()
    }
}

#[tokio::test]
#[ignore]
async fn transcribe_known_wav() {
    let config = whisper_config();
    let engine = WhisperSidecar::new(config);
    engine.validate().expect("Whisper not properly configured");

    let wav_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/hello_ja.wav");
    if !wav_path.exists() {
        eprintln!("Test WAV file not found: {:?}. Skipping.", wav_path);
        return;
    }

    let segment = AudioSegment {
        id: uuid::Uuid::new_v4(),
        started_at: chrono::Utc::now(),
        duration_ms: 2000,
        sample_rate: 16_000,
        channels: 1,
        pcm_format: PcmFormat::F32Le,
        wav_path: Some(wav_path),
        samples: None,
        language: Some("ja".into()),
        hints: vec![],
    };

    let ctx = SttContext::default();
    let result = engine.transcribe(&segment, &ctx).await;

    match result {
        Ok(transcript) => {
            assert!(!transcript.text.is_empty(), "Transcript should not be empty");
            assert!(!transcript.is_partial, "Should be final result");
            println!("Transcribed: {}", transcript.text);
        }
        Err(e) if e.kind == SttErrorKind::NoSpeech => {
            println!("No speech detected (acceptable for test)");
        }
        Err(e) => panic!("Unexpected error: {}", e),
    }
}

#[tokio::test]
async fn transcribe_missing_wav() {
    let config = WhisperConfig::default();
    let engine = WhisperSidecar::new(config);

    let segment = AudioSegment {
        id: uuid::Uuid::new_v4(),
        started_at: chrono::Utc::now(),
        duration_ms: 1000,
        sample_rate: 16_000,
        channels: 1,
        pcm_format: PcmFormat::F32Le,
        wav_path: Some(PathBuf::from("/nonexistent/audio.wav")),
        samples: None,
        language: None,
        hints: vec![],
    };

    let ctx = SttContext::default();
    let err = engine.transcribe(&segment, &ctx).await.unwrap_err();
    assert_eq!(err.kind, SttErrorKind::AudioFormat);
}

#[tokio::test]
async fn transcribe_no_wav_path() {
    let config = WhisperConfig::default();
    let engine = WhisperSidecar::new(config);

    let segment = AudioSegment {
        id: uuid::Uuid::new_v4(),
        started_at: chrono::Utc::now(),
        duration_ms: 1000,
        sample_rate: 16_000,
        channels: 1,
        pcm_format: PcmFormat::F32Le,
        wav_path: None,
        samples: Some(vec![0.0; 16000]),
        language: None,
        hints: vec![],
    };

    let ctx = SttContext::default();
    let err = engine.transcribe(&segment, &ctx).await.unwrap_err();
    assert_eq!(err.kind, SttErrorKind::AudioFormat);
}
