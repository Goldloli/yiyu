import { describe, expect, it } from "vitest";
import { resolveUiLanguage, t } from "../src/lib/i18n";

describe("i18n", () => {
  it("resolves explicit preferences and falls back to system language", () => {
    expect(resolveUiLanguage("zh")).toBe("zh");
    expect(resolveUiLanguage("en")).toBe("en");
    expect(["zh", "en"]).toContain(resolveUiLanguage("system"));
  });

  it("formats message parameters without changing unknown placeholders", () => {
    expect(t("settings.updateAvailable", { version: "0.2.0" }, "en")).toBe(
      "Version 0.2.0 is available"
    );
    expect(t("main.characterCount", { current: 12, max: 20_000 }, "zh")).toBe(
      "12 / 20000"
    );
  });
});
