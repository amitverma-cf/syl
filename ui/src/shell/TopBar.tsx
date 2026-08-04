import { useEffect, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import {
  IconMenu2,
  IconLayoutSidebar,
  IconSearch,
  IconPlus,
  IconMessageCircle,
  IconX,
  IconChevronLeft,
  IconChevronRight,
  IconMinus,
  IconSquare,
  IconTopologyStar3,
  IconFileText,
} from "@tabler/icons-react";
import { useShellStore } from "../store/shellStore";
import type { ConversationSummary } from "../types";
import { useTabScroll } from "./useTabScroll";
import { useContributions } from "../hooks/useContributions";

// Icons for known sidebar-view contribution ids. An extension whose id isn't
// listed here still renders (falls back to a generic icon) — this map only
// controls which icon shows, not whether the entry appears at all.
const SIDEBAR_VIEW_ICONS: Record<string, typeof IconTopologyStar3> = {
  "flow-editor": IconTopologyStar3,
};

// Handlers for known sidebar-view contribution ids, supplied by the caller
// via props below. An extension without a matching handler still renders,
// it just does nothing on click yet — matches how a real future extension
// with no host-side action bound to it would behave.

const isTauri = typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
const appWindow = isTauri ? getCurrentWindow() : null;

interface TopBarProps {
  conversations: ConversationSummary[];
  onNewChat: () => void;
  onOpenFlowEditor: () => void;
}

function tabIcon(type: string) {
  switch (type) {
    case "flow":
      return <IconTopologyStar3 aria-hidden />;
    case "text":
      return <IconFileText aria-hidden />;
    default:
      return <IconMessageCircle aria-hidden />;
  }
}

function TopBar({ conversations, onNewChat, onOpenFlowEditor }: TopBarProps) {
  const {
    platform,
    toggleSidebar,
    appMenuOpen,
    setAppMenuOpen,
    setCmdkOpen,
    openTabs,
    activeTab,
    extraTabs,
    setActiveTab,
    closeTab,
    setOnboardingOpen,
  } = useShellStore();

  const menuRef = useRef<HTMLDivElement>(null);
  const addMenuRef = useRef<HTMLDivElement>(null);
  const { scrollRef, showLeft, showRight, scrollByStep } = useTabScroll(openTabs.length);
  const contributions = useContributions();
  const sidebarViewContributions = contributions.filter((c) => c.kind === "sidebarView");
  const sidebarViewHandlers: Record<string, () => void> = {
    "flow-editor": onOpenFlowEditor,
  };

  useEffect(() => {
    function onDocClick(e: MouseEvent) {
      if (menuRef.current && !menuRef.current.contains(e.target as Node)) setAppMenuOpen(false);
    }
    document.addEventListener("click", onDocClick);
    return () => document.removeEventListener("click", onDocClick);
  }, [setAppMenuOpen]);

  function tabLabel(id: string) {
    if (extraTabs[id]) return extraTabs[id].title;
    return conversations.find((c) => c.id === id)?.title || "Untitled";
  }
  function tabType(id: string) {
    return extraTabs[id]?.type ?? "chat";
  }

  return (
    <div className="app-topbar" data-tauri-drag-region>
      <div ref={menuRef} style={{ position: "relative" }}>
        <div className="topbar-btn" onClick={() => setAppMenuOpen(!appMenuOpen)} title="Menu">
          <IconMenu2 size={16} aria-hidden />
        </div>
        <div className={`app-menu${appMenuOpen ? " open" : ""}`}>
          <div
            className="app-menu-item"
            onClick={() => {
              setAppMenuOpen(false);
              setCmdkOpen(true);
            }}
          >
            Command palette <span style={{ float: "right", opacity: 0.5 }}>⌘K</span>
          </div>
          <div
            className="app-menu-item"
            onClick={() => {
              setAppMenuOpen(false);
              setOnboardingOpen(true);
            }}
          >
            Show welcome guide
          </div>
          <div
            className="app-menu-item"
            data-testid="app-menu-quit"
            onClick={() => {
              setAppMenuOpen(false);
              invoke("quit_app");
            }}
          >
            Quit
          </div>
        </div>
      </div>

      <div className="topbar-btn" onClick={toggleSidebar} title="Toggle sidebar">
        <IconLayoutSidebar size={16} aria-hidden />
      </div>

      <div className="app-title">syl</div>

      <div className="topbar-btn" onClick={() => setCmdkOpen(true)} title="Search (⌘K)">
        <IconSearch size={15} aria-hidden />
      </div>

      <div ref={addMenuRef} style={{ position: "relative" }}>
        <div className="topbar-btn" onClick={onNewChat} title="New chat">
          <IconPlus size={16} aria-hidden />
        </div>
      </div>

      {sidebarViewContributions.map((contribution) => {
        const Icon = SIDEBAR_VIEW_ICONS[contribution.id] ?? IconTopologyStar3;
        const handler = sidebarViewHandlers[contribution.id];
        return (
          <div
            key={contribution.id}
            className="topbar-btn"
            onClick={handler}
            title={`Open ${contribution.title.toLowerCase()}`}
          >
            <Icon size={15} aria-hidden />
          </div>
        );
      })}

      <div className="tab-scroll">
        {showLeft && (
          <div className="tab-chevron" onClick={() => scrollByStep(-1)}>
            <IconChevronLeft size={14} aria-hidden />
          </div>
        )}
        <div className="tabbar" ref={scrollRef}>
          {openTabs.map((id) => (
            <div
              key={id}
              className={`chat-tab${activeTab === id ? " active" : ""}`}
              onClick={() => setActiveTab(id)}
              title={tabLabel(id)}
            >
              {tabIcon(tabType(id))}
              <span className="label">{tabLabel(id)}</span>
              <span
                className="tab-close"
                onClick={(e) => {
                  e.stopPropagation();
                  closeTab(id);
                }}
              >
                <IconX size={11} aria-hidden />
              </span>
            </div>
          ))}
        </div>
        {showRight && (
          <div className="tab-chevron" onClick={() => scrollByStep(1)}>
            <IconChevronRight size={14} aria-hidden />
          </div>
        )}
      </div>

      <div className="win-controls">
        {platform === "mac" && (
          <div className="traffic">
            <div className="dot close" onClick={() => appWindow?.close()} />
            <div className="dot min" onClick={() => appWindow?.minimize()} />
            <div className="dot max" onClick={() => appWindow?.toggleMaximize()} />
          </div>
        )}
        <div className="win-buttons">
          <div className="win-btn" onClick={() => appWindow?.minimize()}>
            <IconMinus size={13} aria-hidden />
          </div>
          <div className="win-btn" onClick={() => appWindow?.toggleMaximize()}>
            <IconSquare size={11} aria-hidden />
          </div>
          <div className="win-btn close-btn" onClick={() => appWindow?.close()}>
            <IconX size={13} aria-hidden />
          </div>
        </div>
      </div>
    </div>
  );
}

export default TopBar;
