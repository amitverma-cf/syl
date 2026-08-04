import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { toast } from "sonner";
import { IconDownload, IconTrash, IconPencil } from "@tabler/icons-react";
import { Button, IconButton, Input, Badge } from "../ui";
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

  const [editingProvider, setEditingProvider] = useState<string | null>(null);
  const [editBaseUrl, setEditBaseUrl] = useState("");
  const [editApiKey, setEditApiKey] = useState("");
  const [savingEdit, setSavingEdit] = useState(false);
  const [removingProvider, setRemovingProvider] = useState<string | null>(null);

  const [deletingKey, setDeletingKey] = useState<string | null>(null);
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

  async function handleDeleteKey(envVar: string) {
    if (!confirm(`Delete the saved ${envVar} key? This can't be undone.`)) return;
    setDeletingKey(envVar);
    setError(null);
    try {
      await invoke("delete_provider_api_key", { envVar });
      refreshProviders();
      toast.success(`${envVar} deleted`);
    } catch (err) {
      setError(String(err));
      toast.error(String(err));
    } finally {
      setDeletingKey(null);
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

  function startEditingProvider(p: CustomProviderConfig) {
    setEditingProvider(p.name);
    setEditBaseUrl(p.baseUrl);
    setEditApiKey("");
  }

  async function handleSaveEditedProvider(e: React.FormEvent) {
    e.preventDefault();
    if (!editingProvider || !editBaseUrl.trim()) return;
    setSavingEdit(true);
    setError(null);
    try {
      await invoke("update_custom_provider", {
        name: editingProvider,
        baseUrl: editBaseUrl.trim(),
        apiKey: editApiKey.trim() || null,
      });
      setEditingProvider(null);
      refreshCustomProviders();
      toast.success(`Provider ${editingProvider} updated`);
    } catch (err) {
      setError(String(err));
      toast.error(String(err));
    } finally {
      setSavingEdit(false);
    }
  }

  async function handleRemoveCustomProvider(name: string) {
    if (!confirm(`Remove provider ${name} and its stored API key? This can't be undone.`)) return;
    setRemovingProvider(name);
    setError(null);
    try {
      await invoke("remove_custom_provider", { name });
      refreshCustomProviders();
      toast.success(`Provider ${name} removed`);
    } catch (err) {
      setError(String(err));
      toast.error(String(err));
    } finally {
      setRemovingProvider(null);
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
          <Input
            type="password"
            value={keyDrafts[p.envVar] ?? ""}
            onChange={(e) => {
              const value = e.currentTarget.value;
              setKeyDrafts((prev) => ({ ...prev, [p.envVar]: value }));
            }}
            placeholder={p.configured ? "Key saved — enter to replace" : "API key"}
          />
          <Button onClick={() => handleSaveKey(p.envVar)} disabled={savingProvider === p.envVar || !keyDrafts[p.envVar]}>
            Save
          </Button>
          {p.configured && <Badge>configured</Badge>}
          {p.configured && (
            <IconButton
              icon={IconTrash}
              iconSize={14}
              variant="danger"
              title="Delete key"
              onClick={() => deletingKey !== p.envVar && handleDeleteKey(p.envVar)}
              style={{ opacity: deletingKey === p.envVar ? 0.5 : 1 }}
            />
          )}
        </div>
      ))}

      <div className="settings-section-title">Custom OpenAI-compatible providers</div>
      {customProviders.length === 0 && (
        <p style={{ fontSize: 12, color: "var(--text-3)", margin: "0 0 8px" }}>
          No custom providers yet.
        </p>
      )}
      {customProviders.map((p) =>
        editingProvider === p.name ? (
          <form key={p.name} onSubmit={handleSaveEditedProvider} className="form-row">
            <span style={{ width: 110, flexShrink: 0, fontSize: 12, color: "rgba(255,255,255,.7)" }}>
              {p.name}
            </span>
            <Input
              value={editBaseUrl}
              onChange={(e) => setEditBaseUrl(e.currentTarget.value)}
              placeholder="Base URL"
            />
            <Input
              type="password"
              value={editApiKey}
              onChange={(e) => setEditApiKey(e.currentTarget.value)}
              placeholder="API key (leave blank to keep current)"
              style={{ flex: "0 0 140px" }}
            />
            <Button type="submit" disabled={savingEdit}>
              {savingEdit ? "Saving…" : "Save"}
            </Button>
            <Button type="button" onClick={() => setEditingProvider(null)}>
              Cancel
            </Button>
          </form>
        ) : (
          <div key={p.name} className="model-row">
            <div>
              <div className="name">{p.name}</div>
              <div className="kind">
                {p.baseUrl} · {p.models.length} models
              </div>
            </div>
            <div style={{ display: "flex", alignItems: "center", gap: 6, flexShrink: 0 }}>
              <IconButton
                icon={IconPencil}
                iconSize={14}
                title="Edit"
                onClick={() => startEditingProvider(p)}
              />
              <IconButton
                icon={IconTrash}
                iconSize={14}
                variant="danger"
                title="Remove"
                onClick={() => removingProvider !== p.name && handleRemoveCustomProvider(p.name)}
                style={{ opacity: removingProvider === p.name ? 0.5 : 1 }}
              />
            </div>
          </div>
        ),
      )}
      <form onSubmit={handleAddCustomProvider} className="form-row">
        <Input
          value={newProviderName}
          onChange={(e) => setNewProviderName(e.currentTarget.value)}
          placeholder="Name"
          style={{ flex: "0 0 110px" }}
        />
        <Input
          value={newProviderUrl}
          onChange={(e) => setNewProviderUrl(e.currentTarget.value)}
          placeholder="Base URL"
        />
        <Input
          type="password"
          value={newProviderKey}
          onChange={(e) => setNewProviderKey(e.currentTarget.value)}
          placeholder="API key (optional)"
          style={{ flex: "0 0 140px" }}
        />
        <Button type="submit" disabled={addingProvider}>
          {addingProvider ? "Fetching…" : "Add"}
        </Button>
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
            <IconButton
              icon={IconTrash}
              iconSize={14}
              variant="danger"
              title="Delete"
              onClick={() => deletingModel !== m.name && handleDeleteModel(m.name)}
              style={{ opacity: deletingModel === m.name ? 0.5 : 1 }}
            />
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
            <Badge>downloaded</Badge>
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
