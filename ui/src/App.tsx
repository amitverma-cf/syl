import { lazy, Suspense, useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Toaster } from "sonner";
import { IconFileText, IconBold, IconItalic, IconUnderline } from "@tabler/icons-react";
import { IconButton } from "./components/ui";
import "./styles/shell.css";
import TopBar from "./shell/TopBar";
import SidebarShell from "./shell/SidebarShell";
import StatusBar from "./shell/StatusBar";
import OnboardingOverlay from "./shell/OnboardingOverlay";
import { TextTabView } from "./shell/ExtraTabViews";
import ChatPanel from "./components/ChatPanel";

const CommandPalette = lazy(() => import("./shell/CommandPalette"));
const SettingsOverlay = lazy(() => import("./shell/SettingsOverlay"));
const FlowEditor = lazy(() => import("./shell/FlowEditor"));
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
    openExtraTab,
    onboardingOpen,
    setOnboardingOpen,
    onboardingDismissed,
    theme,
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
      <Toaster theme={theme} position="bottom-right" toastOptions={{ style: { fontSize: 12.5 } }} />
      <TopBar
        conversations={conversations}
        onNewChat={handleNewChat}
        onOpenFlowEditor={() => openExtraTab("flow")}
      />

      <div className="shell-body">
        <SidebarShell
          conversations={conversations}
          activeConversationId={activeConversationId}
          onSelect={selectConversation}
          onDelete={handleDeleteConversation}
        />

        <div className="main">
          {activeExtra?.type === "text" && (
            <div className="main-header">
              <span className="meta">{activeExtra.title}</span>
              <div className="header-toolbar">
                <IconButton icon={IconBold} iconSize={14} aria-label="Bold" />
                <IconButton icon={IconItalic} iconSize={14} aria-label="Italic" />
                <IconButton icon={IconUnderline} iconSize={14} aria-label="Underline" />
              </div>
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
            ) : activeExtra?.type === "flow" ? (
              <div className="view active">
                <Suspense fallback={null}>
                  <FlowEditor cloudModels={cloudModels} providers={providers} localModels={localModels} />
                </Suspense>
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

      <Suspense fallback={null}>
        <CommandPalette
          conversations={conversations}
          localModels={localModels}
          onSelectConversation={selectConversation}
          onNewChat={handleNewChat}
        />
      </Suspense>
      <OnboardingOverlay />
      <Suspense fallback={null}>
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
      </Suspense>
    </div>
  );
}

export default App;
