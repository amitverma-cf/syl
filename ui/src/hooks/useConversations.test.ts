import { describe, it, expect, vi, beforeEach } from "vitest";
import { renderHook, waitFor, act } from "@testing-library/react";

const invokeMock = vi.hoisted(() => vi.fn());
vi.mock("@tauri-apps/api/core", () => ({ invoke: invokeMock }));

import { useConversations } from "./useConversations";

const conv = (id: string, title = "chat") => ({
  id,
  title,
  flowName: "default",
  createdAt: 0,
  updatedAt: 0,
});

describe("useConversations", () => {
  beforeEach(() => {
    invokeMock.mockReset();
  });

  it("loads the conversation list on mount and defaults activeConversationId to the first one", async () => {
    invokeMock.mockResolvedValueOnce([conv("a"), conv("b")]);
    const onError = vi.fn();
    const { result } = renderHook(() => useConversations(onError));

    await waitFor(() => expect(result.current.conversations).toHaveLength(2));
    expect(result.current.activeConversationId).toBe("a");
    expect(invokeMock).toHaveBeenCalledWith("list_conversations");
    expect(onError).not.toHaveBeenCalled();
  });

  it("reports errors from a failed list_conversations call via onError, not a thrown exception", async () => {
    invokeMock.mockRejectedValueOnce(new Error("db locked"));
    const onError = vi.fn();
    renderHook(() => useConversations(onError));

    await waitFor(() => expect(onError).toHaveBeenCalledWith("Error: db locked"));
  });

  it("newChat creates a conversation, makes it active, and refreshes the list", async () => {
    invokeMock.mockResolvedValueOnce([]); // initial refresh on mount
    const onError = vi.fn();
    const { result } = renderHook(() => useConversations(onError));
    await waitFor(() => expect(invokeMock).toHaveBeenCalledTimes(1));

    invokeMock.mockResolvedValueOnce(undefined); // create_conversation
    invokeMock.mockResolvedValueOnce([conv("new-id", "New chat")]); // refresh after create

    await act(async () => {
      await result.current.newChat();
    });

    expect(invokeMock).toHaveBeenCalledWith("create_conversation", expect.objectContaining({ title: "New chat" }));
    await waitFor(() => expect(result.current.conversations).toHaveLength(1));
  });

  it("newChat reports an error via onError if create_conversation fails, without crashing", async () => {
    invokeMock.mockResolvedValueOnce([]);
    const onError = vi.fn();
    const { result } = renderHook(() => useConversations(onError));
    await waitFor(() => expect(invokeMock).toHaveBeenCalledTimes(1));

    invokeMock.mockRejectedValueOnce(new Error("disk full"));
    await act(async () => {
      await result.current.newChat();
    });

    expect(onError).toHaveBeenCalledWith("Error: disk full");
  });

  it("handleDeleted removes the conversation locally and clears activeConversationId if it was active", async () => {
    invokeMock.mockResolvedValue([conv("a"), conv("b")]);
    const { result } = renderHook(() => useConversations(vi.fn()));
    await act(async () => {
      await Promise.resolve();
      await Promise.resolve();
    });
    expect(result.current.conversations).toHaveLength(2);

    act(() => result.current.handleDeleted("a"));

    expect(result.current.conversations.map((c) => c.id)).toEqual(["b"]);
    expect(result.current.activeConversationId).toBeNull();
  });

  it("handleDeleted leaves activeConversationId alone when deleting a non-active conversation", async () => {
    invokeMock.mockResolvedValue([conv("a"), conv("b")]);
    const { result } = renderHook(() => useConversations(vi.fn()));
    await act(async () => {
      await Promise.resolve();
      await Promise.resolve();
    });
    expect(result.current.conversations).toHaveLength(2);

    act(() => result.current.handleDeleted("b"));

    expect(result.current.activeConversationId).toBe("a");
  });
});
