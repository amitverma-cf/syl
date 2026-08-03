import { describe, it, expect, vi, beforeEach } from "vitest";

const invokeMock = vi.hoisted(() => vi.fn());
vi.mock("@tauri-apps/api/core", () => ({ invoke: invokeMock }));

import { countTokens, cloudContextWindow, isLocalModelId, localModelNameFromId, localContextSize } from "./tokens";

describe("isLocalModelId", () => {
  it("recognizes the local:: prefix", () => {
    expect(isLocalModelId("local::LFM2.5-350M")).toBe(true);
    expect(isLocalModelId("gpt-5")).toBe(false);
    expect(isLocalModelId("")).toBe(false);
  });
});

describe("localModelNameFromId", () => {
  it("strips the local:: prefix", () => {
    expect(localModelNameFromId("local::LFM2.5-350M")).toBe("LFM2.5-350M");
  });
});

describe("cloudContextWindow", () => {
  it("returns the real published window for a known model", () => {
    expect(cloudContextWindow("gpt-4o")).toBe(128000);
    expect(cloudContextWindow("claude-sonnet-5")).toBe(200000);
  });

  it("falls back to a conservative default for an unknown model instead of guessing high", () => {
    expect(cloudContextWindow("some-custom-provider-model")).toBe(128000);
  });
});

describe("countTokens", () => {
  beforeEach(() => {
    invokeMock.mockReset();
  });

  it("returns 0 for empty text without calling into the backend at all", async () => {
    const count = await countTokens("local::whatever", "");
    expect(count).toBe(0);
    expect(invokeMock).not.toHaveBeenCalled();
  });

  it("delegates to the real local-model tokenizer via count_local_tokens for local:: ids", async () => {
    invokeMock.mockResolvedValueOnce(7);
    const count = await countTokens("local::LFM2.5-350M", "hello world");
    expect(count).toBe(7);
    expect(invokeMock).toHaveBeenCalledWith("count_local_tokens", {
      name: "LFM2.5-350M",
      text: "hello world",
    });
  });

  it("falls back to the real BPE tokenizer when the local model isn't loaded (invoke rejects)", async () => {
    invokeMock.mockRejectedValueOnce(new Error("LFM2.5-350M is not loaded"));
    const count = await countTokens("local::LFM2.5-350M", "hello world");
    // "hello world" is 2 tokens under cl100k_base — a real, deterministic value,
    // not a fabricated one.
    expect(count).toBe(2);
  });

  it("uses the real BPE tokenizer directly for cloud model ids, without touching invoke", async () => {
    const count = await countTokens("gpt-4o", "hello world");
    expect(count).toBe(2);
    expect(invokeMock).not.toHaveBeenCalled();
  });

  it("produces a higher count for longer text (sanity check it's not a constant)", async () => {
    const short = await countTokens("gpt-4o", "hi");
    const long = await countTokens(
      "gpt-4o",
      "This is a considerably longer sentence with many more distinct words in it.",
    );
    expect(long).toBeGreaterThan(short);
  });
});

describe("localContextSize", () => {
  it("returns the real value from the local_context_size command", async () => {
    invokeMock.mockReset();
    invokeMock.mockResolvedValueOnce(8192);
    expect(await localContextSize()).toBe(8192);
    expect(invokeMock).toHaveBeenCalledWith("local_context_size");
  });

  it("falls back to a sane default if the command fails (e.g. outside Tauri)", async () => {
    invokeMock.mockReset();
    invokeMock.mockRejectedValueOnce(new Error("not available"));
    expect(await localContextSize()).toBe(4096);
  });
});
