use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::sync::Arc;

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

/// 音声キャプチャエラー
#[derive(Debug, thiserror::Error)]
pub enum AudioCaptureError {
    #[error("No audio input device found")]
    NoDevice,
    #[error("Audio device config error: {0}")]
    Config(String),
    #[error("Audio stream error: {0}")]
    Stream(String),
}

/// キャプチャ設定（実際のデバイスから取得した値）
#[derive(Debug, Clone)]
pub struct CaptureConfig {
    pub sample_rate: u32,
    pub channels: u16,
}

/// デバイスの存在と設定を事前チェックする（stream は作らない）
pub fn check_device() -> Result<CaptureConfig, AudioCaptureError> {
    let host = cpal::default_host();
    let device = host
        .default_input_device()
        .ok_or(AudioCaptureError::NoDevice)?;

    let supported_config = device
        .default_input_config()
        .map_err(|e| AudioCaptureError::Config(e.to_string()))?;

    Ok(CaptureConfig {
        sample_rate: supported_config.sample_rate().0,
        channels: supported_config.channels(),
    })
}

/// マイクキャプチャを開始し、mono PCM サンプルを channel 経由で送出する
///
/// **注意**: cpal::Stream は Send ではないため、この関数は
/// stream を使うスレッド上で呼び出す必要がある。
/// 返された stream は呼び出し側が保持する（drop で停止）。
pub fn start_capture(
    sample_tx: mpsc::Sender<Vec<f32>>,
    stop_flag: Arc<AtomicBool>,
) -> Result<CaptureConfig, AudioCaptureError> {
    let host = cpal::default_host();
    let device = host
        .default_input_device()
        .ok_or(AudioCaptureError::NoDevice)?;

    let supported_config = device
        .default_input_config()
        .map_err(|e| AudioCaptureError::Config(e.to_string()))?;

    let sample_rate = supported_config.sample_rate().0;
    let channels = supported_config.channels();
    let sample_format = supported_config.sample_format();

    let config = cpal::StreamConfig {
        channels,
        sample_rate: cpal::SampleRate(sample_rate),
        buffer_size: cpal::BufferSize::Default,
    };

    let stop_flag_clone = stop_flag.clone();

    let stream = match sample_format {
        cpal::SampleFormat::F32 => device.build_input_stream(
            &config,
            move |data: &[f32], _info: &cpal::InputCallbackInfo| {
                if stop_flag_clone.load(Ordering::Relaxed) {
                    return;
                }
                let mono = to_mono(data, channels);
                let _ = sample_tx.send(mono);
            },
            |err| {
                log::error!("Audio stream error: {}", err);
            },
            None,
        ),
        cpal::SampleFormat::I16 => {
            let tx = sample_tx;
            let flag = stop_flag;
            device.build_input_stream(
                &config,
                move |data: &[i16], _info: &cpal::InputCallbackInfo| {
                    if flag.load(Ordering::Relaxed) {
                        return;
                    }
                    let f32_data: Vec<f32> =
                        data.iter().map(|&s| s as f32 / 32768.0).collect();
                    let mono = to_mono(&f32_data, channels);
                    let _ = tx.send(mono);
                },
                |err| {
                    log::error!("Audio stream error: {}", err);
                },
                None,
            )
        }
        format => {
            return Err(AudioCaptureError::Config(format!(
                "Unsupported sample format: {:?}",
                format
            )));
        }
    }
    .map_err(|e| AudioCaptureError::Stream(e.to_string()))?;

    stream
        .play()
        .map_err(|e| AudioCaptureError::Stream(e.to_string()))?;

    log::info!(
        "Audio capture started: {}Hz, {} channels, {:?}",
        sample_rate,
        channels,
        sample_format
    );

    // stream を意図的にリークさせてスレッド上で生き続けるようにする
    // stop_flag が true になるとコールバックが停止し、
    // スレッド終了時に channel の受信側が drop されて自然停止する
    std::mem::forget(stream);

    Ok(CaptureConfig {
        sample_rate,
        channels,
    })
}

/// ステレオ → モノ変換（チャンネル平均）
fn to_mono(data: &[f32], channels: u16) -> Vec<f32> {
    if channels <= 1 {
        return data.to_vec();
    }
    data.chunks(channels as usize)
        .map(|frame| frame.iter().sum::<f32>() / channels as f32)
        .collect()
}
