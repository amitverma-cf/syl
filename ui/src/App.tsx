import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Toaster } from "sonner";
import { IconFileText, IconWorld, IconBold, IconItalic, IconUnderline } from "@tabler/icons-react";
import "./styles/shell.css";
import TopBar from "./shell/TopBar";
import SidebarShell from "./shell/SidebarShell";
import StatusBar from "./shell/StatusBar";
import CommandPalette from "./shell/CommandPalette";
import OnboardingOverlay from "./shell/OnboardingOverlay";
import SettingsOverlay from "./shell/SettingsOverlay";
import { TextTabView, BrowserTabView, FlowTabView } from "./shell/ExtraTabViews";
import ChatPanel from "./components/ChatPanel";
import { useShellStore } from "./store/shellStore";
import { useConversations } from "./hooks/useConversations";
import { useLocalModels } from "./hooks/useLocalModels";
import { useInvokeResource } from "./hooks/useInvokeResource";
import type {
  CatalogModel,
  CloudModel,
  CustomProviderConfig,
  McpServerConfig,
  ProviderInfo,
} from "./types";

function App() {
  const [backgroundError, setBackgroundError] = useState<string | null>(null);

  const {
    conversations,
    activeConversationId,
    setActiveConversationId,
    refresh: refreshConversations,
    newChat: handleNewChat,
    handleDeleted,
  } = useConversations(setBackgroundError);

  const { localModels, stats, refresh: refreshLocalModels } = useLocalModels(setBackgroundError);

  const [providers, setProviders] = useState<ProviderInfo[]>([]);
  const [cloudModels, setCloudModels] = useState<CloudModel[]>([]);
  const [customProviders, setCustomProviders] = useState<CustomProviderConfig[]>([]);
  const [models, setModels] = useState<CatalogModel[]>([]);
  const [mcpServers, setMcpServers] = useState<McpServerConfig[]>([]);

  const refreshProviders = useInvokeResource("list_providers", setProviders, setBackgroundError);
  useInvokeResource("list_cloud_models", setCloudModels, setBackgroundError);
  const refreshCustomProviders = useInvokeResource(
    "list_custom_providers",
    setCustomProviders,
    setBackgroundError,
  );
  const refreshModels = useInvokeResource("list_available_models", setModels, setBackgroundError);
  const refreshMcpServers = useInvokeResource("list_mcp_servers", setMcpServers, setBackgroundError);

  const {
    openTabs,
    activeTab,
    extraTabs,
    openConversationTab,
    closeTab,
    setActiveTab,
    onboardingOpen,
    setOnboardingOpen,
    onboardingDismissed,
  } = useShellStore();

  useEffect(() => {
    if (activeConversationId && !openTabs.includes(activeConversationId)) {
      openConversationTab(activeConversationId);
    } else if (activeConversationId) {
      setActiveTab(activeConversationId);
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [activeConversationId]);

  useEffect(() => {
    if (!onboardingDismissed && conversations.length === 0 && !onboardingOpen) {
      setOnboardingOpen(true);
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [onboardingDismissed, conversations.length]);

  useEffect(() => {
    function onKey(e: KeyboardEvent) {
      if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === "k") {
        e.preventDefault();
        useShellStore.getState().setCmdkOpen(true);
      }
    }
    document.addEventListener("keydown", onKey);
    return () => document.removeEventListener("keydown", onKey);
  }, []);

  async function handleDeleteConversation(id: string) {
    if (!confirm("Delete this conversation?")) return;
    await invoke("delete_conversation", { id });
    handleDeleted(id);
    closeTab(id);
  }

  function selectConversation(id: string) {
    setActiveConversationId(id);
    openConversationTab(id);
  }

  const activeExtra = activeTab ? extraTabs[activeTab] : null;
  const activeConversation = activeTab && !activeExtra ? activeTab : null;

  return (
    <div className="shell-window" data-platform={useShellStore((s) => s.platform)}>
      <Toaster theme="dark" position="bottom-right" toastOptions={{ style: { fontSize: 12.5 } }} />
      <TopBar conversations={conversations} onNewChat={handleNewChat} />

      <div className="shell-body">
        <SidebarShell
          conversations={conversations}
          activeConversationId={activeConversationId}
          onSelect={selectConversation}
          onDelete={handleDeleteConversation}
        />

        <div className="main">
          {(activeExtra?.type === "text" || activeExtra?.type === "browser") && (
            <div className="main-header">
              <span className="meta">{activeExtra.title}</span>
              {activeExtra.type === "text" && (
                <div className="header-toolbar">
                  <div className="header-icon-btn">
                    <IconBold size={14} aria-hidden />
                  </div>
                  <div className="header-icon-btn">
                    <IconItalic size={14} aria-hidden />
                  </div>
                  <div className="header-icon-btn">
                    <IconUnderline size={14} aria-hidden />
                  </div>
                </div>
              )}
              {activeExtra.type === "browser" && (
                <div className="header-toolbar">
                  <IconWorld size={14} aria-hidden />
                </div>
              )}
            </div>
          )}

          <div className="main-content">
            {backgroundError && (
              <div
                style={{
                  position: "absolute",
                  top: 0,
                  left: 0,
                  right: 0,
                  zIndex: 5,
                  background: "rgba(255,90,90,0.12)",
                  color: "#ff8783",
                  fontSize: 11.5,
                  padding: "6px 14px",
                  display: "flex",
                  justifyContent: "space-between",
                }}
              >
                <span>{backgroundError}</span>
                <span style={{ cursor: "pointer" }} onClick={() => setBackgroundError(null)}>
                  Dismiss
                </span>
              </div>
            )}

            {activeConversation ? (
              <div className="view active">
                <ChatPanel
                  key={activeConversation}
                  conversationId={activeConversation}
                  cloudModels={cloudModels}
                  providers={providers}
                  localModels={localModels}
                  refreshLocalModels={refreshLocalModels}
                  onTurnComplete={refreshConversations}
                />
              </div>
            ) : activeExtra?.type === "text" ? (
              <div className="view active">
                <TextTabView tabId={activeExtra.id} />
              </div>
            ) : activeExtra?.type === "browser" ? (
              <div className="view active">
                <BrowserTabView tabId={activeExtra.id} />
              </div>
            ) : activeExtra?.type === "flow" ? (
              <div className="view active">
                <FlowTabView activeConversationId={activeConversationId} />
              </div>
            ) : (
              <div className="empty-state">
                <IconFileText size={26} aria-hidden />
                <span>Select a conversation or start a new one</span>
                <div className="es-action" onClick={handleNewChat}>
                  New chat
                </div>
              </div>
            )}
          </div>
        </div>
      </div>

      <StatusBar stats={stats} loadedLocalModels={localModels.filter((m) => m.loaded)} />

      <CommandPalette
        conversations={conversations}
        localModels={localModels}
        onSelectConversation={selectConversation}
        onNewChat={handleNewChat}
      />
      <OnboardingOverlay />
      <SettingsOverlay
        activeConversationId={activeConversationId}
        providers={providers}
        refreshProviders={refreshProviders}
        customProviders={customProviders}
        refreshCustomProviders={refreshCustomProviders}
        cloudModels={cloudModels}
        models={models}
        refreshModels={refreshModels}
        localModels={localModels}
        refreshLocalModels={refreshLocalModels}
        mcpServers={mcpServers}
        refreshMcpServers={refreshMcpServers}
        stats={stats}
        conversations={conversations}
      />
    </div>
  );
}

export default App;
