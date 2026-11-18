import { describe, expect, it } from "vitest";
import {
  compactText,
  formatBytes,
  formatEta,
  formatRate,
  providerInitials
} from "../src/lib/format";

describe("format helpers", () => {
  it("formats bytes for release packages", () => {
    expect(formatBytes(1536)).toBe("1.5 KB");
    expect(formatBytes(8_000_000)).toBe("7.6 MB");
  });

  it("keeps compact labels bounded", () => {
    expect(compactText("a long translation string", 8)).toHaveLength(8);
    expect(providerInitials("OpenRouter")).toBe("OP");
    expect(providerInitials("智谱")).toHaveLength(2);
  });

  it("formats update transfer speed and remaining time", () => {
    expect(formatRate(1_048_576)).toBe("1.0 MB/s");
    expect(formatEta(42, "zh")).toBe("剩余 42 秒");
    expect(formatEta(125, "en")).toBe("3 min left");
  });
});
