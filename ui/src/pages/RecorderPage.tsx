import { useSessionStore } from "../store/sessionStore";
import { useToastStore } from "../store/toastStore";
import { isActiveState, isRecording, isBusy } from "../lib/types";
import { TranscriptView } from "../components/TranscriptView";
import { AudioMeter } from "../components/AudioMeter";
import { ModeSelector } from "../components/ModeSelector";
import { Button } from "../components/ui/Button";

export function RecorderPage() {
  const sessionState = useSessionStore((s) => s.sessionState);
  const finalTranscripts = useSessionStore((s) => s.finalTranscripts);
  const currentMode = useSessionStore((s) => s.currentMode);
  const rewriteEnabled = useSessionStore((s) => s.rewriteEnabled);
  const setRewriteEnabled = useSessionStore((s) => s.setRewriteEnabled);
  const startSession = useSessionStore((s) => s.startSession);
  const stopSession = useSessionStore((s) => s.stopSession);
  const toggleRecording = useSessionStore((s) => s.toggleRecording);
  const rewriteLast = useSessionStore((s) => s.rewriteLast);
  const clearTranscripts = useSessionStore((s) => s.clearTranscripts);
  const addToast = useToastStore((s) => s.addToast);

  const active = isActiveState(sessionState);
  const recording = isRecording(sessionState);
  const busy = isBusy(sessionState);
  const isRawMode = currentMode === "raw";

  const handleToggle = async () => {
    try {
      if (!active) {
        await startSession();
        await toggleRecording();
      } else {
        await toggleRecording();
      }
    } catch (e) {
      console.error("[handleToggle] error:", e);
    }
  };

  const handleStop = async () => {
    await stopSession();
  };

  const handleCopy = async () => {
    const allText = finalTranscripts.map((t) => t.text).join("\n");
    if (!allText) {
      addToast("warning", "No text to copy");
      return;
    }
    try {
      await navigator.clipboard.writeText(allText);
      addToast("success", "Copied to clipboard");
    } catch {
      addToast("error", "Failed to copy");
    }
  };

  const handleManualRewrite = async () => {
    if (finalTranscripts.length === 0 || isRawMode) return;
    try {
      await rewriteLast(currentMode);
    } catch (e) {
      console.error("[handleManualRewrite] error:", e);
      addToast("error", "Rewrite failed");
    }
  };

  return (
    <div className="flex h-full flex-col gap-3 p-4">
      {/* Mode selector + rewrite toggle */}
      <div className="flex items-center justify-between">
        <ModeSelector />
        <div className="flex items-center gap-2">
          <label
            className={`flex cursor-pointer items-center gap-1.5 text-xs ${
              isRawMode ? "text-gray-600" : "text-gray-400"
            }`}
          >
            <input
              type="checkbox"
              checked={rewriteEnabled}
              onChange={(e) => setRewriteEnabled(e.target.checked)}
              disabled={isRawMode}
              className="h-3.5 w-3.5 rounded border-gray-600 bg-gray-800 accent-purple-500"
            />
            Auto-rewrite
          </label>
        </div>
      </div>

      {/* Transcript area */}
      <TranscriptView />

      {/* Audio meter */}
      <AudioMeter />

      {/* Controls */}
      <div className="flex items-center gap-2">
        <Button
          variant={recording ? "danger" : "primary"}
          size="lg"
          onClick={handleToggle}
          disabled={busy}
          className="min-w-[120px]"
        >
          {recording ? "Pause" : active ? "Resume" : "Start"}
        </Button>

        {active && (
          <Button variant="secondary" size="lg" onClick={handleStop}>
            Stop
          </Button>
        )}

        <div className="flex-1" />

        <Button
          variant="ghost"
          size="md"
          onClick={handleManualRewrite}
          disabled={finalTranscripts.length === 0 || isRawMode}
        >
          Rewrite
        </Button>

        <Button
          variant="ghost"
          size="md"
          onClick={handleCopy}
          disabled={finalTranscripts.length === 0}
        >
          Copy
        </Button>

        <Button
          variant="ghost"
          size="md"
          onClick={clearTranscripts}
          disabled={finalTranscripts.length === 0}
        >
          Clear
        </Button>
      </div>
    </div>
  );
}
