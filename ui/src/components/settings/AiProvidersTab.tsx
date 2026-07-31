import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import {
  formatBytes,
  type CatalogModel,
  type CustomProviderConfig,
  type LocalModelInfo,
  type ProviderInfo,
} from "../../types";

interface AiProvidersTabProps {
  providers: ProviderInfo[];
  refreshProviders: () => void;
  customProviders: CustomProviderConfig[];
  refreshCustomProviders: () => void;
  catalogModels: CatalogModel[];
  refreshCatalogModels: () => void;
  localModels: LocalModelInfo[];
  refreshLocalModels: () => void;
}

function AiProvidersTab({
  providers,
  refreshProviders,
  customProviders,
  refreshCustomProviders,
  catalogModels,
  refreshCatalogModels,
  localModels,
  refreshLocalModels,
}: AiProvidersTabProps) {
  const [keyDrafts, setKeyDrafts] = useState<Record<string, string>>({});
  const [savingProvider, setSavingProvider] = useState<string | null>(null);

  const [newProviderName, setNewProviderName] = useState("");
  const [newProviderUrl, setNewProviderUrl] = useState("");
  const [newProviderKey, setNewProviderKey] = useState("");
  const [addingProvider, setAddingProvider] = useState(false);

  const [deletingModel, setDeletingModel] = useState<string | null>(null);
  const [downloadingModel, setDownloadingModel] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  async function handleSaveKey(envVar: string) {
    const key = keyDrafts[envVar];
    if (!key) return;
    setSavingProvider(envVar);
    setError(null);
    try {
      await invoke("set_provider_api_key", { envVar, key });
      setKeyDrafts((prev) => ({ ...prev, [envVar]: "" }));
      refreshProviders();
    } catch (err) {
      setError(String(err));
    } finally {
      setSavingProvider(null);
    }
  }

  async function handleAddCustomProvider(e: React.FormEvent) {
    e.preventDefault();
    if (!newProviderName.trim() || !newProviderUrl.trim()) return;
    setAddingProvider(true);
    setError(null);
    try {
      await invoke("add_custom_provider", {
        name: newProviderName.trim(),
        baseUrl: newProviderUrl.trim(),
        apiKey: newProviderKey.trim() || null,
      });
      setNewProviderName("");
      setNewProviderUrl("");
      setNewProviderKey("");
      refreshCustomProviders();
    } catch (err) {
      setError(String(err));
    } finally {
      setAddingProvider(false);
    }
  }

  async function handleSetKind(name: string, kind: "chat" | "embedding" | "image" | "asr" | "tts") {
    setError(null);
    try {
      await invoke("set_local_model_kind", { name, kind });
      refreshLocalModels();
    } catch (err) {
      setError(String(err));
    }
  }

  async function handleDeleteModel(name: string) {
    if (!confirm(`Delete ${name} from .syl/models? This can't be undone.`)) return;
    setDeletingModel(name);
    setError(null);
    try {
      await invoke("delete_local_model", { name });
      refreshLocalModels();
      refreshCatalogModels();
    } catch (err) {
      setError(String(err));
    } finally {
      setDeletingModel(null);
    }
  }

  async function handleDownloadModel(name: string) {
    setDownloadingModel(name);
    setError(null);
    try {
      await invoke("download_model", { name });
      refreshCatalogModels();
      refreshLocalModels();
    } catch (err) {
      setError(String(err));
    } finally {
      setDownloadingModel(null);
    }
  }

  return (
    <div className="flex flex-col gap-6">
      <div className="flex flex-col gap-2">
        <h2 className="text-sm font-medium">Cloud provider API keys</h2>
        {providers.map((p) => (
          <div key={p.envVar} className="flex items-center gap-2">
            <span className="w-28 shrink-0 text-sm">{p.name}</span>
            <input
              type="password"
              value={keyDrafts[p.envVar] ?? ""}
              onChange={(e) =>
                setKeyDrafts((prev) => ({ ...prev, [p.envVar]: e.currentTarget.value }))
              }
              placeholder={p.configured ? "Key saved — enter to replace" : "API key"}
              className="flex-1 rounded border border-neutral-700 bg-neutral-900 px-2 py-1 text-sm"
            />
            <button
              onClick={() => handleSaveKey(p.envVar)}
              disabled={savingProvider === p.envVar || !keyDrafts[p.envVar]}
              className="rounded bg-neutral-100 px-2 py-1 text-xs font-medium text-neutral-950 disabled:opacity-50"
            >
              Save
            </button>
            {p.configured && <span className="text-xs text-green-400">Configured</span>}
          </div>
        ))}
      </div>

      <div className="flex flex-col gap-2">
        <h2 className="text-sm font-medium">Custom OpenAI-compatible providers</h2>
        <form onSubmit={handleAddCustomProvider} className="flex flex-wrap gap-2">
          <input
            value={newProviderName}
            onChange={(e) => setNewProviderName(e.currentTarget.value)}
            placeholder="Name (e.g. nvidia)"
            className="w-40 rounded border border-neutral-700 bg-neutral-900 px-2 py-1 text-sm"
          />
          <input
            value={newProviderUrl}
            onChange={(e) => setNewProviderUrl(e.currentTarget.value)}
            placeholder="Base URL (e.g. https://integrate.api.nvidia.com/v1)"
            className="flex-1 rounded border border-neutral-700 bg-neutral-900 px-2 py-1 text-sm"
          />
          <input
            type="password"
            value={newProviderKey}
            onChange={(e) => setNewProviderKey(e.currentTarget.value)}
            placeholder="API key (optional)"
            className="w-40 rounded border border-neutral-700 bg-neutral-900 px-2 py-1 text-sm"
          />
          <button
            type="submit"
            disabled={addingProvider}
            className="rounded bg-neutral-100 px-2 py-1 text-xs font-medium text-neutral-950 disabled:opacity-50"
          >
            {addingProvider ? "Fetching models..." : "Add"}
          </button>
        </form>
        {customProviders.length === 0 && (
          <p className="text-sm text-neutral-500">No custom providers yet.</p>
        )}
        {customProviders.map((p) => (
          <div
            key={p.name}
            className="flex items-center justify-between rounded border border-neutral-800 px-2 py-1 text-xs text-neutral-400"
          >
            <span>
              <span className="font-mono text-neutral-200">{p.name}</span> — {p.baseUrl} ·{" "}
              {p.models.length} models
            </span>
          </div>
        ))}
      </div>

      <div className="flex flex-col gap-2">
        <div className="flex items-center justify-between">
          <h2 className="text-sm font-medium">Local models</h2>
          <button onClick={refreshLocalModels} className="text-xs text-neutral-400 underline">
            Refresh
          </button>
        </div>
        <p className="text-xs text-neutral-500">
          GGUF files found in <span className="font-mono">.syl/models</span>, categorized by
          <span className="font-mono"> .syl/registry/models.json</span>. Load/unload a model from
          the model dropdown in chat, not here — this tab is for categorizing and removing files.
        </p>
        {localModels.length === 0 && (
          <p className="text-sm text-neutral-500">No .gguf files found.</p>
        )}
        {localModels.map((m) => (
          <div
            key={m.name}
            className="flex items-center justify-between rounded border border-neutral-800 px-3 py-2 text-sm"
          >
            <div>
              <span className="font-mono">{m.name}</span>
              <span className="ml-2 text-xs text-neutral-500">
                {formatBytes(m.sizeBytes)}
                {m.kind && ` · ${m.kind}`}
                {m.loaded && " · loaded"}
              </span>
            </div>
            <div className="flex items-center gap-2">
              {m.kind === null && (
                <>
                  <span className="text-xs text-amber-400">Uncategorized</span>
                  <button
                    onClick={() => handleSetKind(m.name, "chat")}
                    className="rounded border border-neutral-700 px-2 py-1 text-xs text-neutral-400"
                  >
                    Mark as chat
                  </button>
                  <button
                    onClick={() => handleSetKind(m.name, "embedding")}
                    className="rounded border border-neutral-700 px-2 py-1 text-xs text-neutral-400"
                  >
                    Mark as embedding
                  </button>
                  <button
                    onClick={() => handleSetKind(m.name, "image")}
                    className="rounded border border-neutral-700 px-2 py-1 text-xs text-neutral-400"
                  >
                    Mark as image
                  </button>
                  <button
                    onClick={() => handleSetKind(m.name, "asr")}
                    className="rounded border border-neutral-700 px-2 py-1 text-xs text-neutral-400"
                  >
                    Mark as ASR
                  </button>
                  <button
                    onClick={() => handleSetKind(m.name, "tts")}
                    className="rounded border border-neutral-700 px-2 py-1 text-xs text-neutral-400"
                  >
                    Mark as TTS
                  </button>
                </>
              )}
              <button
                onClick={() => handleDeleteModel(m.name)}
                disabled={deletingModel === m.name}
                className="rounded border border-red-900 px-2 py-1 text-xs text-red-400 disabled:opacity-50"
              >
                {deletingModel === m.name ? "Deleting..." : "Delete"}
              </button>
            </div>
          </div>
        ))}
      </div>

      <div className="flex flex-col gap-2">
        <div className="flex items-center justify-between">
          <h2 className="text-sm font-medium">Download more models</h2>
          <button onClick={refreshCatalogModels} className="text-xs text-neutral-400 underline">
            Refresh
          </button>
        </div>
        {catalogModels.length === 0 && (
          <p className="text-sm text-neutral-500">No models in the registry yet.</p>
        )}
        {catalogModels.map((m) => (
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
                onClick={() => handleDownloadModel(m.name)}
                disabled={downloadingModel === m.name}
                className="rounded bg-neutral-100 px-2 py-1 text-xs font-medium text-neutral-950 disabled:opacity-50"
              >
                {downloadingModel === m.name ? "Downloading..." : "Download"}
              </button>
            )}
          </div>
        ))}
      </div>

      {error && <p className="text-sm text-red-400">{error}</p>}
    </div>
  );
}

export default AiProvidersTab;
