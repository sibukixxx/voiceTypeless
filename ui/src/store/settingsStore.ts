import { create } from "zustand";
import type { AppSettings } from "../lib/types";
import { invokeCommand } from "../lib/coreClient";
import { useToastStore } from "./toastStore";
import { useSetupStore } from "./setupStore";

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
  addToAllowlist: (bundleId: string) => Promise<void>;
  removeFromAllowlist: (bundleId: string) => Promise<void>;
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
    const prev = get().settings;
    const newSettings = { ...prev, ...partial };
    set({ settings: newSettings, saving: true });
    try {
      await invokeCommand("update_settings", { settings: newSettings });
      set({ lastSaved: Date.now(), saving: false });
      // 設定変更後にセットアップ状態を再チェック
      useSetupStore.getState().checkSetup();
    } catch (e) {
      set({ settings: prev, saving: false });
      useToastStore.getState().addToast("error", "設定の保存に失敗しました");
    }
  },

  addToAllowlist: async (bundleId) => {
    const current = get().settings.paste_allowlist;
    if (current.includes(bundleId)) return;
    const prev = get().settings;
    const newSettings = {
      ...prev,
      paste_allowlist: [...current, bundleId],
    };
    set({ settings: newSettings });
    try {
      await invokeCommand("update_settings", { settings: newSettings });
    } catch (e) {
      set({ settings: prev });
      useToastStore.getState().addToast("error", "許可リストの更新に失敗しました");
    }
  },

  removeFromAllowlist: async (bundleId) => {
    const prev = get().settings;
    const newSettings = {
      ...prev,
      paste_allowlist: prev.paste_allowlist.filter((id) => id !== bundleId),
    };
    set({ settings: newSettings });
    try {
      await invokeCommand("update_settings", { settings: newSettings });
    } catch (e) {
      set({ settings: prev });
      useToastStore.getState().addToast("error", "許可リストの更新に失敗しました");
    }
  },
}));
