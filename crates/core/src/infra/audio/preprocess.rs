/// 音声前処理設定
#[derive(Debug, Clone)]
pub struct PreprocessConfig {
    /// DC オフセット除去を有効にする
    pub remove_dc_offset: bool,
    /// ゲイン正規化を有効にする
    pub normalize_gain: bool,
    /// 正規化時の目標ピーク値（0.0〜1.0）
    pub target_peak: f32,
}

impl Default for PreprocessConfig {
    fn default() -> Self {
        Self {
            remove_dc_offset: true,
            normalize_gain: true,
            target_peak: 0.9,
        }
    }
}

/// 音声前処理プロセッサ
pub struct AudioPreprocessor;

impl AudioPreprocessor {
    /// DC オフセットを除去する（平均値を引く）
    pub fn remove_dc_offset(samples: &mut [f32]) {
        if samples.is_empty() {
            return;
        }
        let mean: f32 = samples.iter().sum::<f32>() / samples.len() as f32;
        for s in samples.iter_mut() {
            *s -= mean;
        }
    }

    /// ピーク正規化（最大振幅を target_peak に合わせる）
    pub fn normalize_gain(samples: &mut [f32], target_peak: f32) {
        if samples.is_empty() {
            return;
        }
        let peak = samples.iter().map(|s| s.abs()).fold(0.0_f32, f32::max);
        if peak < 1e-6 {
            return; // ほぼ無音 — ゲインを上げすぎない
        }
        let gain = target_peak / peak;
        if (gain - 1.0).abs() < 0.01 {
            return; // 既にほぼ正規化済み
        }
        for s in samples.iter_mut() {
            *s *= gain;
        }
    }

    /// 全前処理を in-place 適用
    pub fn process(samples: &mut [f32], config: &PreprocessConfig) {
        if config.remove_dc_offset {
            Self::remove_dc_offset(samples);
        }
        if config.normalize_gain {
            Self::normalize_gain(samples, config.target_peak);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_remove_dc_offset() {
        let mut samples = vec![1.1, 1.2, 1.3, 1.0];
        AudioPreprocessor::remove_dc_offset(&mut samples);
        let mean: f32 = samples.iter().sum::<f32>() / samples.len() as f32;
        assert!(mean.abs() < 1e-6, "DC offset should be ~0, got {mean}");
    }

    #[test]
    fn test_remove_dc_offset_empty() {
        let mut samples: Vec<f32> = vec![];
        AudioPreprocessor::remove_dc_offset(&mut samples);
        assert!(samples.is_empty());
    }

    #[test]
    fn test_normalize_gain() {
        let mut samples = vec![0.1, -0.2, 0.15, -0.05];
        AudioPreprocessor::normalize_gain(&mut samples, 0.9);
        let peak = samples.iter().map(|s| s.abs()).fold(0.0_f32, f32::max);
        assert!(
            (peak - 0.9).abs() < 0.01,
            "Peak should be ~0.9, got {peak}"
        );
    }

    #[test]
    fn test_normalize_gain_silent() {
        let mut samples = vec![0.0, 0.0, 0.0];
        AudioPreprocessor::normalize_gain(&mut samples, 0.9);
        // ほぼ無音のまま — ゲインを上げすぎない
        assert_eq!(samples, vec![0.0, 0.0, 0.0]);
    }

    #[test]
    fn test_normalize_gain_empty() {
        let mut samples: Vec<f32> = vec![];
        AudioPreprocessor::normalize_gain(&mut samples, 0.9);
        assert!(samples.is_empty());
    }

    #[test]
    fn test_process_applies_both() {
        let mut samples = vec![0.6, 0.7, 0.8, 0.5];
        let config = PreprocessConfig::default();
        AudioPreprocessor::process(&mut samples, &config);

        // DC除去後は平均 ≈ 0
        let mean: f32 = samples.iter().sum::<f32>() / samples.len() as f32;
        assert!(mean.abs() < 1e-5, "DC offset should be ~0, got {mean}");

        // ゲイン正規化後はピーク ≈ target_peak
        let peak = samples.iter().map(|s| s.abs()).fold(0.0_f32, f32::max);
        assert!(
            (peak - config.target_peak).abs() < 0.05,
            "Peak should be ~{}, got {peak}",
            config.target_peak
        );
    }

    #[test]
    fn test_process_disabled() {
        let original = vec![0.6, 0.7, 0.8, 0.5];
        let mut samples = original.clone();
        let config = PreprocessConfig {
            remove_dc_offset: false,
            normalize_gain: false,
            target_peak: 0.9,
        };
        AudioPreprocessor::process(&mut samples, &config);
        assert_eq!(samples, original);
    }
}
