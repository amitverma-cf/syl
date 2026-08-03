import {
  IconBox,
  IconPlug,
  IconTools,
  IconBrain,
  IconClock,
  IconTopologyStar3,
  IconX,
} from "@tabler/icons-react";
import { useShellStore } from "../store/shellStore";
import { Overlay, IconButton, NavItem } from "../components/ui";
import type {
  CatalogModel,
  CloudModel,
  ConversationSummary,
  CustomProviderConfig,
  LocalModelInfo,
  McpServerConfig,
  ProviderInfo,
  SystemStats,
} from "../types";
import AiProvidersTab from "../components/settings/AiProvidersTab";
import McpTab from "../components/settings/McpTab";
import MemoryTab from "../components/settings/MemoryTab";
import ToolsTab from "../components/settings/ToolsTab";
import ScheduledJobsTab from "../components/settings/ScheduledJobsTab";
import FlowTemplatesTab from "../components/settings/FlowTemplatesTab";

const PANES = [
  { key: "models", label: "AI Providers & Models", icon: IconBox },
  { key: "mcp", label: "MCP Servers", icon: IconPlug },
  { key: "memory", label: "Memory", icon: IconBrain },
  { key: "tools", label: "Tools", icon: IconTools },
  { key: "jobs", label: "Scheduled Jobs", icon: IconClock },
  { key: "flows", label: "Flow Templates", icon: IconTopologyStar3 },
] as const;

interface SettingsOverlayProps {
  activeConversationId: string | null;
  providers: ProviderInfo[];
  refreshProviders: () => void;
  customProviders: CustomProviderConfig[];
  refreshCustomProviders: () => void;
  cloudModels: CloudModel[];
  models: CatalogModel[];
  refreshModels: () => void;
  localModels: LocalModelInfo[];
  refreshLocalModels: () => void;
  mcpServers: McpServerConfig[];
  refreshMcpServers: () => void;
  stats: SystemStats | null;
  conversations: ConversationSummary[];
}

function SettingsOverlay(props: SettingsOverlayProps) {
  const { settingsOpen, settingsPane, openSettings, closeSettings } = useShellStore();

  if (!settingsOpen) return null;

  const activePane = PANES.find((p) => p.key === settingsPane) ?? PANES[0];

  return (
    <Overlay className="settings-overlay open" onClose={closeSettings}>
      <div className="settings-card">
        <div className="settings-nav">
          {PANES.map((p) => (
            <NavItem
              key={p.key}
              icon={p.icon}
              active={p.key === settingsPane}
              className="settings-nav-item"
              onClick={() => openSettings(p.key)}
            >
              {p.label}
            </NavItem>
          ))}
        </div>
        <div className="settings-content">
          <div className="settings-content-head">
            <h2>{activePane.label}</h2>
            <IconButton icon={IconX} onClick={closeSettings} aria-label="Close settings" />
          </div>
          <div className="settings-body">
            {settingsPane === "models" && (
              <AiProvidersTab
                providers={props.providers}
                refreshProviders={props.refreshProviders}
                customProviders={props.customProviders}
                refreshCustomProviders={props.refreshCustomProviders}
                catalogModels={props.models}
                refreshCatalogModels={props.refreshModels}
                localModels={props.localModels}
                refreshLocalModels={props.refreshLocalModels}
              />
            )}
            {settingsPane === "mcp" && (
              <McpTab mcpServers={props.mcpServers} refresh={props.refreshMcpServers} />
            )}
            {settingsPane === "memory" && (
              <MemoryTab stats={props.stats} conversations={props.conversations} />
            )}
            {settingsPane === "tools" && <ToolsTab />}
            {settingsPane === "jobs" && <ScheduledJobsTab cloudModels={props.cloudModels} />}
            {settingsPane === "flows" && (
              <FlowTemplatesTab activeConversationId={props.activeConversationId} />
            )}
          </div>
        </div>
      </div>
    </Overlay>
  );
}

export default SettingsOverlay;
