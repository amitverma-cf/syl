import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent, waitFor } from "@testing-library/react";

const invokeMock = vi.hoisted(() => vi.fn());
vi.mock("@tauri-apps/api/core", () => ({ invoke: invokeMock }));

import ToolsTab from "./ToolsTab";

const tools = [
  { name: "read_file", description: "Read a file", inputSchema: {} },
  { name: "run_command", description: "Run a command", inputSchema: {} },
];

const permissions = [
  { toolName: "read_file", decision: "Allow" as const },
  { toolName: "run_command", decision: "Deny" as const },
];

function mockInvoke() {
  invokeMock.mockImplementation((cmd: string) => {
    if (cmd === "list_tools") return Promise.resolve(tools);
    if (cmd === "list_tool_permissions") return Promise.resolve(permissions);
    if (cmd === "clear_tool_permission") return Promise.resolve();
    return Promise.reject(new Error(`unexpected command ${cmd}`));
  });
}

describe("ToolsTab", () => {
  beforeEach(() => {
    invokeMock.mockReset();
    mockInvoke();
  });

  it("lists every registered tool", async () => {
    render(<ToolsTab activeConversationId={null} />);
    await waitFor(() => expect(screen.getByText("read_file")).toBeInTheDocument());
    expect(screen.getByText("run_command")).toBeInTheDocument();
  });

  it("prompts to open a conversation when none is active", async () => {
    render(<ToolsTab activeConversationId={null} />);
    await waitFor(() =>
      expect(
        screen.getByText("Open a conversation to manage its remembered tool permissions."),
      ).toBeInTheDocument(),
    );
    expect(invokeMock).not.toHaveBeenCalledWith("list_tool_permissions", expect.anything());
  });

  it("lists remembered permissions for the active conversation", async () => {
    render(<ToolsTab activeConversationId="conv-1" />);
    await waitFor(() =>
      expect(invokeMock).toHaveBeenCalledWith("list_tool_permissions", { conversationId: "conv-1" }),
    );
    expect(await screen.findByText("always allowed")).toBeInTheDocument();
    expect(screen.getByText("always denied")).toBeInTheDocument();
  });

  it("shows an empty state when nothing is remembered yet", async () => {
    invokeMock.mockImplementation((cmd: string) => {
      if (cmd === "list_tools") return Promise.resolve(tools);
      if (cmd === "list_tool_permissions") return Promise.resolve([]);
      return Promise.reject(new Error(`unexpected command ${cmd}`));
    });
    render(<ToolsTab activeConversationId="conv-1" />);
    await waitFor(() =>
      expect(
        screen.getByText(/No "Always allow"\/"Always deny" decisions remembered yet/),
      ).toBeInTheDocument(),
    );
  });

  it("revoking a permission calls the real command and refreshes the list", async () => {
    render(<ToolsTab activeConversationId="conv-1" />);
    await screen.findByText("always allowed");

    fireEvent.click(
      screen.getAllByTitle("Forget this decision — the next call will prompt again")[0],
    );

    await waitFor(() =>
      expect(invokeMock).toHaveBeenCalledWith("clear_tool_permission", {
        conversationId: "conv-1",
        toolName: "read_file",
      }),
    );
    // Refresh is called again after revoke.
    const listCalls = invokeMock.mock.calls.filter(([cmd]) => cmd === "list_tool_permissions");
    expect(listCalls.length).toBeGreaterThanOrEqual(2);
  });
});
