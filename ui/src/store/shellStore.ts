import { create } from "zustand";

export type Platform = "mac" | "windows" | "linux" | "chromebook";
export type SidebarTab = "chats" | "folder";
export type ExtraTabType = "flow" | "text";

export interface ExtraTab {
  id: string;
  type: ExtraTabType;
  title: string;
  filePath?: string;
}

export interface ContextUsage {
  usedTokens: number;
  totalTokens: number;
  modelLabel: string;
}

function detectPlatform(): Platform {
  const ua = navigator.userAgent;
  if (ua.includes("Win")) return "windows";
  if (ua.includes("Mac")) return "mac";
  if (ua.includes("CrOS")) return "chromebook";
  return "linux";
}

interface ShellState {
  platform: Platform;
  setPlatform: (p: Platform) => void;

  sidebarCollapsed: boolean;
  toggleSidebar: () => void;
  sidebarWidth: number;
  setSidebarWidth: (w: number) => void;
  sidebarTab: SidebarTab;
  setSidebarTab: (t: SidebarTab) => void;

  appMenuOpen: boolean;
  setAppMenuOpen: (v: boolean) => void;

  extraTabs: Record<string, ExtraTab>;
  textDocs: Record<string, string>;
  setTextDoc: (id: string, content: string) => void;
  openTabs: string[];
  activeTab: string | null;
  openConversationTab: (id: string) => void;
  closeTab: (id: string) => void;
  setActiveTab: (id: string) => void;
  openExtraTab: (type: ExtraTabType, title?: string, content?: string, filePath?: string) => string;

  cmdkOpen: boolean;
  setCmdkOpen: (v: boolean) => void;

  onboardingOpen: boolean;
  setOnboardingOpen: (v: boolean) => void;
  onboardingDismissed: boolean;
  dismissOnboarding: () => void;

  settingsOpen: boolean;
  settingsPane: string;
  openSettings: (pane?: string) => void;
  closeSettings: () => void;

  contextUsage: Record<string, ContextUsage>;
  setContextUsage: (conversationId: string, usage: ContextUsage) => void;
}

export const useShellStore = create<ShellState>((set, get) => ({
  platform: detectPlatform(),
  setPlatform: (p) => set({ platform: p }),

  sidebarCollapsed: false,
  toggleSidebar: () => set((s) => ({ sidebarCollapsed: !s.sidebarCollapsed })),
  sidebarWidth: 250,
  setSidebarWidth: (w) => set({ sidebarWidth: Math.min(420, Math.max(180, w)) }),
  sidebarTab: "chats",
  setSidebarTab: (t) => set({ sidebarTab: t }),

  appMenuOpen: false,
  setAppMenuOpen: (v) => set({ appMenuOpen: v }),

  extraTabs: {},
  textDocs: {},
  setTextDoc: (id, content) => set((s) => ({ textDocs: { ...s.textDocs, [id]: content } })),
  openTabs: [],
  activeTab: null,
  openConversationTab: (id) =>
    set((s) => ({
      openTabs: s.openTabs.includes(id) ? s.openTabs : [...s.openTabs, id],
      activeTab: id,
    })),
  closeTab: (id) =>
    set((s) => {
      const openTabs = s.openTabs.filter((t) => t !== id);
      const extraTabs = { ...s.extraTabs };
      delete extraTabs[id];
      let activeTab = s.activeTab;
      if (activeTab === id) {
        activeTab = openTabs[openTabs.length - 1] ?? null;
      }
      return { openTabs, extraTabs, activeTab };
    }),
  setActiveTab: (id) => set({ activeTab: id }),
  openExtraTab: (type, title, content, filePath) => {
    const existingFlow = type === "flow" ? Object.values(get().extraTabs).find((t) => t.type === "flow") : null;
    if (existingFlow) {
      set((s) => ({
        activeTab: existingFlow.id,
        openTabs: s.openTabs.includes(existingFlow.id) ? s.openTabs : [...s.openTabs, existingFlow.id],
      }));
      return existingFlow.id;
    }
    const id = `${type}-${Date.now()}-${Math.random().toString(36).slice(2, 6)}`;
    const titles: Record<ExtraTabType, string> = {
      flow: "Flow editor",
      text: title ?? "untitled.md",
    };
    set((s) => ({
      extraTabs: { ...s.extraTabs, [id]: { id, type, title: titles[type], filePath } },
      textDocs: type === "text" ? { ...s.textDocs, [id]: content ?? "" } : s.textDocs,
      openTabs: [...s.openTabs, id],
      activeTab: id,
    }));
    return id;
  },

  cmdkOpen: false,
  setCmdkOpen: (v) => set({ cmdkOpen: v }),

  onboardingOpen: false,
  setOnboardingOpen: (v) => set({ onboardingOpen: v }),
  onboardingDismissed: localStorage.getItem("syl:onboarded") === "1",
  dismissOnboarding: () => {
    localStorage.setItem("syl:onboarded", "1");
    set({ onboardingDismissed: true, onboardingOpen: false });
  },

  settingsOpen: false,
  settingsPane: "models",
  openSettings: (pane) => set({ settingsOpen: true, settingsPane: pane ?? get().settingsPane }),
  closeSettings: () => set({ settingsOpen: false }),

  contextUsage: {},
  setContextUsage: (conversationId, usage) =>
    set((s) => ({ contextUsage: { ...s.contextUsage, [conversationId]: usage } })),
}));
