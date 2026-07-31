import { useState } from "react";
import "./App.css";
import Sidebar from "./components/Sidebar";
import ChatPanel from "./components/ChatPanel";
import SettingsModal from "./components/SettingsModal";
import StatsBar from "./components/StatsBar";
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
  const [showSettings, setShowSettings] = useState(false);
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
  const refreshMcpServers = useInvokeResource(
    "list_mcp_servers",
    setMcpServers,
    setBackgroundError,
  );

  return (
    <main className="flex h-screen w-screen flex-col bg-neutral-950 text-neutral-100">
      <StatsBar stats={stats} loadedLocalModels={localModels.filter((m) => m.loaded)} />
      {backgroundError && (
        <div className="flex items-center justify-between border-b border-red-900 bg-red-950/40 px-4 py-1.5 text-xs text-red-400">
          <span>{backgroundError}</span>
          <button onClick={() => setBackgroundError(null)} className="underline">
            Dismiss
          </button>
        </div>
      )}
      <div className="flex flex-1 overflow-hidden">
        <Sidebar
          conversations={conversations}
          activeConversationId={activeConversationId}
          onSelect={setActiveConversationId}
          onNewChat={handleNewChat}
          onOpenSettings={() => setShowSettings(true)}
          onDeleted={handleDeleted}
        />

        {activeConversationId ? (
          <ChatPanel
            key={activeConversationId}
            conversationId={activeConversationId}
            cloudModels={cloudModels}
            providers={providers}
            localModels={localModels}
            refreshLocalModels={refreshLocalModels}
            onTurnComplete={refreshConversations}
          />
        ) : (
          <div className="flex flex-1 items-center justify-center text-neutral-500">
            Start a new chat to begin.
          </div>
        )}

        {showSettings && (
          <SettingsModal
            onClose={() => setShowSettings(false)}
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
        )}
      </div>
    </main>
  );
}

export default App;
