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
    <div>
      <div className="settings-section-title" style={{ marginTop: 0 }}>
        Registered tools
      </div>
      <p style={{ fontSize: 11.5, color: "var(--text-3)", margin: "0 0 10px", lineHeight: 1.5 }}>
        Every tool the active flow can hand to a model — native tools plus anything discovered
        from connected MCP servers. Permission prompts appear in the chat when a tool requiring
        approval is called.
      </p>
      {tools.length === 0 && <p style={{ fontSize: 12, color: "var(--text-3)" }}>No tools registered.</p>}
      {tools.map((t) => (
        <div key={t.name} className="model-row" style={{ flexDirection: "column", alignItems: "stretch" }}>
          <div className="name">{t.name}</div>
          <div className="kind" style={{ marginTop: 3 }}>
            {t.description}
          </div>
        </div>
      ))}
      {error && <p className="settings-error">{error}</p>}
    </div>
  );
}

export default ToolsTab;
