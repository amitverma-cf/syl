import { useEffect, useState } from "react";
import { invoke, Channel } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type {
  CloudModel,
  FlowStateInfo,
  GenerationEvent,
  LocalModelInfo,
  PermissionRequest,
  ProviderInfo,
  StoredMessage,
} from "../types";

const LOCAL_MODEL_PREFIX = "local::";

interface ChatPanelProps {
  conversationId: string;
  cloudModels: CloudModel[];
  providers: ProviderInfo[];
  localModels: LocalModelInfo[];
  refreshLocalModels: () => void;
  onTurnComplete: () => void;
}

function groupCloudModelsByProvider(
  cloudModels: CloudModel[],
  providers: ProviderInfo[],
): Map<string, CloudModel[]> {
  const groups = new Map<string, CloudModel[]>();
  for (const m of cloudModels) {
    const known = providers.find((p) => p.name === m.provider);
    if (known && !known.configured) continue;
    const existing = groups.get(m.provider);
    if (existing) existing.push(m);
    else groups.set(m.provider, [m]);
  }
  return groups;
}

function ChatPanel({
  conversationId,
  cloudModels,
  providers,
  localModels,
  refreshLocalModels,
  onTurnComplete,
}: ChatPanelProps) {
  const [messages, setMessages] = useState<StoredMessage[]>([]);
  const [prompt, setPrompt] = useState("");
  const [isGenerating, setIsGenerating] = useState(false);
  const [isLoadingModel, setIsLoadingModel] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [selectedModel, setSelectedModel] = useState<string>("");
  const [activeFlow, setActiveFlow] = useState<FlowStateInfo | null>(null);
  const [permissionRequest, setPermissionRequest] = useState<PermissionRequest | null>(null);

  const [showImagePanel, setShowImagePanel] = useState(false);
  const [imagePrompt, setImagePrompt] = useState("");
  const [selectedImageModel, setSelectedImageModel] = useState("");
  const [isGeneratingImage, setIsGeneratingImage] = useState(false);
  const [generatedImage, setGeneratedImage] = useState<string | null>(null);
  const [imageError, setImageError] = useState<string | null>(null);

  const [selectedAsrModel, setSelectedAsrModel] = useState("");
  const [isRecording, setIsRecording] = useState(false);
  const [asrError, setAsrError] = useState<string | null>(null);

  const [selectedTtsModel, setSelectedTtsModel] = useState("");
  const [isSpeaking, setIsSpeaking] = useState(false);
  const [ttsError, setTtsError] = useState<string | null>(null);

  const cloudGroups = groupCloudModelsByProvider(cloudModels, providers);
  const chatLocalModels = localModels.filter((m) => m.kind === "chat");
  const imageModels = localModels.filter((m) => m.kind === "image");
  const asrModels = localModels.filter((m) => m.kind === "asr");
  const ttsModels = localModels.filter((m) => m.kind === "tts");

  useEffect(() => {
    if (selectedModel) return;
    if (chatLocalModels.length > 0) {
      setSelectedModel(`${LOCAL_MODEL_PREFIX}${chatLocalModels[0].name}`);
    } else if (cloudModels.length > 0) {
      setSelectedModel(cloudModels[0].id);
    }
  }, [chatLocalModels, cloudModels, selectedModel]);

  useEffect(() => {
    if (!selectedImageModel && imageModels.length > 0) {
      setSelectedImageModel(imageModels[0].name);
    }
  }, [imageModels, selectedImageModel]);

  useEffect(() => {
    if (!selectedAsrModel && asrModels.length > 0) {
      setSelectedAsrModel(asrModels[0].name);
    }
  }, [asrModels, selectedAsrModel]);

  useEffect(() => {
    if (!selectedTtsModel && ttsModels.length > 0) {
      setSelectedTtsModel(ttsModels[0].name);
    }
  }, [ttsModels, selectedTtsModel]);

  useEffect(() => {
    setError(null);
    invoke<StoredMessage[]>("list_messages", { conversationId })
      .then(setMessages)
      .catch((err) => setError(String(err)));
    invoke<FlowStateInfo | null>("flow_status", { conversationId })
      .then(setActiveFlow)
      .catch((err) => setError(String(err)));
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

  const isLocalModel = selectedModel.startsWith(LOCAL_MODEL_PREFIX);
  const selectedLocalModelName = isLocalModel
    ? selectedModel.slice(LOCAL_MODEL_PREFIX.length)
    : "";
  const selectedLocalModelInfo = localModels.find((m) => m.name === selectedLocalModelName);
  const selectedLocalModelLoaded = selectedLocalModelInfo?.loaded ?? false;

  async function loadSelectedLocalModel() {
    if (!isLocalModel || selectedLocalModelLoaded) return;
    setIsLoadingModel(true);
    setError(null);
    try {
      await invoke("load_local_model", { name: selectedLocalModelName });
      refreshLocalModels();
    } catch (err) {
      setError(String(err));
    } finally {
      setIsLoadingModel(false);
    }
  }

  async function unloadSelectedLocalModel() {
    if (!isLocalModel || !selectedLocalModelLoaded) return;
    setError(null);
    try {
      await invoke("unload_local_model", { name: selectedLocalModelName });
      refreshLocalModels();
    } catch (err) {
      setError(String(err));
    }
  }

  async function handleSubmit(e: React.FormEvent) {
    e.preventDefault();
    if (!prompt.trim() || isGenerating || !selectedModel) return;

    if (isLocalModel && !selectedLocalModelLoaded) {
      setIsLoadingModel(true);
      try {
        await invoke("load_local_model", { name: selectedLocalModelName });
        refreshLocalModels();
      } catch (err) {
        setError(String(err));
        setIsLoadingModel(false);
        return;
      }
      setIsLoadingModel(false);
    }

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
          .catch((err) => setError(String(err)));
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
        model: isLocalModel ? null : selectedModel,
        localModel: isLocalModel ? selectedLocalModelName : null,
        onEvent: channel,
      });
    } catch (err) {
      setError(String(err));
      setIsGenerating(false);
    }
  }

  async function handleGenerateImage() {
    if (!imagePrompt.trim() || !selectedImageModel || isGeneratingImage) return;
    setIsGeneratingImage(true);
    setImageError(null);
    setGeneratedImage(null);
    try {
      const selectedInfo = imageModels.find((m) => m.name === selectedImageModel);
      if (!selectedInfo?.loaded) {
        await invoke("load_image_model", { name: selectedImageModel });
        refreshLocalModels();
      }
      const dataUrl = await invoke<string>("generate_image", {
        model: selectedImageModel,
        prompt: imagePrompt,
        negativePrompt: "",
      });
      setGeneratedImage(dataUrl);
    } catch (err) {
      setImageError(String(err));
    } finally {
      setIsGeneratingImage(false);
    }
  }

  async function handleRecordAndTranscribe() {
    if (!selectedAsrModel || isRecording) return;
    setIsRecording(true);
    setAsrError(null);
    try {
      const selectedInfo = asrModels.find((m) => m.name === selectedAsrModel);
      if (!selectedInfo?.loaded) {
        await invoke("load_asr_model", { name: selectedAsrModel });
        refreshLocalModels();
      }
      const text = await invoke<string>("transcribe_recording", {
        model: selectedAsrModel,
        seconds: 5,
      });
      setPrompt((prev) => (prev ? `${prev} ${text}` : text));
    } catch (err) {
      setAsrError(String(err));
    } finally {
      setIsRecording(false);
    }
  }

  async function handleSpeakLastReply() {
    const lastAssistant = [...messages].reverse().find((m) => m.role === "assistant");
    if (!lastAssistant?.content || !selectedTtsModel || isSpeaking) return;
    setIsSpeaking(true);
    setTtsError(null);
    try {
      const selectedInfo = ttsModels.find((m) => m.name === selectedTtsModel);
      if (!selectedInfo?.loaded) {
        await invoke("load_tts_model", { name: selectedTtsModel });
        refreshLocalModels();
      }
      await invoke("speak_text", { model: selectedTtsModel, text: lastAssistant.content });
    } catch (err) {
      setTtsError(String(err));
    } finally {
      setIsSpeaking(false);
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
            {chatLocalModels.length > 0 && (
              <optgroup label="Local">
                {chatLocalModels.map((m) => (
                  <option key={m.name} value={`${LOCAL_MODEL_PREFIX}${m.name}`}>
                    {m.name}
                    {m.loaded ? "" : " (not loaded)"}
                  </option>
                ))}
              </optgroup>
            )}
            {Array.from(cloudGroups.entries()).map(([provider, models]) => (
              <optgroup key={provider} label={provider}>
                {models.map((m) => (
                  <option key={m.id} value={m.id}>
                    {m.label}
                  </option>
                ))}
              </optgroup>
            ))}
          </select>
          {isLocalModel && !selectedLocalModelLoaded && (
            <button
              onClick={loadSelectedLocalModel}
              disabled={isLoadingModel}
              className="rounded bg-neutral-100 px-2 py-1 text-xs font-medium text-neutral-950 disabled:opacity-50"
            >
              {isLoadingModel ? "Loading..." : "Load"}
            </button>
          )}
          {isLocalModel && selectedLocalModelLoaded && (
            <button
              onClick={unloadSelectedLocalModel}
              className="rounded border border-neutral-700 px-2 py-1 text-xs text-neutral-400"
            >
              Unload
            </button>
          )}
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

      {imageModels.length > 0 && (
        <div className="rounded border border-neutral-800 bg-neutral-900 p-3">
          <button
            onClick={() => setShowImagePanel((prev) => !prev)}
            className="text-xs text-neutral-400 underline"
          >
            {showImagePanel ? "Hide image generation" : "Generate an image"}
          </button>
          {showImagePanel && (
            <div className="mt-2 flex flex-col gap-2">
              <div className="flex gap-2">
                <select
                  value={selectedImageModel}
                  onChange={(e) => setSelectedImageModel(e.currentTarget.value)}
                  className="rounded border border-neutral-700 bg-neutral-900 px-2 py-1 text-xs"
                >
                  {imageModels.map((m) => (
                    <option key={m.name} value={m.name}>
                      {m.name}
                      {m.loaded ? "" : " (not loaded)"}
                    </option>
                  ))}
                </select>
                <input
                  value={imagePrompt}
                  onChange={(e) => setImagePrompt(e.currentTarget.value)}
                  placeholder="Describe the image..."
                  className="flex-1 rounded border border-neutral-700 bg-neutral-900 px-2 py-1 text-sm"
                />
                <button
                  onClick={handleGenerateImage}
                  disabled={isGeneratingImage || !imagePrompt.trim()}
                  className="rounded bg-neutral-100 px-3 py-1 text-xs font-medium text-neutral-950 disabled:opacity-50"
                >
                  {isGeneratingImage ? "Generating..." : "Generate"}
                </button>
              </div>
              {imageError && <p className="text-sm text-red-400">{imageError}</p>}
              {generatedImage && (
                <img
                  src={generatedImage}
                  alt={imagePrompt}
                  className="max-w-xs rounded border border-neutral-800"
                />
              )}
            </div>
          )}
        </div>
      )}

      {noChatModel && (
        <div className="rounded border border-blue-700 bg-blue-950/40 p-4">
          <p className="text-sm">
            No chat model is set up yet. Download one from Settings → AI Providers.
          </p>
        </div>
      )}

      {(asrModels.length > 0 || ttsModels.length > 0) && (
        <div className="flex items-center gap-2 text-xs">
          {asrModels.length > 0 && (
            <>
              <select
                value={selectedAsrModel}
                onChange={(e) => setSelectedAsrModel(e.currentTarget.value)}
                className="rounded border border-neutral-700 bg-neutral-900 px-2 py-1"
              >
                {asrModels.map((m) => (
                  <option key={m.name} value={m.name}>
                    {m.name}
                  </option>
                ))}
              </select>
              <button
                onClick={handleRecordAndTranscribe}
                disabled={isRecording}
                className="rounded border border-neutral-700 px-2 py-1 text-neutral-400 disabled:opacity-50"
              >
                {isRecording ? "Recording (5s)..." : "🎤 Record"}
              </button>
              {asrError && <span className="text-red-400">{asrError}</span>}
            </>
          )}
          {ttsModels.length > 0 && (
            <>
              <select
                value={selectedTtsModel}
                onChange={(e) => setSelectedTtsModel(e.currentTarget.value)}
                className="rounded border border-neutral-700 bg-neutral-900 px-2 py-1"
              >
                {ttsModels.map((m) => (
                  <option key={m.name} value={m.name}>
                    {m.name}
                  </option>
                ))}
              </select>
              <button
                onClick={handleSpeakLastReply}
                disabled={isSpeaking || messages.every((m) => m.role !== "assistant")}
                className="rounded border border-neutral-700 px-2 py-1 text-neutral-400 disabled:opacity-50"
              >
                {isSpeaking ? "Speaking..." : "🔊 Speak last reply"}
              </button>
              {ttsError && <span className="text-red-400">{ttsError}</span>}
            </>
          )}
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
          disabled={isGenerating || isLoadingModel || !selectedModel}
          className="rounded bg-neutral-100 px-4 py-2 text-sm font-medium text-neutral-950 disabled:opacity-50"
        >
          {isLoadingModel ? "Loading model..." : isGenerating ? "Generating..." : "Send"}
        </button>
      </form>
    </div>
  );
}

export default ChatPanel;
