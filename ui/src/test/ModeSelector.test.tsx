import { describe, it, expect, beforeEach } from "vitest";
import { render, screen } from "@testing-library/react";
import { ModeSelector } from "../components/ModeSelector";
import { resetAllStores } from "./mockTauri";

describe("ModeSelector", () => {
  beforeEach(() => {
    resetAllStores();
  });

  it("renders all mode buttons", () => {
    render(<ModeSelector />);
    expect(screen.getByText("Raw")).toBeInTheDocument();
    expect(screen.getByText("Memo")).toBeInTheDocument();
    expect(screen.getByText("Tech")).toBeInTheDocument();
    expect(screen.getByText("Email JP")).toBeInTheDocument();
    expect(screen.getByText("Minutes")).toBeInTheDocument();
  });

  it("highlights the active mode", () => {
    render(<ModeSelector />);
    const rawButton = screen.getByText("Raw");
    // Default mode is "raw", should have blue styling
    expect(rawButton.className).toContain("bg-blue-600");
  });
});
