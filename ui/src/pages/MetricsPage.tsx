import { useState, useEffect, useCallback } from "react";
import { Card, CardHeader } from "../components/ui/Card";
import { Button } from "../components/ui/Button";
import { invokeCommand } from "../lib/coreClient";
import type { MetricError } from "../lib/types";
import { useToastStore } from "../store/toastStore";

interface BackendMetrics {
  sessions_started: number;
  segments_transcribed: number;
  segments_rewritten: number;
  segments_delivered: number;
  errors: Record<string, number>;
  avg_latency: Record<string, number>;
}

interface DisplayMetrics {
  sessionsTotal: number;
  segmentsTranscribed: number;
  segmentsRewritten: number;
  segmentsDelivered: number;
  avgTranscribeMs: number;
  avgRewriteMs: number;
  avgDeliverMs: number;
  errorCount: number;
  recentErrors: MetricError[];
}

export function MetricsPage() {
  const toasts = useToastStore((s) => s.toasts);
  const [metrics, setMetrics] = useState<DisplayMetrics>({
    sessionsTotal: 0,
    segmentsTranscribed: 0,
    segmentsRewritten: 0,
    segmentsDelivered: 0,
    avgTranscribeMs: 0,
    avgRewriteMs: 0,
    avgDeliverMs: 0,
    errorCount: 0,
    recentErrors: [],
  });

  const fetchMetrics = useCallback(async () => {
    try {
      const data = await invokeCommand<BackendMetrics>("get_metrics");
      const totalErrors = Object.values(data.errors || {}).reduce(
        (a, b) => a + b,
        0,
      );
      setMetrics((m) => ({
        ...m,
        sessionsTotal: data.sessions_started ?? 0,
        segmentsTranscribed: data.segments_transcribed ?? 0,
        segmentsRewritten: data.segments_rewritten ?? 0,
        segmentsDelivered: data.segments_delivered ?? 0,
        avgTranscribeMs: data.avg_latency?.transcribe ?? 0,
        avgRewriteMs: data.avg_latency?.rewrite ?? 0,
        avgDeliverMs: data.avg_latency?.deliver ?? 0,
        errorCount: totalErrors,
      }));
    } catch (e) {
      console.error("Failed to fetch metrics:", e);
    }
  }, []);

  // Fetch metrics on mount and every 5 seconds
  useEffect(() => {
    fetchMetrics();
    const interval = setInterval(fetchMetrics, 5000);
    return () => clearInterval(interval);
  }, [fetchMetrics]);

  // Track errors from toasts
  const errorToasts = toasts.filter((t) => t.type === "error");
  useEffect(() => {
    setMetrics((m) => ({
      ...m,
      recentErrors: errorToasts.map((t) => ({
        timestamp: new Date().toISOString(),
        code: "UI",
        message: t.message,
      })),
    }));
  }, [errorToasts.length]);

  const clearMetrics = useCallback(() => {
    setMetrics({
      sessionsTotal: 0,
      segmentsTranscribed: 0,
      segmentsRewritten: 0,
      segmentsDelivered: 0,
      avgTranscribeMs: 0,
      avgRewriteMs: 0,
      avgDeliverMs: 0,
      errorCount: 0,
      recentErrors: [],
    });
  }, []);

  return (
    <div className="h-full space-y-4 overflow-y-auto p-4">
      <div className="flex items-center justify-between">
        <h2 className="text-lg font-semibold">Metrics</h2>
        <div className="flex gap-2">
          <Button variant="ghost" size="sm" onClick={fetchMetrics}>
            Refresh
          </Button>
          <Button variant="ghost" size="sm" onClick={clearMetrics}>
            Reset
          </Button>
        </div>
      </div>

      {/* Overview */}
      <Card>
        <CardHeader
          title="Overview"
          description="Session and segment counts"
        />
        <div className="grid grid-cols-4 gap-4">
          <div>
            <p className="text-2xl font-bold tabular-nums text-gray-100">
              {metrics.sessionsTotal}
            </p>
            <p className="text-xs text-gray-500">Sessions</p>
          </div>
          <div>
            <p className="text-2xl font-bold tabular-nums text-gray-100">
              {metrics.segmentsTranscribed}
            </p>
            <p className="text-xs text-gray-500">Transcribed</p>
          </div>
          <div>
            <p className="text-2xl font-bold tabular-nums text-gray-100">
              {metrics.segmentsRewritten}
            </p>
            <p className="text-xs text-gray-500">Rewritten</p>
          </div>
          <div>
            <p className="text-2xl font-bold tabular-nums text-gray-100">
              {metrics.segmentsDelivered}
            </p>
            <p className="text-xs text-gray-500">Delivered</p>
          </div>
        </div>
      </Card>

      {/* Latency */}
      <Card>
        <CardHeader
          title="Latency"
          description="Average processing time per stage"
        />
        <div className="grid grid-cols-3 gap-4">
          <div>
            <p className="text-2xl font-bold tabular-nums text-gray-100">
              {metrics.avgTranscribeMs > 0
                ? `${metrics.avgTranscribeMs}ms`
                : "--"}
            </p>
            <p className="text-xs text-gray-500">Transcribe</p>
          </div>
          <div>
            <p className="text-2xl font-bold tabular-nums text-gray-100">
              {metrics.avgRewriteMs > 0
                ? `${metrics.avgRewriteMs}ms`
                : "--"}
            </p>
            <p className="text-xs text-gray-500">Rewrite</p>
          </div>
          <div>
            <p className="text-2xl font-bold tabular-nums text-gray-100">
              {metrics.avgDeliverMs > 0
                ? `${metrics.avgDeliverMs}ms`
                : "--"}
            </p>
            <p className="text-xs text-gray-500">Deliver</p>
          </div>
        </div>
      </Card>

      {/* Errors */}
      <Card>
        <CardHeader
          title="Errors"
          description={`${metrics.errorCount} total errors`}
        />
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
    </div>
  );
}
