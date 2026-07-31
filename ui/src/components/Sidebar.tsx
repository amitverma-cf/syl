import { invoke } from "@tauri-apps/api/core";
import type { ConversationSummary } from "../types";

interface SidebarProps {
  conversations: ConversationSummary[];
  activeConversationId: string | null;
  onSelect: (id: string) => void;
  onNewChat: () => void;
  onOpenSettings: () => void;
  onDeleted: (id: string) => void;
}

function Sidebar({
  conversations,
  activeConversationId,
  onSelect,
  onNewChat,
  onOpenSettings,
  onDeleted,
}: SidebarProps) {
  async function handleDelete(e: React.MouseEvent, id: string) {
    e.stopPropagation();
    if (!confirm("Delete this conversation?")) return;
    await invoke("delete_conversation", { id });
    onDeleted(id);
  }

  return (
    <aside className="flex h-full w-64 shrink-0 flex-col border-r border-neutral-800 bg-neutral-950">
      <div className="p-3">
        <button
          onClick={onNewChat}
          className="w-full rounded bg-neutral-100 px-3 py-2 text-sm font-medium text-neutral-950"
        >
          + New chat
        </button>
      </div>

      <div className="flex-1 overflow-auto px-2">
        {conversations.length === 0 && (
          <p className="px-2 py-4 text-sm text-neutral-500">No conversations yet.</p>
        )}
        {conversations.map((c) => (
          <div
            key={c.id}
            onClick={() => onSelect(c.id)}
            className={`group mb-1 flex w-full items-center justify-between rounded px-3 py-2 text-left text-sm ${
              c.id === activeConversationId
                ? "bg-neutral-800 text-neutral-100"
                : "text-neutral-400 hover:bg-neutral-900 hover:text-neutral-200"
            }`}
          >
            <button className="min-w-0 flex-1 truncate text-left" title={c.title}>
              {c.title || "Untitled"}
            </button>
            <button
              onClick={(e) => handleDelete(e, c.id)}
              className="ml-2 hidden shrink-0 text-neutral-500 hover:text-red-400 group-hover:block"
              title="Delete conversation"
            >
              ✕
            </button>
          </div>
        ))}
      </div>

      <div className="border-t border-neutral-800 p-2">
        <button
          onClick={onOpenSettings}
          className="w-full rounded px-3 py-2 text-left text-sm text-neutral-400 hover:bg-neutral-900 hover:text-neutral-200"
        >
          ⚙ Settings
        </button>
      </div>
    </aside>
  );
}

export default Sidebar;
