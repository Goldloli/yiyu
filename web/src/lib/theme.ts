import type { MotionPreference, ThemePreference } from "./types";

export function systemTheme(): "light" | "dark" {
  return window.matchMedia?.("(prefers-color-scheme: dark)").matches ? "dark" : "light";
}

export function resolveTheme(preference: ThemePreference): "light" | "dark" {
  if (preference === "light" || preference === "dark") return preference;
  return systemTheme();
}

export function applyTheme(preference: ThemePreference) {
  const resolved = resolveTheme(preference);
  document.documentElement.dataset.theme = resolved;
  document.documentElement.style.colorScheme = resolved;
}

export function systemReducedMotion(): boolean {
  return Boolean(window.matchMedia?.("(prefers-reduced-motion: reduce)").matches);
}

export function resolveReducedMotion(preference: MotionPreference): boolean {
  if (preference === "reduce") return true;
  if (preference === "normal") return false;
  return systemReducedMotion();
}

export function applyMotion(preference: MotionPreference) {
  document.documentElement.dataset.motion = resolveReducedMotion(preference)
    ? "reduced"
    : "normal";
}

export function applyGlass(enabled: boolean) {
  document.documentElement.dataset.glass = enabled ? "on" : "off";
}

export function watchSystemAppearance(onChange: () => void): () => void {
  const theme = window.matchMedia?.("(prefers-color-scheme: dark)");
  const motion = window.matchMedia?.("(prefers-reduced-motion: reduce)");
  theme?.addEventListener("change", onChange);
  motion?.addEventListener("change", onChange);
  return () => {
    theme?.removeEventListener("change", onChange);
    motion?.removeEventListener("change", onChange);
  };
}
