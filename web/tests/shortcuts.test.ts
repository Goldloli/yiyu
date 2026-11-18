import { describe, expect, it } from "vitest";
import { formatShortcutLabel, shortcutFromKeyEvent } from "../src/lib/shortcuts";

const base = {
  code: "",
  metaKey: false,
  ctrlKey: false,
  altKey: false,
  shiftKey: false
};

describe("shortcut recording", () => {
  it("formats modifier combinations using stable backend names", () => {
    expect(
      shortcutFromKeyEvent({
        ...base,
        code: "KeyY",
        metaKey: true,
        altKey: true,
        shiftKey: true
      })
    ).toBe("Alt+Shift+Super+Y");
  });

  it("rejects modifier-only and unmodified typing keys", () => {
    expect(shortcutFromKeyEvent({ ...base, code: "AltLeft", altKey: true })).toBeNull();
    expect(shortcutFromKeyEvent({ ...base, code: "KeyA" })).toBeNull();
  });

  it("allows standalone function keys", () => {
    expect(shortcutFromKeyEvent({ ...base, code: "F8" })).toBe("F8");
  });

  it("renders native modifier symbols on macOS", () => {
    expect(formatShortcutLabel("Alt+Shift+Super+Y", "macos")).toBe("⌥ ⇧ ⌘ Y");
    expect(formatShortcutLabel("Control+Alt+V", "windows")).toBe("Control + Alt + V");
  });
});
