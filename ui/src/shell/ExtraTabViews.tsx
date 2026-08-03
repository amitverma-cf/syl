import { writeTextFile } from "@tauri-apps/plugin-fs";
import { toast } from "sonner";
import { useShellStore } from "../store/shellStore";

export function TextTabView({ tabId }: { tabId: string }) {
  const { textDocs, setTextDoc, extraTabs } = useShellStore();
  const filePath = extraTabs[tabId]?.filePath;

  async function handleKeyDown(e: React.KeyboardEvent<HTMLTextAreaElement>) {
    if ((e.metaKey || e.ctrlKey) && e.key === "s") {
      e.preventDefault();
      if (!filePath) return;
      try {
        await writeTextFile(filePath, textDocs[tabId] ?? "");
        toast.success(`Saved ${extraTabs[tabId].title}`);
      } catch (err) {
        toast.error(String(err));
      }
    }
  }

  return (
    <textarea
      className="text-editor"
      value={textDocs[tabId] ?? ""}
      onChange={(e) => setTextDoc(tabId, e.currentTarget.value)}
      onKeyDown={handleKeyDown}
      spellCheck={false}
    />
  );
}
