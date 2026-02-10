// Mock for @tauri-apps/api in test environment
// Since tests run outside Tauri, coreClient falls back to mock mode automatically.
// This file provides helpers for simulating Tauri events in tests.

import { useSessionStore } from "../store/sessionStore";
import { useToastStore } from "../store/toastStore";
import type { SessionState } from "../lib/types";

/**
 * Simulate a session state change event
 */
export function emitSessionStateChanged(state: SessionState) {
  useSessionStore.getState()._setSessionState(state);
}

/**
 * Simulate an audio level event
 */
export function emitAudioLevel(rms: number) {
  useSessionStore.getState()._setAudioLevel(rms);
}

/**
 * Simulate a partial transcript event
 */
export function emitTranscriptPartial(text: string) {
  useSessionStore.getState()._setPartialTranscript(text);
}

/**
 * Simulate a final transcript event
 */
export function emitTranscriptFinal(text: string, confidence: number) {
  useSessionStore.getState()._addFinalTranscript(text, confidence);
}

/**
 * Simulate an error event
 */
export function emitError(code: string, message: string) {
  useToastStore.getState().addToast("error", `[${code}] ${message}`);
}

/**
 * Reset all stores to initial state
 */
export function resetAllStores() {
  useSessionStore.setState({
    sessionState: "idle",
    audioLevel: 0,
    partialTranscript: "",
    finalTranscripts: [],
    currentMode: "raw",
    sessionId: null,
  });
  useToastStore.setState({ toasts: [] });
}
