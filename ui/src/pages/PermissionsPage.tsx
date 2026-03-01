import { useEffect, useState } from "react";
import { Card, CardHeader } from "../components/ui/Card";
import { Button } from "../components/ui/Button";
import { useToastStore } from "../store/toastStore";
import { invokeCommand } from "../lib/coreClient";
import type { PermissionState, PermissionStatus } from "../lib/types";

function permissionDisplay(state: PermissionState) {
  switch (state) {
    case "granted":
      return { label: "Granted", color: "bg-green-500" };
    case "denied":
      return { label: "Denied", color: "bg-red-500" };
    case "not_determined":
      return { label: "Not determined", color: "bg-yellow-500" };
    case "unavailable":
      return { label: "Unavailable", color: "bg-gray-500" };
  }
}

function needsAction(state: PermissionState): boolean {
  return state === "denied" || state === "not_determined";
}

export function PermissionsPage() {
  const [permissions, setPermissions] = useState<PermissionStatus>({
    microphone: "not_determined",
    accessibility: "not_determined",
  });
  const [checking, setChecking] = useState(false);
  const addToast = useToastStore((s) => s.addToast);

  const checkPermissions = async () => {
    setChecking(true);
    try {
      const result =
        await invokeCommand<PermissionStatus>("check_permissions");
      if (result) {
        setPermissions(result);
      }
    } catch {
      addToast("error", "Failed to check permissions");
    } finally {
      setChecking(false);
    }
  };

  const openSystemSettings = async (target: string) => {
    try {
      await invokeCommand("open_system_settings", { target });
    } catch {
      addToast("error", "Failed to open System Settings");
    }
  };

  // Auto-check on mount
  useEffect(() => {
    checkPermissions();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const mic = permissionDisplay(permissions.microphone);
  const acc = permissionDisplay(permissions.accessibility);

  return (
    <div className="h-full space-y-4 overflow-y-auto p-4">
      <h2 className="text-lg font-semibold">Permissions</h2>
      <p className="text-sm text-gray-400">
        voiceTypeless needs the following macOS permissions to work correctly.
      </p>

      {/* Microphone */}
      <Card>
        <CardHeader
          title="Microphone"
          description="Required for voice capture"
        />
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-2">
            <span className={`h-3 w-3 rounded-full ${mic.color}`} />
            <span className="text-sm text-gray-300">{mic.label}</span>
          </div>
          {needsAction(permissions.microphone) && (
            <Button
              variant="secondary"
              size="sm"
              onClick={() => openSystemSettings("microphone")}
            >
              Open System Settings
            </Button>
          )}
        </div>
        {needsAction(permissions.microphone) && (
          <div className="mt-3 rounded-lg bg-gray-800/50 p-3 text-xs text-gray-400">
            <p className="font-medium text-gray-300">How to enable:</p>
            <ol className="mt-1 list-inside list-decimal space-y-0.5">
              <li>
                Open <strong>System Settings</strong>
              </li>
              <li>
                Go to <strong>Privacy &amp; Security &gt; Microphone</strong>
              </li>
              <li>
                Enable <strong>voiceTypeless</strong>
              </li>
            </ol>
          </div>
        )}
      </Card>

      {/* Accessibility */}
      <Card>
        <CardHeader
          title="Accessibility"
          description="Required for auto-paste and global hotkeys"
        />
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-2">
            <span className={`h-3 w-3 rounded-full ${acc.color}`} />
            <span className="text-sm text-gray-300">{acc.label}</span>
          </div>
          {needsAction(permissions.accessibility) && (
            <Button
              variant="secondary"
              size="sm"
              onClick={() => openSystemSettings("accessibility")}
            >
              Open System Settings
            </Button>
          )}
        </div>
        {needsAction(permissions.accessibility) && (
          <div className="mt-3 rounded-lg bg-gray-800/50 p-3 text-xs text-gray-400">
            <p className="font-medium text-gray-300">How to enable:</p>
            <ol className="mt-1 list-inside list-decimal space-y-0.5">
              <li>
                Open <strong>System Settings</strong>
              </li>
              <li>
                Go to{" "}
                <strong>Privacy &amp; Security &gt; Accessibility</strong>
              </li>
              <li>
                Add and enable <strong>voiceTypeless</strong>
              </li>
            </ol>
          </div>
        )}
      </Card>

      {/* Check button */}
      <Button
        variant="primary"
        size="md"
        onClick={checkPermissions}
        disabled={checking}
      >
        {checking ? "Checking..." : "Check Permissions"}
      </Button>
    </div>
  );
}
