import { describe, it, expect, beforeEach } from "vitest";
import { render, screen, fireEvent, waitFor } from "@testing-library/react";
import { HistoryPage } from "../pages/HistoryPage";
import { DictionaryPage } from "../pages/DictionaryPage";
import { SettingsPage } from "../pages/SettingsPage";
import { PermissionsPage } from "../pages/PermissionsPage";
import { MetricsPage } from "../pages/MetricsPage";
import { PasteAllowlistPage } from "../pages/PasteAllowlistPage";
import { useHistoryStore } from "../store/historyStore";
import { useDictionaryStore } from "../store/dictionaryStore";
import { useSettingsStore } from "../store/settingsStore";
import { useNavigationStore } from "../store/navigationStore";
import { useToastStore } from "../store/toastStore";

function resetStores() {
  useHistoryStore.setState({
    items: [],
    query: "",
    nextCursor: null,
    loading: false,
    filterMode: "all",
  });
  useDictionaryStore.setState({
    entries: [],
    loading: false,
    filterScope: "all",
  });
  useSettingsStore.setState({
    settings: {
      stt_engine: "apple",
      default_mode: "raw",
      default_deliver_target: "clipboard",
      rewrite_enabled: false,
      paste_allowlist: [],
      paste_confirm: true,
      audio_retention: "none",
      segment_ttl_days: 0,
      hotkey_toggle: "CmdOrCtrl+Shift+R",
      language: "ja-JP",
      whisper_model_size: "base",
    },
    loading: false,
  });
  useNavigationStore.setState({ currentPage: "recorder" });
  useToastStore.setState({ toasts: [] });
}

// HistoryPage and DictionaryPage have useEffect that calls fetch on mount.
// In mock mode, invokeCommand resolves immediately with undefined,
// so we use waitFor to let async state updates settle.

describe("HistoryPage", () => {
  beforeEach(resetStores);

  it("renders empty state", async () => {
    render(<HistoryPage />);
    await waitFor(() => {
      expect(screen.getByText("No history yet")).toBeInTheDocument();
    });
  });

  it("renders search input", () => {
    render(<HistoryPage />);
    expect(
      screen.getByPlaceholderText("Search transcripts..."),
    ).toBeInTheDocument();
  });

  it("renders mode filter buttons", () => {
    render(<HistoryPage />);
    expect(screen.getByText("All")).toBeInTheDocument();
    expect(screen.getByText("Raw")).toBeInTheDocument();
  });

  it("renders history items", async () => {
    useHistoryStore.setState({
      items: [
        {
          session_id: "s1234567-abcd",
          state: "idle",
          mode: "raw",
          created_at: "2025-01-01T00:00:00Z",
          updated_at: "2025-01-01T00:01:00Z",
          segment_count: 3,
        },
      ],
    });
    render(<HistoryPage />);
    await waitFor(() => {
      expect(screen.getByText(/s1234567/)).toBeInTheDocument();
    });
  });

  it("filters by mode", async () => {
    useHistoryStore.setState({
      items: [
        {
          session_id: "s1-raw",
          state: "idle",
          mode: "raw",
          created_at: "2025-01-01T00:00:00Z",
          updated_at: "2025-01-01T00:01:00Z",
          segment_count: 1,
        },
        {
          session_id: "s2-memo",
          state: "idle",
          mode: "memo",
          created_at: "2025-01-01T00:00:00Z",
          updated_at: "2025-01-01T00:01:00Z",
          segment_count: 2,
        },
      ],
    });
    render(<HistoryPage />);
    await waitFor(() => {
      expect(screen.getByText(/s1-raw/)).toBeInTheDocument();
    });
    // Click the filter button (first "Memo"), not the item badge
    fireEvent.click(screen.getAllByText("Memo")[0]);
    expect(screen.getByText(/s2-memo/)).toBeInTheDocument();
    expect(screen.queryByText(/s1-raw/)).not.toBeInTheDocument();
  });

  it("shows Load more when nextCursor exists", async () => {
    useHistoryStore.setState({
      nextCursor: "next",
      items: [
        {
          session_id: "s1",
          state: "idle",
          mode: "raw",
          created_at: "2025-01-01T00:00:00Z",
          updated_at: "2025-01-01T00:01:00Z",
          segment_count: 1,
        },
      ],
    });
    render(<HistoryPage />);
    await waitFor(() => {
      expect(screen.getByText("Load more")).toBeInTheDocument();
    });
  });

  it("updates query on input", () => {
    render(<HistoryPage />);
    const input = screen.getByPlaceholderText("Search transcripts...");
    fireEvent.change(input, { target: { value: "test query" } });
    expect(useHistoryStore.getState().query).toBe("test query");
  });
});

describe("DictionaryPage", () => {
  beforeEach(resetStores);

  it("renders empty state", async () => {
    render(<DictionaryPage />);
    await waitFor(() => {
      expect(
        screen.getByText(/No dictionary entries yet/),
      ).toBeInTheDocument();
    });
  });

  it("renders Add Entry button", () => {
    render(<DictionaryPage />);
    expect(screen.getByText("+ Add Entry")).toBeInTheDocument();
  });

  it("opens add form on click", () => {
    render(<DictionaryPage />);
    fireEvent.click(screen.getByText("+ Add Entry"));
    expect(screen.getByText("New Entry")).toBeInTheDocument();
    expect(screen.getByText("Save")).toBeInTheDocument();
    expect(screen.getByText("Cancel")).toBeInTheDocument();
  });

  it("validates empty pattern", () => {
    render(<DictionaryPage />);
    fireEvent.click(screen.getByText("+ Add Entry"));
    fireEvent.click(screen.getByText("Save"));
    expect(useToastStore.getState().toasts.length).toBeGreaterThan(0);
  });

  it("cancels editing", () => {
    render(<DictionaryPage />);
    fireEvent.click(screen.getByText("+ Add Entry"));
    fireEvent.click(screen.getByText("Cancel"));
    expect(screen.queryByText("New Entry")).not.toBeInTheDocument();
  });

  it("renders entries", async () => {
    useDictionaryStore.setState({
      entries: [
        {
          id: "1",
          pattern: "リアクト",
          replacement: "React",
          scope: "global",
          priority: 0,
          enabled: true,
        },
      ],
    });
    render(<DictionaryPage />);
    await waitFor(() => {
      expect(screen.getByText("リアクト")).toBeInTheDocument();
    });
    expect(screen.getByText("React")).toBeInTheDocument();
  });

  it("renders scope filter buttons", () => {
    render(<DictionaryPage />);
    expect(screen.getByText("All")).toBeInTheDocument();
    expect(screen.getByText("Global")).toBeInTheDocument();
    expect(screen.getByText("Mode")).toBeInTheDocument();
  });
});

describe("SettingsPage", () => {
  beforeEach(resetStores);

  it("renders settings sections", async () => {
    render(<SettingsPage />);
    await waitFor(() => {
      expect(screen.getByText("Settings")).toBeInTheDocument();
    });
    expect(screen.getByText("STT Engine")).toBeInTheDocument();
    expect(screen.getByText("Output Target")).toBeInTheDocument();
    expect(screen.getByText("Audio Retention")).toBeInTheDocument();
    expect(screen.getByText("Hotkey")).toBeInTheDocument();
  });

  it("shows current hotkey", async () => {
    render(<SettingsPage />);
    await waitFor(() => {
      expect(screen.getByText("CmdOrCtrl+Shift+R")).toBeInTheDocument();
    });
  });

  it("renders Permissions and Metrics links", async () => {
    render(<SettingsPage />);
    await waitFor(() => {
      expect(screen.getByText("Permissions")).toBeInTheDocument();
    });
    expect(screen.getByText("Metrics")).toBeInTheDocument();
  });

  it("navigates to permissions", async () => {
    render(<SettingsPage />);
    await waitFor(() => {
      expect(screen.getByText("Permissions")).toBeInTheDocument();
    });
    fireEvent.click(screen.getByText("Permissions"));
    expect(useNavigationStore.getState().currentPage).toBe("permissions");
  });

  it("navigates to metrics", async () => {
    render(<SettingsPage />);
    await waitFor(() => {
      expect(screen.getByText("Metrics")).toBeInTheDocument();
    });
    fireEvent.click(screen.getByText("Metrics"));
    expect(useNavigationStore.getState().currentPage).toBe("metrics");
  });

  it("shows Manage Allowlist when paste target", async () => {
    useSettingsStore.setState({
      settings: {
        ...useSettingsStore.getState().settings,
        default_deliver_target: "paste",
      },
    });
    render(<SettingsPage />);
    await waitFor(() => {
      expect(screen.getByText("Manage Allowlist")).toBeInTheDocument();
    });
  });

  it("hides Manage Allowlist for clipboard target", async () => {
    render(<SettingsPage />);
    await waitFor(() => {
      expect(screen.getByText("Settings")).toBeInTheDocument();
    });
    expect(screen.queryByText("Manage Allowlist")).not.toBeInTheDocument();
  });
});

describe("PermissionsPage", () => {
  beforeEach(resetStores);

  it("renders permission sections", () => {
    render(<PermissionsPage />);
    expect(screen.getByText("Permissions")).toBeInTheDocument();
    expect(screen.getByText("Microphone")).toBeInTheDocument();
    expect(screen.getByText("Accessibility")).toBeInTheDocument();
  });

  it("shows Not determined as default state", () => {
    render(<PermissionsPage />);
    const notDetermined = screen.getAllByText("Not determined");
    expect(notDetermined).toHaveLength(2);
  });

  it("renders Check Permissions button", async () => {
    render(<PermissionsPage />);
    await waitFor(() => {
      expect(screen.getByText("Check Permissions")).toBeInTheDocument();
    });
  });

  it("shows how-to-enable instructions for non-granted states", () => {
    render(<PermissionsPage />);
    expect(screen.getAllByText("How to enable:")).toHaveLength(2);
  });

  it("renders Open System Settings buttons for non-granted states", () => {
    render(<PermissionsPage />);
    const buttons = screen.getAllByText("Open System Settings");
    expect(buttons).toHaveLength(2);
  });
});

describe("MetricsPage", () => {
  beforeEach(resetStores);

  it("renders metrics sections", () => {
    render(<MetricsPage />);
    expect(screen.getByText("Metrics")).toBeInTheDocument();
    expect(screen.getByText("Overview")).toBeInTheDocument();
    expect(screen.getByText("Latency")).toBeInTheDocument();
  });

  it("shows placeholder labels", () => {
    render(<MetricsPage />);
    const dashes = screen.getAllByText("--");
    expect(dashes.length).toBeGreaterThanOrEqual(3);
    expect(screen.getByText("Sessions")).toBeInTheDocument();
    expect(screen.getByText("Transcribed")).toBeInTheDocument();
  });

  it("shows Reset button", () => {
    render(<MetricsPage />);
    expect(screen.getByText("Reset")).toBeInTheDocument();
  });

  it("shows no errors message", () => {
    render(<MetricsPage />);
    expect(screen.getByText("No errors recorded")).toBeInTheDocument();
  });
});

describe("PasteAllowlistPage", () => {
  beforeEach(resetStores);

  it("renders title", () => {
    render(<PasteAllowlistPage />);
    expect(screen.getByText("Paste Allowlist")).toBeInTheDocument();
  });

  it("shows warning when not in paste_allowlist mode", () => {
    render(<PasteAllowlistPage />);
    expect(
      screen.getByText(/Paste Allowlist is only active/),
    ).toBeInTheDocument();
  });

  it("hides warning when in paste mode", () => {
    useSettingsStore.setState({
      settings: {
        ...useSettingsStore.getState().settings,
        default_deliver_target: "paste",
      },
    });
    render(<PasteAllowlistPage />);
    expect(
      screen.queryByText(/Paste Allowlist is only active/),
    ).not.toBeInTheDocument();
  });

  it("shows empty allowlist message", () => {
    render(<PasteAllowlistPage />);
    expect(
      screen.getByText("No apps in allowlist. Auto-paste is disabled."),
    ).toBeInTheDocument();
  });

  it("adds bundle id to allowlist", () => {
    render(<PasteAllowlistPage />);
    const input = screen.getByPlaceholderText("com.example.app");
    fireEvent.change(input, { target: { value: "com.apple.TextEdit" } });
    fireEvent.click(screen.getByText("Add"));
    expect(
      useSettingsStore.getState().settings.paste_allowlist,
    ).toContain("com.apple.TextEdit");
    expect(screen.getByText("com.apple.TextEdit")).toBeInTheDocument();
  });

  it("removes bundle id from allowlist", () => {
    useSettingsStore.setState({
      settings: {
        ...useSettingsStore.getState().settings,
        paste_allowlist: ["com.apple.TextEdit"],
      },
    });
    render(<PasteAllowlistPage />);
    fireEvent.click(screen.getByText("Remove"));
    expect(
      useSettingsStore.getState().settings.paste_allowlist,
    ).not.toContain("com.apple.TextEdit");
  });

  it("shows safety notice", () => {
    render(<PasteAllowlistPage />);
    expect(screen.getByText("Safety")).toBeInTheDocument();
  });
});
