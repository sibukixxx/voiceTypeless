import { useSettingsStore } from "../store/settingsStore";
import { useNavigationStore } from "../store/navigationStore";
import { Card, CardHeader } from "../components/ui/Card";
import { Select } from "../components/ui/Select";
import { Button } from "../components/ui/Button";
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

export function SettingsPage() {
  const settings = useSettingsStore((s) => s.settings);
  const updateSettings = useSettingsStore((s) => s.updateSettings);
  const navigate = useNavigationStore((s) => s.navigate);

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
          <span className="text-xs text-gray-500">
            (Hotkey configuration coming in Phase 3)
          </span>
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
