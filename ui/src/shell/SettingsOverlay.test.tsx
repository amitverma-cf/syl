import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";

const invokeMock = vi.hoisted(() => vi.fn(() => Promise.resolve([])));
vi.mock("@tauri-apps/api/core", () => ({ invoke: invokeMock }));
vi.mock("sonner", () => ({ toast: { success: vi.fn(), error: vi.fn() } }));

import { useShellStore } from "../store/shellStore";
import SettingsOverlay from "./SettingsOverlay";

const baseProps = {
  activeConversationId: null,
  providers: [],
  refreshProviders: vi.fn(),
  customProviders: [],
  refreshCustomProviders: vi.fn(),
  cloudModels: [],
  models: [],
  refreshModels: vi.fn(),
  localModels: [],
  refreshLocalModels: vi.fn(),
  mcpServers: [],
  refreshMcpServers: vi.fn(),
  stats: null,
  conversations: [],
};

function resetStore() {
  useShellStore.setState({ settingsOpen: false, settingsPane: "models" });
}

describe("SettingsOverlay", () => {
  beforeEach(() => {
    resetStore();
    invokeMock.mockClear();
  });

  it("renders nothing when settingsOpen is false", () => {
    const { container } = render(<SettingsOverlay {...baseProps} />);
    expect(container).toBeEmptyDOMElement();
  });

  it("shows all six nav panes with the current pane marked active", () => {
    useShellStore.setState({ settingsOpen: true, settingsPane: "mcp" });
    const { container } = render(<SettingsOverlay {...baseProps} />);

    const navItems = container.querySelectorAll(".settings-nav-item");
    expect(navItems).toHaveLength(6);
    const active = container.querySelector(".settings-nav-item.active");
    expect(active).toHaveTextContent("MCP Servers");
  });

  it("shows the active pane's title in the header and its content below", () => {
    useShellStore.setState({ settingsOpen: true, settingsPane: "memory" });
    render(<SettingsOverlay {...baseProps} />);

    expect(screen.getByRole("heading", { name: "Memory" })).toBeInTheDocument();
    expect(screen.getByText(/Conversation history and embeddings/)).toBeInTheDocument();
  });

  it("clicking a different nav item switches the active pane", () => {
    useShellStore.setState({ settingsOpen: true, settingsPane: "models" });
    render(<SettingsOverlay {...baseProps} />);

    fireEvent.click(screen.getByText("Tools"));

    expect(useShellStore.getState().settingsPane).toBe("tools");
    expect(screen.getByRole("heading", { name: "Tools" })).toBeInTheDocument();
  });

  it("closes via the header close button", () => {
    useShellStore.setState({ settingsOpen: true, settingsPane: "models" });
    render(<SettingsOverlay {...baseProps} />);

    fireEvent.click(screen.getByRole("button", { name: "Close settings" }));

    expect(useShellStore.getState().settingsOpen).toBe(false);
  });

  it("closes when the backdrop itself is clicked, but not when the card is clicked", () => {
    useShellStore.setState({ settingsOpen: true, settingsPane: "models" });
    const { container } = render(<SettingsOverlay {...baseProps} />);

    fireEvent.click(container.querySelector(".settings-card")!);
    expect(useShellStore.getState().settingsOpen).toBe(true);

    fireEvent.click(container.querySelector(".settings-overlay")!);
    expect(useShellStore.getState().settingsOpen).toBe(false);
  });

  it("falls back to the first pane if settingsPane holds an unrecognized value", () => {
    useShellStore.setState({ settingsOpen: true, settingsPane: "not-a-real-pane" });
    render(<SettingsOverlay {...baseProps} />);
    expect(screen.getByRole("heading", { name: "AI Providers & Models" })).toBeInTheDocument();
  });
});
