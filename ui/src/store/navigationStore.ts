import { create } from "zustand";
import type { Page } from "../lib/types";

interface NavigationStore {
  currentPage: Page;
  navigate: (page: Page) => void;
}

export const useNavigationStore = create<NavigationStore>((set) => ({
  currentPage: "recorder",
  navigate: (page) => set({ currentPage: page }),
}));
