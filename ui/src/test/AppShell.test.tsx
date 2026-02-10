import { describe, it, expect, beforeEach } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import { AppShell } from "../components/AppShell";
import { useNavigationStore } from "../store/navigationStore";
import { resetAllStores } from "./mockTauri";

describe("AppShell", () => {
  beforeEach(() => {
    resetAllStores();
    useNavigationStore.setState({ currentPage: "recorder" });
  });

  it("renders title", () => {
    render(<AppShell><div>Content</div></AppShell>);
    expect(screen.getByText("voiceTypeless")).toBeInTheDocument();
  });

  it("renders navigation items", () => {
    render(<AppShell><div>Content</div></AppShell>);
    expect(screen.getByText("Recorder")).toBeInTheDocument();
    expect(screen.getByText("History")).toBeInTheDocument();
    expect(screen.getByText("Dictionary")).toBeInTheDocument();
    expect(screen.getByText("Settings")).toBeInTheDocument();
  });

  it("renders children", () => {
    render(<AppShell><div>Page Content</div></AppShell>);
    expect(screen.getByText("Page Content")).toBeInTheDocument();
  });

  it("navigates when nav button clicked", () => {
    render(<AppShell><div>Content</div></AppShell>);
    fireEvent.click(screen.getByText("History"));
    expect(useNavigationStore.getState().currentPage).toBe("history");
  });

  it("highlights current page in nav", () => {
    render(<AppShell><div>Content</div></AppShell>);
    const recorderBtn = screen.getByText("Recorder");
    expect(recorderBtn.className).toContain("bg-gray-800");
  });
});
