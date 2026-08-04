import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Button } from "../ui";
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
    <div>
      <div className="settings-section-title" style={{ marginTop: 0 }}>
        Flow templates
      </div>
      <p style={{ fontSize: 11.5, color: "var(--text-3)", margin: "0 0 10px", lineHeight: 1.5 }}>
        Flow files in <code>.syl/flows/</code>. Selecting one here switches the currently open
        conversation to that flow.
      </p>
      {!activeConversationId && (
        <p style={{ fontSize: 12, color: "var(--text-3)" }}>Open a conversation to change its flow.</p>
      )}
      {activeFlow && (
        <p style={{ fontSize: 12.5, color: "var(--text-1)", margin: "0 0 10px" }}>
          Active: <code>{activeFlow.flowName}</code> in state{" "}
          <code style={{ color: "var(--accent)" }}>{activeFlow.stateName}</code>
        </p>
      )}
      <div style={{ display: "flex", flexWrap: "wrap", gap: 8 }}>
        {flows.length === 0 && <p style={{ fontSize: 12, color: "var(--text-3)" }}>No flows in .syl/flows yet.</p>}
        {flows.map((name) => (
          <Button key={name} onClick={() => handleLoad(name)} disabled={!activeConversationId}>
            Use {name}
          </Button>
        ))}
      </div>
      {error && <p className="settings-error">{error}</p>}
    </div>
  );
}

export default FlowTemplatesTab;
