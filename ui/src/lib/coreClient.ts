// ============================================================
// Core Client — Tauri invoke/listen の薄いラッパー
// Tauri 外（ブラウザ単体）では mock モードで動作
// ============================================================

import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

function isTauri(): boolean {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

/**
 * Tauri コマンドを呼ぶ。Tauri 外では mock（console.warn + undefined）
 */
export async function invokeCommand<T>(
  name: string,
  payload?: Record<string, unknown>,
): Promise<T> {
  if (!isTauri()) {
    console.warn(`[mock] invokeCommand: ${name}`, payload);
    return undefined as T;
  }
  return invoke<T>(name, payload);
}

/**
 * Tauri イベントを購読する。Tauri 外では noop
 */
export async function subscribe<T>(
  eventName: string,
  handler: (payload: T) => void,
): Promise<UnlistenFn> {
  if (!isTauri()) {
    console.warn(`[mock] subscribe: ${eventName}`);
    return () => {};
  }
  return listen<T>(eventName, (event) => handler(event.payload));
}
