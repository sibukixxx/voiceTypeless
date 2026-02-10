# voiceTypeless

ローカルファースト音声入力デスクトップアプリ。マイク入力をリアルタイムで文字起こしし、オプションでLLMによるリライトを行い、クリップボードやアクティブアプリにテキストを出力します。

## 特徴

- **ローカルファースト** — Apple Speech / Whisper.cpp によるオンデバイスSTT
- **低レイテンシ** — 初回テキスト表示まで800ms以下を目標
- **プラガブルSTT** — Apple Speech, Whisper.cpp, Cloud STT を切り替え可能
- **リライトモード** — raw（そのまま）/ memo（箇条書き）/ tech（技術用語保持）/ email_jp（丁寧な日本語）/ minutes（議事録）
- **辞書機能** — ドメイン固有の用語変換（パターン→置換）
- **出力ルーティング** — クリップボード、アクティブアプリへのペースト、ファイル追記、Webhook

## 技術スタック

| レイヤー | 技術 |
|---------|------|
| バックエンド | Rust + Tauri v2 |
| フロントエンド | React + Tailwind CSS v4 + Vite |
| STT | Apple Speech (macOS) / Whisper.cpp |
| ストレージ | SQLite (rusqlite) |
| IPC | Tauri commands + events |

## プロジェクト構成

```
voiceTypeless/
├── src-tauri/          # Tauri v2 アプリケーション (Rust)
├── crates/
│   └── core/           # コアライブラリ (vt-core)
│       ├── domain/     # 状態マシン、型定義、辞書モデル
│       ├── usecase/    # アプリケーションサービス、ジョブキュー
│       └── infra/      # STT、ストレージ、出力、後処理
├── ui/                 # React フロントエンド
│   └── src/
│       ├── pages/      # Recorder, History, Dictionary, Settings, etc.
│       ├── components/ # AppShell, Sidebar, etc.
│       ├── store/      # Zustand stores
│       └── lib/        # イベントリスナー、ユーティリティ
└── docs/
    └── contracts/      # IPC 契約定義
```

## セットアップ

### 前提条件

- [Rust](https://rustup.rs/) (1.77.2+)
- [Node.js](https://nodejs.org/) (20+)
- [pnpm](https://pnpm.io/)
- macOS (Apple Speech を使う場合)

### インストール

```bash
# フロントエンド依存のインストール
pnpm --dir ui install

# 開発サーバー起動（Tauri + Vite）
pnpm --dir ui tauri dev
```

### ビルドコマンド

```bash
# 開発サーバー起動
pnpm --dir ui tauri dev

# プロダクションビルド
pnpm --dir ui tauri build

# フロントエンドのみビルド
pnpm --dir ui build

# Rust workspace ビルド
cargo build --workspace

# テスト
cargo test --workspace

# Lint
cargo clippy --workspace
```

## アーキテクチャ

### 三境界設計

1. **Core/Backend** — Rust/Tauri: 状態マシン、コマンド/イベント、永続化
2. **Audio/STT Pipeline** — Rust: マイクキャプチャ、VAD、STTエンジンアダプタ
3. **Frontend/UX** — React/Tailwind: UI、Tauriコマンド呼び出し、イベント購読

境界間の通信は `docs/contracts/` で定義されたスキーマのみを使用します。

### セッション状態マシン

```
Idle → Recording → Transcribing → Rewriting → Delivering → Idle
```

### 後処理パイプライン

```
STT出力 → 正規化 → 辞書置換 → (LLMリライト) → 出力ルーター
```

## ライセンス

TBD
