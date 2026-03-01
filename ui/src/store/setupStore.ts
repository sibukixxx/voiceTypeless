import { create } from "zustand";
import type { SetupStatus } from "../lib/types";
import { invokeCommand } from "../lib/coreClient";

interface SetupStore {
  status: SetupStatus | null;
  dismissed: boolean;

  checkSetup: () => Promise<void>;
  dismiss: () => void;
  undismiss: () => void;
}

export const useSetupStore = create<SetupStore>((set) => ({
  status: null,
  dismissed: false,

  checkSetup: async () => {
    try {
      const status = await invokeCommand<SetupStatus>("check_setup_status");
      if (status) {
        set({ status });
      }
    } catch (e) {
      console.error("Failed to check setup status:", e);
    }
  },

  dismiss: () => set({ dismissed: true }),
  undismiss: () => set({ dismissed: false }),
}));
