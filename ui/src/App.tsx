import { useEffect } from "react";
import { useNavigationStore } from "./store/navigationStore";
import { initEventListeners } from "./lib/eventSetup";
import { AppShell } from "./components/AppShell";
import { RecorderPage } from "./pages/RecorderPage";
import { HistoryPage } from "./pages/HistoryPage";
import { DictionaryPage } from "./pages/DictionaryPage";
import { SettingsPage } from "./pages/SettingsPage";
import { PermissionsPage } from "./pages/PermissionsPage";
import { MetricsPage } from "./pages/MetricsPage";
import { PasteAllowlistPage } from "./pages/PasteAllowlistPage";
import "./App.css";

function PageRouter() {
  const currentPage = useNavigationStore((s) => s.currentPage);

  switch (currentPage) {
    case "recorder":
      return <RecorderPage />;
    case "history":
      return <HistoryPage />;
    case "dictionary":
      return <DictionaryPage />;
    case "settings":
      return <SettingsPage />;
    case "permissions":
      return <PermissionsPage />;
    case "metrics":
      return <MetricsPage />;
    case "paste_allowlist":
      return <PasteAllowlistPage />;
  }
}

function App() {
  useEffect(() => {
    const cleanup = initEventListeners();
    return () => {
      cleanup.then((fn) => fn());
    };
  }, []);

  return (
    <AppShell>
      <PageRouter />
    </AppShell>
  );
}

export default App;
