import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";

const root = new URL("../../", import.meta.url);

describe("Tauri capabilities", () => {
  it("covers the four window routes without filesystem or shell access", () => {
    const capability = JSON.parse(
      readFileSync(
        new URL("src-tauri/capabilities/default.json", root),
        "utf8"
      )
    ) as { windows: string[]; permissions: string[] };
    expect(capability.windows.sort()).toEqual([
      "main",
      "onboarding",
      "popup",
      "settings"
    ]);
    for (const permission of [
      "core:window:allow-show",
      "core:window:allow-hide",
      "core:window:allow-set-focus",
      "core:window:allow-set-size",
      "core:window:allow-set-position",
      "clipboard-manager:default",
      "notification:default",
      "opener:default"
    ]) {
      expect(capability.permissions).toContain(permission);
    }
    expect(
      capability.permissions.some(
        (permission) =>
          permission.startsWith("fs:") || permission.startsWith("shell:")
      )
    ).toBe(false);
    expect(capability.permissions).not.toContain("*");
  });
});
