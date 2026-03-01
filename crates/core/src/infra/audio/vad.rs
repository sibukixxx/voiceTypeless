use crate::domain::settings::SttEngineChoice;

/// VAD（Voice Activity Detection）設定
#[derive(Debug, Clone)]
pub struct VadConfig {
    /// RMS エネルギーしきい値（これ以上で発話とみなす）
    pub energy_threshold: f32,
    /// 無音タイムアウト（ms）：この長さ無音が続いたらセグメント終了
    pub silence_timeout_ms: u64,
    /// セグメント最大長（ms）：強制カット
    pub max_segment_ms: u64,
    /// 発話開始に必要な連続音声時間（ms）：ヒステリシス
    pub speech_start_ms: u64,
    /// 動的しきい値を有効にする（キャリブレーション）
    pub adaptive_threshold: bool,
    /// キャリブレーション期間（ms）
    pub calibration_duration_ms: u64,
    /// ノイズフロアに対する倍率（動的しきい値 = noise_floor * multiplier）
    pub threshold_multiplier: f32,
    /// ZCR（ゼロクロッシング率）を発話判定に使用する
    pub use_zcr: bool,
    /// ZCR しきい値（これ以上で発話候補）
    pub zcr_threshold: f32,
}

impl Default for VadConfig {
    fn default() -> Self {
        Self {
            energy_threshold: 0.02,
            silence_timeout_ms: 700,
            max_segment_ms: 30_000,
            speech_start_ms: 50,
            adaptive_threshold: true,
            calibration_duration_ms: 2000,
            threshold_multiplier: 3.0,
            use_zcr: false,
            zcr_threshold: 0.3,
        }
    }
}

impl VadConfig {
    /// STT エンジンに応じた最適なデフォルト設定を返す
    pub fn for_engine(engine: SttEngineChoice) -> Self {
        match engine {
            SttEngineChoice::Whisper => Self {
                max_segment_ms: 20_000,
                ..Default::default()
            },
            SttEngineChoice::Apple => Self {
                max_segment_ms: 60_000,
                silence_timeout_ms: 1000,
                ..Default::default()
            },
            SttEngineChoice::Cloud => Self::default(),
        }
    }
}

/// VAD イベント
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VadEvent {
    /// 発話開始を検出
    SpeechStart,
    /// 発話終了を検出（無音タイムアウト）
    SpeechEnd,
    /// セグメント最大長に到達（強制カット）
    MaxLengthReached,
}

/// VAD 内部状態
enum VadState {
    /// キャリブレーション中（環境ノイズ計測）
    Calibrating {
        samples_rms: Vec<f32>,
        elapsed_ms: u64,
    },
    /// 無音状態
    Silence,
    /// 発話候補（確定前のヒステリシス）
    PendingSpeech { above_count_ms: u64 },
    /// 発話中
    Speech {
        duration_ms: u64,
        silence_count_ms: u64,
    },
}

/// RMS ベースの Voice Activity Detection プロセッサ
pub struct VadProcessor {
    config: VadConfig,
    state: VadState,
    sample_rate: u32,
    /// 動的しきい値（キャリブレーション後に設定）
    effective_threshold: Option<f32>,
}

impl VadProcessor {
    pub fn new(config: VadConfig, sample_rate: u32) -> Self {
        let initial_state = if config.adaptive_threshold {
            VadState::Calibrating {
                samples_rms: Vec::new(),
                elapsed_ms: 0,
            }
        } else {
            VadState::Silence
        };

        Self {
            config,
            state: initial_state,
            sample_rate,
            effective_threshold: None,
        }
    }

    /// 現在有効なしきい値を返す
    fn current_threshold(&self) -> f32 {
        self.effective_threshold
            .unwrap_or(self.config.energy_threshold)
    }

    /// サンプルの RMS（Root Mean Square）を計算
    pub fn rms(samples: &[f32]) -> f32 {
        if samples.is_empty() {
            return 0.0;
        }
        let sum: f32 = samples.iter().map(|s| s * s).sum();
        (sum / samples.len() as f32).sqrt()
    }

    /// ゼロクロッシング率を計算（0.0〜1.0）
    pub fn zcr(samples: &[f32]) -> f32 {
        if samples.len() < 2 {
            return 0.0;
        }
        let crossings = samples
            .windows(2)
            .filter(|w| (w[0] >= 0.0) != (w[1] >= 0.0))
            .count();
        crossings as f32 / (samples.len() - 1) as f32
    }

    /// サンプルを処理し、VAD イベントを返す
    pub fn process(&mut self, samples: &[f32]) -> Vec<VadEvent> {
        let rms = Self::rms(samples);
        let chunk_duration_ms = (samples.len() as u64 * 1000) / self.sample_rate as u64;

        // キャリブレーション中の処理
        if let VadState::Calibrating {
            ref mut samples_rms,
            ref mut elapsed_ms,
        } = self.state
        {
            samples_rms.push(rms);
            *elapsed_ms += chunk_duration_ms;

            if *elapsed_ms >= self.config.calibration_duration_ms {
                // ノイズフロアを計算（RMS の平均）
                let noise_floor = if samples_rms.is_empty() {
                    0.0
                } else {
                    samples_rms.iter().sum::<f32>() / samples_rms.len() as f32
                };
                let dynamic_threshold = noise_floor * self.config.threshold_multiplier;
                // 最低限 energy_threshold は下回らない
                self.effective_threshold =
                    Some(dynamic_threshold.max(self.config.energy_threshold));
                log::info!(
                    "VAD calibration done: noise_floor={:.4}, threshold={:.4}",
                    noise_floor,
                    self.effective_threshold.unwrap()
                );
                self.state = VadState::Silence;
            }
            return Vec::new();
        }

        let threshold = self.current_threshold();
        let mut is_speech = rms > threshold;

        // ZCR 条件を AND 結合（use_zcr 有効時）
        if self.config.use_zcr && is_speech {
            let zcr_val = Self::zcr(samples);
            is_speech = is_speech && zcr_val >= self.config.zcr_threshold;
        }

        let mut events = Vec::new();

        match &mut self.state {
            VadState::Calibrating { .. } => unreachable!(),
            VadState::Silence => {
                if is_speech {
                    self.state = VadState::PendingSpeech {
                        above_count_ms: chunk_duration_ms,
                    };
                }
            }
            VadState::PendingSpeech { above_count_ms } => {
                if is_speech {
                    *above_count_ms += chunk_duration_ms;
                    if *above_count_ms >= self.config.speech_start_ms {
                        events.push(VadEvent::SpeechStart);
                        self.state = VadState::Speech {
                            duration_ms: *above_count_ms,
                            silence_count_ms: 0,
                        };
                    }
                } else {
                    // ヒステリシス中にしきい値を下回った：発話ではなかった
                    self.state = VadState::Silence;
                }
            }
            VadState::Speech {
                duration_ms,
                silence_count_ms,
            } => {
                *duration_ms += chunk_duration_ms;

                // セグメント最大長チェック
                if *duration_ms >= self.config.max_segment_ms {
                    events.push(VadEvent::MaxLengthReached);
                    self.state = VadState::Silence;
                    return events;
                }

                if is_speech {
                    *silence_count_ms = 0;
                } else {
                    *silence_count_ms += chunk_duration_ms;
                    if *silence_count_ms >= self.config.silence_timeout_ms {
                        events.push(VadEvent::SpeechEnd);
                        self.state = VadState::Silence;
                    }
                }
            }
        }

        events
    }

    /// VAD 状態をリセット（effective_threshold は保持）
    pub fn reset(&mut self) {
        self.state = VadState::Silence;
    }

    /// 現在発話中かどうか
    pub fn is_in_speech(&self) -> bool {
        matches!(self.state, VadState::Speech { .. })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_samples(rms_target: f32, count: usize) -> Vec<f32> {
        // 一定振幅のサンプルを生成（RMS ≈ amplitude / sqrt(2) for sine, = amplitude for DC）
        vec![rms_target; count]
    }

    /// テスト用デフォルト設定（動的しきい値・ZCR 無効）
    fn test_config() -> VadConfig {
        VadConfig {
            adaptive_threshold: false,
            use_zcr: false,
            ..Default::default()
        }
    }

    #[test]
    fn test_rms_calculation() {
        let samples = vec![0.1, -0.1, 0.1, -0.1];
        let rms = VadProcessor::rms(&samples);
        assert!((rms - 0.1).abs() < 0.001);
    }

    #[test]
    fn test_rms_empty() {
        assert_eq!(VadProcessor::rms(&[]), 0.0);
    }

    #[test]
    fn test_zcr_calculation() {
        // 4 samples, 3 windows, 2 crossings: [0.1, -0.1], [-0.1, 0.1]
        let samples = vec![0.1, -0.1, 0.1, -0.1];
        let zcr = VadProcessor::zcr(&samples);
        // 3 windows, all cross zero → zcr = 3/3 = 1.0
        assert!((zcr - 1.0).abs() < 0.01, "zcr should be 1.0, got {zcr}");
    }

    #[test]
    fn test_zcr_no_crossings() {
        let samples = vec![0.1, 0.2, 0.3, 0.4];
        let zcr = VadProcessor::zcr(&samples);
        assert!(zcr.abs() < 0.01, "zcr should be 0, got {zcr}");
    }

    #[test]
    fn test_zcr_empty() {
        assert_eq!(VadProcessor::zcr(&[]), 0.0);
        assert_eq!(VadProcessor::zcr(&[0.1]), 0.0);
    }

    #[test]
    fn test_silence_no_events() {
        let config = VadConfig {
            energy_threshold: 0.01,
            ..test_config()
        };
        let mut vad = VadProcessor::new(config, 16000);
        let samples = make_samples(0.001, 160); // 10ms at 16kHz, below threshold
        let events = vad.process(&samples);
        assert!(events.is_empty());
    }

    #[test]
    fn test_speech_start_with_hysteresis() {
        let config = VadConfig {
            energy_threshold: 0.01,
            speech_start_ms: 30,
            ..test_config()
        };
        let mut vad = VadProcessor::new(config, 16000);
        let loud = make_samples(0.05, 160); // 10ms

        // First chunk: goes to PendingSpeech
        let events = vad.process(&loud);
        assert!(events.is_empty());

        // Second chunk: still pending
        let events = vad.process(&loud);
        assert!(events.is_empty());

        // Third chunk: 30ms reached → SpeechStart
        let events = vad.process(&loud);
        assert_eq!(events, vec![VadEvent::SpeechStart]);
    }

    #[test]
    fn test_speech_end_on_silence_timeout() {
        let config = VadConfig {
            energy_threshold: 0.01,
            speech_start_ms: 10,
            silence_timeout_ms: 30,
            ..test_config()
        };
        let mut vad = VadProcessor::new(config, 16000);
        let loud = make_samples(0.05, 160);
        let quiet = make_samples(0.001, 160);

        // Start speech
        vad.process(&loud);
        vad.process(&loud); // SpeechStart

        // Silence for 30ms
        vad.process(&quiet); // 10ms silence
        vad.process(&quiet); // 20ms silence
        let events = vad.process(&quiet); // 30ms → SpeechEnd
        assert_eq!(events, vec![VadEvent::SpeechEnd]);
    }

    #[test]
    fn test_max_length_reached() {
        let config = VadConfig {
            energy_threshold: 0.01,
            speech_start_ms: 10,
            max_segment_ms: 50,
            silence_timeout_ms: 700,
            ..test_config()
        };
        let mut vad = VadProcessor::new(config, 16000);
        let loud = make_samples(0.05, 160); // 10ms

        // Start speech (10ms hysteresis)
        vad.process(&loud);

        // Continue until max length
        vad.process(&loud); // 20ms (SpeechStart at this point)
        vad.process(&loud); // 30ms
        vad.process(&loud); // 40ms
        let events = vad.process(&loud); // 50ms → MaxLengthReached
        assert_eq!(events, vec![VadEvent::MaxLengthReached]);
    }

    #[test]
    fn test_pending_speech_cancelled_by_silence() {
        let config = VadConfig {
            energy_threshold: 0.01,
            speech_start_ms: 30,
            ..test_config()
        };
        let mut vad = VadProcessor::new(config, 16000);
        let loud = make_samples(0.05, 160);
        let quiet = make_samples(0.001, 160);

        // Start pending
        vad.process(&loud);
        // Interrupted by silence
        let events = vad.process(&quiet);
        assert!(events.is_empty());
        assert!(!vad.is_in_speech());
    }

    #[test]
    fn test_calibration_to_silence_transition() {
        let config = VadConfig {
            adaptive_threshold: true,
            calibration_duration_ms: 30, // 短いキャリブレーション
            threshold_multiplier: 3.0,
            energy_threshold: 0.01,
            ..test_config()
        };
        // adaptive_threshold を明示的に true にオーバーライド
        let config = VadConfig {
            adaptive_threshold: true,
            ..config
        };
        let mut vad = VadProcessor::new(config, 16000);

        // キャリブレーション中（ノイズ RMS = 0.005）
        let noise = make_samples(0.005, 160); // 10ms
        let events = vad.process(&noise);
        assert!(events.is_empty());
        assert!(vad.effective_threshold.is_none());

        vad.process(&noise); // 20ms
        let events = vad.process(&noise); // 30ms → キャリブレーション完了
        assert!(events.is_empty());

        // effective_threshold = 0.005 * 3.0 = 0.015 (> energy_threshold 0.01)
        let threshold = vad.effective_threshold.unwrap();
        assert!(
            (threshold - 0.015).abs() < 0.001,
            "Expected ~0.015, got {threshold}"
        );
    }

    #[test]
    fn test_calibration_noise_floor() {
        let config = VadConfig {
            adaptive_threshold: true,
            calibration_duration_ms: 20,
            threshold_multiplier: 2.0,
            energy_threshold: 0.005,
            ..test_config()
        };
        let config = VadConfig {
            adaptive_threshold: true,
            ..config
        };
        let mut vad = VadProcessor::new(config, 16000);

        let noise = make_samples(0.003, 160); // 10ms
        vad.process(&noise);
        vad.process(&noise); // 20ms → キャリブレーション完了

        // threshold = max(0.003 * 2.0, 0.005) = max(0.006, 0.005) = 0.006
        let threshold = vad.effective_threshold.unwrap();
        assert!(
            (threshold - 0.006).abs() < 0.001,
            "Expected ~0.006, got {threshold}"
        );
    }

    #[test]
    fn test_reset_preserves_threshold() {
        let config = VadConfig {
            adaptive_threshold: false,
            ..test_config()
        };
        let mut vad = VadProcessor::new(config, 16000);
        vad.effective_threshold = Some(0.05);

        vad.reset();
        assert_eq!(vad.effective_threshold, Some(0.05));
        assert!(!vad.is_in_speech());
    }

    #[test]
    fn test_for_engine_whisper() {
        let config = VadConfig::for_engine(SttEngineChoice::Whisper);
        assert_eq!(config.max_segment_ms, 20_000);
    }

    #[test]
    fn test_for_engine_apple() {
        let config = VadConfig::for_engine(SttEngineChoice::Apple);
        assert_eq!(config.max_segment_ms, 60_000);
        assert_eq!(config.silence_timeout_ms, 1000);
    }
}
