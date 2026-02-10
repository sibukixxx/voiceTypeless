import { create } from "zustand";
import type { Toast, ToastType } from "../lib/types";

let nextId = 0;

interface ToastStore {
  toasts: Toast[];
  addToast: (type: ToastType, message: string) => void;
  removeToast: (id: string) => void;
}

export const useToastStore = create<ToastStore>((set) => ({
  toasts: [],

  addToast: (type, message) => {
    const id = String(nextId++);
    set((s) => ({ toasts: [...s.toasts, { id, type, message }] }));
    // Auto-dismiss after 5 seconds
    setTimeout(() => {
      set((s) => ({ toasts: s.toasts.filter((t) => t.id !== id) }));
    }, 5000);
  },

  removeToast: (id) => {
    set((s) => ({ toasts: s.toasts.filter((t) => t.id !== id) }));
  },
}));
