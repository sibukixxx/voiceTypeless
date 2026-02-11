// ============================================================
// Event Setup — Tauri イベント → Zustand ストアの橋渡し
// アプリ起動時に一度だけ呼ぶ
// ============================================================

import { subscribe } from "./coreClient";
import type {
  SessionState,
  AudioLevelPayload,
  TranscriptPartialPayload,
  TranscriptFinalPayload,
  RewriteDonePayload,
  ErrorPayload,
} from "./types";

interface SessionStateChangedPayload {
  session_id: string;
  prev_state: string;
  new_state: SessionState;
  timestamp: string;
}
import { useSessionStore } from "../store/sessionStore";
import { useToastStore } from "../store/toastStore";

export async function initEventListeners(): Promise<() => void> {
  const unlisteners = await Promise.all([
    subscribe<SessionStateChangedPayload>("session_state_changed", (payload) => {
      useSessionStore.getState()._setSessionState(payload.new_state);
    }),

    subscribe<AudioLevelPayload>("audio_level", ({ rms }) => {
      useSessionStore.getState()._setAudioLevel(rms);
    }),

    subscribe<TranscriptPartialPayload>("transcript_partial", ({ text }) => {
      useSessionStore.getState()._setPartialTranscript(text);
    }),

    subscribe<TranscriptFinalPayload>(
      "transcript_final",
      ({ text, confidence }) => {
        useSessionStore.getState()._addFinalTranscript(text, confidence);
      },
    ),

    subscribe<RewriteDonePayload>("rewrite_done", ({ text }) => {
      useSessionStore.getState()._updateLastTranscript(text);
      useToastStore.getState().addToast("success", "Rewrite complete");
    }),

    subscribe<ErrorPayload>("error", ({ code, message }) => {
      useToastStore.getState().addToast("error", `[${code}] ${message}`);
    }),
  ]);

  return () => unlisteners.forEach((fn) => fn());
}
