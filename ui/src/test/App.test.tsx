import { describe, it, expect, beforeEach } from "vitest";
import { render, screen, fireEvent, waitFor } from "@testing-library/react";
import App from "../App";
import { useNavigationStore } from "../store/navigationStore";
import { resetAllStores } from "./mockTauri";

describe("App", () => {
  beforeEach(() => {
    resetAllStores();
    useNavigationStore.setState({ currentPage: "recorder" });
  });

  it("renders RecorderPage by default", () => {
    render(<App />);
    expect(screen.getByText("Start")).toBeInTheDocument();
  });

  it("navigates to HistoryPage", async () => {
    render(<App />);
    fireEvent.click(screen.getByText("History"));
    await waitFor(() => {
      expect(screen.getByText("No history yet")).toBeInTheDocument();
    });
  });

  it("navigates to DictionaryPage", async () => {
    render(<App />);
    fireEvent.click(screen.getByText("Dictionary"));
    await waitFor(() => {
      expect(screen.getByText("+ Add Entry")).toBeInTheDocument();
    });
  });

  it("navigates to SettingsPage", () => {
    render(<App />);
    fireEvent.click(screen.getByText("Settings"));
    expect(screen.getByText("STT Engine")).toBeInTheDocument();
  });
});
