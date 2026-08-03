import { Command } from "cmdk";
import {
  IconMessageCircle,
  IconBox,
  IconSettings,
  IconBolt,
  IconPlus,
  IconTopologyStar3,
  IconFileText,
  IconLayoutSidebar,
} from "@tabler/icons-react";
import { useShellStore } from "../store/shellStore";
import { Overlay } from "../components/ui";
import type { ConversationSummary, LocalModelInfo } from "../types";

interface CommandPaletteProps {
  conversations: ConversationSummary[];
  localModels: LocalModelInfo[];
  onSelectConversation: (id: string) => void;
  onNewChat: () => void;
}

const SETTINGS_PANES = ["models", "providers", "tools", "flows"] as const;

function CommandPalette({ conversations, localModels, onSelectConversation, onNewChat }: CommandPaletteProps) {
  const { cmdkOpen, setCmdkOpen, openSettings, toggleSidebar, openExtraTab } = useShellStore();

  if (!cmdkOpen) return null;

  function run(fn: () => void) {
    fn();
    setCmdkOpen(false);
  }

  return (
    <Overlay className="cmdk-overlay" onClose={() => setCmdkOpen(false)}>
      <div className="cmdk-card">
        <Command label="Command palette" shouldFilter>
          <div className="cmdk-input-row">
            <IconMessageCircle size={15} aria-hidden style={{ opacity: 0 }} />
            <Command.Input autoFocus placeholder="Search conversations, models, settings, commands..." />
          </div>
          <Command.List>
            <Command.Empty>No results found.</Command.Empty>

            <Command.Group heading="Conversations">
              {conversations.map((c) => (
                <Command.Item key={c.id} onSelect={() => run(() => onSelectConversation(c.id))}>
                  <IconMessageCircle aria-hidden />
                  {c.title || "Untitled"}
                </Command.Item>
              ))}
            </Command.Group>

            {localModels.length > 0 && (
              <Command.Group heading="Models">
                {localModels.map((m) => (
                  <Command.Item key={m.name} onSelect={() => setCmdkOpen(false)}>
                    <IconBox aria-hidden />
                    {m.name}
                  </Command.Item>
                ))}
              </Command.Group>
            )}

            <Command.Group heading="Settings">
              {SETTINGS_PANES.map((p) => (
                <Command.Item key={p} onSelect={() => run(() => openSettings(p))}>
                  <IconSettings aria-hidden />
                  {p[0].toUpperCase() + p.slice(1)}
                </Command.Item>
              ))}
            </Command.Group>

            <Command.Group heading="Commands">
              <Command.Item onSelect={() => run(onNewChat)}>
                <IconPlus aria-hidden />
                New chat
              </Command.Item>
              <Command.Item onSelect={() => run(() => openExtraTab("flow"))}>
                <IconTopologyStar3 aria-hidden />
                Open flow editor
              </Command.Item>
              <Command.Item onSelect={() => run(() => openExtraTab("text"))}>
                <IconFileText aria-hidden />
                New text file
              </Command.Item>
              <Command.Item onSelect={() => run(toggleSidebar)}>
                <IconLayoutSidebar aria-hidden />
                Toggle sidebar
              </Command.Item>
              <Command.Item onSelect={() => run(() => useShellStore.getState().setOnboardingOpen(true))}>
                <IconBolt aria-hidden />
                Show welcome guide
              </Command.Item>
            </Command.Group>
          </Command.List>
        </Command>
      </div>
    </Overlay>
  );
}

export default CommandPalette;
