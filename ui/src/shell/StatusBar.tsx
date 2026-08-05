import { useState, type ReactNode } from "react";
import { IconCpu, IconDatabase, IconServer, IconFolderCog, IconGauge } from "@tabler/icons-react";
import { formatBytes, type LocalModelInfo, type SystemStats } from "../types";
import { useShellStore } from "../store/shellStore";

interface StatusBarProps {
  stats: SystemStats | null;
  loadedLocalModels: LocalModelInfo[];
}

interface StatusBarItemConfig {
  id: string;
  testId?: string;
  icon: ReactNode;
  label: string;
  dropdown: ReactNode | null;
}

function StatusBarItem({
  config,
  open,
  onToggle,
}: {
  config: StatusBarItemConfig;
  open: boolean;
  onToggle: () => void;
}) {
  return (
    <>
      <div className="sb-wrap">
        <div
          className="statusbar-item"
          role="button"
          tabIndex={0}
          aria-expanded={open}
          data-testid={config.testId}
          onClick={onToggle}
          onKeyDown={(e) => {
            if (e.key === "Enter" || e.key === " ") {
              e.preventDefault();
              onToggle();
            }
          }}
        >
          {config.icon}
          {config.label}
        </div>
        {open && config.dropdown && (
          <div className="sb-dropdown open" data-testid={config.testId && `${config.testId}-dropdown`}>
            {config.dropdown}
          </div>
        )}
      </div>
      <div className="statusbar-divider" />
    </>
  );
}

function StatusBar({ stats, loadedLocalModels }: StatusBarProps) {
  const [open, setOpen] = useState<string | null>(null);
  const contextUsage = useShellStore((s) => s.contextUsage);
  const activeTab = useShellStore((s) => s.activeTab);
  const extraTabs = useShellStore((s) => s.extraTabs);

  function toggle(key: string) {
    setOpen((cur) => (cur === key ? null : key));
  }

  const activeUsage = activeTab && !extraTabs[activeTab] ? contextUsage[activeTab] : undefined;
  const trackedUsage = Object.entries(contextUsage);

  const items: StatusBarItemConfig[] = [
    {
      id: "context",
      testId: "statusbar-context",
      icon: <IconGauge size={12} aria-hidden />,
      label: activeUsage
        ? `${Math.min(100, Math.round((activeUsage.usedTokens / activeUsage.totalTokens) * 100))}% context`
        : "No active chat",
      dropdown: (
        <>
          {trackedUsage.length === 0 && (
            <div className="sb-row">
              <span>No token usage tracked yet</span>
            </div>
          )}
          {trackedUsage.map(([id, usage]) => (
            <div key={id} style={{ marginBottom: 6 }}>
              <div className="sb-row">
                <span>{usage.modelLabel}</span>
                <span>
                  {usage.usedTokens.toLocaleString()} / {usage.totalTokens.toLocaleString()}
                </span>
              </div>
              <div className="sb-mini-bar">
                <span style={{ width: `${Math.min(100, (usage.usedTokens / usage.totalTokens) * 100)}%` }} />
              </div>
            </div>
          ))}
        </>
      ),
    },
    {
      id: "models",
      icon: <IconServer size={12} aria-hidden />,
      label:
        loadedLocalModels.length > 0 ? `${loadedLocalModels.length} model(s) loaded` : "No model loaded",
      dropdown: (
        <>
          {loadedLocalModels.length === 0 && (
            <div className="sb-row">
              <span>Nothing loaded</span>
            </div>
          )}
          {loadedLocalModels.map((m) => (
            <div className="sb-row" key={m.name}>
              <span>{m.name}</span>
              <span>{formatBytes(m.sizeBytes)}</span>
            </div>
          ))}
        </>
      ),
    },
    {
      id: "cpu",
      icon: <IconCpu size={12} aria-hidden />,
      label: stats ? `${stats.cpuUsagePercent.toFixed(0)}%` : "…",
      dropdown: stats ? (
        <>
          <div className="sb-row">
            <span>CPU usage</span>
            <span>{stats.cpuUsagePercent.toFixed(1)}%</span>
          </div>
          <div className="sb-mini-bar">
            <span style={{ width: `${Math.min(100, stats.cpuUsagePercent)}%` }} />
          </div>
        </>
      ) : null,
    },
    {
      id: "ram",
      icon: <IconDatabase size={12} aria-hidden />,
      label: stats
        ? `${formatBytes(stats.memoryUsedBytes)} / ${formatBytes(stats.memoryTotalBytes)}`
        : "…",
      dropdown: stats ? (
        <>
          <div className="sb-row">
            <span>System RAM</span>
            <span>
              {formatBytes(stats.memoryUsedBytes)} / {formatBytes(stats.memoryTotalBytes)}
            </span>
          </div>
          <div className="sb-mini-bar">
            <span
              style={{ width: `${Math.min(100, (stats.memoryUsedBytes / stats.memoryTotalBytes) * 100)}%` }}
            />
          </div>
          <div className="sb-divider" />
          <div className="sb-row">
            <span>App process</span>
            <span>{formatBytes(stats.processMemoryBytes)}</span>
          </div>
        </>
      ) : null,
    },
    {
      id: "disk",
      icon: <IconFolderCog size={12} aria-hidden />,
      label: stats ? formatBytes(stats.workspaceDiskBytes) : "…",
      dropdown: stats ? (
        <div className="sb-row">
          <span>.syl workspace</span>
          <span>{formatBytes(stats.workspaceDiskBytes)}</span>
        </div>
      ) : null,
    },
  ];

  return (
    <div className="statusbar" onMouseLeave={() => setOpen(null)}>
      {items.map((item) => (
        <StatusBarItem
          key={item.id}
          config={item}
          open={open === item.id}
          onToggle={() => toggle(item.id)}
        />
      ))}
    </div>
  );
}

export default StatusBar;
