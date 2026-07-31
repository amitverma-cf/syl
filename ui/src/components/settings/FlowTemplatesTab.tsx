import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { FlowStateInfo } from "../../types";

interface FlowTemplatesTabProps {
  activeConversationId: string | null;
}

function FlowTemplatesTab({ activeConversationId }: FlowTemplatesTabProps) {
  const [flows, setFlows] = useState<string[]>([]);
  const [activeFlow, setActiveFlow] = useState<FlowStateInfo | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    invoke<string[]>("list_flows")
      .then(setFlows)
      .catch((err) => setError(String(err)));
  }, []);

  useEffect(() => {
    if (!activeConversationId) return;
    invoke<FlowStateInfo | null>("flow_status", { conversationId: activeConversationId })
      .then(setActiveFlow)
      .catch((err) => setError(String(err)));
  }, [activeConversationId]);

  async function handleLoad(name: string) {
    if (!activeConversationId) return;
    setError(null);
    try {
      const info = await invoke<FlowStateInfo>("load_flow", {
        conversationId: activeConversationId,
        name,
      });
      setActiveFlow(info);
    } catch (err) {
      setError(String(err));
    }
  }

  return (
    <div className="flex flex-col gap-2">
      <h2 className="text-sm font-medium">Flow templates</h2>
      <p className="text-xs text-neutral-500">
        Flow files in <span className="font-mono">.syl/flows/</span>. Selecting one here switches
        the currently open conversation to that flow.
      </p>
      {!activeConversationId && (
        <p className="text-sm text-neutral-500">Open a conversation to change its flow.</p>
      )}
      {activeFlow && (
        <p className="text-sm">
          Active: <span className="font-mono">{activeFlow.flowName}</span> in state{" "}
          <span className="font-mono text-emerald-400">{activeFlow.stateName}</span>
        </p>
      )}
      <div className="flex flex-wrap gap-2">
        {flows.length === 0 && <p className="text-sm text-neutral-500">No flows in .syl/flows yet.</p>}
        {flows.map((name) => (
          <button
            key={name}
            onClick={() => handleLoad(name)}
            disabled={!activeConversationId}
            className="rounded bg-neutral-100 px-2 py-1 text-xs font-medium text-neutral-950 disabled:opacity-50"
          >
            Use {name}
          </button>
        ))}
      </div>
      {error && <p className="text-sm text-red-400">{error}</p>}
    </div>
  );
}

export default FlowTemplatesTab;
