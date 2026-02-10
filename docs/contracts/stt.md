# STT Engine Interface Contract

**Version**: v0.1

STTエンジンの共通インタフェース定義。すべてのSTT実装（Apple Speech, Whisper.cpp, Cloud）はこのトレイトを実装する。

## Rust Trait

```rust
#[async_trait::async_trait]
pub trait SttEngine: Send + Sync {
    /// 音声セグメントを文字起こしする。
    async fn transcribe(
        &self,
        audio: &AudioSegment,
        ctx: &SttContext,
    ) -> Result<Transcript, SttError>;

    /// ストリーミング部分結果をサポートするかどうか。
    fn supports_partial(&self) -> bool;

    /// エンジン名 (例: "whisper.cpp", "apple-speech")。
    fn name(&self) -> &str;
}
```

## AudioSegment

文字起こし対象の音声セグメント。VADが生成し、Coreを経由してSTTエンジンに渡される。

```rust
pub struct AudioSegment {
    pub id: Uuid,                              // セグメント固有ID (UUID v4)
    pub started_at: DateTime<Utc>,             // 録音開始時刻
    pub duration_ms: u32,                      // セグメント長 (ms)
    pub sample_rate: u32,                      // サンプルレート (Hz, 標準: 16000)
    pub channels: u16,                         // チャンネル数 (1 = mono)
    pub pcm_format: PcmFormat,                 // F32Le | I16Le
    pub wav_path: Option<PathBuf>,             // WAVファイルパス (sidecar方式)
    pub samples: Option<Vec<f32>>,             // インメモリPCM (f32, mono)
    pub language: Option<String>,              // 言語ヒント ("ja", "en")
    pub hints: Vec<String>,                    // 認識ヒント / ドメイン辞書用語
}
```

### JSON例

```json
{
  "id": "550e8400-e29b-41d4-a716-446655440000",
  "started_at": "2026-02-11T10:30:00Z",
  "duration_ms": 3200,
  "sample_rate": 16000,
  "channels": 1,
  "pcm_format": "F32Le",
  "wav_path": "/tmp/voicetypeless_segments/550e8400-e29b-41d4-a716-446655440000.wav",
  "language": "ja",
  "hints": ["voiceTypeless", "Tauri", "Whisper"]
}
```

## Transcript

文字起こし結果。

```rust
pub struct Transcript {
    pub text: String,                          // 文字起こしテキスト
    pub confidence: Option<f32>,               // 信頼度 (0.0–1.0)
    pub is_partial: bool,                      // 部分結果フラグ
    pub tokens: Option<Vec<TokenInfo>>,        // トークン確率 (任意)
    pub timings: Option<Vec<TimingInfo>>,       // タイミング情報 (任意)
}

pub struct TokenInfo {
    pub token: String,
    pub probability: f32,
}

pub struct TimingInfo {
    pub word: String,
    pub start_ms: u32,
    pub end_ms: u32,
}
```

### JSON例

```json
{
  "text": "今日の会議の議題を確認します",
  "confidence": 0.92,
  "is_partial": false,
  "tokens": null,
  "timings": [
    { "word": "今日の", "start_ms": 0, "end_ms": 800 },
    { "word": "会議の", "start_ms": 800, "end_ms": 1500 },
    { "word": "議題を", "start_ms": 1500, "end_ms": 2200 },
    { "word": "確認します", "start_ms": 2200, "end_ms": 3200 }
  ]
}
```

## SttError

```rust
pub struct SttError {
    pub kind: SttErrorKind,                    // エラー種別
    pub detail: String,                        // 詳細メッセージ
    pub recoverable: bool,                     // リトライ可能か
}

pub enum SttErrorKind {
    AudioFormat,          // 音声フォーマット不正
    EngineNotAvailable,   // エンジン利用不可
    TranscriptionFailed,  // 文字起こし処理エラー
    Timeout,              // タイムアウト
    NoSpeech,             // 発話未検出
    PermissionDenied,     // マイク権限拒否
}
```

## SttContext

セッションレベルのコンテキスト。AudioSegment上の language/hints を補完/上書きする。

```rust
pub struct SttContext {
    pub language: Option<String>,              // 言語 (AudioSegment.language より優先)
    pub dictionary: Vec<String>,               // セッション辞書
}
```

## Implementations

| Engine | Crate | Partial Support | Notes |
|--------|-------|----------------|-------|
| Whisper.cpp | `crates/core/infra/stt` | No | sidecar CLI, ローカル |
| Apple Speech | `stt-apple-bridge` | Yes | macOS only, Swift bridge (Phase 3) |
| Cloud STT | `crates/core/infra/stt` | Yes | Optional, 要APIキー (将来) |

## Whisper.cpp Sidecar CLI I/O 仕様

### 入力
- WAVファイル: 16kHz, mono, 16-bit PCM

### コマンド例
```bash
whisper-cli \
  --model models/ggml-base.bin \
  --language ja \
  --output-json \
  --no-prints \
  --threads 4 \
  --file /tmp/segment.wav
```

### 出力 (JSON)
```json
{
  "transcription": [
    {
      "timestamps": { "from": "00:00:00,000", "to": "00:00:03,200" },
      "offsets": { "from": 0, "to": 3200 },
      "text": " 今日の会議の議題を確認します"
    }
  ]
}
```

## 変更履歴

- **v0.1** (2026-02-11): 初版。AudioSegment にメタデータ追加、SttError を struct 化、Whisper sidecar 仕様追加。
