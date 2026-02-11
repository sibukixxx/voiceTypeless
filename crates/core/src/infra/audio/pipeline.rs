use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::sync::Arc;
use std::thread;

use super::capture::{self, AudioCaptureError};
use super::vad::{VadConfig, VadEvent, VadProcessor};
use crate::infra::stt::{AudioSegment, SttContext, SttEngine};

/// パイプラインイベント（Tauri イベントに変換される）
#[derive(Debug, Clone)]
pub enum PipelineEvent {
    /// マイク入力レベル（RMS）
    AudioLevel(f32),
    /// 部分的な書き起こし結果（ストリーミング STT 用）
    TranscriptPartial { text: String },
    /// 確定した書き起こし結果
    TranscriptFinal { text: String, confidence: f32 },
    /// パイプラインエラー
    Error(String),
}

/// AudioPipeline: capture → VAD → STT → イベント発火のオーケストレータ
///
/// cpal::Stream は Send ではないため、AudioCapture は処理スレッド内で作成・保持する。
/// AudioPipeline 自体は Send + Sync で、Tauri State に格納できる。
pub struct AudioPipeline {
    stop_flag: Arc<AtomicBool>,
    process_thread: Option<thread::JoinHandle<()>>,
}

// AudioPipeline は stop_flag (Arc<AtomicBool>) と JoinHandle だけなので Send + Sync
unsafe impl Send for AudioPipeline {}
unsafe impl Sync for AudioPipeline {}

impl AudioPipeline {
    /// パイプラインを開始する
    ///
    /// まずデバイスの存在を確認し（エラーなら即座に返す）、
    /// その後バックグラウンドスレッドで capture → VAD → STT を処理する。
    pub fn start(
        stt_engine: Arc<dyn SttEngine>,
        event_tx: mpsc::Sender<PipelineEvent>,
        vad_config: VadConfig,
    ) -> Result<Self, AudioCaptureError> {
        // デバイスの事前チェック（高速にエラー検出）
        let _config = capture::check_device()?;

        let stop_flag = Arc::new(AtomicBool::new(false));
        let stop_flag_clone = stop_flag.clone();

        let process_thread = thread::spawn(move || {
            Self::processing_loop(
                stop_flag_clone,
                stt_engine,
                event_tx,
                vad_config,
            );
        });

        Ok(Self {
            stop_flag,
            process_thread: Some(process_thread),
        })
    }

    /// パイプラインを停止する（最終セグメントの処理完了まで待機）
    pub fn stop(&mut self) {
        self.stop_flag.store(true, Ordering::Relaxed);

        if let Some(thread) = self.process_thread.take() {
            let _ = thread.join();
        }
    }

    /// 処理ループ（バックグラウンドスレッドで実行）
    ///
    /// cpal::Stream はこのスレッド上で作成し、スレッド終了時に drop される。
    fn processing_loop(
        stop_flag: Arc<AtomicBool>,
        stt_engine: Arc<dyn SttEngine>,
        event_tx: mpsc::Sender<PipelineEvent>,
        vad_config: VadConfig,
    ) {
        // このスレッド上でキャプチャを開始
        let (sample_tx, sample_rx) = mpsc::channel::<Vec<f32>>();
        let capture_config = match capture::start_capture(sample_tx, stop_flag.clone()) {
            Ok(config) => config,
            Err(e) => {
                let _ = event_tx.send(PipelineEvent::Error(format!(
                    "Failed to start audio capture: {}",
                    e
                )));
                return;
            }
        };

        let sample_rate = capture_config.sample_rate;
        let mut vad = VadProcessor::new(vad_config, sample_rate);
        let mut segment_buffer: Vec<f32> = Vec::new();

        // STT 呼び出し用の tokio ランタイム
        let rt = match tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
        {
            Ok(rt) => rt,
            Err(e) => {
                let _ = event_tx.send(PipelineEvent::Error(format!(
                    "Failed to create tokio runtime: {}",
                    e
                )));
                return;
            }
        };

        while !stop_flag.load(Ordering::Relaxed) {
            match sample_rx.recv_timeout(std::time::Duration::from_millis(100)) {
                Ok(samples) => {
                    // オーディオレベル送信
                    let rms = VadProcessor::rms(&samples);
                    let _ = event_tx.send(PipelineEvent::AudioLevel(rms));

                    // VAD 処理
                    let vad_events = vad.process(&samples);

                    // 発話中 or 発話開始 → バッファに蓄積
                    let speech_starting = vad_events
                        .iter()
                        .any(|e| matches!(e, VadEvent::SpeechStart));
                    if vad.is_in_speech() || speech_starting {
                        segment_buffer.extend_from_slice(&samples);
                    }

                    // VAD イベント処理
                    for vad_event in vad_events {
                        match vad_event {
                            VadEvent::SpeechEnd | VadEvent::MaxLengthReached => {
                                if !segment_buffer.is_empty() {
                                    Self::run_stt(
                                        &rt,
                                        &stt_engine,
                                        &event_tx,
                                        std::mem::take(&mut segment_buffer),
                                        sample_rate,
                                    );
                                }
                            }
                            VadEvent::SpeechStart => {}
                        }
                    }
                }
                Err(mpsc::RecvTimeoutError::Timeout) => continue,
                Err(mpsc::RecvTimeoutError::Disconnected) => break,
            }
        }

        // 残りのセグメントをフラッシュ
        if !segment_buffer.is_empty() {
            log::info!(
                "Flushing remaining segment: {} samples",
                segment_buffer.len()
            );
            Self::run_stt(
                &rt,
                &stt_engine,
                &event_tx,
                segment_buffer,
                sample_rate,
            );
        }
    }

    /// STT エンジンを呼び出し、結果をイベントとして送信
    fn run_stt(
        rt: &tokio::runtime::Runtime,
        stt_engine: &Arc<dyn SttEngine>,
        event_tx: &mpsc::Sender<PipelineEvent>,
        samples: Vec<f32>,
        sample_rate: u32,
    ) {
        let audio = AudioSegment {
            samples,
            sample_rate,
        };
        let ctx = SttContext {
            language: "ja-JP".to_string(),
            dictionary: vec![],
        };

        match rt.block_on(stt_engine.transcribe(audio, ctx)) {
            Ok(result) => {
                if !result.text.is_empty() {
                    let _ = event_tx.send(PipelineEvent::TranscriptFinal {
                        text: result.text,
                        confidence: result.confidence,
                    });
                }
            }
            Err(e) => {
                log::error!("STT error: {}", e);
                let _ = event_tx.send(PipelineEvent::Error(e.to_string()));
            }
        }
    }
}

impl Drop for AudioPipeline {
    fn drop(&mut self) {
        self.stop();
    }
}
