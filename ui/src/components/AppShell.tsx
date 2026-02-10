import type { ReactNode } from "react";
import { useNavigationStore } from "../store/navigationStore";
import { StateIndicator } from "./StateIndicator";
import { ToastContainer } from "./Toast";
import type { Page } from "../lib/types";

interface NavItem {
  page: Page;
  label: string;
}

const NAV_ITEMS: NavItem[] = [
  { page: "recorder", label: "Recorder" },
  { page: "history", label: "History" },
  { page: "dictionary", label: "Dictionary" },
  { page: "settings", label: "Settings" },
];

interface AppShellProps {
  children: ReactNode;
}

export function AppShell({ children }: AppShellProps) {
  const currentPage = useNavigationStore((s) => s.currentPage);
  const navigate = useNavigationStore((s) => s.navigate);

  return (
    <div className="flex h-screen flex-col bg-gray-950 text-gray-100">
      {/* Header */}
      <header className="flex items-center justify-between border-b border-gray-800 px-4 py-2">
        <div className="flex items-center gap-4">
          <h1 className="text-sm font-bold tracking-tight text-white">
            voiceTypeless
          </h1>
          <nav className="flex gap-1">
            {NAV_ITEMS.map((item) => (
              <button
                key={item.page}
                onClick={() => navigate(item.page)}
                className={`rounded-md px-3 py-1.5 text-xs font-medium transition-colors ${
                  currentPage === item.page
                    ? "bg-gray-800 text-white"
                    : "text-gray-500 hover:text-gray-300"
                }`}
              >
                {item.label}
              </button>
            ))}
          </nav>
        </div>
        <StateIndicator />
      </header>

      {/* Main */}
      <main className="flex-1 overflow-hidden">{children}</main>

      {/* Toast overlay */}
      <ToastContainer />
    </div>
  );
}
