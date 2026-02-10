import { create } from "zustand";
import type { HistoryItem, HistoryPage, Mode } from "../lib/types";
import { invokeCommand } from "../lib/coreClient";

interface HistoryStore {
  items: HistoryItem[];
  query: string;
  cursor: string | null;
  hasMore: boolean;
  loading: boolean;
  filterMode: Mode | "all";

  fetchHistory: (query?: string) => Promise<void>;
  loadMore: () => Promise<void>;
  setQuery: (query: string) => void;
  setFilterMode: (mode: Mode | "all") => void;
  rewriteItem: (sessionId: string, mode: Mode) => Promise<void>;
}

export const useHistoryStore = create<HistoryStore>((set, get) => ({
  items: [],
  query: "",
  cursor: null,
  hasMore: false,
  loading: false,
  filterMode: "all",

  fetchHistory: async (query) => {
    const q = query ?? get().query;
    set({ loading: true, query: q });
    try {
      const result = await invokeCommand<HistoryPage>("get_history", {
        query: q || undefined,
        limit: 50,
      });
      if (result) {
        set({
          items: result.items,
          cursor: result.cursor,
          hasMore: result.has_more,
        });
      }
    } finally {
      set({ loading: false });
    }
  },

  loadMore: async () => {
    const { cursor, query, loading } = get();
    if (!cursor || loading) return;
    set({ loading: true });
    try {
      const result = await invokeCommand<HistoryPage>("get_history", {
        query: query || undefined,
        limit: 50,
        cursor,
      });
      if (result) {
        set((s) => ({
          items: [...s.items, ...result.items],
          cursor: result.cursor,
          hasMore: result.has_more,
        }));
      }
    } finally {
      set({ loading: false });
    }
  },

  setQuery: (query) => set({ query }),

  setFilterMode: (mode) => set({ filterMode: mode }),

  rewriteItem: async (sessionId, mode) => {
    await invokeCommand("rewrite_text", {
      session_id: sessionId,
      mode,
    });
  },
}));
