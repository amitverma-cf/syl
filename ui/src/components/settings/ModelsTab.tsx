import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { formatBytes, type CatalogModel } from "../../types";

interface ModelsTabProps {
  models: CatalogModel[];
  refresh: () => void;
}

function ModelsTab({ models, refresh }: ModelsTabProps) {
  const [downloading, setDownloading] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  async function handleDownload(name: string) {
    setDownloading(name);
    setError(null);
    try {
      await invoke("download_model", { name });
      refresh();
    } catch (err) {
      setError(String(err));
    } finally {
      setDownloading(null);
    }
  }

  return (
    <div className="flex flex-col gap-2">
      <div className="flex items-center justify-between">
        <h2 className="text-sm font-medium">Local engines & models</h2>
        <button onClick={refresh} className="text-xs text-neutral-400 underline">
          Refresh
        </button>
      </div>
      {models.length === 0 && <p className="text-sm text-neutral-500">No models in the registry yet.</p>}
      {models.map((m) => (
        <div
          key={m.name}
          className="flex items-center justify-between rounded border border-neutral-800 px-3 py-2 text-sm"
        >
          <div>
            <span className="font-mono">{m.name}</span>
            <span className="ml-2 text-xs text-neutral-500">
              {m.kind} · {m.quantization} · {formatBytes(m.sizeBytes)}
              {!m.fitsInAvailableMemory && " · may not fit in available RAM"}
            </span>
          </div>
          {m.alreadyDownloaded ? (
            <span className="text-xs text-green-400">Downloaded</span>
          ) : (
            <button
              onClick={() => handleDownload(m.name)}
              disabled={downloading === m.name}
              className="rounded bg-neutral-100 px-2 py-1 text-xs font-medium text-neutral-950 disabled:opacity-50"
            >
              {downloading === m.name ? "Downloading..." : "Download"}
            </button>
          )}
        </div>
      ))}
      {error && <p className="text-sm text-red-400">{error}</p>}
    </div>
  );
}

export default ModelsTab;
