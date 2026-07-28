import { useState } from "react";
import type {
  CatalogModel,
  CloudModel,
  ConversationSummary,
  CustomProviderConfig,
  McpServerConfig,
  ProviderInfo,
  SystemStats,
} from "../types";
import ProvidersTab from "./settings/ProvidersTab";
import CustomProvidersTab from "./settings/CustomProvidersTab";
import ModelsTab from "./settings/ModelsTab";
import McpTab from "./settings/McpTab";
import MemoryTab from "./settings/MemoryTab";
import ToolsTab from "./settings/ToolsTab";
import ScheduledJobsTab from "./settings/ScheduledJobsTab";
import FlowTemplatesTab from "./settings/FlowTemplatesTab";

const TABS = [
  "Providers & Keys",
  "Local Engines & Models",
  "Custom Providers",
  "MCP Servers",
  "Memory",
  "Tools",
  "Scheduled Jobs",
  "Flow Templates",
] as const;
type Tab = (typeof TABS)[number];

interface SettingsModalProps {
  onClose: () => void;
  activeConversationId: string | null;
  providers: ProviderInfo[];
  refreshProviders: () => void;
  customProviders: CustomProviderConfig[];
  refreshCustomProviders: () => void;
  cloudModels: CloudModel[];
  models: CatalogModel[];
  refreshModels: () => void;
  mcpServers: McpServerConfig[];
  refreshMcpServers: () => void;
  stats: SystemStats | null;
  conversations: ConversationSummary[];
}

function SettingsModal(props: SettingsModalProps) {
  const [tab, setTab] = useState<Tab>("Providers & Keys");

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/60">
      <div className="flex h-[80vh] w-[80vw] max-w-4xl overflow-hidden rounded border border-neutral-800 bg-neutral-950">
        <aside className="w-56 shrink-0 border-r border-neutral-800 p-2">
          {TABS.map((t) => (
            <button
              key={t}
              onClick={() => setTab(t)}
              className={`mb-1 w-full rounded px-3 py-2 text-left text-sm ${
                t === tab
                  ? "bg-neutral-800 text-neutral-100"
                  : "text-neutral-400 hover:bg-neutral-900 hover:text-neutral-200"
              }`}
            >
              {t}
            </button>
          ))}
        </aside>

        <div className="flex-1 overflow-auto p-4">
          <div className="mb-3 flex items-center justify-between">
            <h1 className="text-lg font-semibold">Settings</h1>
            <button onClick={props.onClose} className="text-sm text-neutral-400 underline">
              Close
            </button>
          </div>

          {tab === "Providers & Keys" && (
            <ProvidersTab providers={props.providers} refresh={props.refreshProviders} />
          )}
          {tab === "Local Engines & Models" && (
            <ModelsTab models={props.models} refresh={props.refreshModels} />
          )}
          {tab === "Custom Providers" && (
            <CustomProvidersTab
              customProviders={props.customProviders}
              refresh={props.refreshCustomProviders}
            />
          )}
          {tab === "MCP Servers" && (
            <McpTab mcpServers={props.mcpServers} refresh={props.refreshMcpServers} />
          )}
          {tab === "Memory" && (
            <MemoryTab stats={props.stats} conversations={props.conversations} />
          )}
          {tab === "Tools" && <ToolsTab />}
          {tab === "Scheduled Jobs" && <ScheduledJobsTab cloudModels={props.cloudModels} />}
          {tab === "Flow Templates" && (
            <FlowTemplatesTab activeConversationId={props.activeConversationId} />
          )}
        </div>
      </div>
    </div>
  );
}

export default SettingsModal;
