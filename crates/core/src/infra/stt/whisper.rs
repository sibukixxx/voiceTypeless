use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::Duration;
use tokio::process::Command;
use tokio::time::timeout;

use crate::domain::stt::{
    AudioSegment, SttContext, SttEngine, SttError, Transcript, TimingInfo,
};

/// Whisper.cpp sidecar の設定。
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct WhisperConfig {
    /// whisper-cli バイナリのパス
    pub binary_path: PathBuf,
    /// GGML モデルファイルのパス
    pub model_path: PathBuf,
    /// デフォルト言語 (例: "ja", "en", "auto")
    pub language: String,
    /// サンプリング温度 (0.0 = greedy)
    pub temperature: f32,
    /// ビームサーチサイズ (1 = greedy)
    pub beam_size: u32,
    /// タイムアウト秒数
    pub timeout_secs: u64,
    /// 使用スレッド数
    pub threads: u32,
    /// タイムスタンプ出力を無効化 (処理高速化)
    pub no_timestamps: bool,
    /// エントロピー閾値 (高エントロピーセグメントをフィルタ)
    pub entropy_thold: f32,
    /// 対数確率閾値 (低確率セグメントをフィルタ)
    pub logprob_thold: f32,
}

impl Default for WhisperConfig {
    fn default() -> Self {
        Self {
            binary_path: PathBuf::from("whisper-cli"),
            model_path: PathBuf::from("models/ggml-base.bin"),
            language: "ja".into(),
            temperature: 0.0,
            beam_size: 1,
            timeout_secs: 30,
            threads: 4,
            no_timestamps: false,
            entropy_thold: 2.4,
            logprob_thold: -1.0,
        }
    }
}

/// Whisper.cpp sidecar プロセスマネージャ。
pub struct WhisperSidecar {
    config: parking_lot::RwLock<WhisperConfig>,
}

impl WhisperSidecar {
    pub fn new(config: WhisperConfig) -> Self {
        Self {
            config: parking_lot::RwLock::new(config),
        }
    }

    /// 設定を動的に更新する（モデル切替・パラメータ変更）。
    pub fn update_config(&self, config: WhisperConfig) {
        *self.config.write() = config;
    }

    /// 現在の設定のクローンを取得する。
    pub fn config(&self) -> WhisperConfig {
        self.config.read().clone()
    }

    /// whisper バイナリとモデルの存在を検証する。
    pub fn validate(&self) -> Result<(), SttError> {
        let config = self.config.read();
        if !config.binary_path.exists()
            && which_binary(&config.binary_path).is_none()
        {
            return Err(SttError::engine_not_available(format!(
                "Whisper binary not found: {:?}",
                config.binary_path
            )));
        }
        if !config.model_path.exists() {
            return Err(SttError::engine_not_available(format!(
                "Whisper model not found: {:?}",
                config.model_path
            )));
        }
        Ok(())
    }

    /// whisper-cli 用のコマンドライン引数を構築する。
    fn build_args(
        config: &WhisperConfig,
        wav_path: &Path,
        language: &str,
        initial_prompt: Option<&str>,
    ) -> Vec<String> {
        let mut args = vec![
            "--model".into(),
            config.model_path.to_string_lossy().into(),
            "--language".into(),
            language.into(),
            "--output-json".into(),
            "--no-prints".into(),
            "--threads".into(),
            config.threads.to_string(),
            "--temperature".into(),
            config.temperature.to_string(),
            "--beam-size".into(),
            config.beam_size.to_string(),
            "--entropy-thold".into(),
            config.entropy_thold.to_string(),
            "--logprob-thold".into(),
            config.logprob_thold.to_string(),
        ];

        if config.no_timestamps {
            args.push("--no-timestamps".into());
        }

        if let Some(prompt) = initial_prompt {
            if !prompt.is_empty() {
                args.push("--prompt".into());
                args.push(prompt.into());
            }
        }

        args.push("--file".into());
        args.push(wav_path.to_string_lossy().into());
        args
    }

    /// hints と dictionary を結合して initial prompt を構築する。
    fn build_initial_prompt(audio: &AudioSegment, ctx: &SttContext) -> Option<String> {
        let mut parts: Vec<&str> = Vec::new();
        for h in &audio.hints {
            if !h.is_empty() {
                parts.push(h.as_str());
            }
        }
        for d in &ctx.dictionary {
            if !d.is_empty() {
                parts.push(d.as_str());
            }
        }
        if parts.is_empty() {
            None
        } else {
            Some(parts.join(", "))
        }
    }

    /// whisper sidecar を実行して出力をパースする。
    async fn run_whisper(
        &self,
        wav_path: &Path,
        language: &str,
        initial_prompt: Option<&str>,
    ) -> Result<WhisperOutput, SttError> {
        // config のスナップショットを取得（ロック保持を最小化）
        let config = self.config.read().clone();

        let args = Self::build_args(&config, wav_path, language, initial_prompt);

        log::debug!(
            "Running whisper: {:?} {:?}",
            config.binary_path,
            args
        );

        let child = Command::new(&config.binary_path)
            .args(&args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true)
            .spawn()
            .map_err(|e| {
                if e.kind() == std::io::ErrorKind::NotFound {
                    SttError::engine_not_available(format!(
                        "Whisper binary not found: {:?}",
                        config.binary_path
                    ))
                } else if e.kind() == std::io::ErrorKind::PermissionDenied {
                    SttError::permission_denied(format!(
                        "Cannot execute whisper binary: {}",
                        e
                    ))
                } else {
                    SttError::transcription_failed(format!(
                        "Failed to spawn whisper process: {}",
                        e
                    ))
                }
            })?;

        let timeout_duration = Duration::from_secs(config.timeout_secs);

        let output = timeout(timeout_duration, child.wait_with_output())
            .await
            .map_err(|_| {
                SttError::timeout(format!(
                    "Whisper timed out after {}s",
                    config.timeout_secs
                ))
            })?
            .map_err(|e| {
                SttError::transcription_failed(format!("Whisper process error: {}", e))
            })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(SttError::transcription_failed(format!(
                "Whisper exited with status {}: {}",
                output.status, stderr
            )));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        parse_whisper_output(&stdout)
    }
}

// ─── Whisper JSON デシリアライゼーション ──────────────────────────

/// whisper-cli --output-json の出力フォーマット。
#[derive(Debug, serde::Deserialize)]
struct WhisperJsonOutput {
    transcription: Vec<WhisperJsonSegment>,
}

#[derive(Debug, serde::Deserialize)]
struct WhisperJsonSegment {
    offsets: WhisperOffsets,
    text: String,
}

#[derive(Debug, serde::Deserialize)]
struct WhisperOffsets {
    from: u32,
    to: u32,
}

// ─── 内部結果型 ─────────────────────────────────────────────────

#[derive(Debug)]
struct WhisperOutput {
    segments: Vec<WhisperSegment>,
    full_text: String,
}

#[derive(Debug)]
struct WhisperSegment {
    text: String,
    start_ms: u32,
    end_ms: u32,
}

/// whisper JSON 出力をパースする。
fn parse_whisper_output(output: &str) -> Result<WhisperOutput, SttError> {
    let json_str = output.trim();

    if json_str.is_empty() {
        return Ok(WhisperOutput {
            segments: Vec::new(),
            full_text: String::new(),
        });
    }

    let parsed: WhisperJsonOutput = serde_json::from_str(json_str).map_err(|e| {
        SttError::transcription_failed(format!(
            "Failed to parse whisper JSON: {}. Raw: {}",
            e,
            &json_str[..json_str.len().min(500)]
        ))
    })?;

    let full_text: String = parsed
        .transcription
        .iter()
        .map(|seg| seg.text.trim())
        .collect::<Vec<&str>>()
        .join(" ")
        .trim()
        .to_string();

    let segments: Vec<WhisperSegment> = parsed
        .transcription
        .into_iter()
        .map(|seg| WhisperSegment {
            text: seg.text.trim().to_string(),
            start_ms: seg.offsets.from,
            end_ms: seg.offsets.to,
        })
        .collect();

    Ok(WhisperOutput {
        segments,
        full_text,
    })
}

/// PATH 上でバイナリを検索する簡易ヘルパー。
fn which_binary(name: &Path) -> Option<PathBuf> {
    let name_str = name.to_string_lossy();
    if name_str.contains('/') || name_str.contains('\\') {
        // 絶対/相対パスの場合はそのまま返す
        return if name.exists() { Some(name.to_path_buf()) } else { None };
    }
    // PATH 上を検索
    if let Ok(path_var) = std::env::var("PATH") {
        for dir in path_var.split(':') {
            let full_path = PathBuf::from(dir).join(name);
            if full_path.exists() {
                return Some(full_path);
            }
        }
    }
    None
}

// ─── SttEngine 実装 ─────────────────────────────────────────────

#[async_trait::async_trait]
impl SttEngine for WhisperSidecar {
    async fn transcribe(
        &self,
        audio: &AudioSegment,
        ctx: &SttContext,
    ) -> Result<Transcript, SttError> {
        // 1. WAVパス必須チェック (sidecar方式)
        let wav_path = audio.wav_path.as_ref().ok_or_else(|| {
            SttError::audio_format(
                "WhisperSidecar requires wav_path (sidecar mode needs a WAV file)",
            )
        })?;

        if !wav_path.exists() {
            return Err(SttError::audio_format(format!(
                "WAV file does not exist: {:?}",
                wav_path
            )));
        }

        // 2. 言語解決: ctx > segment > config default
        let config = self.config.read().clone();
        let language = ctx
            .language
            .as_deref()
            .or(audio.language.as_deref())
            .unwrap_or(&config.language)
            .to_string();

        // 3. hints/dictionary → initial prompt
        let initial_prompt = Self::build_initial_prompt(audio, ctx);

        // 4. whisper 実行
        let output = self
            .run_whisper(wav_path, &language, initial_prompt.as_deref())
            .await?;

        // 5. 空出力チェック
        if output.full_text.is_empty() {
            return Err(SttError::no_speech());
        }

        // 6. Transcript 構築
        let timings: Option<Vec<TimingInfo>> = if output.segments.len() > 1 {
            Some(
                output
                    .segments
                    .iter()
                    .map(|seg| TimingInfo {
                        word: seg.text.clone(),
                        start_ms: seg.start_ms,
                        end_ms: seg.end_ms,
                    })
                    .collect(),
            )
        } else {
            None
        };

        Ok(Transcript {
            text: output.full_text,
            confidence: None,
            is_partial: false,
            tokens: None,
            timings,
        })
    }

    fn supports_partial(&self) -> bool {
        false
    }

    fn name(&self) -> &str {
        "whisper.cpp"
    }
}

// ─── テスト ─────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_valid_json() {
        let json = r#"{
            "transcription": [
                {
                    "timestamps": { "from": "00:00:00,000", "to": "00:00:02,500" },
                    "offsets": { "from": 0, "to": 2500 },
                    "text": " こんにちは"
                },
                {
                    "timestamps": { "from": "00:00:02,500", "to": "00:00:05,000" },
                    "offsets": { "from": 2500, "to": 5000 },
                    "text": " 世界"
                }
            ]
        }"#;

        let output = parse_whisper_output(json).unwrap();
        assert_eq!(output.full_text, "こんにちは 世界");
        assert_eq!(output.segments.len(), 2);
        assert_eq!(output.segments[0].text, "こんにちは");
        assert_eq!(output.segments[0].start_ms, 0);
        assert_eq!(output.segments[0].end_ms, 2500);
        assert_eq!(output.segments[1].text, "世界");
    }

    #[test]
    fn parse_single_segment() {
        let json = r#"{
            "transcription": [
                {
                    "timestamps": { "from": "00:00:00,000", "to": "00:00:03,000" },
                    "offsets": { "from": 0, "to": 3000 },
                    "text": " テスト音声です"
                }
            ]
        }"#;

        let output = parse_whisper_output(json).unwrap();
        assert_eq!(output.full_text, "テスト音声です");
        assert_eq!(output.segments.len(), 1);
    }

    #[test]
    fn parse_empty_string() {
        let output = parse_whisper_output("").unwrap();
        assert!(output.full_text.is_empty());
        assert!(output.segments.is_empty());
    }

    #[test]
    fn parse_whitespace_only() {
        let output = parse_whisper_output("   \n  ").unwrap();
        assert!(output.full_text.is_empty());
    }

    #[test]
    fn parse_invalid_json() {
        let result = parse_whisper_output("{invalid json}");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.kind, crate::domain::stt::SttErrorKind::TranscriptionFailed);
    }

    #[test]
    fn validate_missing_binary() {
        let config = WhisperConfig {
            binary_path: PathBuf::from("/nonexistent/whisper-cli-xyz"),
            model_path: PathBuf::from("/nonexistent/model.bin"),
            ..Default::default()
        };
        let engine = WhisperSidecar::new(config);
        let err = engine.validate().unwrap_err();
        assert_eq!(err.kind, crate::domain::stt::SttErrorKind::EngineNotAvailable);
        assert!(!err.recoverable);
    }

    #[test]
    fn engine_name() {
        let engine = WhisperSidecar::new(WhisperConfig::default());
        assert_eq!(engine.name(), "whisper.cpp");
    }

    #[test]
    fn engine_does_not_support_partial() {
        let engine = WhisperSidecar::new(WhisperConfig::default());
        assert!(!engine.supports_partial());
    }

    #[test]
    fn build_args_format() {
        let config = WhisperConfig {
            beam_size: 5,
            ..Default::default()
        };
        let args = WhisperSidecar::build_args(
            &config,
            Path::new("/tmp/test.wav"),
            "ja",
            None,
        );

        assert!(args.contains(&"--model".to_string()));
        assert!(args.contains(&"models/ggml-base.bin".to_string()));
        assert!(args.contains(&"--language".to_string()));
        assert!(args.contains(&"ja".to_string()));
        assert!(args.contains(&"--output-json".to_string()));
        assert!(args.contains(&"--no-prints".to_string()));
        assert!(args.contains(&"--file".to_string()));
        assert!(args.contains(&"/tmp/test.wav".to_string()));
        assert!(args.contains(&"--entropy-thold".to_string()));
        assert!(args.contains(&"--logprob-thold".to_string()));
        assert!(!args.contains(&"--no-timestamps".to_string()));
        assert!(!args.contains(&"--prompt".to_string()));
    }

    #[test]
    fn build_args_with_no_timestamps() {
        let config = WhisperConfig {
            no_timestamps: true,
            ..Default::default()
        };
        let args = WhisperSidecar::build_args(
            &config,
            Path::new("/tmp/test.wav"),
            "ja",
            None,
        );
        assert!(args.contains(&"--no-timestamps".to_string()));
    }

    #[test]
    fn build_args_with_initial_prompt() {
        let config = WhisperConfig::default();
        let args = WhisperSidecar::build_args(
            &config,
            Path::new("/tmp/test.wav"),
            "ja",
            Some("プログラミング, Rust"),
        );
        assert!(args.contains(&"--prompt".to_string()));
        assert!(args.contains(&"プログラミング, Rust".to_string()));
    }

    #[test]
    fn build_args_empty_prompt_is_skipped() {
        let config = WhisperConfig::default();
        let args = WhisperSidecar::build_args(
            &config,
            Path::new("/tmp/test.wav"),
            "ja",
            Some(""),
        );
        assert!(!args.contains(&"--prompt".to_string()));
    }

    #[test]
    fn build_initial_prompt_from_hints_and_dict() {
        let audio = AudioSegment {
            hints: vec!["Rust".into(), "Tauri".into()],
            ..AudioSegment::new(vec![], 16000, chrono::Utc::now())
        };
        let ctx = SttContext {
            language: None,
            dictionary: vec!["whisper".into(), "".into()],
        };
        let prompt = WhisperSidecar::build_initial_prompt(&audio, &ctx);
        assert_eq!(prompt, Some("Rust, Tauri, whisper".to_string()));
    }

    #[test]
    fn build_initial_prompt_empty_when_no_hints() {
        let audio = AudioSegment::new(vec![], 16000, chrono::Utc::now());
        let ctx = SttContext::default();
        let prompt = WhisperSidecar::build_initial_prompt(&audio, &ctx);
        assert!(prompt.is_none());
    }

    #[test]
    fn update_config_swaps_model() {
        let engine = WhisperSidecar::new(WhisperConfig::default());
        assert_eq!(engine.config().model_path, PathBuf::from("models/ggml-base.bin"));

        let new_config = WhisperConfig {
            model_path: PathBuf::from("models/ggml-large-v3.bin"),
            ..Default::default()
        };
        engine.update_config(new_config);
        assert_eq!(engine.config().model_path, PathBuf::from("models/ggml-large-v3.bin"));
    }

    #[test]
    fn config_serialization() {
        let config = WhisperConfig::default();
        let json = serde_json::to_string(&config).unwrap();
        let config2: WhisperConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(config2.language, "ja");
        assert_eq!(config2.beam_size, 1);
        assert_eq!(config2.no_timestamps, false);
    }
}
