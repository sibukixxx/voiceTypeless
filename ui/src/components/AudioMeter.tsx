import { useSessionStore } from "../store/sessionStore";
import { isRecording } from "../lib/types";

export function AudioMeter() {
  const audioLevel = useSessionStore((s) => s.audioLevel);
  const sessionState = useSessionStore((s) => s.sessionState);
  const active = isRecording(sessionState);

  return (
    <div className="flex items-center gap-2">
      <div className="h-2 flex-1 overflow-hidden rounded-full bg-gray-800">
        <div
          className={`h-full rounded-full transition-all duration-75 ${
            active ? "bg-green-500" : "bg-gray-600"
          }`}
          style={{ width: `${Math.min(audioLevel * 100, 100)}%` }}
        />
      </div>
      <span className="w-8 text-right text-xs tabular-nums text-gray-500">
        {active ? (audioLevel * 100).toFixed(0) : "--"}
      </span>
    </div>
  );
}
