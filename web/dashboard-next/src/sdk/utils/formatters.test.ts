import { describe, expect, it } from "vitest";
import { formatBytes, formatDuration, getStatusColor } from "./formatters";

describe("formatters", () => {
  it("formats bytes", () => {
    expect(formatBytes(0)).toBe("0 B");
    expect(formatBytes(1024)).toContain("KB");
  });

  it("formats duration", () => {
    expect(formatDuration(30)).toContain("30");
    expect(formatDuration(125)).toContain("m");
  });

  it("maps status colors", () => {
    expect(getStatusColor("running")).toBe("#3b82f6");
    expect(getStatusColor("healthy")).toBe("#10b981");
  });
});
