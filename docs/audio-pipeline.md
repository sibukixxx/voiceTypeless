# Audio/STT Pipeline 技術ドキュメント

音声キャプチャ、VAD（Voice Activity Detection）、STTエンジン統合に関する技術仕様。

## アーキテクチャ概要

```
┌─────────────┐    CaptureEvent     ┌──────────────┐    VadEvent        ┌──────────────┐
│ AudioCapture │ ──────────────────→ │ VadProcessor │ ──────────────────→│ SttEngine    │
│ (cpal)       │   Frame / Level     │ (RMS閾値)    │   SegmentReady     │ (whisper.cpp)│
└─────────────┘                      └──────────────┘    (AudioSegment)  └──────────────┘
       │                                    │                                    │
       │ マイク入力                         │ WAV書き出し                        │ Transcript
       │ 16kHz/mono/f32                     │ 16kHz/mono/16bit                   │ {text, ...}
       ▼                                    ▼                                    ▼
  OS Audio API                     $TMPDIR/voicetypeless_segments/       whisper-cli sidecar
  (CoreAudio on macOS)                  {uuid}.wav                         (JSON出力)
```

### データフロー

1. **AudioCapture** が OS のマイク入力を 16kHz/mono/f32 に正規化してフレーム化
2. 各フレーム（20ms = 320サンプル）ごとに RMS を算出し `CaptureEvent::Level` として送出
3. **VadProcessor** がフレームの RMS をヒステリシス付き閾値で判定し、発話区間を検出
4. 発話終了（無音タイムアウトまたは最大長超過）時に WAV ファイルを書き出し `AudioSegment` を生成
5. **WhisperSidecar** が WAV ファイルを whisper-cli に渡し、JSON出力をパースして `Transcript` を返す

---

## モジュール構成

```
crates/core/src/
├── domain/
│   └── stt.rs                  # 契約型: AudioSegment, Transcript, SttError, SttEngine trait,
│                                #         PipelineState, PipelineEvent
└── infra/
    ├── audio/
    │   ├── mod.rs               # 公開API再エクスポート
    │   ├── buffer.rs            # ロックフリーリングバッファ
    │   ├── capture.rs           # cpal マイクキャプチャ + リサンプリング + RMS算出
    │   │                        #   + デバイス切断検知 + 権限エラー分類
    │   ├── vad.rs               # VADプロセッサ（4状態マシン + ヒステリシス）
    │   └── thread_priority.rs   # オーディオスレッド優先度ユーティリティ
    └── stt/
        ├── mod.rs               # 公開API再エクスポート
        ├── pipeline.rs          # SttPipeline（状態遷移 + イベント通知ラッパー）
        └── whisper.rs           # Whisper.cpp sidecar + SttEngine実装
```

---

## domain/stt.rs — STT契約型

Agent A（Core）と Agent B（Audio/STT Pipeline）の境界を定義する共有型。

### AudioSegment

VADが生成する音声セグメント。STTエンジンへの入力。

| フィールド | 型 | 説明 |
|-----------|-----|------|
| `id` | `Uuid` | セグメント固有ID（UUID v4） |
| `started_at` | `DateTime<Utc>` | 録音開始時刻 |
| `duration_ms` | `u32` | セグメント長（ms） |
| `sample_rate` | `u32` | サンプルレート（標準: 16000 Hz） |
| `channels` | `u16` | チャンネル数（1 = mono） |
| `pcm_format` | `PcmFormat` | `F32Le` \| `I16Le` |
| `wav_path` | `Option<PathBuf>` | WAVファイルパス（sidecar方式で必須） |
| `samples` | `Option<Vec<f32>>` | インメモリPCMサンプル（インメモリエンジン用） |
| `language` | `Option<String>` | 言語ヒント（"ja", "en"等） |
| `hints` | `Vec<String>` | 認識ヒント / ドメイン辞書用語 |

### Transcript

STTエンジンの出力。

| フィールド | 型 | 説明 |
|-----------|-----|------|
| `text` | `String` | 文字起こしテキスト |
| `confidence` | `Option<f32>` | 信頼度スコア（0.0–1.0） |
| `is_partial` | `bool` | 部分結果フラグ |
| `tokens` | `Option<Vec<TokenInfo>>` | トークン別の確率情報 |
| `timings` | `Option<Vec<TimingInfo>>` | 単語/セグメント別タイミング情報 |

### SttError

| フィールド | 型 | 説明 |
|-----------|-----|------|
| `kind` | `SttErrorKind` | エラー種別 |
| `detail` | `String` | 人間が読める詳細メッセージ |
| `recoverable` | `bool` | リトライで回復可能か |

**SttErrorKind 一覧:**

| Kind | recoverable | 典型的な原因 |
|------|-------------|-------------|
| `AudioFormat` | false | WAVパス未指定、ファイル不在、フォーマット不正 |
| `EngineNotAvailable` | false | バイナリ未検出、モデル未検出 |
| `TranscriptionFailed` | true | プロセス異常終了、JSONパースエラー |
| `Timeout` | true | 推論タイムアウト |
| `NoSpeech` | true | 音声中に発話未検出 |
| `PermissionDenied` | false | マイク権限拒否、バイナリ実行権限なし |

### SttEngine trait

```rust
#[async_trait::async_trait]
pub trait SttEngine: Send + Sync {
    async fn transcribe(&self, audio: &AudioSegment, ctx: &SttContext) -> Result<Transcript, SttError>;
    fn supports_partial(&self) -> bool;
    fn name(&self) -> &str;
}
```

`async_trait` を使用する理由: ネイティブ async trait（Rust 1.75+）は `impl Future` を返すため `dyn SttEngine` としてオブジェクトセーフにならない。`async_trait` は Future を Box 化してトレイトオブジェクトを可能にする。

---

## infra/audio/buffer.rs — リングバッファ

`ringbuf` クレートのラッパー。cpal コールバックスレッド（Producer）と処理スレッド（Consumer）間でロックフリーにサンプルを転送する。

### 使い方

```rust
use vt_core::infra::audio::RingAudioBuffer;

let buffer = RingAudioBuffer::new(2.0); // 2秒分（16kHz × 2 = 32,000サンプル）
let (mut producer, mut consumer) = buffer.split();

// cpal コールバック内（リアルタイムスレッド）
producer.push(&samples);

// 処理スレッド
let mut buf = vec![0.0f32; 320];
let read = consumer.pop(&mut buf);
```

### 設計判断

- **ロックフリー**: cpal のリアルタイムオーディオコールバックでブロッキングは禁止。SPSC（Single Producer / Single Consumer）リングバッファにより lock-free を保証。
- **オーバーフロー時の挙動**: `push()` はキャパシティ超過分を書き込まず、書き込めたサンプル数を返す。サンプルドロップはログ出力で対応（音声入力では許容範囲）。

---

## infra/audio/capture.rs — マイクキャプチャ

### CaptureConfig

| パラメータ | デフォルト | 説明 |
|-----------|----------|------|
| `target_sample_rate` | 16,000 Hz | Whisper.cpp の要求に合わせた出力レート |
| `target_channels` | 1 (mono) | モノラル出力 |
| `frame_size` | 320 サンプル | = 20ms @ 16kHz。VAD処理単位。 |

### 信号処理パイプライン

cpal コールバック内で以下を順次実行:

1. **ステレオ→モノ変換**: マルチチャンネル入力をチャンネル平均でモノラル化
2. **リサンプリング**: デバイスのネイティブレート（通常 44.1kHz/48kHz）から 16kHz へ線形補間でダウンサンプル
3. **フレーム分割**: アキュムレータに蓄積し、`frame_size` 単位でフレームを切り出し
4. **RMS算出**: フレームごとに RMS（Root Mean Square）を計算し `CaptureEvent::Level` として送出

### CaptureEvent

```rust
pub enum CaptureEvent {
    Frame(Vec<f32>),        // 16kHz/mono/f32 のフレーム（320サンプル）
    Level(f32),             // RMS レベル（0.0–1.0）— VUメータ用
    Error(String),          // ストリームエラー
    DeviceDisconnected,     // デバイス切断検知
    NoInput { silence_secs: f32 },  // 無入力検知（マイクミュート等）
}
```

### CaptureError

Phase 2 で追加されたエラー型。cpal エラーを分類する。

| CaptureErrorKind | 典型的な原因 |
|-----------------|-------------|
| `PermissionDenied` | macOS マイク権限拒否 |
| `DeviceNotFound` | デバイス未検出/切断 |
| `StreamError` | ストリーム実行中のエラー |
| `ConfigError` | サポート外のフォーマット/設定 |

### リサンプリング方式

線形補間（Linear Interpolation）を採用。音声（非音楽）用途では十分な品質。

```
ratio = from_rate / to_rate
output[i] = lerp(input[i * ratio], input[i * ratio + 1], frac)
```

48kHz→16kHz（3:1）の場合、入力3サンプルごとに1サンプルを出力。エイリアシング防止のローパスフィルタは現時点では未実装（Phase 2 で必要に応じて追加）。

### RMS計算

```
RMS = sqrt(Σ(sample²) / N)
```

戻り値は `[0.0, 1.0]` にクランプ。参考値:
- 無音: 0.0
- 通常の発話: 0.05–0.3
- フルスケール正弦波: ≈ 0.707

---

## infra/audio/vad.rs — Voice Activity Detection

### 状態マシン（4状態）

```
              RMS ≥ start_threshold         min_speech_ms 超過
  Silence ───────────────────────→ PendingSpeech ──────────────→ Speaking
     ▲                                  │                           │
     │     RMS < end_threshold          │                           │ silence_frames_needed 超過
     │     (ノイズ → 無視)              │                           │ or max_frames 到達
     │◄─────────────────────────────────┘                           │
     │                                                              ▼
     │         min_gap_ms 経過                              finalize_segment()
     └◄──────────────────────── Cooldown ◄──────────────────────────┘
```

**状態遷移の説明:**
- **Silence → PendingSpeech**: RMS が start_threshold を超えた時点で候補検知開始
- **PendingSpeech → Speaking**: min_speech_ms 以上継続 → 発話確定（SpeechStart イベント発火）
- **PendingSpeech → Silence**: 閾値を下回った → 短いノイズとして無視
- **Speaking → Cooldown**: 無音タイムアウトまたは最大長超過 → セグメント確定
- **Cooldown → Silence**: min_gap_ms 経過 → 次の発話を受け付け可能

### VadConfig

Serde 対応（Settings UI からの動的変更が可能）。

| パラメータ | デフォルト | 説明 |
|-----------|----------|------|
| `speech_start_threshold` | 0.02 | 発話開始判定 RMS 閾値 |
| `speech_end_threshold` | 0.01 | 発話終了判定 RMS 閾値（ヒステリシス） |
| `speech_end_silence_ms` | 700 ms | 無音継続でセグメント確定するまでの時間 |
| `max_segment_ms` | 30,000 ms | セグメント最大長（強制カット） |
| `min_segment_ms` | 500 ms | セグメント最小長（短すぎれば破棄） |
| `min_speech_ms` | 100 ms | 発話確定に必要な最小継続時間（短ノイズフィルタ） |
| `min_gap_ms` | 300 ms | セグメント間の最小ギャップ（過剰分割防止） |
| `sample_rate` | 16,000 Hz | 入力サンプルレート |
| `frame_size` | 320 | フレームサイズ（= 20ms @ 16kHz） |
| `output_dir` | `$TMPDIR/voicetypeless_segments/` | WAV出力先 |

### ヒステリシス

`speech_start_threshold`（0.02）> `speech_end_threshold`（0.01）に設定することで、閾値付近での高速な状態遷移のチャタリングを防止する。

```
RMS
0.03 ─ ─ ─ ─ ─ ─ ─ ┐            ┌─ ─ ─
                     │            │
0.02 ═══════════════╪════════════╪═══════ start_threshold
                     │  ↓ここでは │
0.015                │  まだ      │
                     │  Speaking  │
0.01 ────────────────╪────────────╪─────── end_threshold
                     │            │
0.005                └────────────┘ ← ここで初めて Silence に遷移
```

### VadEvent

| イベント | 発火条件 | 用途 |
|---------|---------|------|
| `SpeechStart` | Silence → Speaking 遷移時 | UI の録音インジケータ表示 |
| `SegmentReady(AudioSegment)` | 無音タイムアウトまたは最大長超過 | STTエンジンへの入力 |
| `SegmentDiscarded { duration_ms }` | セグメントが `min_segment_ms` 未満 | ログ/デバッグ |
| `SegmentForceCut` | `max_segment_ms` 超過 | ログ。直後に `SegmentReady` が続く |

### WAV書き出し

セグメント確定時に `hound` クレートで WAV ファイルを書き出す:
- フォーマット: 16kHz / mono / 16-bit signed PCM
- ファイル名: `{segment_uuid}.wav`
- 書き出し先: `VadConfig.output_dir`
- f32→i16 変換: `(sample * 32767.0).clamp(-32768.0, 32767.0) as i16`

WAV 書き出し失敗時は `wav_path: None` のインメモリフォールバックで `SegmentReady` を発火する。

### flush()

録音停止時に `flush()` を呼ぶことで、Speaking/PendingSpeech 状態中の未確定バッファを強制的にセグメント化する。Silence/Cooldown 状態では何もしない。

### cleanup_old_segments()

古い WAV ファイルをクリーンアップするユーティリティ。指定した `max_age` より古いファイルを output_dir から削除する。

### 性能最適化

- **バッファ事前確保**: `segment_samples` は最大10秒分（160,000サンプル）を事前確保
- **バッファ付きWAV書き出し**: `BufWriter`（64KB）でディスクI/Oを最小化

---

## infra/audio/thread_priority.rs — スレッド優先度

オーディオコールバックスレッドの優先度を引き上げるユーティリティ。

- **macOS**: `pthread_setschedparam` で `SCHED_RR`（Round Robin）を設定。root 権限がない場合は gracefully degrade。
- **他のプラットフォーム**: ベストエフォート（ログ出力のみ）。

使い方: AudioCapture のコールバックスレッド開始時に `set_audio_thread_priority()` を呼ぶ。

---

## infra/stt/pipeline.rs — STT Pipeline

SttEngine の呼び出しをパイプライン状態遷移で包み、`PipelineEvent` を mpsc チャンネル経由で通知するラッパー。

### PipelineState

```
Idle → Listening → Capturing → Processing → Listening → ...
                                    ↓ (fatal error)
                                   Idle
```

| 状態 | 説明 |
|------|------|
| `Idle` | 待機中（マイク停止） |
| `Listening` | 録音中（音声検出待ち） |
| `Capturing` | 音声検出中（VAD speech 検出、セグメント蓄積中） |
| `Processing` | 文字起こし処理中（Whisper 実行中） |

### PipelineEvent

| イベント | 説明 |
|---------|------|
| `StateChanged(PipelineState)` | 状態遷移通知 |
| `PartialTranscript(Transcript)` | 部分結果（ストリーミングエンジン用） |
| `FinalTranscript(Transcript)` | 最終結果 |
| `Error(SttError)` | エラー発生（recoverable なら Listening に復帰） |
| `AudioLevel(f32)` | VU メータレベル |

### エラー時の挙動

- `recoverable = true`: エラーイベント発火後、Listening 状態に復帰
- `recoverable = false`: エラーイベント発火後、Idle 状態に遷移（パイプライン停止）

---

## infra/stt/whisper.rs — Whisper.cpp Sidecar

### 実行方式

whisper.cpp を **sidecar プロセス**（子プロセス）として実行する。ライブラリ直リンク（FFI）ではなく CLI 経由にした理由:

1. **ビルドの単純化**: whisper.cpp のC/C++ビルドチェーンを Cargo に組み込む必要がない
2. **配布の柔軟性**: Apple Silicon / Intel 用のバイナリを個別に同梱可能
3. **障害分離**: whisper プロセスがクラッシュしてもアプリ本体は生存
4. **依存の薄さ**: Core クレートから whisper.cpp への直接依存がない

### WhisperConfig

Serde 対応（設定ファイル保存/読込が可能）。`update_config()` で動的にモデル切替可能。

| パラメータ | デフォルト | 説明 |
|-----------|----------|------|
| `binary_path` | `"whisper-cli"` | whisper-cli バイナリパス（PATHも検索） |
| `model_path` | `"models/ggml-base.bin"` | GGML モデルファイルパス |
| `language` | `"ja"` | デフォルト言語 |
| `temperature` | 0.0 | サンプリング温度（0.0 = 決定的） |
| `beam_size` | 1 | ビームサーチ幅（1 = greedy） |
| `timeout_secs` | 30 | タイムアウト秒数 |
| `threads` | 4 | 推論スレッド数 |
| `no_timestamps` | false | タイムスタンプ出力無効化（処理高速化） |
| `entropy_thold` | 2.4 | エントロピー閾値（高エントロピーセグメントをフィルタ） |
| `logprob_thold` | -1.0 | 対数確率閾値（低確率セグメントをフィルタ） |

### Hints / Initial Prompt

`AudioSegment.hints` と `SttContext.dictionary` を結合して whisper-cli の `--prompt` パラメータとして渡す。これにより認識精度が向上する（ドメイン用語を事前に指定可能）。

```
hints: ["Rust", "Tauri"] + dictionary: ["whisper"] → --prompt "Rust, Tauri, whisper"
```

### CLI 呼び出し

```bash
whisper-cli \
  --model models/ggml-base.bin \
  --language ja \
  --output-json \
  --no-prints \
  --threads 4 \
  --temperature 0 \
  --beam-size 1 \
  --entropy-thold 2.4 \
  --logprob-thold -1 \
  --prompt "ドメイン用語1, 用語2" \
  --file /tmp/voicetypeless_segments/{uuid}.wav
```

### JSON出力パース

whisper-cli `--output-json` の出力形式:

```json
{
  "transcription": [
    {
      "timestamps": { "from": "00:00:00,000", "to": "00:00:02,500" },
      "offsets": { "from": 0, "to": 2500 },
      "text": " こんにちは"
    }
  ]
}
```

`offsets.from` / `offsets.to` をミリ秒として `TimingInfo` にマッピング。複数セグメントの `text` をスペース結合して `Transcript.text` とする。

### 言語解決の優先順位

```
SttContext.language  >  AudioSegment.language  >  WhisperConfig.language
（セッション指定）       （セグメント指定）          （デフォルト設定）
```

### エラーハンドリング

| 条件 | io::ErrorKind | SttErrorKind | recoverable |
|------|--------------|-------------|-------------|
| バイナリ未検出 | `NotFound` | `EngineNotAvailable` | false |
| 実行権限なし | `PermissionDenied` | `PermissionDenied` | false |
| プロセス起動失敗 | その他 | `TranscriptionFailed` | true |
| 推論タイムアウト | — | `Timeout` | true |
| 非ゼロ終了 | — | `TranscriptionFailed` | true |
| JSON パース失敗 | — | `TranscriptionFailed` | true |
| 出力テキスト空 | — | `NoSpeech` | true |

`kill_on_drop(true)` により、Rust 側で Future がキャンセルされた場合にも whisper プロセスを確実に終了する。

---

## 依存クレート

| クレート | バージョン | 用途 |
|---------|----------|------|
| `cpal` | 0.15 | クロスプラットフォーム音声キャプチャ（macOS では CoreAudio） |
| `ringbuf` | 0.4 | ロックフリー SPSC リングバッファ |
| `hound` | 3.5 | WAV ファイル読み書き（Pure Rust、Cランタイム不要） |
| `async-trait` | 0.1 | `SttEngine` trait のオブジェクトセーフティ |
| `uuid` | 1 (v4, serde) | セグメントID生成 |
| `chrono` | 0.4 (serde) | タイムスタンプ |
| `parking_lot` | 0.12 | 高速 RwLock/Mutex（WhisperSidecar config、cpal コールバック用） |
| `tokio` | 1 (full) | 非同期ランタイム、プロセス実行、タイムアウト、mpsc チャンネル |
| `log` | 0.4 | ログ出力 |
| `libc` | 0.2 | pthread スレッド優先度設定（macOS） |

---

## テスト

### ユニットテスト（`cargo test --workspace`）

| モジュール | テスト数 | テスト内容 |
|-----------|---------|-----------|
| `domain::stt` | 6 | AudioSegment 生成、duration_ms 算出、SttError コンストラクタ、Transcript シリアライズ |
| `infra::audio::buffer` | 5 | push/pop ラウンドトリップ、部分読み取り、空バッファ、オーバーフロー、秒数指定 |
| `infra::audio::capture` | 12 | RMS算出、リサンプリング、ゼロフレーム検知、デバイス切断検知、CaptureErrorコンストラクタ、デバイス一覧 |
| `infra::audio::vad` | 16 | 4状態遷移、min_speech_ms/min_gap_ms、cooldown、動的設定更新、Serde、WAV検証 |
| `infra::audio::thread_priority` | 1 | 優先度設定がパニックしないこと |
| `infra::stt::pipeline` | 5 | 状態遷移（成功/recoverable error/fatal error）、AudioLevel、エンジン名 |
| `infra::stt::whisper` | 15 | JSONパース、引数構築（timestamps/prompt/空prompt）、hints構築、モデル切替、Serde |
| **合計** | **60** | — |

### 統合テスト（`cargo test --test stt_integration`）

| テスト | `#[ignore]` | 説明 |
|-------|-------------|------|
| `transcribe_known_wav` | Yes | 既知WAV → WhisperSidecar → Transcript.text 検証 |
| `transcribe_missing_wav` | No | 存在しない WAV パス → `SttErrorKind::AudioFormat` |
| `transcribe_no_wav_path` | No | wav_path 未指定 → `SttErrorKind::AudioFormat` |

`#[ignore]` テストの実行:

```bash
WHISPER_BIN=/path/to/whisper-cli \
WHISPER_MODEL=/path/to/ggml-base.bin \
cargo test --test stt_integration -- --ignored
```

---

## Phase 2 で追加された機能

| 機能 | 概要 |
|------|------|
| B6: デバイス/権限の堅牢化 | `CaptureError`/`CaptureErrorKind`、デバイス切断検知、ゼロフレーム検知、`try_reconnect()`、`list_input_devices()` |
| B7: VAD改善 | 4状態マシン（Silence/PendingSpeech/Speaking/Cooldown）、`min_speech_ms`、`min_gap_ms`、`update_config()`、Serde |
| B8: Whisperパラメータ最適化 | Serde config、`--prompt`（hints/dictionary）、`entropy_thold`/`logprob_thold`、`no_timestamps`、`update_config()`（モデル切替） |
| B9: Partial Transcript | `PipelineState`/`PipelineEvent` 型、`SttPipeline`（状態遷移 + mpsc イベント通知） |
| B10: 性能最適化 | バッファ事前確保、`BufWriter` WAV書き出し、`cleanup_old_segments()`、`set_audio_thread_priority()` |

## 今後の拡張予定

| Phase | 内容 |
|-------|------|
| Phase 3 | Apple Speech ブリッジ（Swift sidecar）、タイムスタンプ/単語境界、ノイズ抑制、sidecar配布戦略 |
