import { useEffect, useCallback } from "react";
import { useHistoryStore } from "../store/historyStore";
import { useToastStore } from "../store/toastStore";
import { Input } from "../components/ui/Input";
import { Button } from "../components/ui/Button";
import { Card } from "../components/ui/Card";
import { MODE_LABELS } from "../lib/types";
import type { Mode } from "../lib/types";

const FILTER_MODES = ["all", "raw", "memo", "tech", "email_jp", "minutes"] as const;

export function HistoryPage() {
  const items = useHistoryStore((s) => s.items);
  const query = useHistoryStore((s) => s.query);
  const hasMore = useHistoryStore((s) => s.hasMore);
  const loading = useHistoryStore((s) => s.loading);
  const filterMode = useHistoryStore((s) => s.filterMode);
  const fetchHistory = useHistoryStore((s) => s.fetchHistory);
  const loadMore = useHistoryStore((s) => s.loadMore);
  const setQuery = useHistoryStore((s) => s.setQuery);
  const setFilterMode = useHistoryStore((s) => s.setFilterMode);
  const addToast = useToastStore((s) => s.addToast);

  useEffect(() => {
    fetchHistory();
  }, [fetchHistory]);

  const handleSearch = useCallback(() => {
    fetchHistory(query);
  }, [fetchHistory, query]);

  const handleCopyItem = async (text: string) => {
    try {
      await navigator.clipboard.writeText(text);
      addToast("success", "Copied");
    } catch {
      addToast("error", "Failed to copy");
    }
  };

  const filteredItems =
    filterMode === "all"
      ? items
      : items.filter((item) => item.mode === filterMode);

  return (
    <div className="flex h-full flex-col gap-3 p-4">
      {/* Search & Filter */}
      <div className="flex items-end gap-2">
        <div className="flex-1">
          <Input
            placeholder="Search transcripts..."
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            onKeyDown={(e) => e.key === "Enter" && handleSearch()}
          />
        </div>
        <Button variant="secondary" size="md" onClick={handleSearch}>
          Search
        </Button>
      </div>

      {/* Mode filter */}
      <div className="flex gap-1">
        {FILTER_MODES.map((mode) => (
          <button
            key={mode}
            onClick={() => setFilterMode(mode)}
            className={`rounded-md px-2.5 py-1 text-xs font-medium transition-colors ${
              filterMode === mode
                ? "bg-gray-700 text-white"
                : "text-gray-500 hover:text-gray-300"
            }`}
          >
            {mode === "all" ? "All" : MODE_LABELS[mode as Mode]}
          </button>
        ))}
      </div>

      {/* History list */}
      <div className="flex-1 space-y-2 overflow-y-auto">
        {filteredItems.length === 0 && !loading && (
          <div className="flex h-32 items-center justify-center">
            <p className="text-sm text-gray-600">No history yet</p>
          </div>
        )}

        {filteredItems.map((item) => (
          <Card key={item.id} className="group">
            <div className="flex items-start justify-between gap-2">
              <div className="min-w-0 flex-1">
                <p className="text-sm leading-relaxed text-gray-200">
                  {item.text}
                </p>
                <div className="mt-2 flex items-center gap-3 text-xs text-gray-500">
                  <span className="rounded bg-gray-800 px-1.5 py-0.5">
                    {MODE_LABELS[item.mode]}
                  </span>
                  <span>{(item.confidence * 100).toFixed(0)}%</span>
                  <span>{new Date(item.created_at).toLocaleString()}</span>
                </div>
              </div>
              <Button
                variant="ghost"
                size="sm"
                onClick={() => handleCopyItem(item.text)}
                className="shrink-0 opacity-0 group-hover:opacity-100"
              >
                Copy
              </Button>
            </div>
          </Card>
        ))}

        {hasMore && (
          <div className="flex justify-center py-2">
            <Button
              variant="ghost"
              size="sm"
              onClick={loadMore}
              disabled={loading}
            >
              {loading ? "Loading..." : "Load more"}
            </Button>
          </div>
        )}
      </div>
    </div>
  );
}
