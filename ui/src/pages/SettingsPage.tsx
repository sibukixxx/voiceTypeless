import { useState, useEffect } from "react";
import { useSettingsStore } from "../store/settingsStore";
import { useNavigationStore } from "../store/navigationStore";
import { useToastStore } from "../store/toastStore";
import { Card, CardHeader } from "../components/ui/Card";
import { Select } from "../components/ui/Select";
import { Button } from "../components/ui/Button";
import { invokeCommand } from "../lib/coreClient";
import type { SttEngine, AudioRetention, WhisperModelSize } from "../lib/types";

const STT_OPTIONS = [
  { value: "soniox", label: "Soniox", description: "Cloud API, high accuracy (requires API key)" },
  { value: "apple", label: "Apple Speech", description: "macOS built-in, low latency" },
  { value: "whisper", label: "Whisper.cpp", description: "Local, high accuracy" },
  { value: "cloud", label: "Cloud STT", description: "Cloud API (requires network)" },
];

const WHISPER_MODEL_OPTIONS = [
  { value: "base", label: "Base (148 MB)", description: "Fast, lower accuracy" },
  { value: "small", label: "Small (488 MB)", description: "Balanced" },
  { value: "medium", label: "Medium (1.5 GB)", description: "High accuracy" },
  { value: "large", label: "Large (3.1 GB)", description: "Best accuracy, slow" },
];

const DELIVER_OPTIONS = [
  { value: "clipboard", label: "Clipboard Only" },
  { value: "paste", label: "Paste to App (Coming soon)", disabled: true },
  { value: "file_append", label: "File Append (Coming soon)", disabled: true },
  { value: "webhook", label: "Webhook (Coming soon)", disabled: true },
];

const RETENTION_OPTIONS = [
  { value: "none", label: "Do not save" },
  { value: "ttl", label: "TTL (auto-delete)" },
  { value: "permanent", label: "Permanent" },
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
  const saving = useSettingsStore((s) => s.saving);
  const lastSaved = useSettingsStore((s) => s.lastSaved);
  const loadSettings = useSettingsStore((s) => s.loadSettings);
  const updateSettings = useSettingsStore((s) => s.updateSettings);
  const navigate = useNavigationStore((s) => s.navigate);
  const addToast = useToastStore((s) => s.addToast);

  const [whisperModelAvailable, setWhisperModelAvailable] = useState(false);
  const [downloading, setDownloading] = useState(false);
  const [sonioxKeyInput, setSonioxKeyInput] = useState(settings.soniox_api_key ?? "");
  const [claudeKeyInput, setClaudeKeyInput] = useState(settings.claude_api_key ?? "");

  // Sync local key inputs when settings change externally
  useEffect(() => {
    setSonioxKeyInput(settings.soniox_api_key ?? "");
  }, [settings.soniox_api_key]);

  useEffect(() => {
    setClaudeKeyInput(settings.claude_api_key ?? "");
  }, [settings.claude_api_key]);

  // Load settings and check whisper model on mount
  useEffect(() => {
    loadSettings();
  }, [loadSettings]);

  // Re-check model availability when model size changes
  useEffect(() => {
    invokeCommand<boolean>("check_whisper_model", {
      modelSize: settings.whisper_model_size ?? "base",
    })
      .then(setWhisperModelAvailable)
      .catch(() => setWhisperModelAvailable(false));
  }, [settings.whisper_model_size]);

  const handleDownloadModel = async () => {
    setDownloading(true);
    try {
      await invokeCommand<string>("download_whisper_model", {
        modelSize: settings.whisper_model_size ?? "base",
      });
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
      <div className="flex items-center justify-between">
        <h2 className="text-lg font-semibold">Settings</h2>
        {saving && (
          <span className="animate-pulse text-xs text-gray-400">Saving...</span>
        )}
        {!saving && lastSaved > 0 && (
          <span className="text-xs text-green-400">Saved</span>
        )}
      </div>

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
        {settings.stt_engine === "soniox" && (
          <div className="mt-3">
            <label className="mb-1 block text-xs text-gray-400">
              Soniox API Key
            </label>
            <input
              type="password"
              value={sonioxKeyInput}
              onChange={(e) => setSonioxKeyInput(e.target.value)}
              onBlur={() =>
                updateSettings({
                  soniox_api_key: sonioxKeyInput || undefined,
                })
              }
              placeholder="Enter Soniox API key"
              className="w-full rounded-lg border border-gray-700 bg-gray-800 px-3 py-2 text-sm text-gray-200 placeholder-gray-500 focus:border-purple-500 focus:outline-none"
            />
            <p className="mt-1 text-xs text-gray-500">
              {settings.soniox_api_key
                ? "API key configured"
                : "API key required — get one at soniox.com"}
            </p>
          </div>
        )}
        {settings.stt_engine === "whisper" && (
          <>
            <div className="mt-3">
              <label className="mb-1 block text-xs text-gray-400">
                Model Size
              </label>
              <Select
                options={WHISPER_MODEL_OPTIONS}
                value={settings.whisper_model_size ?? "base"}
                onChange={(e) =>
                  updateSettings({
                    whisper_model_size: e.target.value as WhisperModelSize,
                  })
                }
              />
            </div>
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
          </>
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
          value={claudeKeyInput}
          onChange={(e) => setClaudeKeyInput(e.target.value)}
          onBlur={() =>
            updateSettings({
              claude_api_key: claudeKeyInput || undefined,
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

      {/* Deliver Target */}
      <Card>
        <CardHeader
          title="Output Target"
          description="How transcribed text is delivered"
        />
        <Select
          options={DELIVER_OPTIONS}
          value={settings.default_deliver_target}
          onChange={(e) =>
            updateSettings({
              default_deliver_target: e.target.value,
            })
          }
        />
        {settings.default_deliver_target === "paste" && (
          <div className="mt-3 space-y-2">
            <label className="flex items-center gap-2 text-sm text-gray-300">
              <input
                type="checkbox"
                checked={settings.paste_confirm}
                onChange={(e) =>
                  updateSettings({ paste_confirm: e.target.checked })
                }
                className="rounded"
              />
              Confirm before pasting
            </label>
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
            {settings.hotkey_toggle}
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
          <Button
            variant="secondary"
            size="sm"
            onClick={() => navigate("diagnostics")}
          >
            Diagnostics
          </Button>
        </div>
      </Card>
    </div>
  );
}
