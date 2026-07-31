import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { ToolSpec } from "../../types";

function ToolsTab() {
  const [tools, setTools] = useState<ToolSpec[]>([]);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    invoke<ToolSpec[]>("list_tools")
      .then(setTools)
      .catch((err) => setError(String(err)));
  }, []);

  return (
    <div className="flex flex-col gap-2">
      <h2 className="text-sm font-medium">Registered tools</h2>
      <p className="text-xs text-neutral-500">
        Every tool the active flow can hand to a model — native tools plus anything discovered
        from connected MCP servers. Permission prompts appear in the chat when a tool requiring
        approval is called.
      </p>
      {tools.length === 0 && <p className="text-sm text-neutral-500">No tools registered.</p>}
      {tools.map((t) => (
        <div key={t.name} className="rounded border border-neutral-800 px-3 py-2 text-sm">
          <span className="font-mono text-neutral-200">{t.name}</span>
          <p className="mt-1 text-xs text-neutral-400">{t.description}</p>
        </div>
      ))}
      {error && <p className="text-sm text-red-400">{error}</p>}
    </div>
  );
}

export default ToolsTab;
