import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { IconX } from "@tabler/icons-react";
import { IconButton, Badge } from "../ui";
import type { ToolSpec, ToolPermissionEntry } from "../../types";

interface ToolsTabProps {
  activeConversationId: string | null;
}

function ToolsTab({ activeConversationId }: ToolsTabProps) {
  const [tools, setTools] = useState<ToolSpec[]>([]);
  const [permissions, setPermissions] = useState<ToolPermissionEntry[]>([]);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    invoke<ToolSpec[]>("list_tools")
      .then(setTools)
      .catch((err) => setError(String(err)));
  }, []);

  function refreshPermissions() {
    const conversationId = activeConversationId;
    Promise.resolve()
      .then(() => (conversationId ? invoke<ToolPermissionEntry[]>("list_tool_permissions", { conversationId }) : []))
      .then(setPermissions)
      .catch((err) => setError(String(err)));
  }

  useEffect(refreshPermissions, [activeConversationId]);

  async function handleRevoke(toolName: string) {
    if (!activeConversationId) return;
    setError(null);
    try {
      await invoke("clear_tool_permission", { conversationId: activeConversationId, toolName });
      refreshPermissions();
    } catch (err) {
      setError(String(err));
    }
  }

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

      <div className="settings-section-title">Remembered permissions (this conversation)</div>
      {!activeConversationId && (
        <p style={{ fontSize: 12, color: "var(--text-3)" }}>Open a conversation to manage its remembered tool permissions.</p>
      )}
      {activeConversationId && permissions.length === 0 && (
        <p style={{ fontSize: 12, color: "var(--text-3)" }}>
          No "Always allow"/"Always deny" decisions remembered yet for this conversation.
        </p>
      )}
      {permissions.map((p) => (
        <div key={p.toolName} className="model-row">
          <div>
            <div className="name">{p.toolName}</div>
          </div>
          <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
            <Badge className={p.decision === "Deny" ? "ui-badge-deny" : undefined}>
              {p.decision === "Allow" ? "always allowed" : "always denied"}
            </Badge>
            <IconButton
              icon={IconX}
              iconSize={13}
              variant="danger"
              title="Forget this decision — the next call will prompt again"
              onClick={() => handleRevoke(p.toolName)}
            />
          </div>
        </div>
      ))}
      {error && <p className="settings-error">{error}</p>}
    </div>
  );
}

export default ToolsTab;
