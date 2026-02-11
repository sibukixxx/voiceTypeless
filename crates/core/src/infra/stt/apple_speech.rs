//! Apple Speech Framework を使った STT エンジン（macOS 専用）
//!
//! Swift の SFSpeechRecognizer を C FFI 経由で呼び出す。
//! 音声データは一時 WAV ファイルとして書き出し、
//! SFSpeechURLRecognitionRequest でファイルベースの認識を行う。

use std::ffi::{CStr, CString};
use std::os::raw::c_char;

use async_trait::async_trait;

use super::{AudioSegment, SttContext, SttEngine, SttError, TranscriptResult};

extern "C" {
    fn swift_speech_is_available() -> bool;
    fn swift_speech_recognize_file(
        file_path: *const c_char,
        language: *const c_char,
    ) -> *mut c_char;
    fn swift_free_string(ptr: *mut c_char);
}

/// Apple Speech STT エンジン
pub struct AppleSttEngine;

impl AppleSttEngine {
    /// Speech.framework が利用可能かチェック
    pub fn is_available() -> bool {
        unsafe { swift_speech_is_available() }
    }
}

#[async_trait]
impl SttEngine for AppleSttEngine {
    async fn transcribe(
        &self,
        audio: AudioSegment,
        ctx: SttContext,
    ) -> Result<TranscriptResult, SttError> {
        // 一時 WAV ファイルに書き出し
        let tmp_path = std::env::temp_dir().join(format!(
            "vt_stt_{}.wav",
            uuid::Uuid::new_v4()
        ));
        let tmp_path_str = tmp_path
            .to_str()
            .ok_or_else(|| SttError::TranscriptionFailed("Invalid temp path".to_string()))?;

        write_wav(tmp_path_str, &audio.samples, audio.sample_rate)
            .map_err(|e| SttError::TranscriptionFailed(format!("WAV write error: {}", e)))?;

        // Swift で Speech.framework を呼び出し
        let path_c = CString::new(tmp_path_str)
            .map_err(|e| SttError::TranscriptionFailed(e.to_string()))?;
        let lang_c = CString::new(ctx.language.as_str())
            .map_err(|e| SttError::TranscriptionFailed(e.to_string()))?;

        let result_ptr =
            unsafe { swift_speech_recognize_file(path_c.as_ptr(), lang_c.as_ptr()) };

        // 一時ファイル削除
        let _ = std::fs::remove_file(&tmp_path);

        if result_ptr.is_null() {
            return Err(SttError::TranscriptionFailed(
                "Null result from Swift".to_string(),
            ));
        }

        let result_str = unsafe { CStr::from_ptr(result_ptr) }
            .to_str()
            .map_err(|e| SttError::TranscriptionFailed(e.to_string()))?
            .to_string();

        // メモリ解放
        unsafe { swift_free_string(result_ptr) };

        // JSON パース: {"text": "...", "confidence": 0.9}
        let parsed: SttResultJson =
            serde_json::from_str(&result_str).map_err(|e| {
                SttError::TranscriptionFailed(format!(
                    "Failed to parse STT result: {} (raw: {})",
                    e, result_str
                ))
            })?;

        Ok(TranscriptResult {
            text: parsed.text,
            confidence: parsed.confidence,
            is_partial: false,
        })
    }

    fn supports_partial(&self) -> bool {
        false
    }
}

/// Swift から返される JSON 構造
#[derive(serde::Deserialize)]
struct SttResultJson {
    text: String,
    confidence: f32,
}

/// f32 PCM サンプルを 16-bit WAV ファイルとして書き出す
fn write_wav(path: &str, samples: &[f32], sample_rate: u32) -> std::io::Result<()> {
    use std::io::Write;

    let mut file = std::fs::File::create(path)?;

    let num_samples = samples.len() as u32;
    let data_size = num_samples * 2; // 16-bit = 2 bytes per sample
    let file_size = 36 + data_size;

    // RIFF header
    file.write_all(b"RIFF")?;
    file.write_all(&file_size.to_le_bytes())?;
    file.write_all(b"WAVE")?;

    // fmt chunk
    file.write_all(b"fmt ")?;
    file.write_all(&16u32.to_le_bytes())?; // chunk size
    file.write_all(&1u16.to_le_bytes())?; // PCM format
    file.write_all(&1u16.to_le_bytes())?; // mono
    file.write_all(&sample_rate.to_le_bytes())?; // sample rate
    file.write_all(&(sample_rate * 2).to_le_bytes())?; // byte rate
    file.write_all(&2u16.to_le_bytes())?; // block align
    file.write_all(&16u16.to_le_bytes())?; // bits per sample

    // data chunk
    file.write_all(b"data")?;
    file.write_all(&data_size.to_le_bytes())?;

    // f32 → i16 変換して書き出し
    for &sample in samples {
        let clamped = sample.clamp(-1.0, 1.0);
        let s = (clamped * 32767.0) as i16;
        file.write_all(&s.to_le_bytes())?;
    }

    Ok(())
}
