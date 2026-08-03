import { describe, it, expect, beforeEach } from "vitest";
import { useShellStore } from "./shellStore";

function reset() {
  localStorage.clear();
  useShellStore.setState({
    sidebarCollapsed: false,
    sidebarWidth: 250,
    sidebarTab: "chats",
    appMenuOpen: false,
    extraTabs: {},
    textDocs: {},
    openTabs: [],
    activeTab: null,
    cmdkOpen: false,
    onboardingOpen: false,
    onboardingDismissed: false,
    settingsOpen: false,
    settingsPane: "models",
    contextUsage: {},
  });
}

describe("shellStore: sidebar", () => {
  beforeEach(reset);

  it("toggleSidebar flips sidebarCollapsed", () => {
    expect(useShellStore.getState().sidebarCollapsed).toBe(false);
    useShellStore.getState().toggleSidebar();
    expect(useShellStore.getState().sidebarCollapsed).toBe(true);
    useShellStore.getState().toggleSidebar();
    expect(useShellStore.getState().sidebarCollapsed).toBe(false);
  });

  it("setSidebarWidth clamps to the [180, 420] range", () => {
    useShellStore.getState().setSidebarWidth(50);
    expect(useShellStore.getState().sidebarWidth).toBe(180);
    useShellStore.getState().setSidebarWidth(1000);
    expect(useShellStore.getState().sidebarWidth).toBe(420);
    useShellStore.getState().setSidebarWidth(300);
    expect(useShellStore.getState().sidebarWidth).toBe(300);
  });

  it("setSidebarTab switches between chats and folder", () => {
    useShellStore.getState().setSidebarTab("folder");
    expect(useShellStore.getState().sidebarTab).toBe("folder");
  });
});

describe("shellStore: tabs", () => {
  beforeEach(reset);

  it("openConversationTab adds a new tab and makes it active", () => {
    useShellStore.getState().openConversationTab("conv-1");
    const s = useShellStore.getState();
    expect(s.openTabs).toEqual(["conv-1"]);
    expect(s.activeTab).toBe("conv-1");
  });

  it("openConversationTab does not duplicate an already-open tab", () => {
    useShellStore.getState().openConversationTab("conv-1");
    useShellStore.getState().openConversationTab("conv-2");
    useShellStore.getState().openConversationTab("conv-1");
    expect(useShellStore.getState().openTabs).toEqual(["conv-1", "conv-2"]);
    expect(useShellStore.getState().activeTab).toBe("conv-1");
  });

  it("closeTab removes the tab and falls back to the last remaining tab if it was active", () => {
    useShellStore.getState().openConversationTab("conv-1");
    useShellStore.getState().openConversationTab("conv-2");
    useShellStore.getState().setActiveTab("conv-1");
    useShellStore.getState().closeTab("conv-1");
    const s = useShellStore.getState();
    expect(s.openTabs).toEqual(["conv-2"]);
    expect(s.activeTab).toBe("conv-2");
  });

  it("closeTab leaves activeTab null when it was the only tab", () => {
    useShellStore.getState().openConversationTab("conv-1");
    useShellStore.getState().closeTab("conv-1");
    const s = useShellStore.getState();
    expect(s.openTabs).toEqual([]);
    expect(s.activeTab).toBeNull();
  });

  it("closeTab does not disturb activeTab when closing a non-active tab", () => {
    useShellStore.getState().openConversationTab("conv-1");
    useShellStore.getState().openConversationTab("conv-2");
    useShellStore.getState().setActiveTab("conv-2");
    useShellStore.getState().closeTab("conv-1");
    expect(useShellStore.getState().activeTab).toBe("conv-2");
  });

  it("closeTab also removes the corresponding extraTabs entry", () => {
    const id = useShellStore.getState().openExtraTab("text", "notes.md", "hello");
    useShellStore.getState().closeTab(id);
    expect(useShellStore.getState().extraTabs[id]).toBeUndefined();
  });

  it("openExtraTab creates a text tab with the given title and content", () => {
    const id = useShellStore.getState().openExtraTab("text", "notes.md", "hello world");
    const s = useShellStore.getState();
    expect(s.extraTabs[id]).toEqual({ id, type: "text", title: "notes.md", filePath: undefined });
    expect(s.textDocs[id]).toBe("hello world");
    expect(s.activeTab).toBe(id);
  });

  it("openExtraTab defaults an untitled text tab's name and content", () => {
    const id = useShellStore.getState().openExtraTab("text");
    const s = useShellStore.getState();
    expect(s.extraTabs[id].title).toBe("untitled.md");
    expect(s.textDocs[id]).toBe("");
  });

  it("openExtraTab is a singleton for the flow type: opening it twice reuses the same tab", () => {
    const first = useShellStore.getState().openExtraTab("flow");
    const second = useShellStore.getState().openExtraTab("flow");
    expect(second).toBe(first);
    expect(useShellStore.getState().openTabs).toEqual([first]);
  });

  it("re-opening the singleton flow tab after it was closed elsewhere still re-adds it to openTabs", () => {
    const id = useShellStore.getState().openExtraTab("flow");
    useShellStore.setState((s) => ({ openTabs: s.openTabs.filter((t) => t !== id) }));
    const reopened = useShellStore.getState().openExtraTab("flow");
    expect(reopened).toBe(id);
    expect(useShellStore.getState().openTabs).toContain(id);
  });
});

describe("shellStore: onboarding", () => {
  beforeEach(reset);

  it("dismissOnboarding persists to localStorage and updates state", () => {
    useShellStore.getState().setOnboardingOpen(true);
    useShellStore.getState().dismissOnboarding();
    const s = useShellStore.getState();
    expect(s.onboardingOpen).toBe(false);
    expect(s.onboardingDismissed).toBe(true);
    expect(localStorage.getItem("syl:onboarded")).toBe("1");
  });
});

describe("shellStore: settings", () => {
  beforeEach(reset);

  it("openSettings opens the overlay on the given pane", () => {
    useShellStore.getState().openSettings("providers");
    const s = useShellStore.getState();
    expect(s.settingsOpen).toBe(true);
    expect(s.settingsPane).toBe("providers");
  });

  it("openSettings without a pane argument keeps the current pane", () => {
    useShellStore.getState().openSettings("tools");
    useShellStore.getState().closeSettings();
    useShellStore.getState().openSettings();
    expect(useShellStore.getState().settingsPane).toBe("tools");
  });

  it("closeSettings closes the overlay without touching the pane", () => {
    useShellStore.getState().openSettings("mcp");
    useShellStore.getState().closeSettings();
    const s = useShellStore.getState();
    expect(s.settingsOpen).toBe(false);
    expect(s.settingsPane).toBe("mcp");
  });
});

describe("shellStore: context usage", () => {
  beforeEach(reset);

  it("setContextUsage records usage per conversation id without clobbering others", () => {
    useShellStore.getState().setContextUsage("conv-1", { usedTokens: 10, totalTokens: 4096, modelLabel: "a" });
    useShellStore.getState().setContextUsage("conv-2", { usedTokens: 20, totalTokens: 8192, modelLabel: "b" });
    const s = useShellStore.getState();
    expect(s.contextUsage["conv-1"]).toEqual({ usedTokens: 10, totalTokens: 4096, modelLabel: "a" });
    expect(s.contextUsage["conv-2"]).toEqual({ usedTokens: 20, totalTokens: 8192, modelLabel: "b" });
  });

  it("setContextUsage overwrites a conversation's previous usage", () => {
    useShellStore.getState().setContextUsage("conv-1", { usedTokens: 10, totalTokens: 4096, modelLabel: "a" });
    useShellStore.getState().setContextUsage("conv-1", { usedTokens: 50, totalTokens: 4096, modelLabel: "a" });
    expect(useShellStore.getState().contextUsage["conv-1"].usedTokens).toBe(50);
  });
});
