import { useRef, useState } from "react";
import { readDir, readTextFile, writeTextFile, mkdir } from "@tauri-apps/plugin-fs";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import {
  IconMessages,
  IconFolder,
  IconFolderOpen,
  IconFolderPlus,
  IconFile,
  IconFilePlus,
  IconTrash,
  IconSettings,
  IconRefresh,
  IconChevronRight,
  IconChevronDown,
  IconLayoutSidebarLeftCollapse,
} from "@tabler/icons-react";
import { useShellStore } from "../store/shellStore";
import type { ConversationSummary } from "../types";

const isTauri = typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

interface TreeNode {
  name: string;
  path: string;
  isDir: boolean;
  expanded: boolean;
  children: TreeNode[] | null;
}

function joinPath(base: string, name: string): string {
  const sep = base.includes("\\") && !base.includes("/") ? "\\" : "/";
  return base.endsWith(sep) ? `${base}${name}` : `${base}${sep}${name}`;
}

async function loadDir(path: string): Promise<TreeNode[]> {
  const entries = await readDir(path);
  return entries
    .map((e) => ({
      name: e.name ?? "",
      path: joinPath(path, e.name ?? ""),
      isDir: e.isDirectory,
      expanded: false,
      children: null as TreeNode[] | null,
    }))
    .sort((a, b) => {
      if (a.isDir !== b.isDir) return a.isDir ? -1 : 1;
      return a.name.localeCompare(b.name);
    });
}

function updateNode(nodes: TreeNode[], path: string, fn: (n: TreeNode) => TreeNode): TreeNode[] {
  return nodes.map((n) => {
    if (n.path === path) return fn(n);
    if (n.children) return { ...n, children: updateNode(n.children, path, fn) };
    return n;
  });
}

interface SidebarShellProps {
  conversations: ConversationSummary[];
  activeConversationId: string | null;
  onSelect: (id: string) => void;
  onDelete: (id: string) => void;
}

function SidebarShell({ conversations, activeConversationId, onSelect, onDelete }: SidebarShellProps) {
  const { sidebarTab, setSidebarTab, sidebarWidth, setSidebarWidth, sidebarCollapsed, openSettings, openExtraTab } =
    useShellStore();
  const [rootPath, setRootPath] = useState<string | null>(null);
  const [tree, setTree] = useState<TreeNode[]>([]);
  const [activeDir, setActiveDir] = useState<string | null>(null);
  const [folderError, setFolderError] = useState<string | null>(null);
  const resizing = useRef(false);

  async function openFolder() {
    if (!isTauri) {
      setFolderError("Folder browsing requires the desktop app (not available in a plain browser preview).");
      return;
    }
    try {
      const selected = await openDialog({ directory: true, multiple: false });
      if (!selected || Array.isArray(selected)) return;
      setRootPath(selected);
      setActiveDir(selected);
      setFolderError(null);
      setTree(await loadDir(selected));
    } catch (err) {
      setFolderError(String(err));
    }
  }

  async function toggleDir(node: TreeNode) {
    if (!node.expanded && node.children === null) {
      try {
        const children = await loadDir(node.path);
        setTree((prev) => updateNode(prev, node.path, (n) => ({ ...n, expanded: true, children })));
      } catch (err) {
        setFolderError(String(err));
      }
    } else {
      setTree((prev) => updateNode(prev, node.path, (n) => ({ ...n, expanded: !n.expanded })));
    }
    setActiveDir(node.path);
  }

  async function refreshTree() {
    if (!rootPath) return;
    async function refreshLevel(nodes: TreeNode[]): Promise<TreeNode[]> {
      return Promise.all(
        nodes.map(async (n) => {
          if (n.isDir && n.expanded) {
            const children = await loadDir(n.path);
            const prevByPath = new Map((n.children ?? []).map((c) => [c.path, c]));
            const merged = children.map((c) => (prevByPath.get(c.path)?.expanded ? { ...c, expanded: true } : c));
            return { ...n, children: await refreshLevel(merged) };
          }
          return n;
        }),
      );
    }
    try {
      setTree(await refreshLevel(tree.length ? tree : await loadDir(rootPath)));
    } catch (err) {
      setFolderError(String(err));
    }
  }

  function collapseAll() {
    function collapse(nodes: TreeNode[]): TreeNode[] {
      return nodes.map((n) => ({ ...n, expanded: false, children: n.children ? collapse(n.children) : n.children }));
    }
    setTree((prev) => collapse(prev));
  }

  async function openFile(node: TreeNode) {
    try {
      const content = await readTextFile(node.path);
      openExtraTab("text", node.name, content, node.path);
    } catch (err) {
      setFolderError(String(err));
    }
  }

  async function createFile() {
    const dir = activeDir ?? rootPath;
    if (!dir) return;
    const name = window.prompt("New file name:");
    if (!name) return;
    try {
      const path = joinPath(dir, name);
      await writeTextFile(path, "");
      await refreshTree();
      openExtraTab("text", name, "", path);
    } catch (err) {
      setFolderError(String(err));
    }
  }

  async function createFolder() {
    const dir = activeDir ?? rootPath;
    if (!dir) return;
    const name = window.prompt("New folder name:");
    if (!name) return;
    try {
      await mkdir(joinPath(dir, name));
      await refreshTree();
    } catch (err) {
      setFolderError(String(err));
    }
  }

  function startResize(e: React.PointerEvent) {
    e.preventDefault();
    resizing.current = true;
    const startX = e.clientX;
    const startWidth = sidebarWidth;
    function onMove(ev: PointerEvent) {
      if (!resizing.current) return;
      setSidebarWidth(startWidth + (ev.clientX - startX));
    }
    function onUp() {
      resizing.current = false;
      window.removeEventListener("pointermove", onMove);
      window.removeEventListener("pointerup", onUp);
    }
    window.addEventListener("pointermove", onMove);
    window.addEventListener("pointerup", onUp);
  }

  function renderTree(nodes: TreeNode[], depth: number) {
    return nodes.map((node) => (
      <div key={node.path}>
        <div
          className={`folder-item${node.isDir ? " dir" : ""}`}
          style={{ marginLeft: depth * 14 }}
          onClick={() => (node.isDir ? toggleDir(node) : openFile(node))}
        >
          {node.isDir ? (
            node.expanded ? (
              <IconChevronDown size={12} aria-hidden />
            ) : (
              <IconChevronRight size={12} aria-hidden />
            )
          ) : (
            <span style={{ width: 12, flexShrink: 0 }} />
          )}
          {node.isDir ? <IconFolder size={14} aria-hidden /> : <IconFile size={14} aria-hidden />}
          {node.name}
        </div>
        {node.isDir && node.expanded && node.children && renderTree(node.children, depth + 1)}
      </div>
    ));
  }

  return (
    <>
      <div
        className={`sidebar${sidebarCollapsed ? " collapsed" : ""}`}
        style={{ width: sidebarCollapsed ? 0 : sidebarWidth }}
      >
        <div className="sidebar-head">
          <div className="sidebar-tabs">
            <div
              className={`sidebar-tab${sidebarTab === "chats" ? " active" : ""}`}
              onClick={() => setSidebarTab("chats")}
            >
              <IconMessages size={14} aria-hidden />
              Chats
            </div>
            <div
              className={`sidebar-tab${sidebarTab === "folder" ? " active" : ""}`}
              onClick={() => setSidebarTab("folder")}
            >
              <IconFolder size={14} aria-hidden />
              Folder
            </div>
          </div>
        </div>

        {sidebarTab === "chats" ? (
          <div className="sidebar-list">
            {conversations.length === 0 && (
              <div className="empty-state" style={{ padding: "24px 10px" }}>
                <IconMessages size={26} aria-hidden />
                <span>No conversations yet</span>
              </div>
            )}
            {conversations.map((c) => (
              <div
                key={c.id}
                className={`conv${c.id === activeConversationId ? " active" : ""}`}
                onClick={() => onSelect(c.id)}
              >
                <span className="label" title={c.title}>
                  {c.title || "Untitled"}
                </span>
                <IconTrash
                  size={13}
                  className="conv-delete"
                  aria-hidden
                  onClick={(e) => {
                    e.stopPropagation();
                    onDelete(c.id);
                  }}
                />
              </div>
            ))}
          </div>
        ) : (
          <div className="folder-list">
            {!rootPath ? (
              <div className="empty-state" style={{ padding: "24px 10px" }}>
                <IconFolderOpen size={26} aria-hidden />
                <span>No folder opened</span>
                <div className="es-action" onClick={openFolder}>
                  Open a folder
                </div>
                {folderError && (
                  <span style={{ color: "#ff8783", fontSize: 11 }}>{folderError}</span>
                )}
              </div>
            ) : (
              <>
                <div style={{ display: "flex", alignItems: "center", gap: 2, padding: "0 0 6px" }}>
                  <span className="header-icon-btn" title="Open different folder" onClick={openFolder}>
                    <IconFolderOpen size={14} aria-hidden />
                  </span>
                  <span className="header-icon-btn" title="New file" onClick={createFile}>
                    <IconFilePlus size={14} aria-hidden />
                  </span>
                  <span className="header-icon-btn" title="New folder" onClick={createFolder}>
                    <IconFolderPlus size={14} aria-hidden />
                  </span>
                  <span className="header-icon-btn" title="Refresh" onClick={refreshTree}>
                    <IconRefresh size={14} aria-hidden />
                  </span>
                  <span className="header-icon-btn" title="Collapse all" onClick={collapseAll}>
                    <IconLayoutSidebarLeftCollapse size={14} aria-hidden />
                  </span>
                </div>
                <div className="folder-path" title={rootPath}>
                  {rootPath}
                </div>
                {folderError && (
                  <div style={{ color: "#ff8783", fontSize: 11, padding: "2px 6px 6px" }}>{folderError}</div>
                )}
                {renderTree(tree, 0)}
              </>
            )}
          </div>
        )}

        <div className="sidebar-footer">
          <div className="settings-entry" onClick={() => openSettings("models")}>
            <IconSettings size={15} aria-hidden />
            <span>Settings</span>
          </div>
        </div>
      </div>
      {!sidebarCollapsed && <div className="sidebar-resize" onPointerDown={startResize} />}
    </>
  );
}

export default SidebarShell;
