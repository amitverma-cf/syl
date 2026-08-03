import { formatBytes, type ConversationSummary, type SystemStats } from "../../types";

interface MemoryTabProps {
  stats: SystemStats | null;
  conversations: ConversationSummary[];
}

function MemoryTab({ stats, conversations }: MemoryTabProps) {
  return (
    <div>
      <div className="settings-section-title" style={{ marginTop: 0 }}>
        Storage
      </div>
      <p style={{ fontSize: 11.5, color: "var(--text-3)", margin: "0 0 10px", lineHeight: 1.5 }}>
        Conversation history and embeddings are stored in a single SQLite database under{" "}
        <code>.syl/memory/conversations.sqlite</code>.
      </p>
      <div className="model-row">
        <div>
          <div className="name">Conversations</div>
          <div className="kind">stored in the local database</div>
        </div>
        <span className="pill">{conversations.length}</span>
      </div>
      <div className="model-row">
        <div>
          <div className="name">Workspace disk usage</div>
          <div className="kind">.syl workspace directory</div>
        </div>
        <span className="pill">{stats ? formatBytes(stats.workspaceDiskBytes) : "—"}</span>
      </div>
    </div>
  );
}

export default MemoryTab;
