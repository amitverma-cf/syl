import { formatBytes, type ConversationSummary, type SystemStats } from "../../types";

interface MemoryTabProps {
  stats: SystemStats | null;
  conversations: ConversationSummary[];
}

function MemoryTab({ stats, conversations }: MemoryTabProps) {
  return (
    <div className="flex flex-col gap-2">
      <h2 className="text-sm font-medium">Memory</h2>
      <p className="text-xs text-neutral-500">
        Conversation history and embeddings are stored in a single SQLite database under
        <span className="font-mono"> .syl/memory/conversations.sqlite</span>.
      </p>
      <div className="grid grid-cols-2 gap-2 text-sm">
        <div className="rounded border border-neutral-800 px-3 py-2">
          <span className="text-xs text-neutral-500">Conversations</span>
          <p className="font-mono">{conversations.length}</p>
        </div>
        <div className="rounded border border-neutral-800 px-3 py-2">
          <span className="text-xs text-neutral-500">Workspace disk usage</span>
          <p className="font-mono">{stats ? formatBytes(stats.workspaceDiskBytes) : "—"}</p>
        </div>
      </div>
    </div>
  );
}

export default MemoryTab;
