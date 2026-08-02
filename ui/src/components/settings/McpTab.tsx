import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { toast } from "sonner";
import { IconTrash } from "@tabler/icons-react";
import type { McpServerConfig, McpToolDescriptor, McpTransportConfig } from "../../types";

interface McpTabProps {
  mcpServers: McpServerConfig[];
  refresh: () => void;
}

function McpTab({ mcpServers, refresh }: McpTabProps) {
  const [transport, setTransport] = useState<"stdio" | "http">("stdio");
  const [name, setName] = useState("");
  const [command, setCommand] = useState("");
  const [args, setArgs] = useState("");
  const [url, setUrl] = useState("");
  const [bearerToken, setBearerToken] = useState("");
  const [adding, setAdding] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [tools, setTools] = useState<Record<string, McpToolDescriptor[]>>({});

  async function handleAdd(e: React.FormEvent) {
    e.preventDefault();
    if (!name.trim()) return;
    if (transport === "stdio" && !command.trim()) return;
    if (transport === "http" && !url.trim()) return;
    setAdding(true);
    setError(null);
    try {
      const transportConfig: McpTransportConfig =
        transport === "stdio"
          ? {
              transport: "stdio",
              command: command.trim(),
              args: args.trim().length > 0 ? args.trim().split(/\s+/) : [],
            }
          : { transport: "http", url: url.trim(), bearerToken: bearerToken.trim() || null };
      const discovered = await invoke<McpToolDescriptor[]>("add_mcp_server", {
        name: name.trim(),
        transport: transportConfig,
      });
      setTools((prev) => ({ ...prev, [name.trim()]: discovered }));
      setName("");
      setCommand("");
      setArgs("");
      setUrl("");
      setBearerToken("");
      refresh();
      toast.success(`${name.trim()} connected`);
    } catch (err) {
      setError(String(err));
      toast.error(String(err));
    } finally {
      setAdding(false);
    }
  }

  async function handleRemove(serverName: string) {
    setError(null);
    try {
      await invoke("remove_mcp_server", { name: serverName });
      setTools((prev) => {
        const next = { ...prev };
        delete next[serverName];
        return next;
      });
      refresh();
      toast.success(`${serverName} removed`);
    } catch (err) {
      setError(String(err));
      toast.error(String(err));
    }
  }

  return (
    <div>
      <div className="settings-section-title">Connected servers</div>
      {mcpServers.length === 0 && (
        <p style={{ fontSize: 12, color: "var(--text-3)", margin: "0 0 8px" }}>
          No MCP servers connected yet.
        </p>
      )}
      {mcpServers.map((s) => (
        <div key={s.name} className="model-row" style={{ flexDirection: "column", alignItems: "stretch" }}>
          <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between" }}>
            <div>
              <div className="name">{s.name}</div>
              <div className="kind">
                {s.transport.transport === "stdio"
                  ? `${s.transport.command} ${s.transport.args.join(" ")}`
                  : s.transport.url}
              </div>
            </div>
            <span className="header-icon-btn" style={{ color: "#ff8783" }} onClick={() => handleRemove(s.name)}>
              <IconTrash size={14} aria-hidden />
            </span>
          </div>
          {tools[s.name] && (
            <div className="kind" style={{ marginTop: 4 }}>
              Tools: {tools[s.name].map((t) => t.name).join(", ")}
            </div>
          )}
        </div>
      ))}

      <div className="settings-section-title">Add MCP server</div>
      <form onSubmit={handleAdd} className="form-row">
        <select
          value={transport}
          onChange={(e) => setTransport(e.currentTarget.value as "stdio" | "http")}
          className="form-select"
          style={{ flex: "0 0 120px" }}
        >
          <option value="stdio">stdio</option>
          <option value="http">http</option>
        </select>
        <input
          value={name}
          onChange={(e) => setName(e.currentTarget.value)}
          placeholder="Name"
          className="form-input"
          style={{ flex: "0 0 100px" }}
        />
        {transport === "stdio" ? (
          <>
            <input
              value={command}
              onChange={(e) => setCommand(e.currentTarget.value)}
              placeholder="Command (e.g. npx)"
              className="form-input"
              style={{ flex: "0 0 110px" }}
            />
            <input
              value={args}
              onChange={(e) => setArgs(e.currentTarget.value)}
              placeholder="Args (space-separated)"
              className="form-input"
            />
          </>
        ) : (
          <>
            <input
              value={url}
              onChange={(e) => setUrl(e.currentTarget.value)}
              placeholder="Server URL"
              className="form-input"
            />
            <input
              type="password"
              value={bearerToken}
              onChange={(e) => setBearerToken(e.currentTarget.value)}
              placeholder="Bearer token (optional)"
              className="form-input"
              style={{ flex: "0 0 140px" }}
            />
          </>
        )}
        <button type="submit" disabled={adding} className="form-btn">
          {adding ? "Connecting…" : "Add"}
        </button>
      </form>
      {error && <p className="settings-error">{error}</p>}
    </div>
  );
}

export default McpTab;
