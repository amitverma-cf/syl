import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import "./App.css";
import Sidebar from "./components/Sidebar";
import ChatPanel from "./components/ChatPanel";
import SettingsModal from "./components/SettingsModal";
import type {
  CatalogModel,
  CloudModel,
  ConversationSummary,
  CustomProviderConfig,
  McpServerConfig,
  ProviderInfo,
  SystemStats,
} from "./types";

function App() {
  const [conversations, setConversations] = useState<ConversationSummary[]>([]);
  const [activeConversationId, setActiveConversationId] = useState<string | null>(null);
  const [showSettings, setShowSettings] = useState(false);

  const [providers, setProviders] = useState<ProviderInfo[]>([]);
  const [cloudModels, setCloudModels] = useState<CloudModel[]>([]);
  const [customProviders, setCustomProviders] = useState<CustomProviderConfig[]>([]);
  const [models, setModels] = useState<CatalogModel[]>([]);
  const [mcpServers, setMcpServers] = useState<McpServerConfig[]>([]);
  const [stats, setStats] = useState<SystemStats | null>(null);

  function refreshConversations() {
    invoke<ConversationSummary[]>("list_conversations")
      .then((list) => {
        setConversations(list);
        if (!activeConversationId && list.length > 0) {
          setActiveConversationId(list[0].id);
        }
      })
      .catch(() => {});
  }

  useEffect(refreshConversations, []);

  function refreshProviders() {
    invoke<ProviderInfo[]>("list_providers").then(setProviders).catch(() => {});
  }
  function refreshCloudModels() {
    invoke<CloudModel[]>("list_cloud_models").then(setCloudModels).catch(() => {});
  }
  function refreshCustomProviders() {
    invoke<CustomProviderConfig[]>("list_custom_providers").then(setCustomProviders).catch(() => {});
  }
  function refreshModels() {
    invoke<CatalogModel[]>("list_available_models").then(setModels).catch(() => {});
  }
  function refreshMcpServers() {
    invoke<McpServerConfig[]>("list_mcp_servers").then(setMcpServers).catch(() => {});
  }

  useEffect(() => {
    refreshProviders();
    refreshCloudModels();
    refreshCustomProviders();
    refreshModels();
    refreshMcpServers();
  }, []);

  useEffect(() => {
    function poll() {
      invoke<SystemStats>("system_stats").then(setStats).catch(() => {});
    }
    poll();
    const interval = setInterval(poll, 5000);
    return () => clearInterval(interval);
  }, []);

  async function handleNewChat() {
    const id = crypto.randomUUID();
    try {
      await invoke("create_conversation", { id, title: "New chat" });
      setActiveConversationId(id);
      refreshConversations();
    } catch {
      // ignore — conversation creation failing surfaces via the chat panel on next interaction
    }
  }

  return (
    <main className="flex h-screen w-screen bg-neutral-950 text-neutral-100">
      <Sidebar
        conversations={conversations}
        activeConversationId={activeConversationId}
        onSelect={setActiveConversationId}
        onNewChat={handleNewChat}
        onOpenSettings={() => setShowSettings(true)}
      />

      {activeConversationId ? (
        <ChatPanel
          key={activeConversationId}
          conversationId={activeConversationId}
          cloudModels={cloudModels}
          providers={providers}
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
          mcpServers={mcpServers}
          refreshMcpServers={refreshMcpServers}
          stats={stats}
          conversations={conversations}
        />
      )}
    </main>
  );
}

export default App;
