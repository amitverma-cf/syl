import { describe, it, expect, vi, beforeEach } from "vitest";
import { renderHook, waitFor, act } from "@testing-library/react";

const invokeMock = vi.hoisted(() => vi.fn());
vi.mock("@tauri-apps/api/core", () => ({ invoke: invokeMock }));

import { useLocalModels } from "./useLocalModels";

const stats = {
  cpuUsagePercent: 5,
  memoryUsedBytes: 1,
  memoryTotalBytes: 2,
  processMemoryBytes: 3,
  workspaceDiskBytes: 4,
};

function mockInvoke(command: string) {
  if (command === "list_local_models")
    return Promise.resolve([{ name: "m", sizeBytes: 1, loaded: true, kind: "chat" }]);
  if (command === "system_stats") return Promise.resolve(stats);
  return Promise.reject(new Error(`unexpected command ${command}`));
}

describe("useLocalModels", () => {
  beforeEach(() => {
    invokeMock.mockReset();
    invokeMock.mockImplementation(mockInvoke);
  });

  it("loads both local models and system stats on mount", async () => {
    const { result } = renderHook(() => useLocalModels(vi.fn()));

    await waitFor(() => expect(result.current.localModels).toHaveLength(1));
    await waitFor(() => expect(result.current.stats).toEqual(stats));
  });

  it("routes errors from either call through onError", async () => {
    invokeMock.mockImplementation((cmd: string) =>
      cmd === "system_stats" ? Promise.reject(new Error("stats down")) : mockInvoke(cmd),
    );
    const onError = vi.fn();
    renderHook(() => useLocalModels(onError));

    await waitFor(() => expect(onError).toHaveBeenCalledWith("Error: stats down"));
  });

  it("refresh() re-fetches local models on demand", async () => {
    const { result } = renderHook(() => useLocalModels(vi.fn()));
    await waitFor(() => expect(result.current.localModels).toHaveLength(1));

    invokeMock.mockClear();
    act(() => {
      result.current.refresh();
    });

    await waitFor(() => expect(invokeMock).toHaveBeenCalledWith("list_local_models"));
  });

  it("sets up a 5-second polling interval for system stats, and clears it on unmount", async () => {
    const setIntervalSpy = vi.spyOn(window, "setInterval");
    const clearIntervalSpy = vi.spyOn(window, "clearInterval");

    const { unmount } = renderHook(() => useLocalModels(vi.fn()));
    await waitFor(() => expect(invokeMock).toHaveBeenCalledWith("system_stats"));

    expect(setIntervalSpy).toHaveBeenCalledWith(expect.any(Function), 5000);
    const intervalId = setIntervalSpy.mock.results[0].value;

    unmount();
    expect(clearIntervalSpy).toHaveBeenCalledWith(intervalId);
  });
});
