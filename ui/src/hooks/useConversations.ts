import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { ConversationSummary } from "../types";

export function useConversations(onError: (message: string) => void) {
  const [conversations, setConversations] = useState<ConversationSummary[]>([]);
  const [activeConversationId, setActiveConversationId] = useState<string | null>(null);

  function refresh() {
    invoke<ConversationSummary[]>("list_conversations")
      .then((list) => {
        setConversations(list);
        setActiveConversationId((current) => current ?? list[0]?.id ?? null);
      })
      .catch((err) => onError(String(err)));
  }

  useEffect(refresh, []);

  async function newChat() {
    const id = crypto.randomUUID();
    try {
      await invoke("create_conversation", { id, title: "New chat" });
      setActiveConversationId(id);
      refresh();
    } catch (err) {
      onError(String(err));
    }
  }

  function handleDeleted(id: string) {
    setConversations((prev) => prev.filter((c) => c.id !== id));
    setActiveConversationId((current) => (current === id ? null : current));
  }

  return {
    conversations,
    activeConversationId,
    setActiveConversationId,
    refresh,
    newChat,
    handleDeleted,
  };
}
