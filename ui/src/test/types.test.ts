import { describe, it, expect } from "vitest";
import {
  isErrorState,
  getStateLabel,
  isActiveState,
  isRecording,
  isBusy,
} from "../lib/types";
import type { SessionState } from "../lib/types";

describe("types - state helpers", () => {
  it("isErrorState returns true for error state", () => {
    const error: SessionState = {
      error: { message: "fail", recoverable: false },
    };
    expect(isErrorState(error)).toBe(true);
  });

  it("isErrorState returns false for string states", () => {
    expect(isErrorState("idle")).toBe(false);
    expect(isErrorState("recording")).toBe(false);
  });

  it("getStateLabel returns correct labels", () => {
    expect(getStateLabel("idle")).toBe("Idle");
    expect(getStateLabel("armed")).toBe("Armed");
    expect(getStateLabel("recording")).toBe("Recording");
    expect(getStateLabel("transcribing")).toBe("Transcribing...");
    expect(getStateLabel("rewriting")).toBe("Rewriting...");
    expect(getStateLabel("delivering")).toBe("Delivering...");
  });

  it("getStateLabel returns Error for error state", () => {
    expect(
      getStateLabel({ error: { message: "x", recoverable: true } }),
    ).toBe("Error");
  });

  it("isActiveState returns false for idle", () => {
    expect(isActiveState("idle")).toBe(false);
  });

  it("isActiveState returns true for non-idle states", () => {
    expect(isActiveState("recording")).toBe(true);
    expect(isActiveState("armed")).toBe(true);
    expect(isActiveState("transcribing")).toBe(true);
  });

  it("isActiveState returns false for error state", () => {
    expect(
      isActiveState({ error: { message: "x", recoverable: true } }),
    ).toBe(false);
  });

  it("isRecording returns true only for recording", () => {
    expect(isRecording("recording")).toBe(true);
    expect(isRecording("idle")).toBe(false);
    expect(isRecording("transcribing")).toBe(false);
  });

  it("isBusy returns true for processing states", () => {
    expect(isBusy("transcribing")).toBe(true);
    expect(isBusy("rewriting")).toBe(true);
    expect(isBusy("delivering")).toBe(true);
  });

  it("isBusy returns false for non-processing states", () => {
    expect(isBusy("idle")).toBe(false);
    expect(isBusy("recording")).toBe(false);
    expect(isBusy("armed")).toBe(false);
  });

  it("isBusy returns false for error state", () => {
    expect(isBusy({ error: { message: "x", recoverable: true } })).toBe(
      false,
    );
  });
});
