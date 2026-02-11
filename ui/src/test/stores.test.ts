import { describe, it, expect, beforeEach } from "vitest";
import { useSessionStore } from "../store/sessionStore";
import { useToastStore } from "../store/toastStore";
import { useNavigationStore } from "../store/navigationStore";
import { useHistoryStore } from "../store/historyStore";
import { useDictionaryStore } from "../store/dictionaryStore";
import { useSettingsStore } from "../store/settingsStore";

describe("sessionStore", () => {
  beforeEach(() => {
    useSessionStore.setState({
      sessionState: "idle",
      audioLevel: 0,
      partialTranscript: "",
      finalTranscripts: [],
      currentMode: "raw",
      sessionId: null,
    });
  });

  it("has correct initial state", () => {
    const state = useSessionStore.getState();
    expect(state.sessionState).toBe("idle");
    expect(state.audioLevel).toBe(0);
    expect(state.partialTranscript).toBe("");
    expect(state.finalTranscripts).toEqual([]);
    expect(state.currentMode).toBe("raw");
    expect(state.sessionId).toBeNull();
  });

  it("_setSessionState updates state", () => {
    useSessionStore.getState()._setSessionState("recording");
    expect(useSessionStore.getState().sessionState).toBe("recording");
  });

  it("_setSessionState handles error state", () => {
    const error = { error: { message: "Mic error", recoverable: true } };
    useSessionStore.getState()._setSessionState(error);
    expect(useSessionStore.getState().sessionState).toEqual(error);
  });

  it("_setAudioLevel updates level", () => {
    useSessionStore.getState()._setAudioLevel(0.75);
    expect(useSessionStore.getState().audioLevel).toBe(0.75);
  });

  it("_setPartialTranscript updates text", () => {
    useSessionStore.getState()._setPartialTranscript("Hello...");
    expect(useSessionStore.getState().partialTranscript).toBe("Hello...");
  });

  it("_addFinalTranscript appends and clears partial", () => {
    useSessionStore.getState()._setPartialTranscript("partial...");
    useSessionStore.getState()._addFinalTranscript("Final text.", 0.95);

    const state = useSessionStore.getState();
    expect(state.partialTranscript).toBe("");
    expect(state.finalTranscripts).toHaveLength(1);
    expect(state.finalTranscripts[0].text).toBe("Final text.");
    expect(state.finalTranscripts[0].confidence).toBe(0.95);
    expect(state.finalTranscripts[0].timestamp).toBeGreaterThan(0);
  });

  it("_updateLastTranscript replaces last transcript text", () => {
    useSessionStore.getState()._addFinalTranscript("Original.", 0.8);
    useSessionStore.getState()._updateLastTranscript("Rewritten.");
    expect(useSessionStore.getState().finalTranscripts[0].text).toBe(
      "Rewritten.",
    );
  });

  it("_updateLastTranscript does nothing when empty", () => {
    useSessionStore.getState()._updateLastTranscript("Noop");
    expect(useSessionStore.getState().finalTranscripts).toEqual([]);
  });

  it("clearTranscripts resets transcripts", () => {
    useSessionStore.getState()._addFinalTranscript("text", 0.9);
    useSessionStore.getState()._setPartialTranscript("partial");
    useSessionStore.getState().clearTranscripts();

    const state = useSessionStore.getState();
    expect(state.partialTranscript).toBe("");
    expect(state.finalTranscripts).toEqual([]);
  });

  it("startSession calls invokeCommand (mock mode)", async () => {
    await useSessionStore.getState().startSession("memo");
    expect(useSessionStore.getState().currentMode).toBe("memo");
  });

  it("stopSession calls invokeCommand (mock mode)", async () => {
    await useSessionStore.getState().stopSession();
    // No error thrown in mock mode
  });

  it("toggleRecording calls invokeCommand (mock mode)", async () => {
    await useSessionStore.getState().toggleRecording();
  });

  it("setMode updates currentMode", async () => {
    await useSessionStore.getState().setMode("tech");
    expect(useSessionStore.getState().currentMode).toBe("tech");
  });

  it("rewriteLast calls invokeCommand (mock mode)", async () => {
    await useSessionStore.getState().rewriteLast("memo");
  });

  it("deliverLast calls invokeCommand (mock mode)", async () => {
    await useSessionStore.getState().deliverLast("clipboard");
  });
});

describe("toastStore", () => {
  beforeEach(() => {
    useToastStore.setState({ toasts: [] });
  });

  it("starts empty", () => {
    expect(useToastStore.getState().toasts).toEqual([]);
  });

  it("addToast adds a toast with generated id", () => {
    useToastStore.getState().addToast("error", "Something failed");
    const toasts = useToastStore.getState().toasts;
    expect(toasts).toHaveLength(1);
    expect(toasts[0].type).toBe("error");
    expect(toasts[0].message).toBe("Something failed");
    expect(toasts[0].id).toBeDefined();
  });

  it("addToast supports multiple toasts", () => {
    useToastStore.getState().addToast("info", "Info 1");
    useToastStore.getState().addToast("success", "Success 1");
    expect(useToastStore.getState().toasts).toHaveLength(2);
  });

  it("removeToast removes by id", () => {
    useToastStore.getState().addToast("warning", "Warn");
    const id = useToastStore.getState().toasts[0].id;
    useToastStore.getState().removeToast(id);
    expect(useToastStore.getState().toasts).toHaveLength(0);
  });
});

describe("navigationStore", () => {
  beforeEach(() => {
    useNavigationStore.setState({ currentPage: "recorder" });
  });

  it("starts at recorder page", () => {
    expect(useNavigationStore.getState().currentPage).toBe("recorder");
  });

  it("navigate changes page", () => {
    useNavigationStore.getState().navigate("history");
    expect(useNavigationStore.getState().currentPage).toBe("history");
  });

  it("navigate to settings", () => {
    useNavigationStore.getState().navigate("settings");
    expect(useNavigationStore.getState().currentPage).toBe("settings");
  });
});

describe("historyStore", () => {
  beforeEach(() => {
    useHistoryStore.setState({
      items: [],
      query: "",
      cursor: null,
      hasMore: false,
      loading: false,
      filterMode: "all",
    });
  });

  it("has correct initial state", () => {
    const state = useHistoryStore.getState();
    expect(state.items).toEqual([]);
    expect(state.query).toBe("");
    expect(state.loading).toBe(false);
    expect(state.filterMode).toBe("all");
  });

  it("setQuery updates query", () => {
    useHistoryStore.getState().setQuery("search term");
    expect(useHistoryStore.getState().query).toBe("search term");
  });

  it("setFilterMode updates filter", () => {
    useHistoryStore.getState().setFilterMode("memo");
    expect(useHistoryStore.getState().filterMode).toBe("memo");
  });

  it("fetchHistory sets loading (mock mode)", async () => {
    await useHistoryStore.getState().fetchHistory("test");
    expect(useHistoryStore.getState().loading).toBe(false);
    expect(useHistoryStore.getState().query).toBe("test");
  });

  it("loadMore does nothing without cursor", async () => {
    await useHistoryStore.getState().loadMore();
    expect(useHistoryStore.getState().loading).toBe(false);
  });
});

describe("dictionaryStore", () => {
  beforeEach(() => {
    useDictionaryStore.setState({
      entries: [],
      loading: false,
      filterScope: "all",
    });
  });

  it("has correct initial state", () => {
    const state = useDictionaryStore.getState();
    expect(state.entries).toEqual([]);
    expect(state.loading).toBe(false);
    expect(state.filterScope).toBe("all");
  });

  it("setFilterScope updates scope", () => {
    useDictionaryStore.getState().setFilterScope("app");
    expect(useDictionaryStore.getState().filterScope).toBe("app");
  });

  it("fetchEntries sets loading (mock mode)", async () => {
    await useDictionaryStore.getState().fetchEntries();
    expect(useDictionaryStore.getState().loading).toBe(false);
  });

  it("removeEntry filters by id", () => {
    useDictionaryStore.setState({
      entries: [
        { id: "1", pattern: "a", replacement: "b", scope: "global", priority: 0, enabled: true },
        { id: "2", pattern: "c", replacement: "d", scope: "global", priority: 0, enabled: true },
      ],
    });
    useDictionaryStore.getState().removeEntry("1");
    expect(useDictionaryStore.getState().entries).toHaveLength(1);
    expect(useDictionaryStore.getState().entries[0].id).toBe("2");
  });
});

describe("settingsStore", () => {
  beforeEach(() => {
    useSettingsStore.setState({
      settings: {
        stt_engine: "apple",
        deliver_policy_type: "clipboard_only",
        audio_retention: "none",
        hotkey: "Cmd+Shift+V",
        paste_allowlist: [],
        language: "ja-JP",
        rewrite_enabled: false,
      },
      loading: false,
    });
  });

  it("has correct defaults", () => {
    const { settings } = useSettingsStore.getState();
    expect(settings.stt_engine).toBe("apple");
    expect(settings.deliver_policy_type).toBe("clipboard_only");
    expect(settings.audio_retention).toBe("none");
  });

  it("updateSettings merges partial", () => {
    useSettingsStore.getState().updateSettings({ stt_engine: "whisper" });
    expect(useSettingsStore.getState().settings.stt_engine).toBe("whisper");
    // Other settings unchanged
    expect(useSettingsStore.getState().settings.deliver_policy_type).toBe(
      "clipboard_only",
    );
  });

  it("addToAllowlist adds bundle id", () => {
    useSettingsStore.getState().addToAllowlist("com.apple.TextEdit");
    expect(useSettingsStore.getState().settings.paste_allowlist).toEqual([
      "com.apple.TextEdit",
    ]);
  });

  it("addToAllowlist prevents duplicates", () => {
    useSettingsStore.getState().addToAllowlist("com.apple.TextEdit");
    useSettingsStore.getState().addToAllowlist("com.apple.TextEdit");
    expect(useSettingsStore.getState().settings.paste_allowlist).toHaveLength(1);
  });

  it("removeFromAllowlist removes bundle id", () => {
    useSettingsStore.getState().addToAllowlist("com.apple.TextEdit");
    useSettingsStore.getState().addToAllowlist("com.example.app");
    useSettingsStore.getState().removeFromAllowlist("com.apple.TextEdit");
    expect(useSettingsStore.getState().settings.paste_allowlist).toEqual([
      "com.example.app",
    ]);
  });

  it("loadSettings completes without error (mock mode)", async () => {
    await useSettingsStore.getState().loadSettings();
    expect(useSettingsStore.getState().loading).toBe(false);
  });
});
