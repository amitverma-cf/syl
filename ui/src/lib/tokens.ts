import { invoke } from "@tauri-apps/api/core";

let encodeFn: ((text: string) => number[]) | null = null;
async function encode(text: string): Promise<number[]> {
  if (!encodeFn) {
    // gpt-tokenizer bundles a full BPE merge table per encoding — import just
    // cl100k_base (the encoding OpenAI's own chat models use) instead of the
    // whole package, and load it lazily so it only costs anything the first
    // time a cloud model's message actually needs tokenizing.
    const mod = await import("gpt-tokenizer/encoding/cl100k_base");
    encodeFn = mod.encode;
  }
  return encodeFn(text);
}

const LOCAL_MODEL_PREFIX = "local::";

/**
 * Real published context-window sizes (in tokens) for well-known cloud
 * models, keyed by the model id as returned by list_cloud_models. Anything
 * not listed (a custom/OpenAI-compatible provider, or a model released after
 * this list was written) falls back to a conservative default rather than
 * pretending to know.
 */
const CLOUD_CONTEXT_WINDOWS: Record<string, number> = {
  "gpt-5": 400000,
  "gpt-4o": 128000,
  "gpt-4o-mini": 128000,
  "gpt-4-turbo": 128000,
  "claude-sonnet-5": 200000,
  "claude-opus-5": 200000,
  "claude-fable-5": 200000,
  "claude-3-5-sonnet-latest": 200000,
  "claude-3-5-haiku-latest": 200000,
  "gemini-2.0-flash": 1000000,
  "gemini-1.5-pro": 2000000,
};
const DEFAULT_CLOUD_CONTEXT_WINDOW = 128000;

export function isLocalModelId(modelId: string): boolean {
  return modelId.startsWith(LOCAL_MODEL_PREFIX);
}

export function localModelNameFromId(modelId: string): string {
  return modelId.slice(LOCAL_MODEL_PREFIX.length);
}

/** Real token count for `text` under the given model: the model's own
 * llama.cpp tokenizer for a loaded local model, or a real cl100k_base BPE
 * tokenizer (the same family OpenAI models use) for cloud models — never a
 * length/4 guess. */
export async function countTokens(modelId: string, text: string): Promise<number> {
  if (!text) return 0;
  if (isLocalModelId(modelId)) {
    const name = localModelNameFromId(modelId);
    try {
      return await invoke<number>("count_local_tokens", { name, text });
    } catch {
      // model not loaded yet (e.g. picked but not loaded) — fall through to
      // the BPE estimate so the UI still shows a real (if less precise) count
      return (await encode(text)).length;
    }
  }
  return (await encode(text)).length;
}

export async function localContextSize(): Promise<number> {
  try {
    return await invoke<number>("local_context_size");
  } catch {
    return 4096;
  }
}

export function cloudContextWindow(modelId: string): number {
  return CLOUD_CONTEXT_WINDOWS[modelId] ?? DEFAULT_CLOUD_CONTEXT_WINDOW;
}
