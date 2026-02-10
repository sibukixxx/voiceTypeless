import { useSessionStore } from "../store/sessionStore";
import {
  getStateLabel,
  isErrorState,
  isRecording,
  isBusy,
} from "../lib/types";

export function StateIndicator() {
  const sessionState = useSessionStore((s) => s.sessionState);

  const label = getStateLabel(sessionState);
  const recording = isRecording(sessionState);
  const busy = isBusy(sessionState);
  const error = isErrorState(sessionState);

  let dotColor = "bg-gray-500"; // idle
  if (recording) dotColor = "bg-red-500 animate-pulse";
  else if (busy) dotColor = "bg-amber-500 animate-pulse";
  else if (error) dotColor = "bg-red-500";
  else if (sessionState === "armed") dotColor = "bg-yellow-500";

  return (
    <div className="flex items-center gap-2">
      <span className={`h-2.5 w-2.5 rounded-full ${dotColor}`} />
      <span
        className={`text-sm font-medium ${
          error ? "text-red-400" : "text-gray-300"
        }`}
      >
        {label}
      </span>
      {error && (
        <span className="text-xs text-red-400/70">
          {sessionState.error.message}
        </span>
      )}
    </div>
  );
}
