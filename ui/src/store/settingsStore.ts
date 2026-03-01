import { create } from "zustand";
import type { AppSettings } from "../lib/types";
import { invokeCommand } from "../lib/coreClient";
import { useToastStore } from "./toastStore";

const DEFAULT_SETTINGS: AppSettings = {
  stt_engine: "soniox",
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
};

interface SettingsStore {
  settings: AppSettings;
  loading: boolean;
  saving: boolean;
  lastSaved: number;

  loadSettings: () => Promise<void>;
  updateSettings: (partial: Partial<AppSettings>) => Promise<void>;
  addToAllowlist: (bundleId: string) => void;
  removeFromAllowlist: (bundleId: string) => void;
}

export const useSettingsStore = create<SettingsStore>((set, get) => ({
  settings: DEFAULT_SETTINGS,
  loading: false,
  saving: false,
  lastSaved: 0,

  loadSettings: async () => {
    set({ loading: true });
    try {
      const settings = await invokeCommand<AppSettings>("get_settings");
      if (settings) {
        set({ settings: { ...DEFAULT_SETTINGS, ...settings } });
      }
    } catch (e) {
      console.error("Failed to load settings:", e);
    } finally {
      set({ loading: false });
    }
  },

  updateSettings: async (partial) => {
    const previousSettings = get().settings;
    const newSettings = { ...previousSettings, ...partial };
    set({ settings: newSettings, saving: true });
    try {
      await invokeCommand("update_settings", { settings: newSettings });
      set({ lastSaved: Date.now(), saving: false });
    } catch (e) {
      console.error("Failed to save settings:", e);
      set({ settings: previousSettings, saving: false });
      useToastStore.getState().addToast("error", "Failed to save settings");
    }
  },

  addToAllowlist: (bundleId) => {
    const current = get().settings.paste_allowlist;
    if (!current.includes(bundleId)) {
      const previousSettings = get().settings;
      const newSettings = {
        ...previousSettings,
        paste_allowlist: [...current, bundleId],
      };
      set({ settings: newSettings });
      invokeCommand("update_settings", { settings: newSettings }).catch(() => {
        set({ settings: previousSettings });
        useToastStore.getState().addToast("error", "Failed to update allowlist");
      });
    }
  },

  removeFromAllowlist: (bundleId) => {
    const previousSettings = get().settings;
    const newSettings = {
      ...previousSettings,
      paste_allowlist: previousSettings.paste_allowlist.filter(
        (id) => id !== bundleId,
      ),
    };
    set({ settings: newSettings });
    invokeCommand("update_settings", { settings: newSettings }).catch(() => {
      set({ settings: previousSettings });
      useToastStore.getState().addToast("error", "Failed to update allowlist");
    });
  },
}));
