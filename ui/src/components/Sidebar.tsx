import type { ConversationSummary } from "../types";

interface SidebarProps {
  conversations: ConversationSummary[];
  activeConversationId: string | null;
  onSelect: (id: string) => void;
  onNewChat: () => void;
  onOpenSettings: () => void;
}

function Sidebar({
  conversations,
  activeConversationId,
  onSelect,
  onNewChat,
  onOpenSettings,
}: SidebarProps) {
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
          <button
            key={c.id}
            onClick={() => onSelect(c.id)}
            className={`mb-1 w-full truncate rounded px-3 py-2 text-left text-sm ${
              c.id === activeConversationId
                ? "bg-neutral-800 text-neutral-100"
                : "text-neutral-400 hover:bg-neutral-900 hover:text-neutral-200"
            }`}
            title={c.title}
          >
            {c.title || "Untitled"}
          </button>
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
