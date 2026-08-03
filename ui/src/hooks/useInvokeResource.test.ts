import { describe, it, expect, vi, beforeEach } from "vitest";
import { renderHook, waitFor, act } from "@testing-library/react";

const invokeMock = vi.hoisted(() => vi.fn());
vi.mock("@tauri-apps/api/core", () => ({ invoke: invokeMock }));

import { useInvokeResource } from "./useInvokeResource";

describe("useInvokeResource", () => {
  beforeEach(() => {
    invokeMock.mockReset();
  });

  it("invokes the given command on mount and passes the result to set", async () => {
    invokeMock.mockResolvedValueOnce(["provider-a"]);
    const set = vi.fn();
    renderHook(() => useInvokeResource("list_providers", set, vi.fn()));

    await waitFor(() => expect(set).toHaveBeenCalledWith(["provider-a"]));
    expect(invokeMock).toHaveBeenCalledWith("list_providers");
  });

  it("routes a failed invoke through onError instead of throwing", async () => {
    invokeMock.mockRejectedValueOnce(new Error("boom"));
    const onError = vi.fn();
    renderHook(() => useInvokeResource("list_providers", vi.fn(), onError));

    await waitFor(() => expect(onError).toHaveBeenCalledWith("Error: boom"));
  });

  it("the returned refresh function re-invokes the command on demand", async () => {
    invokeMock.mockResolvedValueOnce(["first"]);
    const set = vi.fn();
    const { result } = renderHook(() => useInvokeResource("list_providers", set, vi.fn()));
    await waitFor(() => expect(set).toHaveBeenCalledWith(["first"]));

    invokeMock.mockResolvedValueOnce(["second"]);
    await act(async () => {
      result.current();
    });

    expect(set).toHaveBeenCalledWith(["second"]);
    expect(invokeMock).toHaveBeenCalledTimes(2);
  });

  it("re-runs automatically when the command name changes", async () => {
    invokeMock.mockResolvedValueOnce(["a-result"]);
    const set = vi.fn();
    const { rerender } = renderHook(({ cmd }) => useInvokeResource(cmd, set, vi.fn()), {
      initialProps: { cmd: "command_a" },
    });
    await waitFor(() => expect(invokeMock).toHaveBeenCalledWith("command_a"));

    invokeMock.mockResolvedValueOnce(["b-result"]);
    rerender({ cmd: "command_b" });

    await waitFor(() => expect(invokeMock).toHaveBeenCalledWith("command_b"));
  });
});
