import { create } from "zustand";
import type { AppSettings } from "../lib/types";
import { invokeCommand } from "../lib/coreClient";

const DEFAULT_SETTINGS: AppSettings = {
  stt_engine: "apple",
  deliver_policy_type: "clipboard_only",
  audio_retention: "none",
  hotkey: "Cmd+Shift+V",
  paste_allowlist: [],
  language: "ja-JP",
  rewrite_enabled: false,
};

interface SettingsStore {
  settings: AppSettings;
  loading: boolean;

  loadSettings: () => Promise<void>;
  updateSettings: (partial: Partial<AppSettings>) => Promise<void>;
  addToAllowlist: (bundleId: string) => void;
  removeFromAllowlist: (bundleId: string) => void;
}

export const useSettingsStore = create<SettingsStore>((set, get) => ({
  settings: DEFAULT_SETTINGS,
  loading: false,

  loadSettings: async () => {
    set({ loading: true });
    try {
      const settings = await invokeCommand<AppSettings>("get_settings");
      set({ settings: { ...DEFAULT_SETTINGS, ...settings } });
    } catch (e) {
      console.error("Failed to load settings:", e);
    } finally {
      set({ loading: false });
    }
  },

  updateSettings: async (partial) => {
    const newSettings = { ...get().settings, ...partial };
    set({ settings: newSettings });
    try {
      await invokeCommand("update_settings", { settings: newSettings });
    } catch (e) {
      console.error("Failed to save settings:", e);
    }
  },

  addToAllowlist: (bundleId) => {
    const current = get().settings.paste_allowlist;
    if (!current.includes(bundleId)) {
      const newSettings = {
        ...get().settings,
        paste_allowlist: [...current, bundleId],
      };
      set({ settings: newSettings });
      invokeCommand("update_settings", { settings: newSettings }).catch(console.error);
    }
  },

  removeFromAllowlist: (bundleId) => {
    const newSettings = {
      ...get().settings,
      paste_allowlist: get().settings.paste_allowlist.filter(
        (id) => id !== bundleId,
      ),
    };
    set({ settings: newSettings });
    invokeCommand("update_settings", { settings: newSettings }).catch(console.error);
  },
}));
