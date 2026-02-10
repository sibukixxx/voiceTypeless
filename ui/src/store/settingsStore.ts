import { create } from "zustand";
import type { AppSettings } from "../lib/types";

const DEFAULT_SETTINGS: AppSettings = {
  stt_engine: "apple",
  deliver_policy_type: "clipboard_only",
  audio_retention: "none",
  hotkey: "Cmd+Shift+V",
  paste_allowlist: [],
};

interface SettingsStore {
  settings: AppSettings;
  loading: boolean;

  loadSettings: () => Promise<void>;
  updateSettings: (partial: Partial<AppSettings>) => void;
  addToAllowlist: (bundleId: string) => void;
  removeFromAllowlist: (bundleId: string) => void;
}

export const useSettingsStore = create<SettingsStore>((set, get) => ({
  settings: DEFAULT_SETTINGS,
  loading: false,

  loadSettings: async () => {
    set({ loading: true });
    try {
      // TODO: invokeCommand('get_settings') when Core implements it
      // For now, use defaults
    } finally {
      set({ loading: false });
    }
  },

  updateSettings: (partial) => {
    set((s) => ({ settings: { ...s.settings, ...partial } }));
    // TODO: invokeCommand('update_settings', get().settings) when Core implements it
  },

  addToAllowlist: (bundleId) => {
    const current = get().settings.paste_allowlist;
    if (!current.includes(bundleId)) {
      set((s) => ({
        settings: {
          ...s.settings,
          paste_allowlist: [...current, bundleId],
        },
      }));
    }
  },

  removeFromAllowlist: (bundleId) => {
    set((s) => ({
      settings: {
        ...s.settings,
        paste_allowlist: s.settings.paste_allowlist.filter(
          (id) => id !== bundleId,
        ),
      },
    }));
  },
}));
