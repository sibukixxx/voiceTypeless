import { useEffect } from "react";
import { useSetupStore } from "../store/setupStore";
import { useNavigationStore } from "../store/navigationStore";
import type { Page, SetupIssue } from "../lib/types";

export function SetupBanner() {
  const status = useSetupStore((s) => s.status);
  const dismissed = useSetupStore((s) => s.dismissed);
  const checkSetup = useSetupStore((s) => s.checkSetup);
  const dismiss = useSetupStore((s) => s.dismiss);
  const undismiss = useSetupStore((s) => s.undismiss);
  const navigate = useNavigationStore((s) => s.navigate);

  useEffect(() => {
    checkSetup();
  }, [checkSetup]);

  if (!status || status.issues.length === 0) return null;

  const hasError = status.issues.some((i) => i.severity === "error");

  if (dismissed) {
    return (
      <button
        onClick={undismiss}
        className={`w-full rounded px-3 py-1.5 text-left text-xs ${
          hasError
            ? "bg-red-900/30 text-red-400 hover:bg-red-900/50"
            : "bg-yellow-900/30 text-yellow-400 hover:bg-yellow-900/50"
        }`}
      >
        {hasError ? "Setup issues detected" : "Setup warnings"} —{" "}
        {status.issues.length} issue{status.issues.length > 1 ? "s" : ""} (click
        to expand)
      </button>
    );
  }

  return (
    <div
      className={`rounded-lg border p-3 ${
        hasError
          ? "border-red-700/50 bg-red-950/40"
          : "border-yellow-700/50 bg-yellow-950/40"
      }`}
    >
      <div className="mb-2 flex items-center justify-between">
        <span
          className={`text-sm font-medium ${
            hasError ? "text-red-400" : "text-yellow-400"
          }`}
        >
          {hasError
            ? "セットアップに問題があります"
            : "セットアップの注意事項"}
        </span>
        <button
          onClick={dismiss}
          className="text-xs text-gray-500 hover:text-gray-300"
        >
          閉じる
        </button>
      </div>

      <ul className="space-y-2">
        {status.issues.map((issue, idx) => (
          <IssueItem key={idx} issue={issue} onNavigate={navigate} />
        ))}
      </ul>
    </div>
  );
}

function IssueItem({
  issue,
  onNavigate,
}: {
  issue: SetupIssue;
  onNavigate: (page: Page) => void;
}) {
  const isError = issue.severity === "error";

  return (
    <li
      className={`rounded px-2 py-1.5 text-xs ${
        isError ? "bg-red-900/20" : "bg-yellow-900/20"
      }`}
    >
      <p className={isError ? "text-red-300" : "text-yellow-300"}>
        {issue.message}
      </p>
      <p className="mt-0.5 text-gray-400">
        {issue.action}
        {issue.navigate_to && (
          <button
            onClick={() => onNavigate(issue.navigate_to as Page)}
            className={`ml-1 underline ${
              isError
                ? "text-red-400 hover:text-red-300"
                : "text-yellow-400 hover:text-yellow-300"
            }`}
          >
            Settings で設定 →
          </button>
        )}
      </p>
    </li>
  );
}
