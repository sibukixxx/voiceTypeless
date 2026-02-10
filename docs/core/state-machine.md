# Session State Machine — v0.1

## 状態一覧

| State | 意味 |
|-------|------|
| `Idle` | セッション待機中。録音していない。 |
| `Recording` | マイクキャプチャ中。VADがセグメント区切りを検出する。 |
| `Transcribing` | STTエンジンがセグメントを処理中。 |
| `Rewriting` | LLMリライト処理中（modeがrawの場合はスキップ）。 |
| `Delivering` | 出力ルーターがテキストを配信中。 |
| `Error` | エラー状態。`recoverable` フラグで復帰可否を示す。 |

## 状態遷移図

```
                    ┌──────────────────────────────┐
                    │          Error               │
                    │  { code, msg, recoverable }  │
                    └──────┬───────────────────────┘
                           │ recoverable → Idle
                           │ !recoverable → (セッション終了)
                           ▲
                    (任意の状態からエラーへ遷移可能)
                           │
     ┌─────┐  toggle   ┌──────────┐  segment_done  ┌──────────────┐
     │     │──────────▶│          │───────────────▶│              │
     │Idle │           │Recording │                │ Transcribing │
     │     │◀──────────│          │◀───(継続録音)──│              │
     └─────┘  stop     └──────────┘                └──────┬───────┘
        ▲                                                  │
        │                                          transcript_done
        │                                                  │
        │                                                  ▼
        │              ┌────────────┐              ┌──────────────┐
        │              │            │◀─────────────│              │
        └──────────────│ Delivering │  rewrite_done│  Rewriting   │
          deliver_done │            │              │              │
                       └────────────┘              └──────────────┘
                                                          │
                                              (mode=raw → skip)
```

## 遷移ルール

| 現在の状態 | トリガー | 次の状態 | 備考 |
|-----------|---------|---------|------|
| Idle | `toggle_recording` | Recording | マイクキャプチャ開始 |
| Idle | `stop_session` | Idle | セッション終了（DBに保存） |
| Recording | `toggle_recording` | Transcribing | セグメントファイナライズ→STT開始 |
| Recording | `stop_session` | Transcribing | 最後のセグメントをファイナライズ→STT→Idle |
| Recording | `segment_done` (VAD) | Transcribing | VADが無音検出→自動セグメント区切り（録音は継続） |
| Transcribing | `transcript_done` | Rewriting | mode≠raw の場合 |
| Transcribing | `transcript_done` | Delivering | mode=raw の場合（rewriteスキップ） |
| Transcribing | `stop_session` | (cancel→Idle) | 処理中断、部分結果は破棄 |
| Rewriting | `rewrite_done` | Delivering | リライト完了 |
| Rewriting | `stop_session` | (cancel→Idle) | raw textは保持、rewrite結果は破棄 |
| Delivering | `deliver_done` | Idle | 出力完了。次のセグメント待ち。 |
| Error | (recoverable) `retry` / `toggle` | Idle | 復帰可能な場合 |
| Error | (!recoverable) | — | セッション終了 |

## MVPでの制約

- **単一アクティブセッション**：同時に1つのセッションのみ。新しい `start_session` は古いセッションを自動停止。
- **Recording中の `toggle_recording`** は stop→segment finalize の意味。
- **Transcribing中の `stop_session`** は cancel policy: 処理中断し Idle に戻す（部分結果は保存しない）。
- **セグメント自動分割**: VADの無音検出は Recording→Transcribing 遷移を発火するが、録音自体は Recording に留まる（つまり、内部的には Recording 中に並行して Transcribing が走る）。MVP では **直列処理**（1セグメントずつ）。
