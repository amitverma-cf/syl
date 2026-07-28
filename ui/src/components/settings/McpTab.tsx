import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
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
    } catch (err) {
      setError(String(err));
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
    } catch (err) {
      setError(String(err));
    }
  }

  return (
    <div className="flex flex-col gap-2">
      <h2 className="text-sm font-medium">MCP servers</h2>
      <form onSubmit={handleAdd} className="flex flex-wrap items-center gap-2">
        <select
          value={transport}
          onChange={(e) => setTransport(e.currentTarget.value as "stdio" | "http")}
          className="rounded border border-neutral-700 bg-neutral-900 px-2 py-1 text-sm"
        >
          <option value="stdio">Local (stdio)</option>
          <option value="http">Remote (HTTP)</option>
        </select>
        <input
          value={name}
          onChange={(e) => setName(e.currentTarget.value)}
          placeholder="Name"
          className="w-32 rounded border border-neutral-700 bg-neutral-900 px-2 py-1 text-sm"
        />
        {transport === "stdio" ? (
          <>
            <input
              value={command}
              onChange={(e) => setCommand(e.currentTarget.value)}
              placeholder="Command (e.g. npx)"
              className="w-32 rounded border border-neutral-700 bg-neutral-900 px-2 py-1 text-sm"
            />
            <input
              value={args}
              onChange={(e) => setArgs(e.currentTarget.value)}
              placeholder="Args (space-separated)"
              className="flex-1 rounded border border-neutral-700 bg-neutral-900 px-2 py-1 text-sm"
            />
          </>
        ) : (
          <>
            <input
              value={url}
              onChange={(e) => setUrl(e.currentTarget.value)}
              placeholder="Server URL (Streamable HTTP)"
              className="flex-1 rounded border border-neutral-700 bg-neutral-900 px-2 py-1 text-sm"
            />
            <input
              type="password"
              value={bearerToken}
              onChange={(e) => setBearerToken(e.currentTarget.value)}
              placeholder="Bearer token (optional)"
              className="w-40 rounded border border-neutral-700 bg-neutral-900 px-2 py-1 text-sm"
            />
          </>
        )}
        <button
          type="submit"
          disabled={adding}
          className="rounded bg-neutral-100 px-2 py-1 text-xs font-medium text-neutral-950 disabled:opacity-50"
        >
          {adding ? "Connecting..." : "Connect"}
        </button>
      </form>
      {mcpServers.length === 0 && <p className="text-sm text-neutral-500">No MCP servers connected.</p>}
      {mcpServers.map((s) => (
        <div
          key={s.name}
          className="flex flex-col gap-1 rounded border border-neutral-800 px-2 py-1 text-xs text-neutral-400"
        >
          <div className="flex items-center justify-between">
            <span>
              <span className="font-mono text-neutral-200">{s.name}</span> —{" "}
              {s.transport.transport === "stdio"
                ? `${s.transport.command} ${s.transport.args.join(" ")}`
                : s.transport.url}
            </span>
            <button onClick={() => handleRemove(s.name)} className="text-neutral-400 underline">
              Remove
            </button>
          </div>
          {tools[s.name] && <span>Tools: {tools[s.name].map((t) => t.name).join(", ")}</span>}
        </div>
      ))}
      {error && <p className="text-sm text-red-400">{error}</p>}
    </div>
  );
}

export default McpTab;
