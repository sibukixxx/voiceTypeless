import { useState, useEffect, useCallback } from "react";
import { useSettingsStore } from "../store/settingsStore";
import { useNavigationStore } from "../store/navigationStore";
import { invokeCommand } from "../lib/coreClient";
import { Card } from "../components/ui/Card";
import { Button } from "../components/ui/Button";
import type { PermissionStatus, SttEngine } from "../lib/types";

type CheckStatus = "ok" | "warning" | "error" | "checking";

interface DiagnosticItem {
  label: string;
  status: CheckStatus;
  message: string;
  action?: { label: string; onClick: () => void };
}

const STATUS_COLORS: Record<CheckStatus, string> = {
  ok: "bg-green-500",
  warning: "bg-yellow-500",
  error: "bg-red-500",
  checking: "bg-gray-500 animate-pulse",
};

export function DiagnosticsPage() {
  const settings = useSettingsStore((s) => s.settings);
  const navigate = useNavigationStore((s) => s.navigate);
  const [items, setItems] = useState<DiagnosticItem[]>([]);
  const [checking, setChecking] = useState(false);

  const runChecks = useCallback(async () => {
    setChecking(true);
    const results: DiagnosticItem[] = [];

    // 1. Microphone permission
    try {
      const perms = await invokeCommand<PermissionStatus>("check_permissions");
      if (perms?.microphone) {
        results.push({ label: "Microphone Permission", status: "ok", message: "Granted" });
      } else {
        results.push({
          label: "Microphone Permission",
          status: "error",
          message: "Not granted",
          action: { label: "Fix", onClick: () => navigate("permissions") },
        });
      }

      // 2. Accessibility permission
      if (perms?.accessibility) {
        results.push({ label: "Accessibility Permission", status: "ok", message: "Granted" });
      } else {
        results.push({
          label: "Accessibility Permission",
          status: "warning",
          message: "Not granted (required for paste-to-app)",
          action: { label: "Fix", onClick: () => navigate("permissions") },
        });
      }
    } catch {
      results.push({ label: "Microphone Permission", status: "error", message: "Check failed" });
      results.push({ label: "Accessibility Permission", status: "error", message: "Check failed" });
    }

    // 3. STT Engine
    const engine: SttEngine = settings.stt_engine;
    switch (engine) {
      case "apple":
        results.push({ label: "STT Engine (Apple Speech)", status: "ok", message: "Available on macOS" });
        break;
      case "whisper": {
        try {
          const available = await invokeCommand<boolean>("check_whisper_model", {
            modelSize: settings.whisper_model_size ?? "base",
          });
          if (available) {
            results.push({ label: "STT Engine (Whisper)", status: "ok", message: "Model ready" });
          } else {
            results.push({
              label: "STT Engine (Whisper)",
              status: "error",
              message: "Model not downloaded",
              action: { label: "Download", onClick: () => navigate("settings") },
            });
          }
        } catch {
          results.push({ label: "STT Engine (Whisper)", status: "error", message: "Check failed" });
        }
        break;
      }
      case "soniox":
        if (settings.soniox_api_key) {
          results.push({ label: "STT Engine (Soniox)", status: "ok", message: "API key configured" });
        } else {
          results.push({
            label: "STT Engine (Soniox)",
            status: "error",
            message: "API key not set",
            action: { label: "Configure", onClick: () => navigate("settings") },
          });
        }
        break;
      case "cloud":
        results.push({ label: "STT Engine (Cloud)", status: "ok", message: "Cloud STT selected" });
        break;
    }

    // 4. Claude API Key
    if (settings.claude_api_key) {
      results.push({ label: "Claude API Key", status: "ok", message: "Configured" });
    } else {
      results.push({
        label: "Claude API Key",
        status: "warning",
        message: "Not set (rewriting disabled)",
        action: { label: "Configure", onClick: () => navigate("settings") },
      });
    }

    setItems(results);
    setChecking(false);
  }, [settings, navigate]);

  useEffect(() => {
    runChecks();
  }, [runChecks]);

  return (
    <div className="h-full space-y-4 overflow-y-auto p-4">
      <div className="flex items-center justify-between">
        <h2 className="text-lg font-semibold">Diagnostics</h2>
        <Button variant="secondary" size="sm" onClick={runChecks} disabled={checking}>
          {checking ? "Checking..." : "Re-check"}
        </Button>
      </div>

      <div className="space-y-2">
        {items.map((item) => (
          <Card key={item.label}>
            <div className="flex items-center justify-between">
              <div className="flex items-center gap-3">
                <span className={`h-2.5 w-2.5 rounded-full ${STATUS_COLORS[item.status]}`} />
                <div>
                  <p className="text-sm font-medium text-gray-200">{item.label}</p>
                  <p className="text-xs text-gray-400">{item.message}</p>
                </div>
              </div>
              {item.action && (
                <Button variant="secondary" size="sm" onClick={item.action.onClick}>
                  {item.action.label}
                </Button>
              )}
            </div>
          </Card>
        ))}
      </div>

      {items.length === 0 && checking && (
        <div className="flex h-32 items-center justify-center">
          <p className="text-sm text-gray-500">Running checks...</p>
        </div>
      )}
    </div>
  );
}
