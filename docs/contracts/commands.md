# Tauri Commands (UI → Rust)

フロントエンドからRustバックエンドへのコマンドスキーマ定義。

## start_session

セッションを開始する。

```typescript
invoke('start_session', { mode: Mode, deliverPolicy: DeliverPolicy }): Promise<string>
```

- **mode**: `"raw"` | `"memo"` | `"tech"` | `"email_jp"` | `"minutes"`
- **deliverPolicy**: `{ target: "clipboard" | "paste" | "file_append" | "webhook", config?: object }`
- **returns**: `session_id` (UUID string)

## stop_session

現在のセッションを停止する。

```typescript
invoke('stop_session'): Promise<void>
```

## toggle_recording

録音の開始/停止をトグルする。

```typescript
invoke('toggle_recording'): Promise<void>
```

## set_mode

書き起こしモードを変更する。

```typescript
invoke('set_mode', { mode: Mode }): Promise<void>
```

## rewrite_last

最後のセグメントを指定モードで書き直す。

```typescript
invoke('rewrite_last', { mode: Mode }): Promise<void>
```

## deliver_last

最後のセグメントを指定ターゲットに出力する。

```typescript
invoke('deliver_last', { target: DeliverTarget }): Promise<void>
```

## get_history

履歴を検索する。

```typescript
invoke('get_history', { query?: string, limit: number, cursor?: string }): Promise<HistoryPage>
```

## upsert_dictionary

辞書エントリを追加/更新する。

```typescript
invoke('upsert_dictionary', { entry: DictionaryEntry }): Promise<void>
```

## list_prompts

プロンプト一覧を取得する。

```typescript
invoke('list_prompts'): Promise<Prompt[]>
```

## update_prompt

プロンプトを更新する。

```typescript
invoke('update_prompt', { prompt: Prompt }): Promise<void>
```
