import { useState } from "react";
import { writeTextFile } from "@tauri-apps/plugin-fs";
import { toast } from "sonner";
import { IconArrowLeft, IconArrowRight, IconRefresh } from "@tabler/icons-react";
import { useShellStore } from "../store/shellStore";
import FlowTemplatesTab from "../components/settings/FlowTemplatesTab";

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

export function BrowserTabView({ tabId }: { tabId: string }) {
  const { browserUrls, setBrowserUrl } = useShellStore();
  const [draft, setDraft] = useState(browserUrls[tabId] ?? "");
  const url = browserUrls[tabId] ?? "";

  function normalize(value: string) {
    if (!value.trim()) return "";
    if (/^https?:\/\//i.test(value)) return value;
    return `https://${value}`;
  }

  function go() {
    setBrowserUrl(tabId, normalize(draft));
  }

  return (
    <div style={{ display: "flex", flexDirection: "column", height: "100%" }}>
      <div style={{ display: "flex", gap: 4, padding: "8px 10px", flexShrink: 0 }}>
        <div className="header-icon-btn" onClick={() => history.back()}>
          <IconArrowLeft size={15} aria-hidden />
        </div>
        <div className="header-icon-btn" onClick={() => history.forward()}>
          <IconArrowRight size={15} aria-hidden />
        </div>
        <div className="header-icon-btn" onClick={() => setDraft((d) => d)}>
          <IconRefresh size={15} aria-hidden />
        </div>
        <input
          className="header-address"
          value={draft}
          onChange={(e) => setDraft(e.currentTarget.value)}
          onKeyDown={(e) => e.key === "Enter" && go()}
          placeholder="Enter a URL and press Enter"
        />
      </div>
      <div className="browser-preview" style={{ flex: 1 }}>
        {url ? (
          <iframe src={url} title="browser-tab" style={{ width: "100%", height: "100%", border: "none" }} />
        ) : (
          <span>Type a URL above to browse</span>
        )}
      </div>
    </div>
  );
}

export function FlowTabView({ activeConversationId }: { activeConversationId: string | null }) {
  return (
    <div style={{ padding: "16px 18px", overflowY: "auto", flex: 1 }}>
      <FlowTemplatesTab activeConversationId={activeConversationId} />
    </div>
  );
}
