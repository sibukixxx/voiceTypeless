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
}

impl Default for VadConfig {
    fn default() -> Self {
        Self {
            energy_threshold: 0.02,
            silence_timeout_ms: 700,
            max_segment_ms: 30_000,
            speech_start_ms: 50,
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
}

impl VadProcessor {
    pub fn new(config: VadConfig, sample_rate: u32) -> Self {
        Self {
            config,
            state: VadState::Silence,
            sample_rate,
        }
    }

    /// サンプルの RMS（Root Mean Square）を計算
    pub fn rms(samples: &[f32]) -> f32 {
        if samples.is_empty() {
            return 0.0;
        }
        let sum: f32 = samples.iter().map(|s| s * s).sum();
        (sum / samples.len() as f32).sqrt()
    }

    /// サンプルを処理し、VAD イベントを返す
    pub fn process(&mut self, samples: &[f32]) -> Vec<VadEvent> {
        let rms = Self::rms(samples);
        let chunk_duration_ms = (samples.len() as u64 * 1000) / self.sample_rate as u64;
        let is_speech = rms > self.config.energy_threshold;

        let mut events = Vec::new();

        match &mut self.state {
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

    /// VAD 状態をリセット
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
    fn test_silence_no_events() {
        let config = VadConfig {
            energy_threshold: 0.01,
            ..Default::default()
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
            ..Default::default()
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
            ..Default::default()
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
            ..Default::default()
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
}
