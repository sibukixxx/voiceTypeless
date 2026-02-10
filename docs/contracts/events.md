# Tauri Events (Rust → UI) — v0.1

Rustバックエンドからフロントエンドへのイベントスキーマ定義。

## 共通型

```typescript
type SessionState =
  | "idle"
  | "recording"
  | "transcribing"
  | "rewriting"
  | "delivering"
  | { error: { code: string, message: string, recoverable: boolean } };
```

---

## session_state_changed

セッション状態が遷移した。

```typescript
listen('session_state_changed', (event: {
  payload: {
    session_id: string;
    prev_state: string;
    new_state: SessionState;
    timestamp: string;  // ISO 8601
  }
}) => void)
```

**ペイロード例:**
```json
{
  "session_id": "550e8400-e29b-41d4-a716-446655440000",
  "prev_state": "idle",
  "new_state": "recording",
  "timestamp": "2025-01-15T10:30:00Z"
}
```

---

## audio_level

マイク入力レベル（リアルタイム、Recording中のみ）。

```typescript
listen('audio_level', (event: {
  payload: {
    rms: number;  // 0.0 ~ 1.0
  }
}) => void)
```

**ペイロード例:**
```json
{ "rms": 0.42 }
```

**頻度**: ~60ms間隔（UI描画に合わせる）

---

## transcript_partial

部分的な書き起こし結果（リアルタイム更新、STTエンジンが `supports_partial()` の場合のみ）。

```typescript
listen('transcript_partial', (event: {
  payload: {
    session_id: string;
    text: string;
  }
}) => void)
```

**ペイロード例:**
```json
{
  "session_id": "550e8400-e29b-41d4-a716-446655440000",
  "text": "今日の会議の"
}
```

---

## transcript_final

確定した書き起こし結果。

```typescript
listen('transcript_final', (event: {
  payload: {
    session_id: string;
    segment_id: string;
    text: string;
    confidence: number;  // 0.0 ~ 1.0
  }
}) => void)
```

**ペイロード例:**
```json
{
  "session_id": "550e8400-e29b-41d4-a716-446655440000",
  "segment_id": "a1b2c3d4",
  "text": "今日の会議のアジェンダを確認します",
  "confidence": 0.92
}
```

---

## rewrite_done

LLM書き直し完了。

```typescript
listen('rewrite_done', (event: {
  payload: {
    session_id: string;
    segment_id: string;
    text: string;
    mode: string;
  }
}) => void)
```

**ペイロード例:**
```json
{
  "session_id": "550e8400-e29b-41d4-a716-446655440000",
  "segment_id": "a1b2c3d4",
  "text": "- 会議アジェンダの確認",
  "mode": "memo"
}
```

---

## deliver_done

出力完了。

```typescript
listen('deliver_done', (event: {
  payload: {
    session_id: string;
    target: string;
  }
}) => void)
```

**ペイロード例:**
```json
{
  "session_id": "550e8400-e29b-41d4-a716-446655440000",
  "target": "clipboard"
}
```

---

## error

エラー発生。

```typescript
listen('error', (event: {
  payload: {
    code: string;
    message: string;
    recoverable: boolean;
    session_id?: string;
  }
}) => void)
```

**ペイロード例:**
```json
{
  "code": "E_DEVICE",
  "message": "マイクデバイスが見つかりません",
  "recoverable": true,
  "session_id": "550e8400-e29b-41d4-a716-446655440000"
}
```

エラーコード一覧は `docs/contracts/error-codes.md` を参照。
