import { describe, it, expect, beforeEach } from "vitest";
import { render, screen, act } from "@testing-library/react";
import { RecorderPage } from "../pages/RecorderPage";
import {
  emitSessionStateChanged,
  emitTranscriptPartial,
  emitTranscriptFinal,
  resetAllStores,
} from "./mockTauri";

describe("RecorderPage", () => {
  beforeEach(() => {
    resetAllStores();
  });

  it("renders Start button in idle state", () => {
    render(<RecorderPage />);
    expect(screen.getByText("Start")).toBeInTheDocument();
  });

  it("shows Pause button when recording", () => {
    render(<RecorderPage />);
    act(() => {
      emitSessionStateChanged("recording");
    });
    expect(screen.getByText("Pause")).toBeInTheDocument();
  });

  it("shows Stop button when session is active", () => {
    render(<RecorderPage />);
    act(() => {
      emitSessionStateChanged("recording");
    });
    expect(screen.getByText("Stop")).toBeInTheDocument();
  });

  it("displays partial transcript", () => {
    render(<RecorderPage />);
    act(() => {
      emitTranscriptPartial("Hello world...");
    });
    expect(screen.getByText("Hello world...")).toBeInTheDocument();
  });

  it("displays final transcript", () => {
    render(<RecorderPage />);
    act(() => {
      emitTranscriptFinal("Completed text.", 0.95);
    });
    expect(screen.getByText("Completed text.")).toBeInTheDocument();
    expect(screen.getByText("95%")).toBeInTheDocument();
  });

  it("shows empty state message when no transcripts", () => {
    render(<RecorderPage />);
    expect(
      screen.getByText("Start recording to see transcripts here"),
    ).toBeInTheDocument();
  });

  it("disables Copy and Clear when no transcripts", () => {
    render(<RecorderPage />);
    const copyBtn = screen.getByText("Copy");
    const clearBtn = screen.getByText("Clear");
    expect(copyBtn).toBeDisabled();
    expect(clearBtn).toBeDisabled();
  });

  it("enables Copy and Clear after transcript arrives", () => {
    render(<RecorderPage />);
    act(() => {
      emitTranscriptFinal("Some text", 0.9);
    });
    const copyBtn = screen.getByText("Copy");
    const clearBtn = screen.getByText("Clear");
    expect(copyBtn).not.toBeDisabled();
    expect(clearBtn).not.toBeDisabled();
  });
});
