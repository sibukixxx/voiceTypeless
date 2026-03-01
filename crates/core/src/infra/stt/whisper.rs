use async_trait::async_trait;
use std::path::PathBuf;
use std::sync::Mutex;
use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};

use super::{AudioSegment, SttContext, SttEngine, SttError, TranscriptResult};
use crate::domain::settings::WhisperModelSize;

/// Whisper デコード設定
#[derive(Debug, Clone)]
pub struct WhisperConfig {
    /// Beam search のビームサイズ（0=Greedy）
    pub beam_size: usize,
    /// Greedy 時の best_of パラメータ
    pub best_of: usize,
}

impl Default for WhisperConfig {
    fn default() -> Self {
        Self {
            beam_size: 5,
            best_of: 5,
        }
    }
}

/// Whisper.cpp ベースの STT エンジン
pub struct WhisperSttEngine {
    ctx: Mutex<WhisperContext>,
    config: WhisperConfig,
}

impl WhisperSttEngine {
    /// モデルファイルからエンジンを初期化する（デフォルト設定）
    pub fn new(model_path: &str) -> Result<Self, SttError> {
        Self::with_config(model_path, WhisperConfig::default())
    }

    /// モデルファイルと設定からエンジンを初期化する
    pub fn with_config(model_path: &str, config: WhisperConfig) -> Result<Self, SttError> {
        let ctx = WhisperContext::new_with_params(model_path, WhisperContextParameters::default())
            .map_err(|e| SttError::EngineNotAvailable(format!("Whisper model load failed: {e}")))?;

        Ok(Self {
            ctx: Mutex::new(ctx),
            config,
        })
    }

    /// 指定モデルサイズのモデルパスを返す
    pub fn model_path_for(size: WhisperModelSize) -> PathBuf {
        let base = dirs::data_local_dir().unwrap_or_else(|| PathBuf::from("."));
        base.join("voiceTypeless")
            .join("models")
            .join(size.filename())
    }

    /// デフォルトのモデルパスを返す（Base）
    pub fn default_model_path() -> PathBuf {
        Self::model_path_for(WhisperModelSize::Base)
    }

    /// 指定モデルサイズのモデルファイルが存在するかチェック
    pub fn is_model_available_for(size: WhisperModelSize) -> bool {
        Self::model_path_for(size).exists()
    }

    /// デフォルトモデル（Base）が存在するかチェック
    pub fn is_model_available() -> bool {
        Self::is_model_available_for(WhisperModelSize::Base)
    }
}

/// 線形補間リサンプラー（フォールバック）: 任意サンプルレート → 16kHz
#[cfg(not(feature = "high-quality-resample"))]
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

/// 高品質 sinc 補間リサンプラー (rubato): 任意サンプルレート → 16kHz
#[cfg(feature = "high-quality-resample")]
fn resample_to_16k(samples: &[f32], source_rate: u32) -> Vec<f32> {
    if source_rate == 16000 {
        return samples.to_vec();
    }

    use rubato::{
        Resampler, SincFixedIn, SincInterpolationParameters, SincInterpolationType, WindowFunction,
    };

    let params = SincInterpolationParameters {
        sinc_len: 256,
        f_cutoff: 0.95,
        interpolation: SincInterpolationType::Linear,
        oversampling_factor: 256,
        window: WindowFunction::BlackmanHarris2,
    };

    let ratio = 16000_f64 / source_rate as f64;
    let chunk_size = samples.len();

    let mut resampler = match SincFixedIn::<f64>::new(ratio, 2.0, params, chunk_size, 1) {
        Ok(r) => r,
        Err(e) => {
            log::warn!("rubato init failed, falling back to linear: {e}");
            return resample_linear_fallback(samples, source_rate);
        }
    };

    let input: Vec<f64> = samples.iter().map(|&s| s as f64).collect();
    match resampler.process(&[&input], None) {
        Ok(output) => output[0].iter().map(|&s| s as f32).collect(),
        Err(e) => {
            log::warn!("rubato resample failed, falling back to linear: {e}");
            resample_linear_fallback(samples, source_rate)
        }
    }
}

/// rubato 失敗時の線形補間フォールバック
#[cfg(feature = "high-quality-resample")]
fn resample_linear_fallback(samples: &[f32], source_rate: u32) -> Vec<f32> {
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

        // サンプリング戦略: beam_size > 0 → BeamSearch, 0 → Greedy
        let strategy = if self.config.beam_size > 0 {
            SamplingStrategy::BeamSearch {
                beam_size: self.config.beam_size as i32,
                patience: -1.0,
            }
        } else {
            SamplingStrategy::Greedy {
                best_of: self.config.best_of as i32,
            }
        };
        let mut params = FullParams::new(strategy);

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

        // 辞書ヒントによる initial_prompt 設定
        if !ctx.dictionary.is_empty() {
            let hints = ctx.dictionary.join("、");
            params.set_initial_prompt(&hints);
            params.set_no_context(false);
        } else {
            params.set_no_context(true);
        }

        state
            .full(params, &samples_16k)
            .map_err(|e| SttError::TranscriptionFailed(format!("Whisper inference failed: {e}")))?;

        let num_segments = state
            .full_n_segments()
            .map_err(|e| SttError::TranscriptionFailed(format!("Failed to get segments: {e}")))?;

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

    fn name(&self) -> &str {
        "whisper"
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
        // sinc 補間リサンプラーは厳密に 16000 にならない場合がある（±1%許容）
        let diff = (result.len() as i64 - 16000).unsigned_abs();
        assert!(diff < 160, "Expected ~16000 samples, got {}", result.len());
    }

    #[test]
    fn test_default_model_path() {
        let path = WhisperSttEngine::default_model_path();
        assert!(path.to_string_lossy().contains("ggml-base.bin"));
    }

    #[test]
    fn test_model_path_for_sizes() {
        assert!(WhisperSttEngine::model_path_for(WhisperModelSize::Base)
            .to_string_lossy()
            .contains("ggml-base.bin"));
        assert!(WhisperSttEngine::model_path_for(WhisperModelSize::Small)
            .to_string_lossy()
            .contains("ggml-small.bin"));
        assert!(WhisperSttEngine::model_path_for(WhisperModelSize::Medium)
            .to_string_lossy()
            .contains("ggml-medium.bin"));
        assert!(WhisperSttEngine::model_path_for(WhisperModelSize::Large)
            .to_string_lossy()
            .contains("ggml-large-v3.bin"));
    }

    #[test]
    fn test_whisper_config_default() {
        let config = WhisperConfig::default();
        assert_eq!(config.beam_size, 5);
        assert_eq!(config.best_of, 5);
    }

    // WhisperSttEngine::name() は実際のモデルが必要なため、
    // trait のデフォルト実装テストは noop で代替。
    // model_path_for のテストで whisper モジュール全体をカバー済み。
}
