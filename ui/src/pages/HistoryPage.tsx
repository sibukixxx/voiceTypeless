import { useEffect, useCallback } from "react";
import { useHistoryStore } from "../store/historyStore";
import { useToastStore } from "../store/toastStore";
import { Input } from "../components/ui/Input";
import { Button } from "../components/ui/Button";
import { Card } from "../components/ui/Card";
import { FilterButtonGroup } from "../components/ui/FilterButtonGroup";
import { MODE_LABELS } from "../lib/types";
import type { Mode } from "../lib/types";

const FILTER_MODES = ["all", "raw", "memo", "tech", "email_jp", "minutes"] as const;

export function HistoryPage() {
  const items = useHistoryStore((s) => s.items);
  const query = useHistoryStore((s) => s.query);
  const nextCursor = useHistoryStore((s) => s.nextCursor);
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

  const handleCopyItem = async (sessionId: string) => {
    try {
      await navigator.clipboard.writeText(sessionId);
      addToast("success", "Session ID copied");
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
      <FilterButtonGroup
        options={FILTER_MODES}
        selected={filterMode}
        onChange={setFilterMode}
        labelFn={(mode) => mode === "all" ? "All" : MODE_LABELS[mode as Mode]}
      />

      {/* History list */}
      <div className="flex-1 space-y-2 overflow-y-auto">
        {filteredItems.length === 0 && !loading && (
          <div className="flex h-32 items-center justify-center">
            <p className="text-sm text-gray-600">No history yet</p>
          </div>
        )}

        {filteredItems.map((item) => (
          <Card key={item.session_id} className="group">
            <div className="flex items-start justify-between gap-2">
              <div className="min-w-0 flex-1">
                {item.preview_text ? (
                  <p className="line-clamp-2 text-sm leading-relaxed text-gray-200">
                    {item.preview_text}
                  </p>
                ) : (
                  <p className="text-sm leading-relaxed text-gray-200">
                    Session: {item.session_id.slice(0, 8)}...
                  </p>
                )}
                <div className="mt-2 flex items-center gap-3 text-xs text-gray-500">
                  <span className="rounded bg-gray-800 px-1.5 py-0.5">
                    {MODE_LABELS[item.mode]}
                  </span>
                  <span>{item.segment_count} segments</span>
                  <span className="rounded bg-gray-800 px-1.5 py-0.5">
                    {item.state}
                  </span>
                  <span>{new Date(item.created_at).toLocaleString()}</span>
                </div>
              </div>
              <Button
                variant="ghost"
                size="sm"
                onClick={() => handleCopyItem(item.session_id)}
                className="shrink-0 opacity-0 group-hover:opacity-100"
              >
                Copy ID
              </Button>
            </div>
          </Card>
        ))}

        {nextCursor && (
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
