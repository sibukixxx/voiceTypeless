# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

**voiceTypeless** is a local-first voice dictation desktop app built with **Tauri v2** (Rust backend + web frontend). It captures microphone input, transcribes via pluggable STT engines, optionally rewrites with LLM, and delivers text to the target app or clipboard.

Key design goals: low latency (<800ms to first text), local-first (minimal cloud), pluggable STT/rewrite engines, workflow integration (paste-to-app, file append, webhooks).

## Tech Stack

- **Backend**: Rust (Tauri v2 core) — state machine, audio capture, VAD, job management, SQLite storage, OS integration
- **Frontend**: React (or Preact) + Tailwind CSS — recording UI, history, dictionary management, settings
- **STT Engines** (pluggable): Apple Speech (macOS, via Swift bridge), Whisper.cpp (local, via sidecar/FFI), Cloud STT (optional)
- **IPC**: Tauri commands (UI→Rust) and events (Rust→UI)

## Project Structure

```
crates/
  core/
    domain/       # State machine, transcript types, dictionary models
    usecase/      # start_recording, stop_recording, transcribe_segment, rewrite_text, deliver_output
    infra/
      audio/      # cpal-based capture, VAD
      stt/        # apple_client, whisper_client, cloud_client (all behind STT trait)
      storage/    # SQLite repos
      output/     # clipboard, paste-to-app, file_append
    api/
      tauri_commands.rs
      events.rs
  stt-apple-bridge/   # Swift package for Speech.framework integration
ui/
  src/
    pages/        # Main recorder, History, Dictionary, Prompts, Settings
    components/
    store/        # UI state management
docs/
  contracts/      # Interface contracts between agents/modules
    commands.md   # Tauri command schemas (UI→Core)
    events.md     # Tauri event schemas (Core→UI)
    stt.md        # STT engine interface contract
```

## Architecture

### Three-boundary design

The codebase is split into three isolated domains connected only by contracts:

1. **Core/Backend** (Agent A) — Rust/Tauri: state machine, commands/events, persistence
2. **Audio/STT Pipeline** (Agent B) — Rust: mic capture, VAD, STT engine adapters
3. **Frontend/UX** (Agent C) — React/Tailwind: UI, Tauri command calls, event subscriptions

Cross-boundary communication uses only the schemas defined in `docs/contracts/`. No domain reaches into another's internals.

### Session state machine

All recording sessions follow this state flow:
`Idle → Armed → Recording → Transcribing → Rewriting → Delivering → Idle`

With an `Error { recoverable }` state reachable from any active state. The state machine is the central coordination point — UI subscribes to state change events.

### STT Engine trait

```rust
fn transcribe(audio: AudioSegment, ctx: SttContext) -> TranscriptResult;
fn supports_partial() -> bool;
```

All STT implementations (Apple Speech, Whisper.cpp, Cloud) implement this trait. Core calls it without knowing the underlying engine.

### Post-processing pipeline

STT output → Normalizer (fullwidth/halfwidth, punctuation) → Dictionary replacement → (optional) LLM rewrite → Output router

### Output routing

Delivery targets: clipboard, paste-to-active-app (with allowlist), file append, HTTP webhook. Governed by `DeliverPolicy` set per session.

## Key Contracts (IPC)

### Commands (UI → Rust)
- `start_session(mode, deliver_policy) → session_id`
- `stop_session()`, `toggle_recording()`
- `set_mode(mode)` — raw | memo | tech | minutes
- `rewrite_last(mode)`, `deliver_last(target)`
- `get_history(query, limit, cursor)`, `upsert_dictionary(entry)`, `list_prompts()`, `update_prompt()`

### Events (Rust → UI)
- `session_state_changed`, `audio_level { rms }`, `transcript_partial`, `transcript_final { text, confidence }`, `rewrite_done`, `deliver_done`, `error { code, message, recoverable }`

## Database (SQLite)

Core tables: `sessions`, `segments`, `jobs`, `dictionary_entries`, `prompts`, `settings`

Audio files are **not persisted by default** — segments are deleted after successful transcription. Optional encrypted retention with configurable TTL.

## Development Notes

### VAD (Voice Activity Detection)

Minimum viable: RMS energy threshold + silence timeout (≥700ms). Max segment length ~20-40s with forced cut. Hysteresis to avoid splitting on breath pauses.

### Apple Speech bridge

Speech.framework requires Swift — implemented as a separate Swift package (`stt-apple-bridge`). Communicates with Rust via JSON over process boundary or FFI. Requires microphone permission entitlement.

### Rewrite modes

LLM rewriting uses purpose-specific prompt templates (not freeform). Modes: `raw` (no rewrite), `memo` (filler removal + bullet points), `tech` (preserve technical terms + code blocks), `email_jp` (polite Japanese), `minutes` (decisions/TODOs/discussion points). The domain dictionary is passed to the rewriter to prevent unwanted term changes.

### Cost control

Keep STT local (Apple Speech or Whisper.cpp). LLM rewrite is optional per-invocation (hotkey toggle). Rule-based normalization handles 80% of cleanup; LLM is the final polish only.

## Development Commands

```bash
# 開発サーバー起動（Tauri + Vite dev server）
pnpm --dir ui tauri dev

# プロダクションビルド
pnpm --dir ui tauri build

# フロントエンドのみビルド
pnpm --dir ui build

# フロントエンドのみ dev server
pnpm --dir ui dev

# Rust workspace ビルド
cargo build --workspace

# Rust テスト
cargo test --workspace

# Rust lint
cargo clippy --workspace
```

### パッケージマネージャ

- **Node**: pnpm
- **Rust**: cargo (workspace)

### ディレクトリ構成

- `ui/` — React + Vite + Tailwind CSS v4 フロントエンド
- `src-tauri/` — Tauri v2 アプリケーション（Rust）
- `crates/core/` — コアライブラリ（domain, usecase, infra）
- `docs/contracts/` — 境界間の契約定義（commands, events, stt）

## Language

- Primary development language: Japanese (comments, docs, commit messages may be in Japanese)
- Code identifiers: English
