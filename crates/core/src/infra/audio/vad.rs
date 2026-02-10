use std::path::PathBuf;
use uuid::Uuid;

use crate::domain::stt::{AudioSegment, PcmFormat};

/// VAD (Voice Activity Detection) の設定。
/// Serde対応でSettings UIからの設定変更に対応。
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct VadConfig {
    /// 発話開始判定の RMS 閾値 (デフォルト: 0.02)
    pub speech_start_threshold: f32,
    /// 発話終了判定の RMS 閾値 (デフォルト: 0.01)。
    /// start_threshold より低く設定してヒステリシスを実現する。
    pub speech_end_threshold: f32,
    /// 発話終了と判定する無音継続時間 (ms, デフォルト: 700)
    pub speech_end_silence_ms: u32,
    /// セグメント最大長 (ms, デフォルト: 30000)。超過時は強制カット。
    pub max_segment_ms: u32,
    /// セグメント最小長 (ms, デフォルト: 500)。短すぎるセグメントは破棄。
    pub min_segment_ms: u32,
    /// 発話開始確認に必要な最小発話継続時間 (ms, デフォルト: 100)。
    /// 短いノイズ（クリック、咳等）を無視する。
    pub min_speech_ms: u32,
    /// セグメント間の最小ギャップ (ms, デフォルト: 300)。
    /// 短い無音での過剰分割を防止する。セグメント確定後、この時間内の新しい発話は
    /// 無視される（VADのデバウンス）。
    pub min_gap_ms: u32,
    /// サンプルレート (Hz, デフォルト: 16000)
    pub sample_rate: u32,
    /// フレームサイズ (サンプル数, デフォルト: 320 = 20ms @ 16kHz)
    pub frame_size: usize,
    /// WAVセグメントファイルの出力ディレクトリ
    #[serde(default = "default_output_dir")]
    pub output_dir: PathBuf,
}

fn default_output_dir() -> PathBuf {
    std::env::temp_dir().join("voicetypeless_segments")
}

impl Default for VadConfig {
    fn default() -> Self {
        Self {
            speech_start_threshold: 0.02,
            speech_end_threshold: 0.01,
            speech_end_silence_ms: 700,
            max_segment_ms: 30_000,
            min_segment_ms: 500,
            min_speech_ms: 100,
            min_gap_ms: 300,
            sample_rate: 16_000,
            frame_size: 320,
            output_dir: default_output_dir(),
        }
    }
}

/// VADが発するイベント。
#[derive(Debug)]
pub enum VadEvent {
    /// 発話検出（min_speech_ms を超えて確定した場合のみ）
    SpeechStart,
    /// セグメント確定 (文字起こし可能)
    SegmentReady(AudioSegment),
    /// セグメントが短すぎて破棄された
    SegmentDiscarded { duration_ms: u32 },
    /// 最大長超過による強制カット
    SegmentForceCut,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum VadState {
    /// 無音状態
    Silence,
    /// 発話候補検知中 (min_speech_ms 確認待ち)
    PendingSpeech,
    /// 発話確定中（録音中）
    Speaking,
    /// セグメント確定後の cooldown（min_gap_ms 待ち）
    Cooldown,
}

/// RMS閾値ベースのVADプロセッサ。フレームを入力するとイベントを返す。
///
/// 状態遷移:
///   Silence → PendingSpeech (RMS >= start_threshold)
///   PendingSpeech → Speaking (min_speech_ms 継続)
///   PendingSpeech → Silence (閾値を下回った → ノイズとして無視)
///   Speaking → Silence (無音タイムアウト → finalize)
///   Speaking → Silence (max_segment_ms 超過 → force cut)
///   Silence/Speaking → Cooldown (セグメント確定後)
///   Cooldown → Silence (min_gap_ms 経過)
pub struct VadProcessor {
    config: VadConfig,
    state: VadState,
    /// 現在のセグメントのサンプル蓄積バッファ
    segment_samples: Vec<f32>,
    /// 現在のセグメントの開始時刻
    segment_started_at: Option<chrono::DateTime<chrono::Utc>>,
    /// 連続無音フレーム数 (Speaking 状態中)
    silence_frame_count: u32,
    /// PendingSpeech 状態の継続フレーム数
    pending_frame_count: u32,
    /// Cooldown 残りフレーム数
    cooldown_frame_count: u32,
    /// 無音タイムアウトに必要なフレーム数
    silence_frames_needed: u32,
    /// 最大セグメント長に対応するフレーム数
    max_frames: u32,
    /// min_speech_ms に対応するフレーム数
    min_speech_frames: u32,
    /// min_gap_ms に対応するフレーム数
    min_gap_frames: u32,
    /// 現在のセグメントのフレーム数
    segment_frame_count: u32,
    /// セグメントに付与する言語ヒント
    language: Option<String>,
    /// セグメントに付与する認識ヒント
    hints: Vec<String>,
    /// PendingSpeech 中に蓄積する pre-buffer
    pending_samples: Vec<f32>,
}

impl VadProcessor {
    pub fn new(config: VadConfig) -> Self {
        let frame_duration_ms =
            (config.frame_size as f32 / config.sample_rate as f32 * 1000.0) as u32;
        let fd = frame_duration_ms.max(1);
        let silence_frames_needed = config.speech_end_silence_ms / fd;
        let max_frames = config.max_segment_ms / fd;
        let min_speech_frames = config.min_speech_ms / fd;
        let min_gap_frames = config.min_gap_ms / fd;

        // Pre-allocate for typical segment (max 30s @ 16kHz = 480k samples)
        let prealloc_samples = (config.sample_rate as usize) * (config.max_segment_ms as usize / 1000).min(10);
        let pending_capacity = config.frame_size * (min_speech_frames as usize + 1);

        Self {
            config,
            state: VadState::Silence,
            segment_samples: Vec::with_capacity(prealloc_samples),
            segment_started_at: None,
            silence_frame_count: 0,
            pending_frame_count: 0,
            cooldown_frame_count: 0,
            silence_frames_needed,
            max_frames,
            min_speech_frames,
            min_gap_frames,
            segment_frame_count: 0,
            language: None,
            hints: Vec::new(),
            pending_samples: Vec::with_capacity(pending_capacity),
        }
    }

    /// 今後のセグメントに付与する言語ヒントを設定する。
    pub fn set_language(&mut self, lang: Option<String>) {
        self.language = lang;
    }

    /// 今後のセグメントに付与する認識ヒントを設定する。
    pub fn set_hints(&mut self, hints: Vec<String>) {
        self.hints = hints;
    }

    /// 設定を動的に更新する。次のセグメントから反映。
    pub fn update_config(&mut self, config: VadConfig) {
        let frame_duration_ms =
            (config.frame_size as f32 / config.sample_rate as f32 * 1000.0) as u32;
        let fd = frame_duration_ms.max(1);
        self.silence_frames_needed = config.speech_end_silence_ms / fd;
        self.max_frames = config.max_segment_ms / fd;
        self.min_speech_frames = config.min_speech_ms / fd;
        self.min_gap_frames = config.min_gap_ms / fd;
        self.config = config;
    }

    /// 1フレームの音声を処理する。0個以上のイベントを返す。
    pub fn process_frame(&mut self, frame: &[f32], rms: f32) -> Vec<VadEvent> {
        let mut events = Vec::new();

        match self.state {
            VadState::Silence => {
                if rms >= self.config.speech_start_threshold {
                    if self.min_speech_frames == 0 {
                        // min_speech_ms が 0 なら即座に Speaking
                        self.state = VadState::Speaking;
                        self.segment_samples.clear();
                        self.segment_samples.extend_from_slice(frame);
                        self.segment_started_at = Some(chrono::Utc::now());
                        self.silence_frame_count = 0;
                        self.segment_frame_count = 1;
                        events.push(VadEvent::SpeechStart);
                    } else {
                        // PendingSpeech に遷移して確認待ち
                        self.state = VadState::PendingSpeech;
                        self.pending_samples.clear();
                        self.pending_samples.extend_from_slice(frame);
                        self.pending_frame_count = 1;
                    }
                }
            }
            VadState::PendingSpeech => {
                self.pending_samples.extend_from_slice(frame);
                self.pending_frame_count += 1;

                if rms < self.config.speech_end_threshold {
                    // 閾値を下回った → ノイズとして無視
                    self.state = VadState::Silence;
                    self.pending_samples.clear();
                    self.pending_frame_count = 0;
                } else if self.pending_frame_count >= self.min_speech_frames {
                    // min_speech_ms を超えた → Speaking に確定
                    self.state = VadState::Speaking;
                    self.segment_samples.clear();
                    self.segment_samples.append(&mut self.pending_samples);
                    self.segment_started_at = Some(chrono::Utc::now());
                    self.silence_frame_count = 0;
                    self.segment_frame_count = self.pending_frame_count;
                    self.pending_frame_count = 0;
                    events.push(VadEvent::SpeechStart);
                }
            }
            VadState::Speaking => {
                self.segment_samples.extend_from_slice(frame);
                self.segment_frame_count += 1;

                if rms < self.config.speech_end_threshold {
                    self.silence_frame_count += 1;
                } else {
                    self.silence_frame_count = 0;
                }

                // 最大セグメント長の強制カット
                if self.segment_frame_count >= self.max_frames {
                    events.push(VadEvent::SegmentForceCut);
                    events.extend(self.finalize_segment());
                    return events;
                }

                // 無音タイムアウトによるセグメント確定
                if self.silence_frame_count >= self.silence_frames_needed {
                    events.extend(self.finalize_segment());
                }
            }
            VadState::Cooldown => {
                self.cooldown_frame_count += 1;
                if self.cooldown_frame_count >= self.min_gap_frames {
                    self.state = VadState::Silence;
                    self.cooldown_frame_count = 0;
                }
            }
        }

        events
    }

    /// 現在のセグメントを確定する。WAV書き出し → AudioSegment 生成 → 状態リセット。
    fn finalize_segment(&mut self) -> Vec<VadEvent> {
        let mut events = Vec::new();
        let samples = std::mem::take(&mut self.segment_samples);
        let started_at = self
            .segment_started_at
            .take()
            .unwrap_or_else(chrono::Utc::now);

        self.silence_frame_count = 0;
        self.segment_frame_count = 0;

        // Cooldown に遷移（min_gap_ms > 0 の場合）
        if self.min_gap_frames > 0 {
            self.state = VadState::Cooldown;
            self.cooldown_frame_count = 0;
        } else {
            self.state = VadState::Silence;
        }

        let duration_ms = if self.config.sample_rate > 0 {
            ((samples.len() as f64 / self.config.sample_rate as f64) * 1000.0) as u32
        } else {
            0
        };

        // 短すぎるセグメントは破棄
        if duration_ms < self.config.min_segment_ms {
            events.push(VadEvent::SegmentDiscarded { duration_ms });
            return events;
        }

        let segment_id = Uuid::new_v4();
        match self.write_wav(&segment_id, &samples) {
            Ok(wav_path) => {
                let segment = AudioSegment {
                    id: segment_id,
                    started_at,
                    duration_ms,
                    sample_rate: self.config.sample_rate,
                    channels: 1,
                    pcm_format: PcmFormat::F32Le,
                    wav_path: Some(wav_path),
                    samples: Some(samples),
                    language: self.language.clone(),
                    hints: self.hints.clone(),
                };
                events.push(VadEvent::SegmentReady(segment));
            }
            Err(e) => {
                log::error!(
                    "Failed to write WAV file for segment {}: {}",
                    segment_id,
                    e
                );
                let segment = AudioSegment {
                    id: segment_id,
                    started_at,
                    duration_ms,
                    sample_rate: self.config.sample_rate,
                    channels: 1,
                    pcm_format: PcmFormat::F32Le,
                    wav_path: None,
                    samples: Some(samples),
                    language: self.language.clone(),
                    hints: self.hints.clone(),
                };
                events.push(VadEvent::SegmentReady(segment));
            }
        }

        events
    }

    /// 録音停止時に現在のバッファを強制セグメント化する。
    pub fn flush(&mut self) -> Vec<VadEvent> {
        match self.state {
            VadState::Speaking if !self.segment_samples.is_empty() => self.finalize_segment(),
            VadState::PendingSpeech if !self.pending_samples.is_empty() => {
                // PendingSpeech 中のバッファも Speaking として確定試行
                self.segment_samples = std::mem::take(&mut self.pending_samples);
                self.segment_started_at = Some(chrono::Utc::now());
                self.state = VadState::Speaking;
                self.finalize_segment()
            }
            _ => Vec::new(),
        }
    }

    /// 全状態をリセットする。
    pub fn reset(&mut self) {
        self.state = VadState::Silence;
        self.segment_samples.clear();
        self.pending_samples.clear();
        self.segment_started_at = None;
        self.silence_frame_count = 0;
        self.pending_frame_count = 0;
        self.cooldown_frame_count = 0;
        self.segment_frame_count = 0;
    }

    /// 現在の VAD 状態を返す（デバッグ/監視用）。
    pub fn is_speaking(&self) -> bool {
        matches!(self.state, VadState::Speaking | VadState::PendingSpeech)
    }

    /// サンプルをWAVファイルとして output_dir に書き出す。
    /// BufWriter を使用してディスクI/Oを最小化する。
    fn write_wav(&self, segment_id: &Uuid, samples: &[f32]) -> Result<PathBuf, String> {
        std::fs::create_dir_all(&self.config.output_dir)
            .map_err(|e| format!("Failed to create output dir: {}", e))?;

        let filename = format!("{}.wav", segment_id);
        let path = self.config.output_dir.join(filename);

        let spec = hound::WavSpec {
            channels: 1,
            sample_rate: self.config.sample_rate,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };

        let file = std::fs::File::create(&path)
            .map_err(|e| format!("Failed to create WAV file: {}", e))?;
        let buf_writer = std::io::BufWriter::with_capacity(64 * 1024, file);
        let mut writer = hound::WavWriter::new(buf_writer, spec)
            .map_err(|e| format!("Failed to create WAV writer: {}", e))?;

        for &sample in samples {
            let s =
                (sample * i16::MAX as f32).clamp(i16::MIN as f32, i16::MAX as f32) as i16;
            writer
                .write_sample(s)
                .map_err(|e| format!("Failed to write sample: {}", e))?;
        }

        writer
            .finalize()
            .map_err(|e| format!("Failed to finalize WAV: {}", e))?;

        Ok(path)
    }

    /// output_dir 内の古いWAVファイルを削除する。
    /// `max_age` より古いファイルを削除。
    pub fn cleanup_old_segments(&self, max_age: std::time::Duration) -> u32 {
        let mut removed = 0u32;
        let entries = match std::fs::read_dir(&self.config.output_dir) {
            Ok(e) => e,
            Err(_) => return 0,
        };

        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("wav") {
                continue;
            }
            if let Ok(metadata) = entry.metadata() {
                if let Ok(modified) = metadata.modified() {
                    if let Ok(age) = modified.elapsed() {
                        if age > max_age
                            && std::fs::remove_file(&path).is_ok()
                        {
                            removed += 1;
                        }
                    }
                }
            }
        }
        removed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_config(output_dir: PathBuf) -> VadConfig {
        VadConfig {
            speech_start_threshold: 0.02,
            speech_end_threshold: 0.01,
            speech_end_silence_ms: 700,
            max_segment_ms: 30_000,
            min_segment_ms: 500,
            min_speech_ms: 0, // テストではデフォルトで無効化
            min_gap_ms: 0,    // テストではデフォルトで無効化
            sample_rate: 16_000,
            frame_size: 320,
            output_dir,
        }
    }

    fn silent_frame() -> Vec<f32> {
        vec![0.0; 320]
    }

    fn loud_frame(amplitude: f32) -> Vec<f32> {
        vec![amplitude; 320]
    }

    #[test]
    fn silence_produces_no_events() {
        let dir = std::env::temp_dir().join("vad_test_silence");
        let mut vad = VadProcessor::new(make_config(dir));

        for _ in 0..100 {
            let events = vad.process_frame(&silent_frame(), 0.0);
            assert!(events.is_empty());
        }
    }

    #[test]
    fn loud_frame_triggers_speech_start() {
        let dir = std::env::temp_dir().join("vad_test_speech_start");
        let mut vad = VadProcessor::new(make_config(dir));

        let events = vad.process_frame(&loud_frame(0.1), 0.1);
        assert_eq!(events.len(), 1);
        assert!(matches!(events[0], VadEvent::SpeechStart));
    }

    #[test]
    fn speech_then_silence_produces_segment() {
        let dir = std::env::temp_dir().join("vad_test_segment");
        let _ = std::fs::remove_dir_all(&dir);
        let mut vad = VadProcessor::new(make_config(dir));

        for _ in 0..50 {
            vad.process_frame(&loud_frame(0.1), 0.1);
        }

        let mut got_segment = false;
        for _ in 0..40 {
            let events = vad.process_frame(&silent_frame(), 0.005);
            for event in &events {
                if matches!(event, VadEvent::SegmentReady(_)) {
                    got_segment = true;
                }
            }
        }
        assert!(got_segment, "Expected SegmentReady after silence timeout");
    }

    #[test]
    fn short_segment_is_discarded() {
        let dir = std::env::temp_dir().join("vad_test_discard");
        let _ = std::fs::remove_dir_all(&dir);
        let mut vad = VadProcessor::new(make_config(dir));

        for _ in 0..10 {
            vad.process_frame(&loud_frame(0.1), 0.1);
        }

        let events = vad.flush();
        assert!(
            events.iter().any(|e| matches!(e, VadEvent::SegmentDiscarded { .. })),
            "Expected SegmentDiscarded for short burst via flush"
        );
    }

    #[test]
    fn max_segment_force_cut() {
        let dir = std::env::temp_dir().join("vad_test_forcecut");
        let _ = std::fs::remove_dir_all(&dir);
        let mut config = make_config(dir);
        config.max_segment_ms = 1000;
        let mut vad = VadProcessor::new(config);

        let mut force_cut = false;
        let mut segment_ready = false;
        for _ in 0..55 {
            let events = vad.process_frame(&loud_frame(0.1), 0.1);
            for event in &events {
                if matches!(event, VadEvent::SegmentForceCut) {
                    force_cut = true;
                }
                if matches!(event, VadEvent::SegmentReady(_)) {
                    segment_ready = true;
                }
            }
        }
        assert!(force_cut, "Expected SegmentForceCut");
        assert!(segment_ready, "Expected SegmentReady after force cut");
    }

    #[test]
    fn hysteresis_prevents_false_start() {
        let dir = std::env::temp_dir().join("vad_test_hysteresis");
        let mut vad = VadProcessor::new(make_config(dir));

        let events = vad.process_frame(&loud_frame(0.015), 0.015);
        assert!(events.is_empty(), "Should not start speech below start_threshold");
    }

    #[test]
    fn flush_emits_current_segment() {
        let dir = std::env::temp_dir().join("vad_test_flush");
        let _ = std::fs::remove_dir_all(&dir);
        let mut vad = VadProcessor::new(make_config(dir));

        for _ in 0..100 {
            vad.process_frame(&loud_frame(0.1), 0.1);
        }

        let events = vad.flush();
        assert!(!events.is_empty());
        assert!(
            events.iter().any(|e| matches!(e, VadEvent::SegmentReady(_))),
            "flush should emit SegmentReady"
        );
    }

    #[test]
    fn flush_when_silent_is_empty() {
        let dir = std::env::temp_dir().join("vad_test_flush_empty");
        let mut vad = VadProcessor::new(make_config(dir));
        let events = vad.flush();
        assert!(events.is_empty());
    }

    #[test]
    fn wav_file_is_valid() {
        let dir = std::env::temp_dir().join("vad_test_wav_valid");
        let _ = std::fs::remove_dir_all(&dir);
        let mut vad = VadProcessor::new(make_config(dir.clone()));

        for _ in 0..50 {
            vad.process_frame(&loud_frame(0.1), 0.1);
        }

        let events = vad.flush();
        let segment = events.into_iter().find_map(|e| match e {
            VadEvent::SegmentReady(seg) => Some(seg),
            _ => None,
        });

        let segment = segment.expect("Expected a segment");
        let wav_path = segment.wav_path.expect("Expected wav_path");
        assert!(wav_path.exists(), "WAV file should exist");

        let reader = hound::WavReader::open(&wav_path).expect("Should be valid WAV");
        assert_eq!(reader.spec().sample_rate, 16000);
        assert_eq!(reader.spec().channels, 1);
        assert_eq!(reader.spec().bits_per_sample, 16);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn reset_clears_state() {
        let dir = std::env::temp_dir().join("vad_test_reset");
        let mut vad = VadProcessor::new(make_config(dir));

        vad.process_frame(&loud_frame(0.1), 0.1);
        vad.reset();

        let events = vad.flush();
        assert!(events.is_empty());
    }

    // ─── Phase 2 テスト ─────────────────────────────

    #[test]
    fn min_speech_ms_filters_short_noise() {
        let dir = std::env::temp_dir().join("vad_test_min_speech");
        let mut config = make_config(dir);
        config.min_speech_ms = 200; // 200ms = 10フレーム必要
        let mut vad = VadProcessor::new(config);

        // 2フレーム(40ms)だけ発話 → min_speech_ms(200ms) に足りない
        vad.process_frame(&loud_frame(0.1), 0.1);
        vad.process_frame(&loud_frame(0.1), 0.1);

        // 無音に戻る → PendingSpeech から Silence に落ちる
        let events = vad.process_frame(&silent_frame(), 0.005);
        // SpeechStart は出ていないはず
        assert!(events.is_empty());
        assert!(!vad.is_speaking());
    }

    #[test]
    fn min_speech_ms_allows_long_speech() {
        let dir = std::env::temp_dir().join("vad_test_min_speech_ok");
        let mut config = make_config(dir);
        config.min_speech_ms = 100; // 100ms = 5フレーム
        let mut vad = VadProcessor::new(config);

        let mut speech_started = false;
        for _ in 0..10 {
            let events = vad.process_frame(&loud_frame(0.1), 0.1);
            for event in &events {
                if matches!(event, VadEvent::SpeechStart) {
                    speech_started = true;
                }
            }
        }
        assert!(speech_started, "Should emit SpeechStart after min_speech_ms");
        assert!(vad.is_speaking());
    }

    #[test]
    fn min_gap_ms_prevents_rapid_segments() {
        let dir = std::env::temp_dir().join("vad_test_min_gap");
        let _ = std::fs::remove_dir_all(&dir);
        let mut config = make_config(dir);
        config.min_gap_ms = 200; // 200ms = 10フレーム の cooldown

        let mut vad = VadProcessor::new(config);

        // 1秒間の発話
        for _ in 0..50 {
            vad.process_frame(&loud_frame(0.1), 0.1);
        }
        // 無音で確定
        for _ in 0..40 {
            vad.process_frame(&silent_frame(), 0.005);
        }

        // cooldown 中にすぐ発話しても SpeechStart は出ない
        let events = vad.process_frame(&loud_frame(0.1), 0.1);
        assert!(
            !events.iter().any(|e| matches!(e, VadEvent::SpeechStart)),
            "Should not emit SpeechStart during cooldown"
        );
    }

    #[test]
    fn cooldown_expires_and_allows_new_speech() {
        let dir = std::env::temp_dir().join("vad_test_cooldown_expire");
        let _ = std::fs::remove_dir_all(&dir);
        let mut config = make_config(dir);
        config.min_gap_ms = 100; // 100ms = 5フレーム
        config.speech_end_silence_ms = 100; // 100ms = 5フレーム

        let mut vad = VadProcessor::new(config);

        // 発話 → 確定
        for _ in 0..50 {
            vad.process_frame(&loud_frame(0.1), 0.1);
        }
        for _ in 0..10 {
            vad.process_frame(&silent_frame(), 0.005);
        }

        // cooldown 消化（5フレーム以上）
        for _ in 0..10 {
            vad.process_frame(&silent_frame(), 0.005);
        }

        // 新しい発話が検出されるはず
        let events = vad.process_frame(&loud_frame(0.1), 0.1);
        assert!(
            events.iter().any(|e| matches!(e, VadEvent::SpeechStart)),
            "Should emit SpeechStart after cooldown expired"
        );
    }

    #[test]
    fn update_config_changes_thresholds() {
        let dir = std::env::temp_dir().join("vad_test_update_config");
        let mut vad = VadProcessor::new(make_config(dir.clone()));

        // デフォルト threshold: 0.02 → 0.015 では反応しない
        let events = vad.process_frame(&loud_frame(0.015), 0.015);
        assert!(events.is_empty());

        // 閾値を下げて更新
        let mut new_config = make_config(dir);
        new_config.speech_start_threshold = 0.01;
        vad.update_config(new_config);

        // 0.015 で反応するはず
        let events = vad.process_frame(&loud_frame(0.015), 0.015);
        assert!(events.iter().any(|e| matches!(e, VadEvent::SpeechStart)));
    }

    #[test]
    fn config_serialization() {
        let config = VadConfig::default();
        let json = serde_json::to_string_pretty(&config).unwrap();
        let config2: VadConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(config.speech_start_threshold, config2.speech_start_threshold);
        assert_eq!(config.min_speech_ms, config2.min_speech_ms);
        assert_eq!(config.min_gap_ms, config2.min_gap_ms);
    }
}
