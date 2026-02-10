import { useEffect, useState } from "react";
import { useDictionaryStore } from "../store/dictionaryStore";
import { useToastStore } from "../store/toastStore";
import { Input } from "../components/ui/Input";
import { Button } from "../components/ui/Button";
import { Card, CardHeader } from "../components/ui/Card";
import { Select } from "../components/ui/Select";
import { Toggle } from "../components/ui/Toggle";
import type { DictionaryEntry, DictionaryScope } from "../lib/types";

const SCOPE_OPTIONS = [
  { value: "global", label: "Global" },
  { value: "app", label: "App" },
  { value: "project", label: "Project" },
  { value: "mode", label: "Mode" },
];

const FILTER_SCOPES = ["all", "global", "app", "project", "mode"] as const;

const EMPTY_ENTRY: DictionaryEntry = {
  pattern: "",
  replacement: "",
  scope: "global",
  priority: 0,
  enabled: true,
};

export function DictionaryPage() {
  const entries = useDictionaryStore((s) => s.entries);
  const loading = useDictionaryStore((s) => s.loading);
  const filterScope = useDictionaryStore((s) => s.filterScope);
  const fetchEntries = useDictionaryStore((s) => s.fetchEntries);
  const upsertEntry = useDictionaryStore((s) => s.upsertEntry);
  const setFilterScope = useDictionaryStore((s) => s.setFilterScope);
  const addToast = useToastStore((s) => s.addToast);

  const [editingEntry, setEditingEntry] = useState<DictionaryEntry | null>(
    null,
  );
  const [isNew, setIsNew] = useState(false);

  useEffect(() => {
    fetchEntries();
  }, [fetchEntries]);

  const filteredEntries =
    filterScope === "all"
      ? entries
      : entries.filter((e) => e.scope === filterScope);

  const handleAdd = () => {
    setEditingEntry({ ...EMPTY_ENTRY });
    setIsNew(true);
  };

  const handleEdit = (entry: DictionaryEntry) => {
    setEditingEntry({ ...entry });
    setIsNew(false);
  };

  const handleSave = async () => {
    if (!editingEntry) return;
    if (!editingEntry.pattern.trim()) {
      addToast("warning", "Pattern is required");
      return;
    }
    if (!editingEntry.replacement.trim()) {
      addToast("warning", "Replacement is required");
      return;
    }
    await upsertEntry(editingEntry);
    addToast("success", isNew ? "Entry added" : "Entry updated");
    setEditingEntry(null);
  };

  const handleCancel = () => {
    setEditingEntry(null);
  };

  return (
    <div className="flex h-full flex-col gap-3 p-4">
      {/* Header */}
      <div className="flex items-center justify-between">
        <h2 className="text-lg font-semibold">Dictionary</h2>
        <Button variant="primary" size="sm" onClick={handleAdd}>
          + Add Entry
        </Button>
      </div>

      {/* Scope filter */}
      <div className="flex gap-1">
        {FILTER_SCOPES.map((scope) => (
          <button
            key={scope}
            onClick={() => setFilterScope(scope)}
            className={`rounded-md px-2.5 py-1 text-xs font-medium transition-colors ${
              filterScope === scope
                ? "bg-gray-700 text-white"
                : "text-gray-500 hover:text-gray-300"
            }`}
          >
            {scope === "all" ? "All" : scope.charAt(0).toUpperCase() + scope.slice(1)}
          </button>
        ))}
      </div>

      {/* Edit form */}
      {editingEntry && (
        <Card className="border-blue-500/30">
          <CardHeader title={isNew ? "New Entry" : "Edit Entry"} />
          <div className="space-y-3">
            <Input
              label="Pattern"
              placeholder="e.g. リアクト"
              value={editingEntry.pattern}
              onChange={(e) =>
                setEditingEntry({ ...editingEntry, pattern: e.target.value })
              }
            />
            <Input
              label="Replacement"
              placeholder="e.g. React"
              value={editingEntry.replacement}
              onChange={(e) =>
                setEditingEntry({
                  ...editingEntry,
                  replacement: e.target.value,
                })
              }
            />
            <div className="flex gap-3">
              <Select
                label="Scope"
                options={SCOPE_OPTIONS}
                value={editingEntry.scope}
                onChange={(e) =>
                  setEditingEntry({
                    ...editingEntry,
                    scope: e.target.value as DictionaryScope,
                  })
                }
              />
              <Input
                label="Priority"
                type="number"
                value={editingEntry.priority}
                onChange={(e) =>
                  setEditingEntry({
                    ...editingEntry,
                    priority: Number(e.target.value),
                  })
                }
              />
            </div>
            <Toggle
              label="Enabled"
              checked={editingEntry.enabled}
              onChange={(checked) =>
                setEditingEntry({ ...editingEntry, enabled: checked })
              }
            />
            <div className="flex gap-2 pt-2">
              <Button variant="primary" size="sm" onClick={handleSave}>
                Save
              </Button>
              <Button variant="ghost" size="sm" onClick={handleCancel}>
                Cancel
              </Button>
            </div>
          </div>
        </Card>
      )}

      {/* Entry list */}
      <div className="flex-1 space-y-2 overflow-y-auto">
        {loading && (
          <p className="text-center text-sm text-gray-500">Loading...</p>
        )}
        {!loading && filteredEntries.length === 0 && (
          <div className="flex h-32 items-center justify-center">
            <p className="text-sm text-gray-600">
              No dictionary entries yet. Add one to improve transcription
              accuracy.
            </p>
          </div>
        )}
        {filteredEntries.map((entry) => (
          <Card
            key={entry.id ?? entry.pattern}
            className={`group cursor-pointer hover:border-gray-700 ${!entry.enabled ? "opacity-50" : ""}`}
          >
            <div
              className="flex items-center justify-between"
              onClick={() => handleEdit(entry)}
            >
              <div className="flex items-center gap-3">
                <div>
                  <span className="text-sm text-red-400 line-through">
                    {entry.pattern}
                  </span>
                  <span className="mx-2 text-gray-600">-&gt;</span>
                  <span className="text-sm text-green-400">
                    {entry.replacement}
                  </span>
                </div>
              </div>
              <div className="flex items-center gap-2 text-xs text-gray-500">
                <span className="rounded bg-gray-800 px-1.5 py-0.5">
                  {entry.scope}
                </span>
                <span>P{entry.priority}</span>
              </div>
            </div>
          </Card>
        ))}
      </div>
    </div>
  );
}
