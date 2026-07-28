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

const CONVERSATION_ID = "default";

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
      } else if (event.type === "error") {
        setError(event.message);
        setIsGenerating(false);
      }
    };

    try {
      await invoke("generate", {
        prompt: userPrompt,
        conversationId: CONVERSATION_ID,
        onEvent: channel,
      });
    } catch (err) {
      setError(String(err));
      setIsGenerating(false);
    }
  }

  return (
    <main className="flex h-screen w-screen flex-col items-center gap-4 bg-neutral-950 p-8 text-neutral-100">
      <h1 className="text-2xl font-semibold">syl</h1>

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
