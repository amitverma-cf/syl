import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";

const writeTextFileMock = vi.hoisted(() => vi.fn());
vi.mock("@tauri-apps/plugin-fs", () => ({ writeTextFile: writeTextFileMock }));

const toastSuccess = vi.hoisted(() => vi.fn());
const toastError = vi.hoisted(() => vi.fn());
vi.mock("sonner", () => ({ toast: { success: toastSuccess, error: toastError } }));

import { useShellStore } from "../store/shellStore";
import { TextTabView } from "./ExtraTabViews";

function resetStore() {
  useShellStore.setState({ textDocs: {}, extraTabs: {} });
}

describe("TextTabView", () => {
  beforeEach(() => {
    resetStore();
    writeTextFileMock.mockReset();
    toastSuccess.mockReset();
    toastError.mockReset();
  });

  it("renders the tab's current content", () => {
    useShellStore.setState({ textDocs: { "tab-1": "hello world" } });
    render(<TextTabView tabId="tab-1" />);
    expect(screen.getByRole("textbox")).toHaveValue("hello world");
  });

  it("typing updates the tab's content in the shared store", () => {
    render(<TextTabView tabId="tab-1" />);
    fireEvent.change(screen.getByRole("textbox"), { target: { value: "new content" } });
    expect(useShellStore.getState().textDocs["tab-1"]).toBe("new content");
  });

  it("Ctrl+S with no associated file path does nothing (no save attempt)", () => {
    useShellStore.setState({ textDocs: { "tab-1": "x" }, extraTabs: {} });
    render(<TextTabView tabId="tab-1" />);
    fireEvent.keyDown(screen.getByRole("textbox"), { key: "s", ctrlKey: true });
    expect(writeTextFileMock).not.toHaveBeenCalled();
  });

  it("Ctrl+S with a real file path saves the content and shows a success toast", async () => {
    useShellStore.setState({
      textDocs: { "tab-1": "content to save" },
      extraTabs: { "tab-1": { id: "tab-1", type: "text", title: "notes.md", filePath: "/tmp/notes.md" } },
    });
    writeTextFileMock.mockResolvedValueOnce(undefined);
    render(<TextTabView tabId="tab-1" />);

    fireEvent.keyDown(screen.getByRole("textbox"), { key: "s", ctrlKey: true });
    await Promise.resolve();
    await Promise.resolve();

    expect(writeTextFileMock).toHaveBeenCalledWith("/tmp/notes.md", "content to save");
    expect(toastSuccess).toHaveBeenCalledWith("Saved notes.md");
  });

  it("shows an error toast (not a thrown exception) if the real save fails", async () => {
    useShellStore.setState({
      textDocs: { "tab-1": "x" },
      extraTabs: { "tab-1": { id: "tab-1", type: "text", title: "notes.md", filePath: "/tmp/notes.md" } },
    });
    writeTextFileMock.mockRejectedValueOnce(new Error("disk full"));
    render(<TextTabView tabId="tab-1" />);

    fireEvent.keyDown(screen.getByRole("textbox"), { key: "s", ctrlKey: true });
    await Promise.resolve();
    await Promise.resolve();

    expect(toastError).toHaveBeenCalledWith("Error: disk full");
  });

  it("a plain 's' keypress without Ctrl/Cmd does not trigger a save", () => {
    useShellStore.setState({
      textDocs: { "tab-1": "x" },
      extraTabs: { "tab-1": { id: "tab-1", type: "text", title: "notes.md", filePath: "/tmp/notes.md" } },
    });
    render(<TextTabView tabId="tab-1" />);
    fireEvent.keyDown(screen.getByRole("textbox"), { key: "s" });
    expect(writeTextFileMock).not.toHaveBeenCalled();
  });
});
