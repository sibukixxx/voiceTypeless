# Tauri Events (Rust → UI)

Rustバックエンドからフロントエンドへのイベントスキーマ定義。

## session_state_changed

セッション状態が遷移した。

```typescript
listen('session_state_changed', (event: { payload: SessionState }) => void)
```

**SessionState**: `"idle"` | `"armed"` | `"recording"` | `"transcribing"` | `"rewriting"` | `"delivering"` | `{ error: { message: string, recoverable: boolean } }`

## audio_level

マイク入力レベル（リアルタイム）。

```typescript
listen('audio_level', (event: { payload: { rms: number } }) => void)
```

- **rms**: 0.0 ~ 1.0 の正規化された音量レベル

## transcript_partial

部分的な書き起こし結果（リアルタイム更新）。

```typescript
listen('transcript_partial', (event: { payload: { text: string } }) => void)
```

## transcript_final

確定した書き起こし結果。

```typescript
listen('transcript_final', (event: { payload: { text: string, confidence: number } }) => void)
```

- **confidence**: 0.0 ~ 1.0

## rewrite_done

LLM書き直し完了。

```typescript
listen('rewrite_done', (event: { payload: { text: string, mode: string } }) => void)
```

## deliver_done

出力完了。

```typescript
listen('deliver_done', (event: { payload: { target: string } }) => void)
```

## error

エラー発生。

```typescript
listen('error', (event: { payload: { code: string, message: string, recoverable: boolean } }) => void)
```
