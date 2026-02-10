import { describe, it, expect, beforeEach } from "vitest";
import { render, screen, act } from "@testing-library/react";
import { StateIndicator } from "../components/StateIndicator";
import { emitSessionStateChanged, resetAllStores } from "./mockTauri";

describe("StateIndicator", () => {
  beforeEach(() => {
    resetAllStores();
  });

  it("shows Idle by default", () => {
    render(<StateIndicator />);
    expect(screen.getByText("Idle")).toBeInTheDocument();
  });

  it("shows Recording when recording", () => {
    render(<StateIndicator />);
    act(() => {
      emitSessionStateChanged("recording");
    });
    expect(screen.getByText("Recording")).toBeInTheDocument();
  });

  it("shows Transcribing when transcribing", () => {
    render(<StateIndicator />);
    act(() => {
      emitSessionStateChanged("transcribing");
    });
    expect(screen.getByText("Transcribing...")).toBeInTheDocument();
  });

  it("shows error state with message", () => {
    render(<StateIndicator />);
    act(() => {
      emitSessionStateChanged({
        error: { message: "Mic unavailable", recoverable: true },
      });
    });
    expect(screen.getByText("Error")).toBeInTheDocument();
    expect(screen.getByText("Mic unavailable")).toBeInTheDocument();
  });
});
