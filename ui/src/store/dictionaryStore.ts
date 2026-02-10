import { create } from "zustand";
import type { DictionaryEntry, DictionaryScope } from "../lib/types";
import { invokeCommand } from "../lib/coreClient";

interface DictionaryStore {
  entries: DictionaryEntry[];
  loading: boolean;
  filterScope: DictionaryScope | "all";

  fetchEntries: (scope?: DictionaryScope) => Promise<void>;
  upsertEntry: (entry: DictionaryEntry) => Promise<void>;
  removeEntry: (id: string) => void;
  setFilterScope: (scope: DictionaryScope | "all") => void;
}

export const useDictionaryStore = create<DictionaryStore>((set) => ({
  entries: [],
  loading: false,
  filterScope: "all",

  fetchEntries: async (scope) => {
    set({ loading: true });
    try {
      const entries = await invokeCommand<DictionaryEntry[]>(
        "list_dictionary",
        scope ? { scope } : undefined,
      );
      if (entries) {
        set({ entries });
      }
    } finally {
      set({ loading: false });
    }
  },

  upsertEntry: async (entry) => {
    await invokeCommand("upsert_dictionary", { entry });
    // Re-fetch to get the updated list
    const entries = await invokeCommand<DictionaryEntry[]>("list_dictionary");
    if (entries) {
      set({ entries });
    }
  },

  removeEntry: (id) => {
    set((s) => ({ entries: s.entries.filter((e) => e.id !== id) }));
  },

  setFilterScope: (scope) => set({ filterScope: scope }),
}));
