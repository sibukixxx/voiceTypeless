import { describe, it, expect } from "vitest";
import { initEventListeners } from "../lib/eventSetup";

// Tests run outside Tauri — initEventListeners subscribes in mock mode (noop)

describe("eventSetup", () => {
  it("initEventListeners returns a cleanup function", async () => {
    const cleanup = await initEventListeners();
    expect(typeof cleanup).toBe("function");
    // Calling cleanup should not throw
    cleanup();
  });
});
