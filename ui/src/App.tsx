import { useEffect, useState } from "react";
import { invoke, Channel } from "@tauri-apps/api/core";
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

const CONVERSATION_ID = "default";

function App() {
  const [messages, setMessages] = useState<StoredMessage[]>([]);
  const [prompt, setPrompt] = useState("");
  const [isGenerating, setIsGenerating] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    invoke<StoredMessage[]>("list_messages", { conversationId: CONVERSATION_ID })
      .then(setMessages)
      .catch((err) => setError(String(err)));
  }, []);

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
    </main>
  );
}

export default App;
