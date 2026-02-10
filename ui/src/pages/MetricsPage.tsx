import { useState, useEffect, useCallback } from "react";
import { Card, CardHeader } from "../components/ui/Card";
import { Button } from "../components/ui/Button";
import type { MetricError } from "../lib/types";
import { useToastStore } from "../store/toastStore";

// Metrics are computed locally from observed events
interface LocalMetrics {
  sessionsTotal: number;
  avgLatencyMs: number;
  errorCount: number;
  recentErrors: MetricError[];
}

export function MetricsPage() {
  const toasts = useToastStore((s) => s.toasts);
  const [metrics, setMetrics] = useState<LocalMetrics>({
    sessionsTotal: 0,
    avgLatencyMs: 0,
    errorCount: 0,
    recentErrors: [],
  });

  // Track errors from toasts as a proxy for error events
  const errorToasts = toasts.filter((t) => t.type === "error");

  useEffect(() => {
    setMetrics((m) => ({
      ...m,
      errorCount: errorToasts.length,
    }));
  }, [errorToasts.length]);

  const clearMetrics = useCallback(() => {
    setMetrics({
      sessionsTotal: 0,
      avgLatencyMs: 0,
      errorCount: 0,
      recentErrors: [],
    });
  }, []);

  return (
    <div className="h-full space-y-4 overflow-y-auto p-4">
      <div className="flex items-center justify-between">
        <h2 className="text-lg font-semibold">Metrics</h2>
        <Button variant="ghost" size="sm" onClick={clearMetrics}>
          Reset
        </Button>
      </div>

      {/* Latency */}
      <Card>
        <CardHeader
          title="Latency"
          description="Record -> Transcribe -> Deliver pipeline"
        />
        <div className="grid grid-cols-3 gap-4">
          <div>
            <p className="text-2xl font-bold tabular-nums text-gray-100">
              {metrics.avgLatencyMs > 0
                ? `${metrics.avgLatencyMs}ms`
                : "--"}
            </p>
            <p className="text-xs text-gray-500">Avg total</p>
          </div>
          <div>
            <p className="text-2xl font-bold tabular-nums text-gray-100">
              {metrics.sessionsTotal}
            </p>
            <p className="text-xs text-gray-500">Sessions</p>
          </div>
          <div>
            <p className="text-2xl font-bold tabular-nums text-gray-100">
              {metrics.errorCount}
            </p>
            <p className="text-xs text-gray-500">Errors</p>
          </div>
        </div>
      </Card>

      {/* Error log */}
      <Card>
        <CardHeader title="Recent Errors" />
        {metrics.recentErrors.length === 0 ? (
          <p className="text-sm text-gray-600">No errors recorded</p>
        ) : (
          <div className="space-y-2">
            {metrics.recentErrors.map((err, i) => (
              <div
                key={i}
                className="flex items-start gap-2 rounded-lg bg-gray-800/50 p-2 text-xs"
              >
                <span className="shrink-0 text-gray-500">
                  {new Date(err.timestamp).toLocaleTimeString()}
                </span>
                <span className="font-mono text-red-400">[{err.code}]</span>
                <span className="text-gray-300">{err.message}</span>
              </div>
            ))}
          </div>
        )}
      </Card>

      {/* Log viewer placeholder */}
      <Card>
        <CardHeader
          title="Log Viewer"
          description="Application logs for debugging"
        />
        <div className="h-48 overflow-y-auto rounded-lg bg-gray-950 p-3 font-mono text-xs text-gray-500">
          <p>Log output will appear here when connected to Core...</p>
        </div>
      </Card>
    </div>
  );
}
