import { describe, it, expect } from "vitest";
import { formatBytes } from "./types";

describe("formatBytes", () => {
  it("formats sub-gigabyte sizes in whole megabytes", () => {
    expect(formatBytes(500 * 1024 * 1024)).toBe("500 MB");
    expect(formatBytes(0)).toBe("0 MB");
  });

  it("formats gigabyte-and-above sizes with one decimal place", () => {
    expect(formatBytes(1.5 * 1024 * 1024 * 1024)).toBe("1.5 GB");
    expect(formatBytes(2 * 1024 * 1024 * 1024)).toBe("2.0 GB");
  });

  it("switches units exactly at the 1 GB boundary", () => {
    const oneGb = 1024 * 1024 * 1024;
    expect(formatBytes(oneGb - 1)).toMatch(/MB$/);
    expect(formatBytes(oneGb)).toBe("1.0 GB");
  });
});
