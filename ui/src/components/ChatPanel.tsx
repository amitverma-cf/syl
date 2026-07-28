import { useEffect, useState } from "react";
import { invoke, Channel } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type {
  CloudModel,
  FlowStateInfo,
  GenerationEvent,
  PermissionRequest,
  ProviderInfo,
  StoredMessage,
} from "../types";

interface ChatPanelProps {
  conversationId: string;
  cloudModels: CloudModel[];
  providers: ProviderInfo[];
  onTurnComplete: () => void;
}

function ChatPanel({ conversationId, cloudModels, providers, onTurnComplete }: ChatPanelProps) {
  const [messages, setMessages] = useState<StoredMessage[]>([]);
  const [prompt, setPrompt] = useState("");
  const [isGenerating, setIsGenerating] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [selectedModel, setSelectedModel] = useState<string>("");
  const [activeFlow, setActiveFlow] = useState<FlowStateInfo | null>(null);
  const [permissionRequest, setPermissionRequest] = useState<PermissionRequest | null>(null);

  useEffect(() => {
    setError(null);
    invoke<StoredMessage[]>("list_messages", { conversationId })
      .then(setMessages)
      .catch((err) => setError(String(err)));
    invoke<FlowStateInfo | null>("flow_status", { conversationId })
      .then(setActiveFlow)
      .catch(() => {});
  }, [conversationId]);

  useEffect(() => {
    const unlisten = listen<PermissionRequest>("tool-permission-request", (event) => {
      setPermissionRequest(event.payload);
    });
    return () => {
      unlisten.then((fn) => fn());
    };
  }, []);

  async function respondToPermission(
    response: "allowOnce" | "allowAlways" | "deny" | "denyAlways",
  ) {
    if (!permissionRequest) return;
    try {
      await invoke("respond_permission", { requestId: permissionRequest.requestId, response });
    } catch (err) {
      setError(String(err));
    }
    setPermissionRequest(null);
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
        invoke<FlowStateInfo | null>("flow_status", { conversationId })
          .then(setActiveFlow)
          .catch(() => {});
        onTurnComplete();
      } else if (event.type === "error") {
        setError(event.message);
        setIsGenerating(false);
      }
    };

    try {
      await invoke("generate", {
        prompt: userPrompt,
        conversationId,
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
    <div className="flex h-full flex-1 flex-col gap-3 p-4">
      <div className="flex items-center justify-between text-xs text-neutral-500">
        {activeFlow ? (
          <span>
            Flow <span className="font-mono text-neutral-300">{activeFlow.flowName}</span> ·
            state <span className="font-mono text-emerald-400">{activeFlow.stateName}</span>
          </span>
        ) : (
          <span>No active flow</span>
        )}
        <div className="flex items-center gap-2">
          <span>Model</span>
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
      </div>

      {permissionRequest && (
        <div className="rounded border border-amber-600 bg-amber-950/40 p-4">
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

      <div className="flex flex-1 flex-col gap-3 overflow-auto rounded border border-neutral-800 bg-neutral-900 p-4">
        {messages.length === 0 && <p className="text-sm text-neutral-500">No messages yet.</p>}
        {messages.map((m, i) => (
          <div key={i} className="flex flex-col gap-1">
            <span className="text-xs uppercase tracking-wide text-neutral-500">{m.role}</span>
            <p className="whitespace-pre-wrap text-sm">{m.content}</p>
          </div>
        ))}
      </div>

      {error && <p className="text-sm text-red-400">{error}</p>}

      {noChatModel && (
        <div className="rounded border border-blue-700 bg-blue-950/40 p-4">
          <p className="text-sm">
            No chat model is set up yet. Download one from Settings → Local Engines & Models.
          </p>
        </div>
      )}

      <form onSubmit={handleSubmit} className="flex gap-2">
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
    </div>
  );
}

export default ChatPanel;
