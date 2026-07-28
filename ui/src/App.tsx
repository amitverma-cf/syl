import { useEffect, useState } from "react";
import { invoke, Channel } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import "./App.css";

type GenerationEvent =
  | { type: "piece"; text: string }
  | { type: "done" }
  | { type: "error"; message: string };

interface StoredMessage {
  role: string;
  content: string;
  createdAt: number;
}

interface PermissionRequest {
  requestId: number;
  tool: string;
  args: Record<string, unknown>;
}

interface CatalogModel {
  name: string;
  kind: "chat" | "embedding";
  sizeBytes: number;
  quantization: string;
  requiredEngine: string;
  alreadyDownloaded: boolean;
  fitsInAvailableMemory: boolean;
}

interface SystemStats {
  cpuUsagePercent: number;
  memoryUsedBytes: number;
  memoryTotalBytes: number;
  processMemoryBytes: number;
  workspaceDiskBytes: number;
}

interface FlowStateInfo {
  flowName: string;
  stateName: string;
  systemPrompt: string;
}

interface ProviderInfo {
  name: string;
  envVar: string;
  configured: boolean;
}

interface CloudModel {
  id: string;
  provider: string;
  label: string;
}

const CONVERSATION_ID = "default";

function formatBytes(bytes: number): string {
  const gb = bytes / 1024 / 1024 / 1024;
  if (gb >= 1) return `${gb.toFixed(1)} GB`;
  return `${(bytes / 1024 / 1024).toFixed(0)} MB`;
}

function App() {
  const [messages, setMessages] = useState<StoredMessage[]>([]);
  const [prompt, setPrompt] = useState("");
  const [isGenerating, setIsGenerating] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const [permissionRequest, setPermissionRequest] = useState<PermissionRequest | null>(null);

  const [toolName, setToolName] = useState("read_file");
  const [toolPath, setToolPath] = useState("notes.txt");
  const [toolContent, setToolContent] = useState("");
  const [toolCommand, setToolCommand] = useState("echo hello");
  const [toolResult, setToolResult] = useState<string | null>(null);

  const [models, setModels] = useState<CatalogModel[]>([]);
  const [downloadingModel, setDownloadingModel] = useState<string | null>(null);
  const [stats, setStats] = useState<SystemStats | null>(null);

  const [availableFlows, setAvailableFlows] = useState<string[]>([]);
  const [activeFlow, setActiveFlow] = useState<FlowStateInfo | null>(null);

  const [providers, setProviders] = useState<ProviderInfo[]>([]);
  const [cloudModels, setCloudModels] = useState<CloudModel[]>([]);
  const [selectedModel, setSelectedModel] = useState<string>("");
  const [showSettings, setShowSettings] = useState(false);
  const [apiKeyDrafts, setApiKeyDrafts] = useState<Record<string, string>>({});
  const [savingProvider, setSavingProvider] = useState<string | null>(null);

  useEffect(() => {
    invoke<StoredMessage[]>("list_messages", { conversationId: CONVERSATION_ID })
      .then(setMessages)
      .catch((err) => setError(String(err)));
  }, []);

  useEffect(() => {
    const unlisten = listen<PermissionRequest>("tool-permission-request", (event) => {
      setPermissionRequest(event.payload);
    });
    return () => {
      unlisten.then((fn) => fn());
    };
  }, []);

  function refreshModels() {
    invoke<CatalogModel[]>("list_available_models")
      .then(setModels)
      .catch((err) => setError(String(err)));
  }

  useEffect(() => {
    refreshModels();
  }, []);

  useEffect(() => {
    invoke<string[]>("list_flows")
      .then(setAvailableFlows)
      .catch(() => {});
    invoke<FlowStateInfo | null>("flow_status")
      .then(setActiveFlow)
      .catch(() => {});
  }, []);

  async function handleLoadFlow(name: string) {
    setError(null);
    try {
      const info = await invoke<FlowStateInfo>("load_flow", { name });
      setActiveFlow(info);
    } catch (err) {
      setError(String(err));
    }
  }

  async function handleUnloadFlow() {
    await invoke("unload_flow");
    setActiveFlow(null);
  }

  useEffect(() => {
    function poll() {
      invoke<SystemStats>("system_stats").then(setStats).catch(() => {});
    }
    poll();
    const interval = setInterval(poll, 5000);
    return () => clearInterval(interval);
  }, []);

  function refreshProviders() {
    invoke<ProviderInfo[]>("list_providers")
      .then(setProviders)
      .catch(() => {});
  }

  useEffect(() => {
    refreshProviders();
    invoke<CloudModel[]>("list_cloud_models")
      .then(setCloudModels)
      .catch(() => {});
  }, []);

  async function handleSaveApiKey(envVar: string) {
    const key = apiKeyDrafts[envVar];
    if (!key) return;
    setSavingProvider(envVar);
    setError(null);
    try {
      await invoke("set_provider_api_key", { envVar, key });
      setApiKeyDrafts((prev) => ({ ...prev, [envVar]: "" }));
      refreshProviders();
    } catch (err) {
      setError(String(err));
    } finally {
      setSavingProvider(null);
    }
  }

  async function handleDownloadModel(name: string) {
    setDownloadingModel(name);
    setError(null);
    try {
      await invoke("download_model", { name });
      refreshModels();
    } catch (err) {
      setError(String(err));
    } finally {
      setDownloadingModel(null);
    }
  }

  async function respondToPermission(
    response: "allowOnce" | "allowAlways" | "deny" | "denyAlways",
  ) {
    if (!permissionRequest) return;
    try {
      await invoke("respond_permission", {
        requestId: permissionRequest.requestId,
        response,
      });
    } catch (err) {
      setError(String(err));
    }
    setPermissionRequest(null);
  }

  async function handleToolCall() {
    setToolResult(null);
    setError(null);
    try {
      const args =
        toolName === "read_file"
          ? { path: toolPath }
          : toolName === "write_file"
            ? { path: toolPath, content: toolContent }
            : { command: toolCommand };
      const result = await invoke("call_tool", {
        conversationId: CONVERSATION_ID,
        name: toolName,
        args,
      });
      setToolResult(JSON.stringify(result, null, 2));
    } catch (err) {
      setError(String(err));
    }
  }

  async function handleSubmit(e: React.FormEvent) {
    e.preventDefault();
    if (!prompt.trim() || isGenerating) return;

    const userPrompt = prompt;
    setPrompt("");
    setError(null);
    setIsGenerating(true);
    setMessages((prev) => [
      ...prev,
      { role: "user", content: userPrompt, createdAt: Date.now() / 1000 },
      { role: "assistant", content: "", createdAt: Date.now() / 1000 },
    ]);

    const channel = new Channel<GenerationEvent>();
    channel.onmessage = (event) => {
      if (event.type === "piece") {
        setMessages((prev) => {
          const next = [...prev];
          const last = next[next.length - 1];
          next[next.length - 1] = { ...last, content: last.content + event.text };
          return next;
        });
      } else if (event.type === "done") {
        setIsGenerating(false);
        invoke<FlowStateInfo | null>("flow_status")
          .then(setActiveFlow)
          .catch(() => {});
      } else if (event.type === "error") {
        setError(event.message);
        setIsGenerating(false);
      }
    };

    try {
      await invoke("generate", {
        prompt: userPrompt,
        conversationId: CONVERSATION_ID,
        model: selectedModel || null,
        onEvent: channel,
      });
    } catch (err) {
      setError(String(err));
      setIsGenerating(false);
    }
  }

  const noChatModel = error?.toLowerCase().includes("no chat model") ?? false;

  return (
    <main className="flex h-screen w-screen flex-col items-center gap-4 bg-neutral-950 p-8 text-neutral-100">
      <div className="flex w-full max-w-2xl items-center justify-between">
        <h1 className="text-2xl font-semibold">syl</h1>
        <button
          onClick={() => setShowSettings((prev) => !prev)}
          className="text-xs text-neutral-400 underline"
        >
          Settings
        </button>
        {stats && (
          <div className="flex gap-3 text-xs text-neutral-500">
            <span>CPU {stats.cpuUsagePercent.toFixed(0)}%</span>
            <span>
              RAM {formatBytes(stats.memoryUsedBytes)} / {formatBytes(stats.memoryTotalBytes)}
            </span>
            <span>Process {formatBytes(stats.processMemoryBytes)}</span>
            <span>.syl {formatBytes(stats.workspaceDiskBytes)}</span>
          </div>
        )}
      </div>

      {showSettings && (
        <div className="flex w-full max-w-2xl flex-col gap-2 rounded border border-neutral-800 bg-neutral-900 p-4">
          <span className="text-sm font-medium">Cloud provider API keys</span>
          {providers.map((p) => (
            <div key={p.envVar} className="flex items-center gap-2">
              <span className="w-24 shrink-0 text-sm">{p.name}</span>
              <input
                type="password"
                value={apiKeyDrafts[p.envVar] ?? ""}
                onChange={(e) =>
                  setApiKeyDrafts((prev) => ({ ...prev, [p.envVar]: e.currentTarget.value }))
                }
                placeholder={p.configured ? "Key saved — enter to replace" : "API key"}
                className="flex-1 rounded border border-neutral-700 bg-neutral-900 px-2 py-1 text-sm"
              />
              <button
                onClick={() => handleSaveApiKey(p.envVar)}
                disabled={savingProvider === p.envVar || !apiKeyDrafts[p.envVar]}
                className="rounded bg-neutral-100 px-2 py-1 text-xs font-medium text-neutral-950 disabled:opacity-50"
              >
                Save
              </button>
              {p.configured && <span className="text-xs text-green-400">Configured</span>}
            </div>
          ))}
        </div>
      )}

      {permissionRequest && (
        <div className="w-full max-w-2xl rounded border border-amber-600 bg-amber-950/40 p-4">
          <p className="text-sm">
            Allow tool <span className="font-mono">{permissionRequest.tool}</span> to run with{" "}
            <span className="font-mono">{JSON.stringify(permissionRequest.args)}</span>?
          </p>
          <div className="mt-2 flex flex-wrap gap-2">
            <button
              onClick={() => respondToPermission("allowOnce")}
              className="rounded bg-amber-500 px-3 py-1 text-sm font-medium text-neutral-950"
            >
              Allow Once
            </button>
            <button
              onClick={() => respondToPermission("allowAlways")}
              className="rounded bg-amber-600 px-3 py-1 text-sm font-medium text-neutral-950"
            >
              Allow Always
            </button>
            <button
              onClick={() => respondToPermission("deny")}
              className="rounded border border-amber-500 px-3 py-1 text-sm font-medium"
            >
              Deny
            </button>
            <button
              onClick={() => respondToPermission("denyAlways")}
              className="rounded border border-amber-500 px-3 py-1 text-sm font-medium"
            >
              Deny Always
            </button>
          </div>
        </div>
      )}

      <div className="flex w-full max-w-2xl flex-1 flex-col gap-3 overflow-auto rounded border border-neutral-800 bg-neutral-900 p-4">
        {messages.length === 0 && (
          <p className="text-sm text-neutral-500">No messages yet.</p>
        )}
        {messages.map((m, i) => (
          <div key={i} className="flex flex-col gap-1">
            <span className="text-xs uppercase tracking-wide text-neutral-500">
              {m.role}
            </span>
            <p className="whitespace-pre-wrap text-sm">{m.content}</p>
          </div>
        ))}
      </div>

      {error && <p className="w-full max-w-2xl text-sm text-red-400">{error}</p>}

      {noChatModel && (
        <div className="w-full max-w-2xl rounded border border-blue-700 bg-blue-950/40 p-4">
          <p className="mb-2 text-sm">
            No chat model is set up yet. Download one from the catalog below to get started.
          </p>
        </div>
      )}

      <div className="flex w-full max-w-2xl items-center gap-2">
        <span className="text-xs text-neutral-500">Model</span>
        <select
          value={selectedModel}
          onChange={(e) => setSelectedModel(e.currentTarget.value)}
          className="rounded border border-neutral-700 bg-neutral-900 px-2 py-1 text-xs"
        >
          <option value="">Local (default)</option>
          {cloudModels.map((m) => {
            const configured = providers.find((p) => p.name === m.provider)?.configured ?? false;
            return (
              <option key={m.id} value={m.id} disabled={!configured}>
                {m.label} ({m.provider}){configured ? "" : " — needs API key"}
              </option>
            );
          })}
        </select>
      </div>

      <form onSubmit={handleSubmit} className="flex w-full max-w-2xl gap-2">
        <input
          value={prompt}
          onChange={(e) => setPrompt(e.currentTarget.value)}
          placeholder="Type a prompt..."
          disabled={isGenerating}
          className="flex-1 rounded border border-neutral-700 bg-neutral-900 px-3 py-2 text-sm outline-none focus:border-neutral-500"
        />
        <button
          type="submit"
          disabled={isGenerating}
          className="rounded bg-neutral-100 px-4 py-2 text-sm font-medium text-neutral-950 disabled:opacity-50"
        >
          {isGenerating ? "Generating..." : "Send"}
        </button>
      </form>

      <div className="flex w-full max-w-2xl flex-col gap-2 rounded border border-neutral-800 bg-neutral-900 p-4">
        <div className="flex items-center justify-between">
          <span className="text-sm font-medium">Flow</span>
          {activeFlow && (
            <button onClick={handleUnloadFlow} className="text-xs text-neutral-400 underline">
              Unload
            </button>
          )}
        </div>
        {activeFlow ? (
          <p className="text-sm">
            <span className="font-mono">{activeFlow.flowName}</span> is in state{" "}
            <span className="font-mono text-emerald-400">{activeFlow.stateName}</span>
          </p>
        ) : (
          <div className="flex flex-wrap gap-2">
            {availableFlows.length === 0 && (
              <p className="text-sm text-neutral-500">No flows in .syl/flows yet.</p>
            )}
            {availableFlows.map((name) => (
              <button
                key={name}
                onClick={() => handleLoadFlow(name)}
                className="rounded bg-neutral-100 px-2 py-1 text-xs font-medium text-neutral-950"
              >
                Load {name}
              </button>
            ))}
          </div>
        )}
      </div>

      <div className="flex w-full max-w-2xl flex-col gap-2 rounded border border-neutral-800 bg-neutral-900 p-4">
        <div className="flex items-center justify-between">
          <span className="text-sm font-medium">Model catalog</span>
          <button
            onClick={refreshModels}
            className="text-xs text-neutral-400 underline"
          >
            Refresh
          </button>
        </div>
        {models.length === 0 && (
          <p className="text-sm text-neutral-500">No models in the registry yet.</p>
        )}
        {models.map((m) => (
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

      <div className="flex w-full max-w-2xl flex-col gap-2 rounded border border-neutral-800 bg-neutral-900 p-4">
        <div className="flex gap-2">
          <select
            value={toolName}
            onChange={(e) => setToolName(e.currentTarget.value)}
            className="rounded border border-neutral-700 bg-neutral-900 px-2 py-1 text-sm"
          >
            <option value="read_file">read_file</option>
            <option value="write_file">write_file</option>
            <option value="run_command">run_command</option>
          </select>
          {toolName !== "run_command" ? (
            <input
              value={toolPath}
              onChange={(e) => setToolPath(e.currentTarget.value)}
              placeholder="path"
              className="flex-1 rounded border border-neutral-700 bg-neutral-900 px-2 py-1 text-sm"
            />
          ) : (
            <input
              value={toolCommand}
              onChange={(e) => setToolCommand(e.currentTarget.value)}
              placeholder="command"
              className="flex-1 rounded border border-neutral-700 bg-neutral-900 px-2 py-1 text-sm"
            />
          )}
        </div>
        {toolName === "write_file" && (
          <input
            value={toolContent}
            onChange={(e) => setToolContent(e.currentTarget.value)}
            placeholder="content"
            className="rounded border border-neutral-700 bg-neutral-900 px-2 py-1 text-sm"
          />
        )}
        <button
          onClick={handleToolCall}
          className="self-start rounded bg-neutral-100 px-3 py-1 text-sm font-medium text-neutral-950"
        >
          Call tool
        </button>
        {toolResult && (
          <pre className="whitespace-pre-wrap rounded bg-neutral-950 p-2 text-xs">
            {toolResult}
          </pre>
        )}
      </div>
    </main>
  );
}

export default App;
