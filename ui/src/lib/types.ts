// ============================================================
// Contract Types — docs/contracts/ に準拠
// UI は commands/events の契約にだけ依存する
// ============================================================

// === Modes ===
export type Mode = "raw" | "memo" | "tech" | "email_jp" | "minutes";

export const MODE_LABELS: Record<Mode, string> = {
  raw: "Raw",
  memo: "Memo",
  tech: "Tech",
  email_jp: "Email JP",
  minutes: "Minutes",
};

export const MODE_DESCRIPTIONS: Record<Mode, string> = {
  raw: "そのまま出力",
  memo: "要点を箇条書き",
  tech: "技術用語を保持",
  email_jp: "丁寧な日本語メール",
  minutes: "決定/ToDo/論点",
};

// === Deliver ===
export type DeliverTarget = "clipboard" | "paste" | "file_append" | "webhook";

export interface DeliverPolicy {
  target: DeliverTarget;
  config?: Record<string, unknown>;
}

// === Session State ===
export interface SessionStateError {
  error: { message: string; recoverable: boolean };
}
export type SessionStateSimple =
  | "idle"
  | "armed"
  | "recording"
  | "transcribing"
  | "rewriting"
  | "delivering";
export type SessionState = SessionStateSimple | SessionStateError;

export function isErrorState(state: SessionState): state is SessionStateError {
  return typeof state === "object" && "error" in state;
}

export function getStateLabel(state: SessionState): string {
  if (isErrorState(state)) return "Error";
  const labels: Record<SessionStateSimple, string> = {
    idle: "Idle",
    armed: "Armed",
    recording: "Recording",
    transcribing: "Transcribing...",
    rewriting: "Rewriting...",
    delivering: "Delivering...",
  };
  return labels[state];
}

export function isActiveState(state: SessionState): boolean {
  if (isErrorState(state)) return false;
  return state !== "idle";
}

export function isRecording(state: SessionState): boolean {
  return !isErrorState(state) && state === "recording";
}

export function isBusy(state: SessionState): boolean {
  if (isErrorState(state)) return false;
  return (
    state === "transcribing" ||
    state === "rewriting" ||
    state === "delivering"
  );
}

// === Event Payloads ===
export interface AudioLevelPayload {
  rms: number;
}

export interface TranscriptPartialPayload {
  text: string;
}

export interface TranscriptFinalPayload {
  text: string;
  confidence: number;
  segment_id?: string;
}

export interface RewriteDonePayload {
  session_id: string;
  segment_id: string;
  text: string;
  mode: string;
}

export interface DeliverDonePayload {
  target: string;
}

export interface ErrorPayload {
  code: string;
  message: string;
  recoverable: boolean;
}

// === History ===
export interface HistoryItem {
  id: string;
  session_id: string;
  text: string;
  mode: Mode;
  confidence: number;
  created_at: string;
}

export interface HistoryPage {
  items: HistoryItem[];
  cursor: string | null;
  has_more: boolean;
}

// === Dictionary ===
export type DictionaryScope = "global" | "app" | "project" | "mode";

export interface DictionaryEntry {
  id?: string;
  pattern: string;
  replacement: string;
  scope: DictionaryScope;
  priority: number;
  enabled: boolean;
}

// === Prompts ===
export interface Prompt {
  id: string;
  mode: Mode;
  template: string;
  description: string;
}

// === Settings ===
export type SttEngine = "apple" | "whisper" | "cloud";
export type AudioRetention = "none" | "1day" | "7days" | "30days";
export type DeliverPolicyType =
  | "clipboard_only"
  | "paste_allowlist"
  | "confirm";

export interface AppSettings {
  stt_engine: SttEngine;
  deliver_policy_type: DeliverPolicyType;
  audio_retention: AudioRetention;
  hotkey: string;
  paste_allowlist: string[];
  claude_api_key?: string;
  language: string;
  rewrite_enabled: boolean;
}

// === Permissions (Phase 3) ===
export interface PermissionStatus {
  microphone: boolean;
  accessibility: boolean;
}

// === Metrics (Phase 3) ===
export interface LatencyMetrics {
  record_to_transcribe_ms: number;
  transcribe_to_deliver_ms: number;
  total_ms: number;
}

export interface MetricError {
  timestamp: string;
  code: string;
  message: string;
}

// === UI Navigation ===
export type Page =
  | "recorder"
  | "history"
  | "dictionary"
  | "settings"
  | "permissions"
  | "metrics"
  | "paste_allowlist";

// === Transcript (UI internal) ===
export interface FinalTranscript {
  text: string;
  confidence: number;
  timestamp: number;
  segmentId?: string;
  rawText?: string;
  rewrittenText?: string;
  isRewriting?: boolean;
}

// === Toast (UI internal) ===
export type ToastType = "info" | "success" | "error" | "warning";

export interface Toast {
  id: string;
  type: ToastType;
  message: string;
}
