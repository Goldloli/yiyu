import type { Backend } from "./contract";
import { MockBackend } from "./mock-backend";
import { createTauriBackend } from "./tauri-backend";

export function isTauriEnvironment(): boolean {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

export function platformName(): "macos" | "windows" | "web" {
  if (!isTauriEnvironment()) {
    if (navigator.userAgent.includes("Mac")) return "macos";
    if (navigator.userAgent.includes("Windows")) return "windows";
    return "web";
  }
  return navigator.userAgent.includes("Windows") ? "windows" : "macos";
}

let backend: Backend | null = null;

export function getBackend(): Backend {
  if (!backend) {
    backend = isTauriEnvironment() ? createTauriBackend() : new MockBackend();
  }
  return backend;
}
