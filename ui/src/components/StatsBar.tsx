import { formatBytes, type LocalModelInfo, type SystemStats } from "../types";

interface StatsBarProps {
  stats: SystemStats | null;
  loadedLocalModels: LocalModelInfo[];
}

function StatsBar({ stats, loadedLocalModels }: StatsBarProps) {
  return (
    <div className="flex items-center gap-4 border-b border-neutral-800 bg-neutral-950 px-4 py-1.5 text-xs text-neutral-500">
      {stats ? (
        <>
          <span>CPU {stats.cpuUsagePercent.toFixed(0)}%</span>
          <span>
            RAM {formatBytes(stats.memoryUsedBytes)} / {formatBytes(stats.memoryTotalBytes)}
          </span>
          <span>App {formatBytes(stats.processMemoryBytes)}</span>
          <span>.syl {formatBytes(stats.workspaceDiskBytes)}</span>
        </>
      ) : (
        <span>Loading system stats…</span>
      )}
      {loadedLocalModels.length > 0 && (
        <span>
          Loaded:{" "}
          {loadedLocalModels
            .map((m) => `${m.name} (${formatBytes(m.sizeBytes)})`)
            .join(", ")}
        </span>
      )}
    </div>
  );
}

export default StatsBar;
