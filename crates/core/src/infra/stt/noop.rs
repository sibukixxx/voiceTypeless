use async_trait::async_trait;

use super::{AudioSegment, SttContext, SttEngine, SttError, TranscriptResult};

/// NoopSttService: 固定文字列を返すモック実装。
/// Agent Bが実STTエンジンを実装するまでのスタブ。
pub struct NoopSttService;

#[async_trait]
impl SttEngine for NoopSttService {
    async fn transcribe(
        &self,
        _audio: AudioSegment,
        _ctx: SttContext,
    ) -> Result<TranscriptResult, SttError> {
        Ok(TranscriptResult {
            text: "[STTスタブ] これはモック書き起こし結果です".to_string(),
            confidence: 1.0,
            is_partial: false,
        })
    }

    fn supports_partial(&self) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_noop_stt_returns_fixed_text() {
        let stt = NoopSttService;
        let result = stt
            .transcribe(
                AudioSegment {
                    samples: vec![0.0; 100],
                    sample_rate: 16000,
                },
                SttContext {
                    language: "ja-JP".to_string(),
                    dictionary: vec![],
                },
            )
            .await
            .unwrap();

        assert!(!result.text.is_empty());
        assert_eq!(result.confidence, 1.0);
        assert!(!result.is_partial);
    }

    #[test]
    fn test_noop_does_not_support_partial() {
        let stt = NoopSttService;
        assert!(!stt.supports_partial());
    }
}
