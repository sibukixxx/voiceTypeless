use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Device, Stream, StreamConfig};
use parking_lot::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tokio::sync::mpsc;

/// 音声キャプチャの設定。
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CaptureConfig {
    /// 目標サンプルレート (デフォルト: 16000 Hz)
    pub target_sample_rate: u32,
    /// 目標チャンネル数 (デフォルト: 1 = mono)
    pub target_channels: u16,
    /// フレームサイズ (サンプル数)。デフォルト: 320 (= 20ms @ 16kHz)
    pub frame_size: usize,
    /// 無入力検知閾値 (秒)。この秒数連続でゼロサンプルなら NoInput を発火。デフォルト: 3.0
    pub no_input_timeout_secs: f32,
}

impl Default for CaptureConfig {
    fn default() -> Self {
        Self {
            target_sample_rate: 16_000,
            target_channels: 1,
            frame_size: 320,
            no_input_timeout_secs: 3.0,
        }
    }
}

/// キャプチャシステムが発するイベント。
#[derive(Debug, Clone)]
pub enum CaptureEvent {
    /// 音声フレーム (16kHz, mono, f32)
    Frame(Vec<f32>),
    /// RMS レベル (0.0–1.0)。VUメータ用。
    Level(f32),
    /// ストリームエラー (recoverable)
    Error(CaptureError),
    /// デバイス切断検知
    DeviceDisconnected,
    /// 入力なし検知 (全サンプルがゼロの状態が一定時間継続)
    NoInput { silence_secs: f32 },
}

/// キャプチャ関連のエラー。
#[derive(Debug, Clone)]
pub struct CaptureError {
    pub kind: CaptureErrorKind,
    pub detail: String,
    pub recoverable: bool,
}

/// キャプチャエラー種別。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaptureErrorKind {
    /// マイク権限が拒否された
    PermissionDenied,
    /// デバイスが見つからない / 切断された
    DeviceNotFound,
    /// ストリームエラー
    StreamError,
    /// 設定エラー（サンプルレート非対応等）
    ConfigError,
}

impl std::fmt::Display for CaptureError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "CaptureError::{:?}: {}", self.kind, self.detail)
    }
}

impl CaptureError {
    pub fn permission_denied(detail: impl Into<String>) -> Self {
        Self { kind: CaptureErrorKind::PermissionDenied, detail: detail.into(), recoverable: false }
    }
    pub fn device_not_found(detail: impl Into<String>) -> Self {
        Self { kind: CaptureErrorKind::DeviceNotFound, detail: detail.into(), recoverable: true }
    }
    pub fn stream_error(detail: impl Into<String>) -> Self {
        Self { kind: CaptureErrorKind::StreamError, detail: detail.into(), recoverable: true }
    }
    pub fn config_error(detail: impl Into<String>) -> Self {
        Self { kind: CaptureErrorKind::ConfigError, detail: detail.into(), recoverable: false }
    }
}

/// マイク入力のライフサイクルを管理する。
pub struct AudioCapture {
    config: CaptureConfig,
    stream: Option<Stream>,
    device: Option<Device>,
    device_name: Option<String>,
    /// コールバックからの最新フレームカウント（無入力検知用）
    frame_counter: Arc<AtomicU64>,
    /// 連続ゼロサンプルフレーム数
    zero_frame_counter: Arc<AtomicU64>,
}

impl AudioCapture {
    pub fn new(config: CaptureConfig) -> Self {
        Self {
            config,
            stream: None,
            device: None,
            device_name: None,
            frame_counter: Arc::new(AtomicU64::new(0)),
            zero_frame_counter: Arc::new(AtomicU64::new(0)),
        }
    }

    /// デフォルト入力デバイスを選択する。デバイス名を返す。
    pub fn select_default_device(&mut self) -> Result<String, CaptureError> {
        let host = cpal::default_host();
        let device = host.default_input_device().ok_or_else(|| {
            CaptureError::device_not_found("No default input device found")
        })?;
        let name = device.name().unwrap_or_else(|_| "Unknown".into());
        self.device_name = Some(name.clone());
        self.device = Some(device);
        Ok(name)
    }

    /// 利用可能な入力デバイス一覧を返す。
    pub fn list_input_devices() -> Vec<String> {
        let host = cpal::default_host();
        host.input_devices()
            .map(|devices| {
                devices
                    .filter_map(|d| d.name().ok())
                    .collect()
            })
            .unwrap_or_default()
    }

    /// 現在選択中のデバイス名。
    pub fn current_device_name(&self) -> Option<&str> {
        self.device_name.as_deref()
    }

    /// キャプチャを開始する。CaptureEvent を tx チャネルに送信する。
    pub fn start(&mut self, tx: mpsc::UnboundedSender<CaptureEvent>) -> Result<(), CaptureError> {
        let device = self
            .device
            .as_ref()
            .ok_or_else(|| CaptureError::device_not_found(
                "No device selected. Call select_default_device first.",
            ))?;

        let supported_config = device.default_input_config().map_err(|e| {
            classify_cpal_error(&e)
        })?;

        let native_sample_rate = supported_config.sample_rate().0;
        let native_channels = supported_config.channels();
        let target_rate = self.config.target_sample_rate;
        let frame_size = self.config.frame_size;

        let accumulator = Arc::new(Mutex::new(Vec::with_capacity(frame_size * 2)));

        let tx_err = tx.clone();
        let acc = accumulator.clone();

        let frame_counter = self.frame_counter.clone();
        let zero_frame_counter = self.zero_frame_counter.clone();
        let no_input_frames_threshold = (self.config.no_input_timeout_secs
            * self.config.target_sample_rate as f32
            / self.config.frame_size as f32) as u64;
        let tx_no_input = tx.clone();
        let no_input_timeout_secs = self.config.no_input_timeout_secs;

        // カウンタリセット
        self.frame_counter.store(0, Ordering::Relaxed);
        self.zero_frame_counter.store(0, Ordering::Relaxed);

        let stream_config: StreamConfig = supported_config.into();

        let stream = device
            .build_input_stream(
                &stream_config,
                move |data: &[f32], _: &cpal::InputCallbackInfo| {
                    // 1. ステレオ→モノ変換
                    let mono: Vec<f32> = if native_channels > 1 {
                        data.chunks(native_channels as usize)
                            .map(|ch| ch.iter().sum::<f32>() / native_channels as f32)
                            .collect()
                    } else {
                        data.to_vec()
                    };

                    // 2. リサンプリング
                    let resampled = if native_sample_rate != target_rate {
                        resample_linear(&mono, native_sample_rate, target_rate)
                    } else {
                        mono
                    };

                    // 3. フレーム分割
                    let mut acc = acc.lock();
                    acc.extend_from_slice(&resampled);

                    while acc.len() >= frame_size {
                        let frame: Vec<f32> = acc.drain(..frame_size).collect();
                        let rms = compute_rms(&frame);

                        frame_counter.fetch_add(1, Ordering::Relaxed);

                        // 無入力検知: 全サンプルがゼロ（またはほぼゼロ）
                        if is_zero_frame(&frame) {
                            let count = zero_frame_counter.fetch_add(1, Ordering::Relaxed) + 1;
                            if count == no_input_frames_threshold {
                                let _ = tx_no_input.send(CaptureEvent::NoInput {
                                    silence_secs: no_input_timeout_secs,
                                });
                            }
                        } else {
                            zero_frame_counter.store(0, Ordering::Relaxed);
                        }

                        let _ = tx.send(CaptureEvent::Level(rms));
                        let _ = tx.send(CaptureEvent::Frame(frame));
                    }
                },
                move |err| {
                    let error_str = format!("{}", err);
                    // デバイス切断を検知
                    if is_device_disconnected_error(&error_str) {
                        let _ = tx_err.send(CaptureEvent::DeviceDisconnected);
                    } else {
                        let _ = tx_err.send(CaptureEvent::Error(
                            CaptureError::stream_error(error_str),
                        ));
                    }
                },
                None,
            )
            .map_err(|e| classify_build_stream_error(&e))?;

        stream
            .play()
            .map_err(|e| CaptureError::stream_error(format!("Failed to start stream: {}", e)))?;
        self.stream = Some(stream);
        Ok(())
    }

    /// デバイス切断後の再接続を試みる。
    /// 成功時は新デバイス名を返す。
    pub fn try_reconnect(
        &mut self,
        tx: mpsc::UnboundedSender<CaptureEvent>,
    ) -> Result<String, CaptureError> {
        self.stop();
        let name = self.select_default_device()?;
        self.start(tx)?;
        log::info!("Reconnected to audio device: {}", name);
        Ok(name)
    }

    /// キャプチャを停止する。
    pub fn stop(&mut self) {
        self.stream = None;
    }

    /// キャプチャ中かどうか。
    pub fn is_capturing(&self) -> bool {
        self.stream.is_some()
    }

    /// 処理済みフレーム数（デバッグ/監視用）。
    pub fn frame_count(&self) -> u64 {
        self.frame_counter.load(Ordering::Relaxed)
    }
}

/// サンプルバッファの RMS (Root Mean Square) を計算する。
/// 戻り値は 0.0–1.0 (クランプ)。
pub fn compute_rms(samples: &[f32]) -> f32 {
    if samples.is_empty() {
        return 0.0;
    }
    let sum_sq: f32 = samples.iter().map(|s| s * s).sum();
    (sum_sq / samples.len() as f32).sqrt().min(1.0)
}

/// フレームが実質ゼロ（入力なし）かどうかを判定する。
/// 閾値は -96dBFS 相当。
fn is_zero_frame(frame: &[f32]) -> bool {
    const ZERO_THRESHOLD: f32 = 0.000016; // ≈ -96 dBFS
    frame.iter().all(|&s| s.abs() < ZERO_THRESHOLD)
}

/// デバイス切断エラーかどうかをヒューリスティックに判定する。
fn is_device_disconnected_error(error: &str) -> bool {
    let lower = error.to_lowercase();
    lower.contains("disconnect")
        || lower.contains("removed")
        || lower.contains("not found")
        || lower.contains("invalid device")
        || lower.contains("device lost")
}

/// cpal DefaultStreamConfigError を CaptureError に変換する。
fn classify_cpal_error(e: &cpal::DefaultStreamConfigError) -> CaptureError {
    match e {
        cpal::DefaultStreamConfigError::DeviceNotAvailable => {
            CaptureError::device_not_found("Audio device not available")
        }
        cpal::DefaultStreamConfigError::StreamTypeNotSupported => {
            CaptureError::config_error("Stream type not supported by device")
        }
        cpal::DefaultStreamConfigError::BackendSpecific { err } => {
            let detail = format!("{}", err);
            if detail.to_lowercase().contains("permission") {
                CaptureError::permission_denied(detail)
            } else {
                CaptureError::config_error(detail)
            }
        }
    }
}

/// cpal BuildStreamError を CaptureError に変換する。
fn classify_build_stream_error(e: &cpal::BuildStreamError) -> CaptureError {
    match e {
        cpal::BuildStreamError::DeviceNotAvailable => {
            CaptureError::device_not_found("Audio device not available")
        }
        cpal::BuildStreamError::StreamConfigNotSupported => {
            CaptureError::config_error("Stream config not supported")
        }
        cpal::BuildStreamError::InvalidArgument => {
            CaptureError::config_error("Invalid argument for stream")
        }
        cpal::BuildStreamError::StreamIdOverflow => {
            CaptureError::stream_error("Stream ID overflow")
        }
        cpal::BuildStreamError::BackendSpecific { err } => {
            let detail = format!("{}", err);
            if detail.to_lowercase().contains("permission") {
                CaptureError::permission_denied(detail)
            } else {
                CaptureError::stream_error(detail)
            }
        }
    }
}

/// 線形補間によるリサンプラー。
/// 音声 (非音楽) 用途には十分な品質。48kHz→16kHz 等の整数倍比に対応。
fn resample_linear(input: &[f32], from_rate: u32, to_rate: u32) -> Vec<f32> {
    if from_rate == to_rate || input.is_empty() {
        return input.to_vec();
    }
    let ratio = from_rate as f64 / to_rate as f64;
    let output_len = (input.len() as f64 / ratio) as usize;
    let mut output = Vec::with_capacity(output_len);
    for i in 0..output_len {
        let src_idx = i as f64 * ratio;
        let idx0 = src_idx.floor() as usize;
        let idx1 = (idx0 + 1).min(input.len() - 1);
        let frac = src_idx - idx0 as f64;
        let sample = input[idx0] as f64 * (1.0 - frac) + input[idx1] as f64 * frac;
        output.push(sample as f32);
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rms_silence() {
        let samples = vec![0.0f32; 320];
        assert_eq!(compute_rms(&samples), 0.0);
    }

    #[test]
    fn rms_full_scale() {
        let samples = vec![1.0f32; 320];
        let rms = compute_rms(&samples);
        assert!((rms - 1.0).abs() < 0.001);
    }

    #[test]
    fn rms_sine_wave() {
        let samples: Vec<f32> = (0..16000)
            .map(|i| (2.0 * std::f32::consts::PI * 440.0 * i as f32 / 16000.0).sin())
            .collect();
        let rms = compute_rms(&samples);
        assert!((rms - 0.707).abs() < 0.01, "RMS was {}", rms);
    }

    #[test]
    fn rms_empty() {
        assert_eq!(compute_rms(&[]), 0.0);
    }

    #[test]
    fn resample_same_rate() {
        let input = vec![1.0, 2.0, 3.0];
        let output = resample_linear(&input, 16000, 16000);
        assert_eq!(output, input);
    }

    #[test]
    fn resample_downsample_3to1() {
        let input: Vec<f32> = (0..48).map(|i| i as f32).collect();
        let output = resample_linear(&input, 48000, 16000);
        assert_eq!(output.len(), 16);
        assert!((output[0] - 0.0).abs() < 0.001);
        assert!((output[1] - 3.0).abs() < 0.001);
    }

    #[test]
    fn resample_empty() {
        let output = resample_linear(&[], 48000, 16000);
        assert!(output.is_empty());
    }

    #[test]
    fn resample_upsample() {
        let input = vec![0.0, 1.0, 2.0, 3.0];
        let output = resample_linear(&input, 16000, 48000);
        assert_eq!(output.len(), 12);
        assert!((output[0] - 0.0).abs() < 0.001);
    }

    #[test]
    fn zero_frame_detection() {
        assert!(is_zero_frame(&vec![0.0; 320]));
        assert!(is_zero_frame(&vec![0.00001; 320]));
        assert!(!is_zero_frame(&vec![0.1; 320]));
    }

    #[test]
    fn mixed_frame_not_zero() {
        let mut frame = vec![0.0; 320];
        frame[160] = 0.5;
        assert!(!is_zero_frame(&frame));
    }

    #[test]
    fn device_disconnect_detection() {
        assert!(is_device_disconnected_error("Device disconnected"));
        assert!(is_device_disconnected_error("device was removed"));
        assert!(is_device_disconnected_error("Not Found in audio"));
        assert!(!is_device_disconnected_error("buffer underrun"));
    }

    #[test]
    fn capture_error_constructors() {
        let e = CaptureError::permission_denied("no mic access");
        assert_eq!(e.kind, CaptureErrorKind::PermissionDenied);
        assert!(!e.recoverable);

        let e = CaptureError::device_not_found("unplugged");
        assert_eq!(e.kind, CaptureErrorKind::DeviceNotFound);
        assert!(e.recoverable);

        let e = CaptureError::stream_error("overflow");
        assert_eq!(e.kind, CaptureErrorKind::StreamError);
        assert!(e.recoverable);
    }

    #[test]
    fn list_devices_does_not_panic() {
        // デバイスがなくてもパニックしないことを確認
        let _ = AudioCapture::list_input_devices();
    }
}
