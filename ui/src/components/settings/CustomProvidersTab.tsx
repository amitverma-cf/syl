import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { CustomProviderConfig } from "../../types";

interface CustomProvidersTabProps {
  customProviders: CustomProviderConfig[];
  refresh: () => void;
}

function CustomProvidersTab({ customProviders, refresh }: CustomProvidersTabProps) {
  const [name, setName] = useState("");
  const [baseUrl, setBaseUrl] = useState("");
  const [apiKey, setApiKey] = useState("");
  const [adding, setAdding] = useState(false);
  const [error, setError] = useState<string | null>(null);

  async function handleAdd(e: React.FormEvent) {
    e.preventDefault();
    if (!name.trim() || !baseUrl.trim()) return;
    setAdding(true);
    setError(null);
    try {
      await invoke("add_custom_provider", {
        name: name.trim(),
        baseUrl: baseUrl.trim(),
        apiKey: apiKey.trim() || null,
      });
      setName("");
      setBaseUrl("");
      setApiKey("");
      refresh();
    } catch (err) {
      setError(String(err));
    } finally {
      setAdding(false);
    }
  }

  return (
    <div className="flex flex-col gap-2">
      <h2 className="text-sm font-medium">Custom OpenAI-compatible providers</h2>
      <form onSubmit={handleAdd} className="flex flex-wrap gap-2">
        <input
          value={name}
          onChange={(e) => setName(e.currentTarget.value)}
          placeholder="Name (e.g. my-server)"
          className="w-40 rounded border border-neutral-700 bg-neutral-900 px-2 py-1 text-sm"
        />
        <input
          value={baseUrl}
          onChange={(e) => setBaseUrl(e.currentTarget.value)}
          placeholder="Base URL (e.g. http://localhost:1234/v1)"
          className="flex-1 rounded border border-neutral-700 bg-neutral-900 px-2 py-1 text-sm"
        />
        <input
          type="password"
          value={apiKey}
          onChange={(e) => setApiKey(e.currentTarget.value)}
          placeholder="API key (optional)"
          className="w-40 rounded border border-neutral-700 bg-neutral-900 px-2 py-1 text-sm"
        />
        <button
          type="submit"
          disabled={adding}
          className="rounded bg-neutral-100 px-2 py-1 text-xs font-medium text-neutral-950 disabled:opacity-50"
        >
          {adding ? "Fetching models..." : "Add"}
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
      {error && <p className="text-sm text-red-400">{error}</p>}
    </div>
  );
}

export default CustomProvidersTab;
