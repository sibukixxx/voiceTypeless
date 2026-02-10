# Tauri Commands (UI → Rust) — v0.1

フロントエンドからRustバックエンドへのコマンドスキーマ定義。

## 共通型

```typescript
type Mode = "raw" | "memo" | "tech" | "email_jp" | "minutes";

type DeliverPolicy = {
  target: "clipboard";  // Phase1はclipboardのみ
};

type SessionSummary = {
  session_id: string;
  state: SessionState;
  mode: Mode;
  created_at: string;   // ISO 8601
  updated_at: string;
  segment_count: number;
};

type SegmentSummary = {
  segment_id: string;
  session_id: string;
  raw_text: string;
  rewritten_text: string | null;
  confidence: number;
  created_at: string;
};

type HistoryPage = {
  items: SessionSummary[];
  next_cursor: string | null;
};

type DictionaryEntry = {
  id?: string;
  scope: "global" | "mode";
  mode?: Mode;
  pattern: string;
  replacement: string;
  priority: number;
  enabled: boolean;
};
```

---

## start_session

セッションを開始する。

```typescript
invoke('start_session', {
  mode: Mode,
  deliverPolicy: DeliverPolicy
}): Promise<string>
```

**リクエスト例:**
```json
{
  "mode": "memo",
  "deliverPolicy": { "target": "clipboard" }
}
```

**レスポンス例:**
```json
"550e8400-e29b-41d4-a716-446655440000"
```

**副作用**: `session_state_changed` イベント（state: `"idle"` → `"idle"`、セッション作成済み）を emit。

**エラー**: `E_INTERNAL`（セッション作成失敗時）

---

## stop_session

現在のアクティブセッションを停止する。

```typescript
invoke('stop_session'): Promise<void>
```

**副作用**: `session_state_changed` イベントで state を `"idle"` に遷移。Recording中なら音声キャプチャを停止し、未処理セグメントをファイナライズ。

**エラー**: `E_INTERNAL`（アクティブセッションなし時）

---

## toggle_recording

録音の開始/停止をトグルする。

```typescript
invoke('toggle_recording'): Promise<void>
```

**動作**:
- `Idle` → `Recording` に遷移（録音開始）
- `Recording` → セグメントをファイナライズし `Transcribing` に遷移
- その他の状態 → `E_INVALID_STATE`

**副作用**: `session_state_changed` イベント emit。

**エラー**: `E_INVALID_STATE`, `E_PERMISSION`（マイク権限なし）, `E_DEVICE`

---

## set_mode

書き起こし/リライトモードを変更する（セッション中でも変更可能）。

```typescript
invoke('set_mode', { mode: Mode }): Promise<void>
```

**リクエスト例:**
```json
{ "mode": "tech" }
```

**エラー**: `E_INTERNAL`（アクティブセッションなし時）

---

## get_history

セッション履歴を検索する（カーソルベースページネーション）。

```typescript
invoke('get_history', {
  query?: string,
  limit: number,
  cursor?: string
}): Promise<HistoryPage>
```

**リクエスト例:**
```json
{ "limit": 20 }
```

**レスポンス例:**
```json
{
  "items": [
    {
      "session_id": "550e8400-e29b-41d4-a716-446655440000",
      "state": "idle",
      "mode": "memo",
      "created_at": "2025-01-15T10:30:00Z",
      "updated_at": "2025-01-15T10:35:00Z",
      "segment_count": 3
    }
  ],
  "next_cursor": null
}
```

---

## get_session

特定セッションの詳細（セグメント付き）を取得する。

```typescript
invoke('get_session', { sessionId: string }): Promise<SessionDetail>
```

**レスポンス例:**
```json
{
  "session_id": "550e8400-e29b-41d4-a716-446655440000",
  "state": "idle",
  "mode": "memo",
  "created_at": "2025-01-15T10:30:00Z",
  "segments": [
    {
      "segment_id": "a1b2c3d4",
      "session_id": "550e8400-e29b-41d4-a716-446655440000",
      "raw_text": "今日の会議のアジェンダを確認します",
      "rewritten_text": "- 会議アジェンダの確認",
      "confidence": 0.92,
      "created_at": "2025-01-15T10:31:00Z"
    }
  ]
}
```

---

## upsert_dictionary

辞書エントリを追加/更新する。

```typescript
invoke('upsert_dictionary', { entry: DictionaryEntry }): Promise<string>
```

**リクエスト例:**
```json
{
  "entry": {
    "scope": "global",
    "pattern": "くろーど",
    "replacement": "Claude",
    "priority": 10,
    "enabled": true
  }
}
```

**レスポンス**: 作成/更新された `entry_id` (string)。

---

## list_dictionary

辞書エントリ一覧を取得する。

```typescript
invoke('list_dictionary', { scope?: string }): Promise<DictionaryEntry[]>
```

**レスポンス例:**
```json
[
  {
    "id": "d1e2f3",
    "scope": "global",
    "pattern": "くろーど",
    "replacement": "Claude",
    "priority": 10,
    "enabled": true
  }
]
```

---

## rewrite_last

最後のセグメントを指定モードで書き直す。

```typescript
invoke('rewrite_last', { mode: Mode }): Promise<void>
```

**副作用**: `rewrite_done` イベント emit。

**エラー**: `E_INTERNAL`（セグメントなし時）

---

## deliver_last

最後のセグメントを指定ターゲットに出力する。

```typescript
invoke('deliver_last', { target: "clipboard" }): Promise<void>
```

**副作用**: `deliver_done` イベント emit。

**エラー**: `E_INTERNAL`（セグメントなし時）
