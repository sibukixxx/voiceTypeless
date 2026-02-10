import { describe, it, expect } from "vitest";
import { invokeCommand, subscribe } from "../lib/coreClient";

// Tests run outside Tauri, so these exercise mock/fallback paths

describe("coreClient (mock mode)", () => {
  it("invokeCommand returns undefined in mock mode", async () => {
    const result = await invokeCommand<string>("start_session", {
      mode: "raw",
    });
    expect(result).toBeUndefined();
  });

  it("subscribe returns a noop unlisten function in mock mode", async () => {
    const unlisten = await subscribe("session_state_changed", () => {});
    expect(typeof unlisten).toBe("function");
    // Calling unlisten should not throw
    unlisten();
  });
});
