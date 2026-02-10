import { useState } from "react";
import { useSettingsStore } from "../store/settingsStore";
import { useToastStore } from "../store/toastStore";
import { Card, CardHeader } from "../components/ui/Card";
import { Input } from "../components/ui/Input";
import { Button } from "../components/ui/Button";

export function PasteAllowlistPage() {
  const settings = useSettingsStore((s) => s.settings);
  const addToAllowlist = useSettingsStore((s) => s.addToAllowlist);
  const removeFromAllowlist = useSettingsStore((s) => s.removeFromAllowlist);
  const addToast = useToastStore((s) => s.addToast);

  const [newBundleId, setNewBundleId] = useState("");

  const handleAdd = () => {
    const trimmed = newBundleId.trim();
    if (!trimmed) {
      addToast("warning", "Bundle ID is required");
      return;
    }
    if (settings.paste_allowlist.includes(trimmed)) {
      addToast("warning", "Already in allowlist");
      return;
    }
    addToAllowlist(trimmed);
    setNewBundleId("");
    addToast("success", `Added ${trimmed}`);
  };

  const isPasteMode = settings.deliver_policy_type === "paste_allowlist";

  return (
    <div className="h-full space-y-4 overflow-y-auto p-4">
      <h2 className="text-lg font-semibold">Paste Allowlist</h2>

      {!isPasteMode && (
        <Card className="border-amber-500/30">
          <p className="text-sm text-amber-300">
            Paste Allowlist is only active when Output Policy is set to "Paste
            Allowlist". Current policy:{" "}
            <strong>{settings.deliver_policy_type}</strong>
          </p>
        </Card>
      )}

      <Card>
        <CardHeader
          title="Allowed Applications"
          description="Only these apps will receive auto-pasted text. Use macOS bundle IDs (e.g. com.apple.TextEdit)."
        />

        {/* Add new */}
        <div className="flex items-end gap-2">
          <div className="flex-1">
            <Input
              placeholder="com.example.app"
              value={newBundleId}
              onChange={(e) => setNewBundleId(e.target.value)}
              onKeyDown={(e) => e.key === "Enter" && handleAdd()}
            />
          </div>
          <Button variant="primary" size="md" onClick={handleAdd}>
            Add
          </Button>
        </div>

        {/* List */}
        <div className="mt-4 space-y-2">
          {settings.paste_allowlist.length === 0 && (
            <p className="text-sm text-gray-600">
              No apps in allowlist. Auto-paste is disabled.
            </p>
          )}
          {settings.paste_allowlist.map((bundleId) => (
            <div
              key={bundleId}
              className="flex items-center justify-between rounded-lg bg-gray-800/50 px-3 py-2"
            >
              <span className="font-mono text-sm text-gray-300">
                {bundleId}
              </span>
              <Button
                variant="ghost"
                size="sm"
                onClick={() => removeFromAllowlist(bundleId)}
              >
                Remove
              </Button>
            </div>
          ))}
        </div>
      </Card>

      {/* Safety notice */}
      <Card className="border-amber-500/20">
        <CardHeader title="Safety" />
        <p className="text-xs text-gray-400">
          Auto-paste sends text directly to the active app window. Only add apps
          you trust. When "Confirm Each Time" mode is enabled, you'll be asked
          before each paste operation.
        </p>
      </Card>
    </div>
  );
}
