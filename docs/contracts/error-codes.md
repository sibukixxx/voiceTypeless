# Error Codes — v0.1

Core共通のエラーコード定義。`error` イベントの `code` フィールドに使用。

## コード一覧

| Code | 意味 | recoverable | 説明 |
|------|------|-------------|------|
| `E_PERMISSION` | 権限エラー | `true` | マイク/アクセシビリティ権限が不足。ユーザーにシステム設定を案内。 |
| `E_DEVICE` | デバイスエラー | `true` | マイクデバイスが見つからない、または使用中。デバイス再接続で復帰可能。 |
| `E_TIMEOUT` | タイムアウト | `true` | STT処理やリライト処理がタイムアウト。リトライ可能。 |
| `E_STT_UNAVAILABLE` | STTエンジン利用不可 | `true` | 選択されたSTTエンジンが利用できない（未インストール、API Key不正等）。 |
| `E_INVALID_STATE` | 不正な状態遷移 | `true` | 現在の状態では許可されない操作（例: Transcribing中にtoggle_recording）。 |
| `E_INTERNAL` | 内部エラー | `false` | 想定外のエラー。ログを確認。 |
| `E_STORAGE` | ストレージエラー | `false` | SQLiteの読み書きに失敗。 |
| `E_REWRITE` | リライトエラー | `true` | LLMリライト処理に失敗。raw textは保持。 |

## エラーペイロード

```typescript
{
  code: string;         // 上記コードのいずれか
  message: string;      // ユーザー向けメッセージ（日本語）
  recoverable: boolean; // UIがリトライUIを表示するかの判断に使用
  session_id?: string;  // エラーが特定セッションに紐づく場合
}
```

## Rust側の型

```rust
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct AppError {
    pub code: ErrorCode,
    pub message: String,
    pub recoverable: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ErrorCode {
    EPermission,
    EDevice,
    ETimeout,
    ESttUnavailable,
    EInvalidState,
    EInternal,
    EStorage,
    ERewrite,
}
```
