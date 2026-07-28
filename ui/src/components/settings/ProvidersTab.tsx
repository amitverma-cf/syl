import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { ProviderInfo } from "../../types";

interface ProvidersTabProps {
  providers: ProviderInfo[];
  refresh: () => void;
}

function ProvidersTab({ providers, refresh }: ProvidersTabProps) {
  const [drafts, setDrafts] = useState<Record<string, string>>({});
  const [saving, setSaving] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  async function handleSave(envVar: string) {
    const key = drafts[envVar];
    if (!key) return;
    setSaving(envVar);
    setError(null);
    try {
      await invoke("set_provider_api_key", { envVar, key });
      setDrafts((prev) => ({ ...prev, [envVar]: "" }));
      refresh();
    } catch (err) {
      setError(String(err));
    } finally {
      setSaving(null);
    }
  }

  return (
    <div className="flex flex-col gap-2">
      <h2 className="text-sm font-medium">Cloud provider API keys</h2>
      {providers.map((p) => (
        <div key={p.envVar} className="flex items-center gap-2">
          <span className="w-28 shrink-0 text-sm">{p.name}</span>
          <input
            type="password"
            value={drafts[p.envVar] ?? ""}
            onChange={(e) => setDrafts((prev) => ({ ...prev, [p.envVar]: e.currentTarget.value }))}
            placeholder={p.configured ? "Key saved — enter to replace" : "API key"}
            className="flex-1 rounded border border-neutral-700 bg-neutral-900 px-2 py-1 text-sm"
          />
          <button
            onClick={() => handleSave(p.envVar)}
            disabled={saving === p.envVar || !drafts[p.envVar]}
            className="rounded bg-neutral-100 px-2 py-1 text-xs font-medium text-neutral-950 disabled:opacity-50"
          >
            Save
          </button>
          {p.configured && <span className="text-xs text-green-400">Configured</span>}
        </div>
      ))}
      {error && <p className="text-sm text-red-400">{error}</p>}
    </div>
  );
}

export default ProvidersTab;
