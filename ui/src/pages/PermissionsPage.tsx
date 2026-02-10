import { useState } from "react";
import { Card, CardHeader } from "../components/ui/Card";
import { Button } from "../components/ui/Button";
import { useToastStore } from "../store/toastStore";
import { invokeCommand } from "../lib/coreClient";
import type { PermissionStatus } from "../lib/types";

export function PermissionsPage() {
  const [permissions, setPermissions] = useState<PermissionStatus>({
    microphone: false,
    accessibility: false,
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
            <span
              className={`h-3 w-3 rounded-full ${permissions.microphone ? "bg-green-500" : "bg-red-500"}`}
            />
            <span className="text-sm text-gray-300">
              {permissions.microphone ? "Granted" : "Not granted"}
            </span>
          </div>
        </div>
        {!permissions.microphone && (
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
            <span
              className={`h-3 w-3 rounded-full ${permissions.accessibility ? "bg-green-500" : "bg-red-500"}`}
            />
            <span className="text-sm text-gray-300">
              {permissions.accessibility ? "Granted" : "Not granted"}
            </span>
          </div>
        </div>
        {!permissions.accessibility && (
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
