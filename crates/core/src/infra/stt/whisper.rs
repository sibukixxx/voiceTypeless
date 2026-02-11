use async_trait::async_trait;
use std::path::PathBuf;
use std::sync::Mutex;
use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};

use super::{AudioSegment, SttContext, SttEngine, SttError, TranscriptResult};

/// Whisper.cpp ベースの STT エンジン
pub struct WhisperSttEngine {
    ctx: Mutex<WhisperContext>,
}

impl WhisperSttEngine {
    /// モデルファイルからエンジンを初期化する
    pub fn new(model_path: &str) -> Result<Self, SttError> {
        let ctx = WhisperContext::new_with_params(model_path, WhisperContextParameters::default())
            .map_err(|e| SttError::EngineNotAvailable(format!("Whisper model load failed: {e}")))?;

        Ok(Self {
            ctx: Mutex::new(ctx),
        })
    }

    /// デフォルトのモデルパスを返す
    pub fn default_model_path() -> PathBuf {
        let base = dirs::data_local_dir()
            .unwrap_or_else(|| PathBuf::from("."));
        base.join("voiceTypeless")
            .join("models")
            .join("ggml-base.bin")
    }

    /// モデルファイルが存在するかチェック
    pub fn is_model_available() -> bool {
        Self::default_model_path().exists()
    }
}

/// 線形補間リサンプラー: 任意サンプルレート → 16kHz
fn resample_to_16k(samples: &[f32], source_rate: u32) -> Vec<f32> {
    if source_rate == 16000 {
        return samples.to_vec();
    }

    let ratio = source_rate as f64 / 16000.0;
    let output_len = (samples.len() as f64 / ratio) as usize;
    let mut output = Vec::with_capacity(output_len);

    for i in 0..output_len {
        let src_idx = i as f64 * ratio;
        let idx_floor = src_idx.floor() as usize;
        let idx_ceil = (idx_floor + 1).min(samples.len() - 1);
        let frac = (src_idx - idx_floor as f64) as f32;

        let sample = samples[idx_floor] * (1.0 - frac) + samples[idx_ceil] * frac;
        output.push(sample);
    }

    output
}

#[async_trait]
impl SttEngine for WhisperSttEngine {
    async fn transcribe(
        &self,
        audio: AudioSegment,
        ctx: SttContext,
    ) -> Result<TranscriptResult, SttError> {
        // 16kHz にリサンプリング
        let samples_16k = resample_to_16k(&audio.samples, audio.sample_rate);

        if samples_16k.is_empty() {
            return Ok(TranscriptResult {
                text: String::new(),
                confidence: 0.0,
                is_partial: false,
            });
        }

        let whisper_ctx = self.ctx.lock().map_err(|e| {
            SttError::TranscriptionFailed(format!("Whisper context lock failed: {e}"))
        })?;

        let mut state = whisper_ctx.create_state().map_err(|e| {
            SttError::TranscriptionFailed(format!("Whisper state creation failed: {e}"))
        })?;

        let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });

        // 言語設定
        let lang = if ctx.language.starts_with("ja") {
            "ja"
        } else if ctx.language.starts_with("en") {
            "en"
        } else if ctx.language.starts_with("zh") {
            "zh"
        } else if ctx.language.starts_with("ko") {
            "ko"
        } else {
            "auto"
        };
        params.set_language(Some(lang));
        params.set_print_special(false);
        params.set_print_progress(false);
        params.set_print_realtime(false);
        params.set_print_timestamps(false);
        params.set_suppress_blank(true);
        params.set_no_context(true);

        state
            .full(params, &samples_16k)
            .map_err(|e| SttError::TranscriptionFailed(format!("Whisper inference failed: {e}")))?;

        let num_segments = state.full_n_segments().map_err(|e| {
            SttError::TranscriptionFailed(format!("Failed to get segments: {e}"))
        })?;

        let mut text = String::new();
        for i in 0..num_segments {
            if let Ok(segment_text) = state.full_get_segment_text(i) {
                text.push_str(&segment_text);
            }
        }

        let text = text.trim().to_string();

        Ok(TranscriptResult {
            text,
            confidence: 0.8, // Whisper.cpp は confidence を直接返さないため固定値
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

    #[test]
    fn test_resample_identity() {
        let samples = vec![1.0, 2.0, 3.0, 4.0];
        let result = resample_to_16k(&samples, 16000);
        assert_eq!(result, samples);
    }

    #[test]
    fn test_resample_downsample() {
        let samples: Vec<f32> = (0..48000).map(|i| (i as f32) / 48000.0).collect();
        let result = resample_to_16k(&samples, 48000);
        assert_eq!(result.len(), 16000);
    }

    #[test]
    fn test_default_model_path() {
        let path = WhisperSttEngine::default_model_path();
        assert!(path.to_string_lossy().contains("ggml-base.bin"));
    }
}
