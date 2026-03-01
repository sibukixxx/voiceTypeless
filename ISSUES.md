# ISSUES

最終更新: 2026-03-01

## 現状の課題

1. **Cloud STT が未実装で Noop にフォールバックする**
- 事象: `stt_engine = cloud` を選んでも実際には Noop STT が使われる。
- 影響: 本番利用時にモック文字列が返る可能性があり、設定と実挙動が一致しない。
- 根拠: `src-tauri/src/lib.rs` の `SttEngineChoice::Cloud` 分岐で「not yet implemented」ログの後に Noop へフォールバック。

2. **OS 統合（権限確認 / アクティブアプリ取得）がスタブ実装**
- 事象: macOS でも権限状態が `NotDetermined` 固定になり、アクティブアプリ取得も `None` を返す。
- 影響: auto-paste 判定や権限 UI が実環境と一致しない。
- 根拠: `crates/core/src/infra/os_integration.rs` の `check_microphone_permission` / `check_accessibility_permission` / `get_active_app_bundle_id`。

3. **配信先ルーティングが実質 clipboard 固定**
- 事象: `deliver_last` は `OutputRouter::deliver_clipboard` を直接呼ぶ。
- 影響: `DeliverTarget`（paste / file_append / webhook）を型として持っていても実際には使えない。
- 根拠: `crates/core/src/usecase/app_service.rs` の `deliver` / `deliver_last`。

4. **履歴検索 API の `query` が無視される**
- 事象: コマンド引数の `query` を受け取るが処理せず破棄している。
- 影響: UI 側で検索 UI を作ってもバックエンド検索が機能しない。
- 根拠: `src-tauri/src/commands.rs` の `get_history` で `let _ = args.query;`。

5. **契約ドキュメントと実装の乖離がある**
- 事象: 契約では `transcript_partial`/`transcript_final` に `session_id` を含む例があるが、実装 payload は `text`（+ `confidence`, `segment_id`）中心。
- 影響: 新規実装者が docs を信じると誤実装しやすい。
- 根拠: `docs/contracts/events.md` と `src-tauri/src/events.rs`。

6. **UI ドキュメントがテンプレート初期状態**
- 事象: `ui/README.md` が Vite 初期テンプレートのまま。
- 影響: 開発手順・画面構成・契約の参照先が UI 側から分かりにくい。
- 根拠: `ui/README.md` の内容がプロジェクト固有情報を持たない。

## 次に実装すべき機能（優先順）

1. **Cloud STT 実装（または設定から Cloud を一時非表示）**
- まず「選べるのに動かない」状態を解消する。
- 最低限: 未実装時は UI で選択不可にし、明示エラーを返す。

2. **macOS ネイティブ統合の本実装**
- 権限チェック（Microphone / Accessibility）を実値返却にする。
- frontmost app の bundle id 取得を実装し、allowlist 判定を有効化する。

3. **DeliverPolicy ベースの出力ルーター実装**
- `clipboard` 以外に `paste`, `file_append`, `webhook` を段階実装。
- `deliver_last` がセッション/設定の target を尊重するよう統一する。

4. **履歴検索のバックエンド実装**
- `get_history(query, limit, cursor)` で query を DB 検索に反映。
- UI 側検索とカーソルページングを接続し、E2E テストを追加。

5. **契約ドキュメントの同期**
- `docs/contracts/commands.md` と `docs/contracts/events.md` を現行 API に合わせて更新。
- 変更時に docs 同期を担保するチェック（PR テンプレ or CI）を追加。

6. **運用向けエラーハンドリングの強化**
- Noop フォールバック時に UI 通知を出す。
- STT エンジン初期化失敗理由（キー未設定/モデル未配置）をユーザーに表示する。
