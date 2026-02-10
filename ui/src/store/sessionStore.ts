import { create } from "zustand";
import type {
  Mode,
  SessionState,
  DeliverPolicy,
  FinalTranscript,
} from "../lib/types";
import { invokeCommand } from "../lib/coreClient";

interface SessionStore {
  // State
  sessionState: SessionState;
  audioLevel: number;
  partialTranscript: string;
  finalTranscripts: FinalTranscript[];
  currentMode: Mode;
  sessionId: string | null;

  // Actions
  startSession: (mode?: Mode, deliverPolicy?: DeliverPolicy) => Promise<void>;
  stopSession: () => Promise<void>;
  toggleRecording: () => Promise<void>;
  setMode: (mode: Mode) => Promise<void>;
  rewriteLast: (mode: Mode) => Promise<void>;
  deliverLast: (target: string) => Promise<void>;
  clearTranscripts: () => void;

  // Event-driven setters (called by eventSetup)
  _setSessionState: (state: SessionState) => void;
  _setAudioLevel: (level: number) => void;
  _setPartialTranscript: (text: string) => void;
  _addFinalTranscript: (text: string, confidence: number) => void;
  _updateLastTranscript: (text: string) => void;
}

export const useSessionStore = create<SessionStore>((set, get) => ({
  sessionState: "idle",
  audioLevel: 0,
  partialTranscript: "",
  finalTranscripts: [],
  currentMode: "raw",
  sessionId: null,

  startSession: async (mode, deliverPolicy) => {
    const m = mode ?? get().currentMode;
    const dp: DeliverPolicy = deliverPolicy ?? { target: "clipboard" };
    const sessionId = await invokeCommand<string>("start_session", {
      mode: m,
      deliverPolicy: dp,
    });
    set({ sessionId, currentMode: m });
  },

  stopSession: async () => {
    await invokeCommand("stop_session");
  },

  toggleRecording: async () => {
    await invokeCommand("toggle_recording");
  },

  setMode: async (mode) => {
    await invokeCommand("set_mode", { mode });
    set({ currentMode: mode });
  },

  rewriteLast: async (mode) => {
    await invokeCommand("rewrite_last", { mode });
  },

  deliverLast: async (target) => {
    await invokeCommand("deliver_last", { target });
  },

  clearTranscripts: () => {
    set({ partialTranscript: "", finalTranscripts: [] });
  },

  // --- Event-driven setters ---
  _setSessionState: (state) => set({ sessionState: state }),
  _setAudioLevel: (level) => set({ audioLevel: level }),
  _setPartialTranscript: (text) => set({ partialTranscript: text }),
  _addFinalTranscript: (text, confidence) =>
    set((s) => ({
      finalTranscripts: [
        ...s.finalTranscripts,
        { text, confidence, timestamp: Date.now() },
      ],
      partialTranscript: "",
    })),
  _updateLastTranscript: (text) =>
    set((s) => {
      const transcripts = [...s.finalTranscripts];
      if (transcripts.length > 0) {
        transcripts[transcripts.length - 1] = {
          ...transcripts[transcripts.length - 1],
          text,
        };
      }
      return { finalTranscripts: transcripts };
    }),
}));
