import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { toast } from "sonner";
import { IconDownload, IconTrash } from "@tabler/icons-react";
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
      toast.success(`${envVar} saved`);
    } catch (err) {
      setError(String(err));
      toast.error(String(err));
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
      toast.success(`Provider ${newProviderName.trim()} added`);
    } catch (err) {
      setError(String(err));
      toast.error(String(err));
    } finally {
      setAddingProvider(false);
    }
  }

  async function handleSetKind(name: string, kind: "chat" | "embedding" | "image" | "asr" | "tts") {
    setError(null);
    try {
      await invoke("set_local_model_kind", { name, kind });
      refreshLocalModels();
      toast.success(`${name} marked as ${kind}`);
    } catch (err) {
      setError(String(err));
      toast.error(String(err));
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
      toast.success(`${name} deleted`);
    } catch (err) {
      setError(String(err));
      toast.error(String(err));
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
      toast.success(`${name} downloaded and verified`);
    } catch (err) {
      setError(String(err));
      toast.error(String(err));
    } finally {
      setDownloadingModel(null);
    }
  }

  const kindOptions: Array<"chat" | "embedding" | "image" | "asr" | "tts"> = [
    "chat",
    "embedding",
    "image",
    "asr",
    "tts",
  ];

  return (
    <div>
      <div className="settings-section-title">Cloud provider API keys</div>
      {providers.map((p) => (
        <div key={p.envVar} className="form-row">
          <span style={{ width: 90, flexShrink: 0, fontSize: 12, color: "rgba(255,255,255,.7)" }}>
            {p.name}
          </span>
          <input
            type="password"
            value={keyDrafts[p.envVar] ?? ""}
            onChange={(e) => {
              const value = e.currentTarget.value;
              setKeyDrafts((prev) => ({ ...prev, [p.envVar]: value }));
            }}
            placeholder={p.configured ? "Key saved — enter to replace" : "API key"}
            className="form-input"
          />
          <button
            onClick={() => handleSaveKey(p.envVar)}
            disabled={savingProvider === p.envVar || !keyDrafts[p.envVar]}
            className="form-btn"
          >
            Save
          </button>
          {p.configured && <span className="pill">configured</span>}
        </div>
      ))}

      <div className="settings-section-title">Custom OpenAI-compatible providers</div>
      {customProviders.length === 0 && (
        <p style={{ fontSize: 12, color: "var(--text-3)", margin: "0 0 8px" }}>
          No custom providers yet.
        </p>
      )}
      {customProviders.map((p) => (
        <div key={p.name} className="model-row">
          <div>
            <div className="name">{p.name}</div>
            <div className="kind">
              {p.baseUrl} · {p.models.length} models
            </div>
          </div>
        </div>
      ))}
      <form onSubmit={handleAddCustomProvider} className="form-row">
        <input
          value={newProviderName}
          onChange={(e) => setNewProviderName(e.currentTarget.value)}
          placeholder="Name"
          className="form-input"
          style={{ flex: "0 0 110px" }}
        />
        <input
          value={newProviderUrl}
          onChange={(e) => setNewProviderUrl(e.currentTarget.value)}
          placeholder="Base URL"
          className="form-input"
        />
        <input
          type="password"
          value={newProviderKey}
          onChange={(e) => setNewProviderKey(e.currentTarget.value)}
          placeholder="API key (optional)"
          className="form-input"
          style={{ flex: "0 0 140px" }}
        />
        <button type="submit" disabled={addingProvider} className="form-btn">
          {addingProvider ? "Fetching…" : "Add"}
        </button>
      </form>

      <div className="settings-section-title">Local models</div>
      <p style={{ fontSize: 11.5, color: "var(--text-3)", margin: "0 0 8px", lineHeight: 1.5 }}>
        GGUF files found in <code>.syl/models</code>. Load/unload from the composer's model picker
        — this list is for categorizing and removing files.
      </p>
      {localModels.length === 0 && (
        <p style={{ fontSize: 12, color: "var(--text-3)" }}>No .gguf files found.</p>
      )}
      {localModels.map((m) => (
        <div key={m.name} className="model-row">
          <div>
            <div className="name">{m.name}</div>
            <div className="kind">
              {formatBytes(m.sizeBytes)}
              {m.kind && ` · ${m.kind}`}
              {m.loaded && " · loaded"}
            </div>
          </div>
          <div style={{ display: "flex", alignItems: "center", gap: 6, flexShrink: 0 }}>
            {m.kind === null && (
              <div className="kind-btn-row">
                {kindOptions.map((k) => (
                  <span key={k} className="kind-btn" onClick={() => handleSetKind(m.name, k)}>
                    {k}
                  </span>
                ))}
              </div>
            )}
            <span
              className="header-icon-btn"
              title="Delete"
              onClick={() => deletingModel !== m.name && handleDeleteModel(m.name)}
              style={{ opacity: deletingModel === m.name ? 0.5 : 1, color: "#ff8783" }}
            >
              <IconTrash size={14} aria-hidden />
            </span>
          </div>
        </div>
      ))}

      <div className="settings-section-title">Browse catalog</div>
      {catalogModels.length === 0 && (
        <p style={{ fontSize: 12, color: "var(--text-3)" }}>No models in the registry yet.</p>
      )}
      {catalogModels.map((m) => (
        <div key={m.name} className="catalog-row">
          <div>
            <div className="name">{m.name}</div>
            <div className="kind">
              {m.kind} · {m.quantization} · {formatBytes(m.sizeBytes)}
              {!m.fitsInAvailableMemory && " · may not fit in available RAM"}
            </div>
          </div>
          {m.alreadyDownloaded ? (
            <span className="pill">downloaded</span>
          ) : (
            <div
              className="catalog-download-btn"
              onClick={() => downloadingModel !== m.name && handleDownloadModel(m.name)}
              style={{ opacity: downloadingModel === m.name ? 0.6 : 1 }}
            >
              <IconDownload size={13} aria-hidden />
              {downloadingModel === m.name ? "Downloading…" : "Download"}
            </div>
          )}
        </div>
      ))}

      {error && <p className="settings-error">{error}</p>}
    </div>
  );
}

export default AiProvidersTab;
