# voiceTypeless UI

`ui/` は voiceTypeless のフロントエンドです。  
Tauri バックエンドと `invoke` / `listen` で連携し、録音操作・文字起こし表示・辞書管理・設定管理を提供します。

## 技術スタック

- React 19
- TypeScript
- Vite
- Tailwind CSS v4
- Zustand
- Vitest + Testing Library

## セットアップ

リポジトリルートで実行:

```bash
pnpm --dir ui install
```

## 開発コマンド

リポジトリルートで実行:

```bash
# UI 単体開発サーバー
pnpm --dir ui dev

# Tauri + UI 統合起動
pnpm --dir ui tauri dev

# 型チェック + 本番ビルド
pnpm --dir ui build

# Lint
pnpm --dir ui lint

# テスト
pnpm --dir ui test
pnpm --dir ui test:watch
pnpm --dir ui test:coverage
```

## 画面構成

主画面 (`AppShell` ヘッダーから遷移):

- `Recorder` (`src/pages/RecorderPage.tsx`)
- `History` (`src/pages/HistoryPage.tsx`)
- `Dictionary` (`src/pages/DictionaryPage.tsx`)
- `Settings` (`src/pages/SettingsPage.tsx`)

補助画面（状態や設定から遷移）:

- `Permissions` (`src/pages/PermissionsPage.tsx`)
- `Metrics` (`src/pages/MetricsPage.tsx`)
- `Paste Allowlist` (`src/pages/PasteAllowlistPage.tsx`)

## ディレクトリ構成

```text
ui/
├── src/
│   ├── components/   # AppShell, Recorder UI, 共通 UI 部品
│   ├── pages/        # 画面コンポーネント
│   ├── store/        # Zustand ストア
│   ├── lib/          # Tauri クライアント、イベント購読、型
│   └── test/         # Vitest テスト
├── vite.config.ts
└── vitest.config.ts
```

## 契約 (Contracts)

UI は `docs/contracts/` を契約の一次情報源として実装します:

- コマンド: `../docs/contracts/commands.md`
- イベント: `../docs/contracts/events.md`
- エラーコード: `../docs/contracts/error-codes.md`
- STT 仕様: `../docs/contracts/stt.md`

UI の型定義は `src/lib/types.ts` にあり、上記契約と整合する前提です。
