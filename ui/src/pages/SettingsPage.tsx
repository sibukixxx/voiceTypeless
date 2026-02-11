import { useState, useEffect } from "react";
import { useSettingsStore } from "../store/settingsStore";
import { useNavigationStore } from "../store/navigationStore";
import { useToastStore } from "../store/toastStore";
import { Card, CardHeader } from "../components/ui/Card";
import { Select } from "../components/ui/Select";
import { Button } from "../components/ui/Button";
import { invokeCommand } from "../lib/coreClient";
import type { SttEngine, DeliverPolicyType, AudioRetention } from "../lib/types";

const STT_OPTIONS = [
  { value: "apple", label: "Apple Speech", description: "macOS built-in, low latency" },
  { value: "whisper", label: "Whisper.cpp", description: "Local, high accuracy" },
  { value: "cloud", label: "Cloud STT", description: "Cloud API (requires network)" },
];

const DELIVER_OPTIONS = [
  { value: "clipboard_only", label: "Clipboard Only" },
  { value: "paste_allowlist", label: "Paste Allowlist" },
  { value: "confirm", label: "Confirm Each Time" },
];

const RETENTION_OPTIONS = [
  { value: "none", label: "Do not save" },
  { value: "1day", label: "1 day" },
  { value: "7days", label: "7 days" },
  { value: "30days", label: "30 days" },
];

const LANGUAGE_OPTIONS = [
  { value: "ja-JP", label: "Japanese (ja-JP)" },
  { value: "en-US", label: "English (en-US)" },
  { value: "zh-CN", label: "Chinese (zh-CN)" },
  { value: "ko-KR", label: "Korean (ko-KR)" },
];

export function SettingsPage() {
  const settings = useSettingsStore((s) => s.settings);
  const loading = useSettingsStore((s) => s.loading);
  const loadSettings = useSettingsStore((s) => s.loadSettings);
  const updateSettings = useSettingsStore((s) => s.updateSettings);
  const navigate = useNavigationStore((s) => s.navigate);
  const addToast = useToastStore((s) => s.addToast);

  const [whisperModelAvailable, setWhisperModelAvailable] = useState(false);
  const [downloading, setDownloading] = useState(false);

  // Load settings and check whisper model on mount
  useEffect(() => {
    loadSettings();
    invokeCommand<boolean>("check_whisper_model")
      .then(setWhisperModelAvailable)
      .catch(() => setWhisperModelAvailable(false));
  }, [loadSettings]);

  const handleDownloadModel = async () => {
    setDownloading(true);
    try {
      await invokeCommand<string>("download_whisper_model");
      setWhisperModelAvailable(true);
      addToast("success", "Whisper model downloaded");
    } catch (e) {
      console.error("Model download failed:", e);
      addToast("error", "Model download failed");
    } finally {
      setDownloading(false);
    }
  };

  if (loading) {
    return (
      <div className="flex h-full items-center justify-center">
        <p className="text-sm text-gray-500">Loading settings...</p>
      </div>
    );
  }

  return (
    <div className="h-full space-y-4 overflow-y-auto p-4">
      <h2 className="text-lg font-semibold">Settings</h2>

      {/* STT Engine */}
      <Card>
        <CardHeader
          title="STT Engine"
          description="Speech-to-text engine for transcription"
        />
        <Select
          options={STT_OPTIONS}
          value={settings.stt_engine}
          onChange={(e) =>
            updateSettings({ stt_engine: e.target.value as SttEngine })
          }
        />
        {settings.stt_engine === "whisper" && (
          <div className="mt-3 flex items-center gap-3">
            <span
              className={`h-2 w-2 rounded-full ${
                whisperModelAvailable ? "bg-green-500" : "bg-red-500"
              }`}
            />
            <span className="text-xs text-gray-400">
              {whisperModelAvailable
                ? "Model ready"
                : "Model not found"}
            </span>
            {!whisperModelAvailable && (
              <Button
                variant="secondary"
                size="sm"
                onClick={handleDownloadModel}
                disabled={downloading}
              >
                {downloading ? "Downloading..." : "Download Model"}
              </Button>
            )}
          </div>
        )}
      </Card>

      {/* Language */}
      <Card>
        <CardHeader
          title="Language"
          description="STT recognition language"
        />
        <Select
          options={LANGUAGE_OPTIONS}
          value={settings.language ?? "ja-JP"}
          onChange={(e) =>
            updateSettings({ language: e.target.value })
          }
        />
      </Card>

      {/* Claude API Key */}
      <Card>
        <CardHeader
          title="Claude API Key"
          description="Required for LLM rewriting (stored locally)"
        />
        <input
          type="password"
          value={settings.claude_api_key ?? ""}
          onChange={(e) =>
            updateSettings({
              claude_api_key: e.target.value || undefined,
            })
          }
          placeholder="sk-ant-..."
          className="w-full rounded-lg border border-gray-700 bg-gray-800 px-3 py-2 text-sm text-gray-200 placeholder-gray-500 focus:border-purple-500 focus:outline-none"
        />
        <p className="mt-1 text-xs text-gray-500">
          {settings.claude_api_key
            ? "API key configured"
            : "No API key set — rewriting will use noop mode"}
        </p>
      </Card>

      {/* Deliver Policy */}
      <Card>
        <CardHeader
          title="Output Policy"
          description="How transcribed text is delivered"
        />
        <Select
          options={DELIVER_OPTIONS}
          value={settings.deliver_policy_type}
          onChange={(e) =>
            updateSettings({
              deliver_policy_type: e.target.value as DeliverPolicyType,
            })
          }
        />
        {settings.deliver_policy_type === "paste_allowlist" && (
          <div className="mt-3">
            <Button
              variant="secondary"
              size="sm"
              onClick={() => navigate("paste_allowlist")}
            >
              Manage Allowlist
            </Button>
          </div>
        )}
      </Card>

      {/* Audio Retention */}
      <Card>
        <CardHeader
          title="Audio Retention"
          description="How long to keep audio recordings"
        />
        <Select
          options={RETENTION_OPTIONS}
          value={settings.audio_retention}
          onChange={(e) =>
            updateSettings({
              audio_retention: e.target.value as AudioRetention,
            })
          }
        />
      </Card>

      {/* Hotkey */}
      <Card>
        <CardHeader
          title="Hotkey"
          description="Global shortcut to toggle recording"
        />
        <div className="flex items-center gap-3">
          <kbd className="rounded-md border border-gray-700 bg-gray-800 px-3 py-1.5 text-sm font-mono text-gray-300">
            {settings.hotkey}
          </kbd>
        </div>
      </Card>

      {/* Permissions & Metrics links */}
      <Card>
        <CardHeader title="Advanced" />
        <div className="flex gap-2">
          <Button
            variant="secondary"
            size="sm"
            onClick={() => navigate("permissions")}
          >
            Permissions
          </Button>
          <Button
            variant="secondary"
            size="sm"
            onClick={() => navigate("metrics")}
          >
            Metrics
          </Button>
        </div>
      </Card>
    </div>
  );
}
