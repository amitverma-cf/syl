import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent, waitFor } from "@testing-library/react";

const invokeMock = vi.hoisted(() => vi.fn());
vi.mock("@tauri-apps/api/core", () => ({ invoke: invokeMock }));

import { useShellStore } from "../store/shellStore";
import TopBar from "./TopBar";
import type { ConversationSummary } from "../types";

const FLOW_EDITOR_CONTRIBUTION = {
  extensionId: "flow-editor",
  kind: "sidebarView",
  id: "flow-editor",
  title: "Flow Editor",
};

const conv = (id: string, title: string): ConversationSummary => ({
  id,
  title,
  flowName: "default",
  createdAt: 0,
  updatedAt: 0,
});

function resetStore() {
  useShellStore.setState({
    platform: "windows",
    appMenuOpen: false,
    openTabs: [],
    activeTab: null,
    extraTabs: {},
    cmdkOpen: false,
    onboardingOpen: false,
    sidebarCollapsed: false,
  });
}

describe("TopBar", () => {
  beforeEach(() => {
    resetStore();
    invokeMock.mockReset();
    invokeMock.mockResolvedValue([FLOW_EDITOR_CONTRIBUTION]);
  });

  it("shows the app title and no tabs when nothing is open", () => {
    render(<TopBar conversations={[]} onNewChat={vi.fn()} onOpenFlowEditor={vi.fn()} />);
    expect(screen.getByText("syl")).toBeInTheDocument();
  });

  it("renders a tab per open conversation, using its real title", () => {
    useShellStore.setState({ openTabs: ["a", "b"], activeTab: "a" });
    render(
      <TopBar
        conversations={[conv("a", "Debugging cpal"), conv("b", "Registry notes")]}
        onNewChat={vi.fn()}
        onOpenFlowEditor={vi.fn()}
      />,
    );
    expect(screen.getByText("Debugging cpal")).toBeInTheDocument();
    expect(screen.getByText("Registry notes")).toBeInTheDocument();
  });

  it("falls back to 'Untitled' for a conversation id with no matching summary", () => {
    useShellStore.setState({ openTabs: ["ghost"], activeTab: "ghost" });
    render(<TopBar conversations={[]} onNewChat={vi.fn()} onOpenFlowEditor={vi.fn()} />);
    expect(screen.getByText("Untitled")).toBeInTheDocument();
  });

  it("uses the extra tab's own title instead of looking it up in conversations", () => {
    useShellStore.setState({
      openTabs: ["flow-1"],
      activeTab: "flow-1",
      extraTabs: { "flow-1": { id: "flow-1", type: "flow", title: "Flow editor" } },
    });
    render(<TopBar conversations={[]} onNewChat={vi.fn()} onOpenFlowEditor={vi.fn()} />);
    expect(screen.getByText("Flow editor")).toBeInTheDocument();
  });

  it("clicking a tab makes it active", () => {
    useShellStore.setState({ openTabs: ["a", "b"], activeTab: "a" });
    render(<TopBar conversations={[conv("a", "A"), conv("b", "B")]} onNewChat={vi.fn()} onOpenFlowEditor={vi.fn()} />);

    fireEvent.click(screen.getByText("B"));
    expect(useShellStore.getState().activeTab).toBe("b");
  });

  it("clicking a tab's close button closes it without also activating it", () => {
    useShellStore.setState({ openTabs: ["a", "b"], activeTab: "a" });
    const { container } = render(
      <TopBar conversations={[conv("a", "A"), conv("b", "B")]} onNewChat={vi.fn()} onOpenFlowEditor={vi.fn()} />,
    );

    const bTab = screen.getByText("B").closest(".chat-tab")!;
    fireEvent.click(bTab.querySelector(".tab-close")!);

    expect(useShellStore.getState().openTabs).toEqual(["a"]);
    expect(useShellStore.getState().activeTab).toBe("a");
    expect(container.querySelectorAll(".chat-tab")).toHaveLength(1);
  });

  it("clicking the + button calls onNewChat", () => {
    const onNewChat = vi.fn();
    render(<TopBar conversations={[]} onNewChat={onNewChat} onOpenFlowEditor={vi.fn()} />);
    fireEvent.click(screen.getByTitle("New chat"));
    expect(onNewChat).toHaveBeenCalledTimes(1);
  });

  it("clicking the flow editor icon calls onOpenFlowEditor", async () => {
    const onOpenFlowEditor = vi.fn();
    render(<TopBar conversations={[]} onNewChat={vi.fn()} onOpenFlowEditor={onOpenFlowEditor} />);
    await waitFor(() => expect(screen.getByTitle("Open flow editor")).toBeInTheDocument());
    fireEvent.click(screen.getByTitle("Open flow editor"));
    expect(onOpenFlowEditor).toHaveBeenCalledTimes(1);
  });

  it("clicking the search icon opens the command palette", () => {
    render(<TopBar conversations={[]} onNewChat={vi.fn()} onOpenFlowEditor={vi.fn()} />);
    fireEvent.click(screen.getByTitle("Search (⌘K)"));
    expect(useShellStore.getState().cmdkOpen).toBe(true);
  });

  it("clicking the sidebar toggle flips sidebarCollapsed", () => {
    render(<TopBar conversations={[]} onNewChat={vi.fn()} onOpenFlowEditor={vi.fn()} />);
    fireEvent.click(screen.getByTitle("Toggle sidebar"));
    expect(useShellStore.getState().sidebarCollapsed).toBe(true);
  });

  it("the app menu is closed by default and opens on click", () => {
    const { container } = render(<TopBar conversations={[]} onNewChat={vi.fn()} onOpenFlowEditor={vi.fn()} />);
    expect(container.querySelector(".app-menu")).not.toHaveClass("open");

    fireEvent.click(screen.getByTitle("Menu"));
    expect(container.querySelector(".app-menu")).toHaveClass("open");
  });

  it("'Command palette' menu item closes the menu and opens the palette", () => {
    render(<TopBar conversations={[]} onNewChat={vi.fn()} onOpenFlowEditor={vi.fn()} />);
    fireEvent.click(screen.getByTitle("Menu"));
    fireEvent.click(screen.getByText(/Command palette/));

    expect(useShellStore.getState().appMenuOpen).toBe(false);
    expect(useShellStore.getState().cmdkOpen).toBe(true);
  });

  it("'Show welcome guide' menu item closes the menu and opens onboarding", () => {
    render(<TopBar conversations={[]} onNewChat={vi.fn()} onOpenFlowEditor={vi.fn()} />);
    fireEvent.click(screen.getByTitle("Menu"));
    fireEvent.click(screen.getByText("Show welcome guide"));

    expect(useShellStore.getState().appMenuOpen).toBe(false);
    expect(useShellStore.getState().onboardingOpen).toBe(true);
  });

  it("clicking outside the app menu closes it", () => {
    const { container } = render(<TopBar conversations={[]} onNewChat={vi.fn()} onOpenFlowEditor={vi.fn()} />);
    fireEvent.click(screen.getByTitle("Menu"));
    expect(container.querySelector(".app-menu")).toHaveClass("open");

    fireEvent.click(document.body);
    expect(container.querySelector(".app-menu")).not.toHaveClass("open");
  });

  it("shows mac traffic-light dots only on the mac platform", () => {
    useShellStore.setState({ platform: "mac" });
    const { container, rerender } = render(<TopBar conversations={[]} onNewChat={vi.fn()} onOpenFlowEditor={vi.fn()} />);
    expect(container.querySelector(".traffic")).toBeInTheDocument();

    useShellStore.setState({ platform: "windows" });
    rerender(<TopBar conversations={[]} onNewChat={vi.fn()} onOpenFlowEditor={vi.fn()} />);
    expect(container.querySelector(".traffic")).not.toBeInTheDocument();
  });
});
