import { useEffect, useRef, useState } from "react";
import { invoke, Channel } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { toast } from "sonner";
import ReactMarkdown from "react-markdown";
import remarkGfm from "remark-gfm";
import {
  IconCopy,
  IconArrowBackUp,
  IconGitBranch,
  IconVolume,
  IconMicrophone,
  IconPhoto,
  IconArrowUp,
  IconChevronDown,
} from "@tabler/icons-react";
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

function estimateTokens(text: string): number {
  return Math.max(1, Math.round(text.length / 4));
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
  const [modelMenuOpen, setModelMenuOpen] = useState(false);
  const [activeFlow, setActiveFlow] = useState<FlowStateInfo | null>(null);
  const [permissionRequest, setPermissionRequest] = useState<PermissionRequest | null>(null);
  const transcriptRef = useRef<HTMLDivElement>(null);

  const [selectedImageModel, setSelectedImageModel] = useState("");
  const [isGeneratingImage, setIsGeneratingImage] = useState(false);

  const [selectedAsrModel, setSelectedAsrModel] = useState("");
  const [isRecording, setIsRecording] = useState(false);

  const [selectedTtsModel, setSelectedTtsModel] = useState("");
  const [isSpeaking, setIsSpeaking] = useState(false);

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
    if (!selectedImageModel && imageModels.length > 0) setSelectedImageModel(imageModels[0].name);
  }, [imageModels, selectedImageModel]);
  useEffect(() => {
    if (!selectedAsrModel && asrModels.length > 0) setSelectedAsrModel(asrModels[0].name);
  }, [asrModels, selectedAsrModel]);
  useEffect(() => {
    if (!selectedTtsModel && ttsModels.length > 0) setSelectedTtsModel(ttsModels[0].name);
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
    transcriptRef.current?.scrollTo({ top: transcriptRef.current.scrollHeight });
  }, [messages]);

  useEffect(() => {
    const unlisten = listen<PermissionRequest>("tool-permission-request", (event) => {
      setPermissionRequest(event.payload);
    });
    return () => {
      unlisten.then((fn) => fn());
    };
  }, []);

  async function respondToPermission(response: "allowOnce" | "allowAlways" | "deny" | "denyAlways") {
    if (!permissionRequest) return;
    try {
      await invoke("respond_permission", { requestId: permissionRequest.requestId, response });
      toast(`Tool call ${response === "deny" || response === "denyAlways" ? "denied" : "allowed"}`, {
        description: permissionRequest.tool,
      });
    } catch (err) {
      setError(String(err));
    }
    setPermissionRequest(null);
  }

  const isLocalModel = selectedModel.startsWith(LOCAL_MODEL_PREFIX);
  const selectedLocalModelName = isLocalModel ? selectedModel.slice(LOCAL_MODEL_PREFIX.length) : "";
  const selectedLocalModelInfo = localModels.find((m) => m.name === selectedLocalModelName);
  const selectedLocalModelLoaded = selectedLocalModelInfo?.loaded ?? false;
  const selectedModelLabel = isLocalModel
    ? selectedLocalModelName
    : cloudModels.find((m) => m.id === selectedModel)?.label ?? "Select a model";

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

  async function handleSubmit(e: React.SyntheticEvent) {
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
        toast.error(event.message);
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
      toast.error(String(err));
      setIsGenerating(false);
    }
  }

  async function handleGenerateImage() {
    if (!prompt.trim() || !selectedImageModel || isGeneratingImage) return;
    setIsGeneratingImage(true);
    setError(null);
    try {
      const selectedInfo = imageModels.find((m) => m.name === selectedImageModel);
      if (!selectedInfo?.loaded) {
        await invoke("load_image_model", { name: selectedImageModel });
        refreshLocalModels();
      }
      const dataUrl = await invoke<string>("generate_image", {
        model: selectedImageModel,
        prompt,
        negativePrompt: "",
      });
      setMessages((prev) => [
        ...prev,
        { role: "user", content: prompt, createdAt: Date.now() / 1000 },
        { role: "assistant", content: `![generated image](${dataUrl})`, createdAt: Date.now() / 1000 },
      ]);
      setPrompt("");
    } catch (err) {
      setError(String(err));
      toast.error(String(err));
    } finally {
      setIsGeneratingImage(false);
    }
  }

  async function handleRecordAndTranscribe() {
    if (!selectedAsrModel || isRecording) return;
    setIsRecording(true);
    setError(null);
    try {
      const selectedInfo = asrModels.find((m) => m.name === selectedAsrModel);
      if (!selectedInfo?.loaded) {
        await invoke("load_asr_model", { name: selectedAsrModel });
        refreshLocalModels();
      }
      const text = await invoke<string>("transcribe_recording", { model: selectedAsrModel, seconds: 5 });
      setPrompt((prev) => (prev ? `${prev} ${text}` : text));
    } catch (err) {
      setError(String(err));
      toast.error(String(err));
    } finally {
      setIsRecording(false);
    }
  }

  async function handleSpeak(text: string) {
    if (!text || !selectedTtsModel || isSpeaking) {
      if ("speechSynthesis" in window && text) {
        const utter = new SpeechSynthesisUtterance(text);
        window.speechSynthesis.speak(utter);
      }
      return;
    }
    setIsSpeaking(true);
    setError(null);
    try {
      const selectedInfo = ttsModels.find((m) => m.name === selectedTtsModel);
      if (!selectedInfo?.loaded) {
        await invoke("load_tts_model", { name: selectedTtsModel });
        refreshLocalModels();
      }
      await invoke("speak_text", { model: selectedTtsModel, text });
    } catch (err) {
      setError(String(err));
      toast.error(String(err));
    } finally {
      setIsSpeaking(false);
    }
  }

  async function handleCopy(text: string) {
    await navigator.clipboard.writeText(text);
    toast("Copied to clipboard");
  }

  const noChatModel = error?.toLowerCase().includes("no chat model") ?? false;

  return (
    <>
      {permissionRequest && (
        <div className="permission-banner">
          <div className="pb-text">
            Allow tool <code>{permissionRequest.tool}</code> to run with{" "}
            <code>{JSON.stringify(permissionRequest.args)}</code>?
          </div>
          <div className="pb-actions">
            <div className="pb-btn primary" onClick={() => respondToPermission("allowOnce")}>
              Allow once
            </div>
            <div className="pb-btn" onClick={() => respondToPermission("allowAlways")}>
              Allow always
            </div>
            <div className="pb-btn" onClick={() => respondToPermission("deny")}>
              Deny
            </div>
            <div className="pb-btn" onClick={() => respondToPermission("denyAlways")}>
              Deny always
            </div>
          </div>
        </div>
      )}

      {activeFlow && (
        <div style={{ padding: "6px 18px 0", fontSize: 11, color: "var(--text-3)" }}>
          Flow <code>{activeFlow.flowName}</code> · state{" "}
          <code style={{ color: "var(--accent)" }}>{activeFlow.stateName}</code>
        </div>
      )}

      <div className="transcript" ref={transcriptRef}>
        {messages.length === 0 && (
          <div className="empty-state">
            <span>No messages yet — say something</span>
          </div>
        )}
        {messages.map((m, i) =>
          m.role === "user" ? (
            <div className="msg-row user" key={i}>
              <div className="msg-user-box">{m.content}</div>
              <div className="msg-actions">
                <div className="msg-action" title="Copy" onClick={() => handleCopy(m.content)}>
                  <IconCopy size={13} aria-hidden />
                </div>
                <div className="msg-action" title="Split into new chat">
                  <IconGitBranch size={13} aria-hidden />
                </div>
                <div className="msg-action" title="Undo">
                  <IconArrowBackUp size={13} aria-hidden />
                </div>
                <span className="msg-time">{new Date(m.createdAt * 1000).toLocaleTimeString()}</span>
              </div>
            </div>
          ) : (
            <div className="msg-row assistant" key={i}>
              <div className="msg-assistant-content markdown-body">
                <ReactMarkdown remarkPlugins={[remarkGfm]}>{m.content || "…"}</ReactMarkdown>
              </div>
              <div className="msg-actions">
                <div className="msg-action" title="Copy" onClick={() => handleCopy(m.content)}>
                  <IconCopy size={13} aria-hidden />
                </div>
                <div className="msg-action" title="Read aloud" onClick={() => handleSpeak(m.content)}>
                  <IconVolume size={13} aria-hidden />
                </div>
                <span className="msg-time">{new Date(m.createdAt * 1000).toLocaleTimeString()}</span>
                <span className="msg-tokens">{estimateTokens(m.content)} tok</span>
              </div>
            </div>
          ),
        )}
      </div>

      {error && !noChatModel && <p style={{ color: "#ff8783", fontSize: 12, padding: "0 18px" }}>{error}</p>}
      {noChatModel && (
        <div className="empty-state" style={{ flex: "none", padding: "8px 18px" }}>
          <span>No chat model set up yet — download one from Settings.</span>
        </div>
      )}

      <div className="composer-wrap">
        <form className="composer" onSubmit={handleSubmit}>
          <textarea
            className="input"
            value={prompt}
            onChange={(e) => setPrompt(e.currentTarget.value)}
            placeholder="Message syl..."
            disabled={isGenerating}
            rows={1}
            onKeyDown={(e) => {
              if (e.key === "Enter" && !e.shiftKey) {
                e.preventDefault();
                handleSubmit(e);
              }
            }}
          />
          <div className="composer-row">
            {asrModels.length > 0 && (
              <div
                className={`composer-btn${isRecording ? " active" : ""}`}
                title="Record"
                onClick={handleRecordAndTranscribe}
              >
                <IconMicrophone size={15} aria-hidden />
              </div>
            )}
            {imageModels.length > 0 && (
              <div
                className={`composer-btn${isGeneratingImage ? " active" : ""}`}
                title="Generate image from prompt"
                onClick={handleGenerateImage}
              >
                <IconPhoto size={15} aria-hidden />
              </div>
            )}
            <div className="spacer" />

            {isLocalModel && !selectedLocalModelLoaded && (
              <div className="form-btn" onClick={loadSelectedLocalModel} style={{ fontSize: 11 }}>
                {isLoadingModel ? "Loading…" : "Load model"}
              </div>
            )}
            {isLocalModel && selectedLocalModelLoaded && (
              <div className="form-btn" onClick={unloadSelectedLocalModel} style={{ fontSize: 11 }}>
                Unload
              </div>
            )}

            <div className="dropdown-wrap">
              <div className="dropdown" onClick={() => setModelMenuOpen((v) => !v)}>
                <span>{selectedModelLabel}</span>
                <IconChevronDown size={13} aria-hidden />
              </div>
              {modelMenuOpen && (
                <div className="option-menu open" onMouseLeave={() => setModelMenuOpen(false)}>
                  {chatLocalModels.length > 0 && (
                    <>
                      <div className="cmdk-group-label" style={{ padding: "4px 8px" }}>
                        LOCAL
                      </div>
                      {chatLocalModels.map((m) => (
                        <div
                          key={m.name}
                          className={`option-item${selectedModel === `${LOCAL_MODEL_PREFIX}${m.name}` ? " sel" : ""}`}
                          onClick={() => {
                            setSelectedModel(`${LOCAL_MODEL_PREFIX}${m.name}`);
                            setModelMenuOpen(false);
                          }}
                        >
                          {m.name}
                          <span className="option-sub">{m.loaded ? "loaded" : ""}</span>
                        </div>
                      ))}
                    </>
                  )}
                  {Array.from(cloudGroups.entries()).map(([provider, models]) => (
                    <div key={provider}>
                      <div className="cmdk-group-label" style={{ padding: "4px 8px" }}>
                        {provider.toUpperCase()}
                      </div>
                      {models.map((m) => (
                        <div
                          key={m.id}
                          className={`option-item${selectedModel === m.id ? " sel" : ""}`}
                          onClick={() => {
                            setSelectedModel(m.id);
                            setModelMenuOpen(false);
                          }}
                        >
                          {m.label}
                        </div>
                      ))}
                    </div>
                  ))}
                </div>
              )}
            </div>

            <div
              className={`send${!prompt.trim() || isGenerating || isLoadingModel || !selectedModel ? " disabled" : ""}`}
              onClick={handleSubmit}
            >
              <IconArrowUp size={16} aria-hidden />
            </div>
          </div>
        </form>
      </div>
    </>
  );
}

export default ChatPanel;
