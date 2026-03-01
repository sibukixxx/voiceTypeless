use async_trait::async_trait;
use reqwest::multipart;
use serde::Deserialize;
use std::time::Duration;

use super::{AudioSegment, SttContext, SttEngine, SttError, TranscriptResult};

const SONIOX_API_BASE: &str = "https://api.soniox.com/v1";
const SONIOX_MODEL: &str = "stt-async-v4";
const POLL_INTERVAL: Duration = Duration::from_secs(1);
const MAX_POLL_ATTEMPTS: u32 = 60;

/// Soniox Cloud STT エンジン（非同期書き起こし API）
pub struct SonioxSttEngine {
    api_key: String,
    client: reqwest::Client,
}

#[derive(Deserialize)]
struct FileUploadResponse {
    id: String,
}

#[derive(Deserialize)]
struct CreateTranscriptionResponse {
    id: String,
    status: String,
}

#[derive(Deserialize)]
struct TranscriptionStatusResponse {
    status: String,
    error_message: Option<String>,
}

#[derive(Deserialize)]
struct TranscriptToken {
    confidence: Option<f32>,
}

#[derive(Deserialize)]
struct TranscriptResponse {
    text: String,
    tokens: Option<Vec<TranscriptToken>>,
}

impl SonioxSttEngine {
    pub fn new(api_key: String) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(120))
            .build()
            .unwrap_or_default();
        Self { api_key, client }
    }

    /// PCM f32 mono サンプルを 16-bit WAV バイト列に変換
    fn encode_wav(samples: &[f32], sample_rate: u32) -> Vec<u8> {
        let num_samples = samples.len();
        let bits_per_sample: u16 = 16;
        let num_channels: u16 = 1;
        let byte_rate = sample_rate * u32::from(num_channels) * u32::from(bits_per_sample) / 8;
        let block_align = num_channels * bits_per_sample / 8;
        let data_size = (num_samples * 2) as u32;
        let file_size = 36 + data_size;

        let mut buf = Vec::with_capacity(44 + num_samples * 2);

        // RIFF header
        buf.extend_from_slice(b"RIFF");
        buf.extend_from_slice(&file_size.to_le_bytes());
        buf.extend_from_slice(b"WAVE");

        // fmt chunk
        buf.extend_from_slice(b"fmt ");
        buf.extend_from_slice(&16u32.to_le_bytes()); // chunk size
        buf.extend_from_slice(&1u16.to_le_bytes()); // PCM format
        buf.extend_from_slice(&num_channels.to_le_bytes());
        buf.extend_from_slice(&sample_rate.to_le_bytes());
        buf.extend_from_slice(&byte_rate.to_le_bytes());
        buf.extend_from_slice(&block_align.to_le_bytes());
        buf.extend_from_slice(&bits_per_sample.to_le_bytes());

        // data chunk
        buf.extend_from_slice(b"data");
        buf.extend_from_slice(&data_size.to_le_bytes());

        for &sample in samples {
            let clamped = sample.clamp(-1.0, 1.0);
            let val = (clamped * 32767.0) as i16;
            buf.extend_from_slice(&val.to_le_bytes());
        }

        buf
    }

    /// ファイルを Soniox にアップロードして file_id を取得
    async fn upload_audio(&self, wav_data: Vec<u8>) -> Result<String, SttError> {
        let part = multipart::Part::bytes(wav_data)
            .file_name("audio.wav")
            .mime_str("audio/wav")
            .map_err(|e| SttError::TranscriptionFailed(format!("MIME error: {e}")))?;

        let form = multipart::Form::new().part("file", part);

        let resp = self
            .client
            .post(format!("{SONIOX_API_BASE}/files"))
            .bearer_auth(&self.api_key)
            .multipart(form)
            .send()
            .await
            .map_err(|e| SttError::TranscriptionFailed(format!("Upload failed: {e}")))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(SttError::TranscriptionFailed(format!(
                "Upload failed ({status}): {body}"
            )));
        }

        let file_resp: FileUploadResponse = resp
            .json()
            .await
            .map_err(|e| SttError::TranscriptionFailed(format!("Parse upload response: {e}")))?;

        Ok(file_resp.id)
    }

    /// 書き起こしジョブを作成
    async fn create_transcription(
        &self,
        file_id: &str,
        language: &str,
    ) -> Result<String, SttError> {
        let lang_hint = language_to_hint(language);

        let mut body = serde_json::json!({
            "model": SONIOX_MODEL,
            "file_id": file_id,
        });

        if !lang_hint.is_empty() {
            body["language_hints"] = serde_json::json!([lang_hint]);
        }

        let resp = self
            .client
            .post(format!("{SONIOX_API_BASE}/transcriptions"))
            .bearer_auth(&self.api_key)
            .json(&body)
            .send()
            .await
            .map_err(|e| SttError::TranscriptionFailed(format!("Create transcription: {e}")))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(SttError::TranscriptionFailed(format!(
                "Create transcription failed ({status}): {body}"
            )));
        }

        let tx_resp: CreateTranscriptionResponse = resp
            .json()
            .await
            .map_err(|e| SttError::TranscriptionFailed(format!("Parse transcription resp: {e}")))?;

        log::debug!(
            "Soniox transcription created: id={}, status={}",
            tx_resp.id,
            tx_resp.status
        );

        Ok(tx_resp.id)
    }

    /// ステータスをポーリングして完了を待つ
    async fn wait_for_completion(&self, transcription_id: &str) -> Result<(), SttError> {
        for attempt in 0..MAX_POLL_ATTEMPTS {
            tokio::time::sleep(POLL_INTERVAL).await;

            let resp = self
                .client
                .get(format!(
                    "{SONIOX_API_BASE}/transcriptions/{transcription_id}"
                ))
                .bearer_auth(&self.api_key)
                .send()
                .await
                .map_err(|e| SttError::TranscriptionFailed(format!("Poll status: {e}")))?;

            if !resp.status().is_success() {
                continue;
            }

            let status_resp: TranscriptionStatusResponse = resp
                .json()
                .await
                .map_err(|e| SttError::TranscriptionFailed(format!("Parse status: {e}")))?;

            match status_resp.status.as_str() {
                "completed" => return Ok(()),
                "error" => {
                    let msg = status_resp
                        .error_message
                        .unwrap_or_else(|| "Unknown error".to_string());
                    return Err(SttError::TranscriptionFailed(format!(
                        "Soniox transcription error: {msg}"
                    )));
                }
                _ => {
                    log::debug!(
                        "Soniox poll attempt {}/{}: status={}",
                        attempt + 1,
                        MAX_POLL_ATTEMPTS,
                        status_resp.status
                    );
                }
            }
        }

        Err(SttError::Timeout)
    }

    /// 書き起こし結果を取得
    async fn get_transcript(&self, transcription_id: &str) -> Result<TranscriptResult, SttError> {
        let resp = self
            .client
            .get(format!(
                "{SONIOX_API_BASE}/transcriptions/{transcription_id}/transcript"
            ))
            .bearer_auth(&self.api_key)
            .send()
            .await
            .map_err(|e| SttError::TranscriptionFailed(format!("Get transcript: {e}")))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(SttError::TranscriptionFailed(format!(
                "Get transcript failed ({status}): {body}"
            )));
        }

        let transcript: TranscriptResponse = resp
            .json()
            .await
            .map_err(|e| SttError::TranscriptionFailed(format!("Parse transcript: {e}")))?;

        // トークンの confidence 平均を算出
        let confidence = transcript
            .tokens
            .as_ref()
            .and_then(|tokens| {
                let scored: Vec<f32> = tokens.iter().filter_map(|t| t.confidence).collect();
                if scored.is_empty() {
                    None
                } else {
                    Some(scored.iter().sum::<f32>() / scored.len() as f32)
                }
            })
            .unwrap_or(0.9);

        Ok(TranscriptResult {
            text: transcript.text,
            confidence,
            is_partial: false,
        })
    }

    /// アップロードしたファイルを削除（ベストエフォート）
    async fn cleanup_file(&self, file_id: &str) {
        let _ = self
            .client
            .delete(format!("{SONIOX_API_BASE}/files/{file_id}"))
            .bearer_auth(&self.api_key)
            .send()
            .await;
    }

    /// 書き起こしジョブを削除（ベストエフォート）
    async fn cleanup_transcription(&self, transcription_id: &str) {
        let _ = self
            .client
            .delete(format!(
                "{SONIOX_API_BASE}/transcriptions/{transcription_id}"
            ))
            .bearer_auth(&self.api_key)
            .send()
            .await;
    }
}

#[async_trait]
impl SttEngine for SonioxSttEngine {
    async fn transcribe(
        &self,
        audio: AudioSegment,
        ctx: SttContext,
    ) -> Result<TranscriptResult, SttError> {
        if audio.samples.is_empty() {
            return Err(SttError::AudioFormat("Empty audio segment".to_string()));
        }

        // 1. PCM → WAV 変換
        let wav_data = Self::encode_wav(&audio.samples, audio.sample_rate);

        // 2. ファイルアップロード
        let file_id = self.upload_audio(wav_data).await?;

        // 3. 書き起こしジョブ作成
        let transcription_id = match self.create_transcription(&file_id, &ctx.language).await {
            Ok(id) => id,
            Err(e) => {
                self.cleanup_file(&file_id).await;
                return Err(e);
            }
        };

        // 4. 完了までポーリング
        if let Err(e) = self.wait_for_completion(&transcription_id).await {
            self.cleanup_file(&file_id).await;
            self.cleanup_transcription(&transcription_id).await;
            return Err(e);
        }

        // 5. 結果取得
        let result = self.get_transcript(&transcription_id).await;

        // 6. クリーンアップ
        self.cleanup_file(&file_id).await;
        self.cleanup_transcription(&transcription_id).await;

        result
    }

    fn supports_partial(&self) -> bool {
        false
    }

    fn name(&self) -> &str {
        "soniox"
    }
}

/// BCP 47 言語タグから Soniox の language_hints 用コードに変換
fn language_to_hint(lang: &str) -> &str {
    match lang {
        "ja-JP" | "ja" => "ja",
        "en-US" | "en-GB" | "en" => "en",
        "zh-CN" | "zh-TW" | "zh" => "zh",
        "ko-KR" | "ko" => "ko",
        _ => "",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_wav_header() {
        let samples = vec![0.0f32; 100];
        let wav = SonioxSttEngine::encode_wav(&samples, 16000);

        // RIFF header
        assert_eq!(&wav[0..4], b"RIFF");
        assert_eq!(&wav[8..12], b"WAVE");
        assert_eq!(&wav[12..16], b"fmt ");
        assert_eq!(&wav[36..40], b"data");

        // Total size: 44 header + 100 samples * 2 bytes = 244
        assert_eq!(wav.len(), 244);
    }

    #[test]
    fn test_encode_wav_clamps_values() {
        let samples = vec![-2.0, 2.0, 0.5, -0.5];
        let wav = SonioxSttEngine::encode_wav(&samples, 16000);

        // data starts at byte 44
        let s0 = i16::from_le_bytes([wav[44], wav[45]]);
        let s1 = i16::from_le_bytes([wav[46], wav[47]]);
        assert_eq!(s0, -32767); // clamped to -1.0
        assert_eq!(s1, 32767); // clamped to 1.0
    }

    #[test]
    fn test_language_to_hint() {
        assert_eq!(language_to_hint("ja-JP"), "ja");
        assert_eq!(language_to_hint("en-US"), "en");
        assert_eq!(language_to_hint("zh-CN"), "zh");
        assert_eq!(language_to_hint("ko-KR"), "ko");
        assert_eq!(language_to_hint("fr-FR"), "");
    }

    #[test]
    fn soniox_name_returns_soniox() {
        let engine = SonioxSttEngine::new("test-key".to_string());
        assert_eq!(engine.name(), "soniox");
    }
}
