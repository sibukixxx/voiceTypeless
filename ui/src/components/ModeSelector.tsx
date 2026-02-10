import { useSessionStore } from "../store/sessionStore";
import { MODE_LABELS, MODE_DESCRIPTIONS, isBusy } from "../lib/types";
import type { Mode } from "../lib/types";

const MODES: Mode[] = ["raw", "memo", "tech", "email_jp", "minutes"];

export function ModeSelector() {
  const currentMode = useSessionStore((s) => s.currentMode);
  const sessionState = useSessionStore((s) => s.sessionState);
  const setMode = useSessionStore((s) => s.setMode);
  const busy = isBusy(sessionState);

  return (
    <div className="flex flex-wrap gap-1">
      {MODES.map((mode) => (
        <button
          key={mode}
          disabled={busy}
          onClick={() => setMode(mode)}
          title={MODE_DESCRIPTIONS[mode]}
          className={`rounded-md px-3 py-1.5 text-xs font-medium transition-colors ${
            currentMode === mode
              ? "bg-blue-600 text-white"
              : "bg-gray-800 text-gray-400 hover:bg-gray-700 hover:text-gray-200"
          } disabled:cursor-not-allowed disabled:opacity-50`}
        >
          {MODE_LABELS[mode]}
        </button>
      ))}
    </div>
  );
}
