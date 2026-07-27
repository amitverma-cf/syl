import { useState } from "react";
import { invoke, Channel } from "@tauri-apps/api/core";
import "./App.css";

type GenerationEvent =
  | { type: "piece"; text: string }
  | { type: "done" }
  | { type: "error"; message: string };

function App() {
  const [prompt, setPrompt] = useState("");
  const [output, setOutput] = useState("");
  const [isGenerating, setIsGenerating] = useState(false);
  const [error, setError] = useState<string | null>(null);

  async function handleSubmit(e: React.FormEvent) {
    e.preventDefault();
    if (!prompt.trim() || isGenerating) return;

    setOutput("");
    setError(null);
    setIsGenerating(true);

    const channel = new Channel<GenerationEvent>();
    channel.onmessage = (event) => {
      if (event.type === "piece") {
        setOutput((prev) => prev + event.text);
      } else if (event.type === "done") {
        setIsGenerating(false);
      } else if (event.type === "error") {
        setError(event.message);
        setIsGenerating(false);
      }
    };

    try {
      await invoke("generate", { prompt, onEvent: channel });
    } catch (err) {
      setError(String(err));
      setIsGenerating(false);
    }
  }

  return (
    <main className="flex h-screen w-screen flex-col items-center gap-4 bg-neutral-950 p-8 text-neutral-100">
      <h1 className="text-2xl font-semibold">syl</h1>

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

      {error && (
        <p className="w-full max-w-2xl text-sm text-red-400">{error}</p>
      )}

      <pre className="w-full max-w-2xl flex-1 overflow-auto whitespace-pre-wrap rounded border border-neutral-800 bg-neutral-900 p-4 text-sm">
        {output}
      </pre>
    </main>
  );
}

export default App;
