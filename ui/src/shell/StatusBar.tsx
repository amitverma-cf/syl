import { useState } from "react";
import { IconCpu, IconDatabase, IconServer, IconFolderCog } from "@tabler/icons-react";
import { formatBytes, type LocalModelInfo, type SystemStats } from "../types";

interface StatusBarProps {
  stats: SystemStats | null;
  loadedLocalModels: LocalModelInfo[];
}

function StatusBar({ stats, loadedLocalModels }: StatusBarProps) {
  const [open, setOpen] = useState<string | null>(null);

  function toggle(key: string) {
    setOpen((cur) => (cur === key ? null : key));
  }

  return (
    <div className="statusbar" onMouseLeave={() => setOpen(null)}>
      <div className="sb-wrap">
        <div className="statusbar-item" onClick={() => toggle("models")}>
          <IconServer size={12} aria-hidden />
          {loadedLocalModels.length > 0 ? `${loadedLocalModels.length} model(s) loaded` : "No model loaded"}
        </div>
        {open === "models" && (
          <div className="sb-dropdown open">
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
          </div>
        )}
      </div>

      <div className="statusbar-divider" />

      <div className="sb-wrap">
        <div className="statusbar-item" onClick={() => toggle("cpu")}>
          <IconCpu size={12} aria-hidden />
          {stats ? `${stats.cpuUsagePercent.toFixed(0)}%` : "…"}
        </div>
        {open === "cpu" && stats && (
          <div className="sb-dropdown open">
            <div className="sb-row">
              <span>CPU usage</span>
              <span>{stats.cpuUsagePercent.toFixed(1)}%</span>
            </div>
            <div className="sb-mini-bar">
              <span style={{ width: `${Math.min(100, stats.cpuUsagePercent)}%` }} />
            </div>
          </div>
        )}
      </div>

      <div className="statusbar-divider" />

      <div className="sb-wrap">
        <div className="statusbar-item" onClick={() => toggle("ram")}>
          <IconDatabase size={12} aria-hidden />
          {stats ? `${formatBytes(stats.memoryUsedBytes)} / ${formatBytes(stats.memoryTotalBytes)}` : "…"}
        </div>
        {open === "ram" && stats && (
          <div className="sb-dropdown open">
            <div className="sb-row">
              <span>System RAM</span>
              <span>
                {formatBytes(stats.memoryUsedBytes)} / {formatBytes(stats.memoryTotalBytes)}
              </span>
            </div>
            <div className="sb-mini-bar">
              <span style={{ width: `${Math.min(100, (stats.memoryUsedBytes / stats.memoryTotalBytes) * 100)}%` }} />
            </div>
            <div className="sb-divider" />
            <div className="sb-row">
              <span>App process</span>
              <span>{formatBytes(stats.processMemoryBytes)}</span>
            </div>
          </div>
        )}
      </div>

      <div className="statusbar-divider" />

      <div className="sb-wrap">
        <div className="statusbar-item" onClick={() => toggle("disk")}>
          <IconFolderCog size={12} aria-hidden />
          {stats ? formatBytes(stats.workspaceDiskBytes) : "…"}
        </div>
        {open === "disk" && stats && (
          <div className="sb-dropdown open">
            <div className="sb-row">
              <span>.syl workspace</span>
              <span>{formatBytes(stats.workspaceDiskBytes)}</span>
            </div>
          </div>
        )}
      </div>
    </div>
  );
}

export default StatusBar;
