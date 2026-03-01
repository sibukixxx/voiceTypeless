import { create } from "zustand";
import type { HistoryItem, HistoryPage, Mode } from "../lib/types";
import { invokeCommand } from "../lib/coreClient";

interface HistoryStore {
  items: HistoryItem[];
  query: string;
  nextCursor: string | null;
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
  nextCursor: null,
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
          nextCursor: result.next_cursor,
        });
      }
    } finally {
      set({ loading: false });
    }
  },

  loadMore: async () => {
    const { nextCursor, query, loading } = get();
    if (!nextCursor || loading) return;
    set({ loading: true });
    try {
      const result = await invokeCommand<HistoryPage>("get_history", {
        query: query || undefined,
        limit: 50,
        cursor: nextCursor,
      });
      if (result) {
        set((s) => ({
          items: [...s.items, ...result.items],
          nextCursor: result.next_cursor,
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
