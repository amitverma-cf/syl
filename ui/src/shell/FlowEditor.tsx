import { useEffect, useMemo, useState } from "react";
import { useAutoGrowTextarea } from "../hooks/useAutoGrowTextarea";
import { invoke } from "@tauri-apps/api/core";
import { toast } from "sonner";
import { IconButton, Button, Input, Textarea, Select } from "../components/ui";
import {
  IconPlus,
  IconTrash,
  IconDeviceFloppy,
  IconFolderOpen,
  IconFileText,
  IconFlag,
  IconFlagFilled,
  IconSend,
  IconChevronDown,
  IconSparkles,
} from "@tabler/icons-react";
import type { CloudModel, FlowDef, FlowStateDef, LocalModelInfo, ProviderInfo } from "../types";

const COL_WIDTH = 220;
const ROW_HEIGHT = 96;
const NODE_WIDTH = 168;
const NODE_HEIGHT = 56;
const LOCAL_MODEL_PREFIX = "local::";

function blankFlow(): FlowDef {
  return {
    name: "new-flow",
    initial_state: "start",
    states: [
      { name: "start", system_prompt: "You are a helpful assistant.", tool_allowlist: [], transitions: [] },
    ],
  };
}

function layout(flow: FlowDef): Map<string, { col: number; row: number }> {
  const depth = new Map<string, number>();
  const byName = new Map(flow.states.map((s) => [s.name, s]));
  if (byName.has(flow.initial_state)) {
    const queue = [flow.initial_state];
    depth.set(flow.initial_state, 0);
    while (queue.length) {
      const cur = queue.shift() as string;
      const state = byName.get(cur);
      for (const t of state?.transitions ?? []) {
        if (!depth.has(t.to_state) && byName.has(t.to_state)) {
          depth.set(t.to_state, (depth.get(cur) ?? 0) + 1);
          queue.push(t.to_state);
        }
      }
    }
  }
  const maxDepth = depth.size > 0 ? Math.max(...depth.values()) : -1;
  let nextCol = maxDepth + 1;
  for (const s of flow.states) {
    if (!depth.has(s.name)) depth.set(s.name, nextCol++);
  }

  const byCol = new Map<number, string[]>();
  for (const s of flow.states) {
    const col = depth.get(s.name) ?? 0;
    if (!byCol.has(col)) byCol.set(col, []);
    byCol.get(col)!.push(s.name);
  }
  const positions = new Map<string, { col: number; row: number }>();
  for (const [col, names] of byCol) {
    names.forEach((name, row) => positions.set(name, { col, row }));
  }
  return positions;
}

function nodeCenter(pos: { col: number; row: number }) {
  return {
    x: pos.col * COL_WIDTH + NODE_WIDTH / 2 + 40,
    y: pos.row * ROW_HEIGHT + NODE_HEIGHT / 2 + 40,
  };
}

interface FlowEditorProps {
  cloudModels: CloudModel[];
  providers: ProviderInfo[];
  localModels: LocalModelInfo[];
}

function FlowEditor({ cloudModels, providers, localModels }: FlowEditorProps) {
  const [flow, setFlow] = useState<FlowDef>(blankFlow());
  const [savedName, setSavedName] = useState<string | null>(null);
  const [confirmDeleteFlow, setConfirmDeleteFlow] = useState(false);
  const [selected, setSelected] = useState<string | null>("start");
  const [availableFlows, setAvailableFlows] = useState<string[]>([]);
  const [loadMenuOpen, setLoadMenuOpen] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const [aiPrompt, setAiPrompt] = useState("");
  const aiPromptRef = useAutoGrowTextarea<HTMLTextAreaElement>(aiPrompt);
  const [aiBusy, setAiBusy] = useState(false);
  const [aiDraftRaw, setAiDraftRaw] = useState<string | null>(null);
  const [aiDraftError, setAiDraftError] = useState<string | null>(null);
  const [aiModelOverride, setAiModel] = useState<string | null>(null);
  const [aiModelMenuOpen, setAiModelMenuOpen] = useState(false);

  const chatLocalModels = useMemo(() => localModels.filter((m) => m.kind === "chat"), [localModels]);
  const cloudGroups = useMemo(() => {
    const groups = new Map<string, CloudModel[]>();
    for (const m of cloudModels) {
      const known = providers.find((p) => p.name === m.provider);
      if (known && !known.configured) continue;
      const list = groups.get(m.provider);
      if (list) list.push(m);
      else groups.set(m.provider, [m]);
    }
    return groups;
  }, [cloudModels, providers]);

  const defaultAiModel =
    chatLocalModels.length > 0 ? `${LOCAL_MODEL_PREFIX}${chatLocalModels[0].name}` : (cloudModels[0]?.id ?? "");
  const aiModel = aiModelOverride ?? defaultAiModel;

  useEffect(() => {
    invoke<string[]>("list_flows")
      .then(setAvailableFlows)
      .catch((err) => setError(String(err)));
  }, []);

  const positions = useMemo(() => layout(flow), [flow]);
  const selectedState = flow.states.find((s) => s.name === selected) ?? null;

  function updateState(name: string, fn: (s: FlowStateDef) => FlowStateDef) {
    setFlow((f) => ({ ...f, states: f.states.map((s) => (s.name === name ? fn(s) : s)) }));
  }

  function addState() {
    const base = "new-state";
    let n = 1;
    while (flow.states.some((s) => s.name === `${base}-${n}`)) n++;
    const name = `${base}-${n}`;
    setFlow((f) => ({
      ...f,
      states: [...f.states, { name, system_prompt: "", tool_allowlist: [], transitions: [] }],
    }));
    setSelected(name);
  }

  function deleteState(name: string) {
    if (flow.states.length <= 1) {
      toast.error("A flow needs at least one state");
      return;
    }
    setFlow((f) => ({
      name: f.name,
      initial_state: f.initial_state === name ? (f.states.find((s) => s.name !== name)?.name ?? name) : f.initial_state,
      states: f.states
        .filter((s) => s.name !== name)
        .map((s) => ({ ...s, transitions: s.transitions.filter((t) => t.to_state !== name) })),
    }));
    setSelected((cur) => (cur === name ? null : cur));
  }

  function renameState(oldName: string, newName: string) {
    if (!newName || newName === oldName) return;
    if (flow.states.some((s) => s.name === newName)) {
      toast.error(`A state named "${newName}" already exists`);
      return;
    }
    setFlow((f) => ({
      name: f.name,
      initial_state: f.initial_state === oldName ? newName : f.initial_state,
      states: f.states.map((s) => ({
        ...(s.name === oldName ? { ...s, name: newName } : s),
        transitions: s.transitions.map((t) => (t.to_state === oldName ? { ...t, to_state: newName } : t)),
      })),
    }));
    setSelected(newName);
  }

  async function handleSave() {
    setError(null);
    try {
      await invoke("save_flow", { name: flow.name, json: JSON.stringify(flow) });
      setSavedName(flow.name);
      setAvailableFlows((prev) => (prev.includes(flow.name) ? prev : [...prev, flow.name]));
      toast.success(`Saved flow "${flow.name}"`);
    } catch (err) {
      setError(String(err));
      toast.error(String(err));
    }
  }

  async function handleLoad(name: string) {
    setLoadMenuOpen(false);
    setError(null);
    try {
      const loaded = await invoke<FlowDef>("get_flow", { name });
      setFlow(loaded);
      setSavedName(name);
      setSelected(loaded.initial_state);
      setConfirmDeleteFlow(false);
    } catch (err) {
      setError(String(err));
      toast.error(String(err));
    }
  }

  function handleNew() {
    setFlow(blankFlow());
    setSavedName(null);
    setSelected("start");
    setError(null);
    setConfirmDeleteFlow(false);
  }

  async function handleDeleteFlow() {
    if (!savedName) return;
    if (!confirmDeleteFlow) {
      setConfirmDeleteFlow(true);
      return;
    }
    setConfirmDeleteFlow(false);
    try {
      await invoke("delete_flow", { name: savedName });
      setAvailableFlows((prev) => prev.filter((n) => n !== savedName));
      toast.success(`Deleted "${savedName}"`);
      handleNew();
    } catch (err) {
      toast.error(String(err));
    }
  }

  async function handleGenerate() {
    if (!aiPrompt.trim() || aiBusy || !aiModel) return;
    setAiBusy(true);
    setAiDraftRaw(null);
    setAiDraftError(null);
    const isLocal = aiModel.startsWith(LOCAL_MODEL_PREFIX);
    try {
      const text = await invoke<string>("generate_flow_draft", {
        prompt: aiPrompt,
        model: isLocal ? null : aiModel,
        localModel: isLocal ? aiModel.slice(LOCAL_MODEL_PREFIX.length) : null,
      });
      setAiDraftRaw(text);
    } catch (err) {
      toast.error(String(err));
    } finally {
      setAiBusy(false);
    }
  }

  function extractJson(text: string): string {
    const fenced = text.match(/```(?:json)?\s*([\s\S]*?)```/i);
    return (fenced ? fenced[1] : text).trim();
  }

  async function handleInsertDraft() {
    if (!aiDraftRaw) return;
    setAiDraftError(null);
    try {
      const candidate = extractJson(aiDraftRaw);
      const validated = await invoke<FlowDef>("validate_flow_json", { json: candidate });
      setFlow(validated);
      setSavedName(null);
      setSelected(validated.initial_state);
      setAiDraftRaw(null);
      setAiPrompt("");
      toast.success("Flow generated — review it, then Save");
    } catch (err) {
      setAiDraftError(String(err));
    }
  }

  const width = Math.max(600, (Math.max(0, ...[...positions.values()].map((p) => p.col)) + 1) * COL_WIDTH + 80);
  const height = Math.max(400, (Math.max(0, ...[...positions.values()].map((p) => p.row)) + 1) * ROW_HEIGHT + 80);

  const aiModelLabel = aiModel.startsWith(LOCAL_MODEL_PREFIX)
    ? aiModel.slice(LOCAL_MODEL_PREFIX.length)
    : cloudModels.find((m) => m.id === aiModel)?.label ?? "Select a model";

  return (
    <div style={{ display: "flex", flexDirection: "column", height: "100%", minHeight: 0 }}>
      <div
        data-testid="flow-toolbar"
        style={{
          display: "flex",
          alignItems: "center",
          gap: 6,
          padding: "8px 14px",
          flexShrink: 0,
          borderBottom: "1px solid var(--border-soft)",
        }}
      >
        <Input
          data-testid="flow-name-input"
          style={{ flex: "0 0 180px" }}
          value={flow.name}
          onChange={(e) => {
            const value = e.currentTarget.value;
            setFlow((f) => ({ ...f, name: value }));
          }}
        />
        {savedName && flow.name !== savedName && (
          <span style={{ fontSize: 10.5, color: "var(--text-3)" }}>unsaved as new</span>
        )}
        <div className="spacer" />
        <Button data-testid="flow-add-state" onClick={addState} style={{ display: "flex", alignItems: "center", gap: 4 }}>
          <IconPlus size={13} aria-hidden />
          Add state
        </Button>
        <div className="dropdown-wrap">
          <Button
            data-testid="flow-load-btn"
            onClick={() => setLoadMenuOpen((v) => !v)}
            style={{ display: "flex", alignItems: "center", gap: 4 }}
          >
            <IconFolderOpen size={13} aria-hidden />
            Load
          </Button>
          {loadMenuOpen && (
            <div className="option-menu open" style={{ right: "auto", left: 0 }} onMouseLeave={() => setLoadMenuOpen(false)}>
              {availableFlows.length === 0 && <div className="cmdk-empty">No saved flows yet</div>}
              {availableFlows.map((name) => (
                <div key={name} className="option-item" data-flow-name={name} onClick={() => handleLoad(name)}>
                  {name}
                </div>
              ))}
            </div>
          )}
        </div>
        <Button data-testid="flow-new-btn" onClick={handleNew} style={{ display: "flex", alignItems: "center", gap: 4 }}>
          <IconFileText size={13} aria-hidden />
          New
        </Button>
        <Button data-testid="flow-save-btn" onClick={handleSave} style={{ display: "flex", alignItems: "center", gap: 4 }}>
          <IconDeviceFloppy size={13} aria-hidden />
          Save
        </Button>
        {savedName && confirmDeleteFlow && (
          <Button variant="danger" onClick={() => setConfirmDeleteFlow(false)}>
            Cancel
          </Button>
        )}
        {savedName && (
          <IconButton
            icon={IconTrash}
            iconSize={14}
            variant="danger"
            data-testid="flow-delete-btn"
            title={confirmDeleteFlow ? `Really delete "${savedName}"?` : "Delete flow from disk"}
            onClick={handleDeleteFlow}
          />
        )}
      </div>

      {error && <p className="settings-error" style={{ padding: "0 14px" }}>{error}</p>}

      <div style={{ flex: 1, display: "flex", minHeight: 0 }}>
        <div style={{ flex: 1, overflow: "auto", position: "relative" }}>
          <svg width={width} height={height} style={{ position: "absolute", top: 0, left: 0, pointerEvents: "none" }}>
            <defs>
              <marker id="arrow" markerWidth="8" markerHeight="8" refX="7" refY="4" orient="auto">
                <path d="M0,0 L8,4 L0,8 Z" fill="var(--accent)" />
              </marker>
            </defs>
            {flow.states.flatMap((s) =>
              s.transitions.map((t, i) => {
                const from = positions.get(s.name);
                const to = positions.get(t.to_state);
                if (!from || !to) return null;
                const a = nodeCenter(from);
                const b = nodeCenter(to);
                const midX = (a.x + b.x) / 2;
                return (
                  <g key={`${s.name}-${i}`}>
                    <path
                      d={`M${a.x + NODE_WIDTH / 2},${a.y} C${midX},${a.y} ${midX},${b.y} ${b.x - NODE_WIDTH / 2},${b.y}`}
                      fill="none"
                      stroke="rgba(169,194,255,0.45)"
                      strokeWidth={1.5}
                      markerEnd="url(#arrow)"
                    />
                    <text x={midX} y={(a.y + b.y) / 2 - 6} fill="var(--text-2)" fontSize={10} textAnchor="middle">
                      {t.on}
                    </text>
                  </g>
                );
              }),
            )}
          </svg>

          {flow.states.map((s) => {
            const pos = positions.get(s.name);
            if (!pos) return null;
            const isInitial = s.name === flow.initial_state;
            const isTerminal = s.transitions.length === 0;
            return (
              <div
                key={s.name}
                onClick={() => setSelected(s.name)}
                style={{
                  position: "absolute",
                  left: pos.col * COL_WIDTH + 40,
                  top: pos.row * ROW_HEIGHT + 40,
                  width: NODE_WIDTH,
                  minHeight: NODE_HEIGHT,
                  borderRadius: 12,
                  padding: "9px 11px",
                  background: s.name === selected ? "rgba(169,194,255,0.16)" : "var(--bg-2)",
                  cursor: "pointer",
                  fontSize: 12,
                }}
              >
                <div style={{ display: "flex", alignItems: "center", gap: 5 }}>
                  {isInitial ? (
                    <IconFlagFilled size={12} color="var(--accent)" aria-hidden />
                  ) : isTerminal ? (
                    <IconFlag size={12} color="var(--text-3)" aria-hidden />
                  ) : null}
                  <span style={{ fontWeight: 600, color: "var(--text-0)" }}>{s.name}</span>
                </div>
                <div
                  style={{
                    marginTop: 4,
                    color: "var(--text-3)",
                    fontSize: 10.5,
                    overflow: "hidden",
                    textOverflow: "ellipsis",
                    display: "-webkit-box",
                    WebkitLineClamp: 2,
                    WebkitBoxOrient: "vertical",
                  }}
                >
                  {s.system_prompt || "(no system prompt)"}
                </div>
              </div>
            );
          })}
        </div>

        {selectedState && (
          <div
            data-testid="flow-side-panel"
            style={{ width: 300, flexShrink: 0, borderLeft: "1px solid var(--border-soft)", padding: 14, overflowY: "auto" }}
          >
            <div className="settings-section-title" style={{ marginTop: 0 }}>
              State
            </div>
            <Input
              data-testid="flow-state-name-input"
              style={{ marginBottom: 8, width: "100%" }}
              value={selectedState.name}
              onChange={(e) => renameState(selectedState.name, e.currentTarget.value)}
            />
            <div className="form-row">
              <Button
                style={{ flex: 1, textAlign: "center" }}
                onClick={() => setFlow((f) => ({ ...f, initial_state: selectedState.name }))}
              >
                {flow.initial_state === selectedState.name ? "Is start state" : "Set as start"}
              </Button>
              <IconButton icon={IconTrash} iconSize={14} variant="danger" onClick={() => deleteState(selectedState.name)} />
            </div>

            <div className="settings-section-title">System prompt</div>
            <Textarea
              style={{ width: "100%", minHeight: 90, resize: "vertical" }}
              value={selectedState.system_prompt}
              onChange={(e) => {
                const value = e.currentTarget.value;
                updateState(selectedState.name, (s) => ({ ...s, system_prompt: value }));
              }}
            />

            <div className="settings-section-title">Tool allowlist</div>
            <Input
              style={{ width: "100%" }}
              placeholder="comma-separated tool names"
              value={selectedState.tool_allowlist.join(", ")}
              onChange={(e) => {
                const list = e.currentTarget.value
                  .split(",")
                  .map((t) => t.trim())
                  .filter(Boolean);
                updateState(selectedState.name, (s) => ({ ...s, tool_allowlist: list }));
              }}
            />

            <div className="settings-section-title">Transitions</div>
            {selectedState.transitions.map((t, i) => (
              <div className="form-row" key={i}>
                <Input
                  placeholder="on"
                  value={t.on}
                  onChange={(e) => {
                    const value = e.currentTarget.value;
                    updateState(selectedState.name, (s) => ({
                      ...s,
                      transitions: s.transitions.map((tt, idx) => (idx === i ? { ...tt, on: value } : tt)),
                    }));
                  }}
                />
                <Select
                  value={t.to_state}
                  onChange={(e) => {
                    const value = e.currentTarget.value;
                    updateState(selectedState.name, (s) => ({
                      ...s,
                      transitions: s.transitions.map((tt, idx) => (idx === i ? { ...tt, to_state: value } : tt)),
                    }));
                  }}
                >
                  {flow.states.map((st) => (
                    <option key={st.name} value={st.name}>
                      {st.name}
                    </option>
                  ))}
                </Select>
                <IconButton
                  icon={IconTrash}
                  iconSize={13}
                  variant="danger"
                  onClick={() =>
                    updateState(selectedState.name, (s) => ({
                      ...s,
                      transitions: s.transitions.filter((_, idx) => idx !== i),
                    }))
                  }
                />
              </div>
            ))}
            <Button
              style={{ textAlign: "center" }}
              onClick={() =>
                updateState(selectedState.name, (s) => ({
                  ...s,
                  transitions: [...s.transitions, { on: "message", to_state: flow.states[0]?.name ?? s.name }],
                }))
              }
            >
              + Add transition
            </Button>
          </div>
        )}
      </div>

      <div style={{ borderTop: "1px solid var(--border-soft)", padding: "10px 14px", flexShrink: 0 }}>
        {aiDraftRaw && (
          <div style={{ background: "var(--bg-2)", borderRadius: 10, padding: 10, marginBottom: 8, fontSize: 11.5 }}>
            <pre style={{ whiteSpace: "pre-wrap", margin: 0, maxHeight: 160, overflowY: "auto", color: "var(--text-1)" }}>
              {aiDraftRaw}
            </pre>
            {aiDraftError && <p className="settings-error">{aiDraftError}</p>}
            <div style={{ display: "flex", gap: 6, marginTop: 8 }}>
              <Button data-testid="flow-ai-insert" onClick={handleInsertDraft}>
                Insert into canvas
              </Button>
              <Button onClick={() => setAiDraftRaw(null)}>Discard</Button>
            </div>
          </div>
        )}
        <div className="composer" style={{ padding: "8px 10px" }}>
          <textarea
            ref={aiPromptRef}
            data-testid="flow-ai-textarea"
            className="input"
            rows={1}
            placeholder="Describe the flow you want, e.g. 'a support triage flow that routes billing vs technical questions'"
            value={aiPrompt}
            onChange={(e) => setAiPrompt(e.currentTarget.value)}
            onKeyDown={(e) => {
              if (e.key === "Enter" && !e.shiftKey) {
                e.preventDefault();
                handleGenerate();
              }
            }}
          />
          <div className="composer-row">
            <IconSparkles size={14} aria-hidden style={{ color: "var(--accent)" }} />
            <span style={{ fontSize: 11, color: "var(--text-3)" }}>Generate with AI</span>
            <div className="spacer" />
            <div className="dropdown-wrap">
              <div className="dropdown" onClick={() => setAiModelMenuOpen((v) => !v)}>
                <span>{aiModelLabel}</span>
                <IconChevronDown size={12} aria-hidden />
              </div>
              {aiModelMenuOpen && (
                <div className="option-menu open" onMouseLeave={() => setAiModelMenuOpen(false)}>
                  {chatLocalModels.map((m) => (
                    <div
                      key={m.name}
                      className={`option-item${aiModel === `${LOCAL_MODEL_PREFIX}${m.name}` ? " sel" : ""}`}
                      onClick={() => {
                        setAiModel(`${LOCAL_MODEL_PREFIX}${m.name}`);
                        setAiModelMenuOpen(false);
                      }}
                    >
                      {m.name}
                      <span className="option-sub">{m.loaded ? "loaded" : ""}</span>
                    </div>
                  ))}
                  {Array.from(cloudGroups.entries()).map(([provider, models]) => (
                    <div key={provider}>
                      <div className="cmdk-group-label" style={{ padding: "4px 8px" }}>
                        {provider.toUpperCase()}
                      </div>
                      {models.map((m) => (
                        <div
                          key={m.id}
                          className={`option-item${aiModel === m.id ? " sel" : ""}`}
                          onClick={() => {
                            setAiModel(m.id);
                            setAiModelMenuOpen(false);
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
              data-testid="flow-ai-send"
              className={`send${!aiPrompt.trim() || aiBusy || !aiModel ? " disabled" : ""}`}
              onClick={handleGenerate}
            >
              <IconSend size={14} aria-hidden />
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}

export default FlowEditor;
