import { useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { readDir, readTextFile, writeTextFile, mkdir, rename, remove } from "@tauri-apps/plugin-fs";
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
  IconCheck,
  IconX,
  IconPencil,
} from "@tabler/icons-react";
import { useShellStore } from "../store/shellStore";
import { IconButton } from "../components/ui";
import type { ConversationSummary } from "../types";
import { joinPath, parentOf, updateNode, type TreeNode } from "./folderTree";

const isTauri = typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

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

interface SidebarShellProps {
  conversations: ConversationSummary[];
  activeConversationId: string | null;
  onSelect: (id: string) => void;
  onDelete: (id: string) => void;
}

function SidebarShell({ conversations, activeConversationId, onSelect, onDelete }: SidebarShellProps) {
  const { sidebarTab, setSidebarTab, sidebarWidth, setSidebarWidth, sidebarCollapsed, openSettings, openExtraTab } =
    useShellStore();
  const [confirmDeleteId, setConfirmDeleteId] = useState<string | null>(null);
  const [rootPath, setRootPath] = useState<string | null>(null);
  const [tree, setTree] = useState<TreeNode[]>([]);
  const [activeDir, setActiveDir] = useState<string | null>(null);
  const [folderError, setFolderError] = useState<string | null>(null);
  const [pendingCreate, setPendingCreate] = useState<{ type: "file" | "folder"; dir: string } | null>(null);
  const [createValue, setCreateValue] = useState("");
  const [renamingPath, setRenamingPath] = useState<string | null>(null);
  const [renameValue, setRenameValue] = useState("");
  const [confirmDeletePath, setConfirmDeletePath] = useState<string | null>(null);
  const resizing = useRef(false);

  async function openFolder() {
    if (!isTauri) {
      setFolderError("Folder browsing requires the desktop app (not available in a plain browser preview).");
      return;
    }
    try {
      const selected = await openDialog({ directory: true, multiple: false });
      if (!selected || Array.isArray(selected)) return;
      // The capability file grants the fs plugin no path scope by default —
      // this is what actually opens up read/write access to the folder the
      // user just chose, rather than the app already having full-disk access.
      await invoke("grant_folder_access", { path: selected });
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

  function startCreateFile() {
    const dir = activeDir ?? rootPath;
    if (!dir) return;
    setRenamingPath(null);
    setPendingCreate({ type: "file", dir });
    setCreateValue("");
  }

  function startCreateFolder() {
    const dir = activeDir ?? rootPath;
    if (!dir) return;
    setRenamingPath(null);
    setPendingCreate({ type: "folder", dir });
    setCreateValue("");
  }

  async function confirmCreate() {
    if (!pendingCreate) return;
    const { type, dir } = pendingCreate;
    const name = createValue.trim();
    if (!name) {
      setPendingCreate(null);
      return;
    }
    try {
      const path = joinPath(dir, name);
      if (type === "file") {
        await writeTextFile(path, "");
      } else {
        await mkdir(path);
      }
      setPendingCreate(null);
      await refreshTree();
      if (type === "file") openExtraTab("text", name, "", path);
    } catch (err) {
      setFolderError(String(err));
      setPendingCreate(null);
    }
  }

  function startRename(node: TreeNode) {
    setPendingCreate(null);
    setRenamingPath(node.path);
    setRenameValue(node.name);
  }

  async function confirmRename(node: TreeNode) {
    const name = renameValue.trim();
    if (!name || name === node.name) {
      setRenamingPath(null);
      return;
    }
    try {
      const newPath = joinPath(parentOf(node), name);
      await rename(node.path, newPath);
      setRenamingPath(null);
      if (activeDir === node.path) setActiveDir(newPath);
      await refreshTree();
    } catch (err) {
      setFolderError(String(err));
      setRenamingPath(null);
    }
  }

  async function deleteNode(node: TreeNode) {
    if (confirmDeletePath !== node.path) {
      setConfirmDeletePath(node.path);
      return;
    }
    setConfirmDeletePath(null);
    try {
      await remove(node.path, node.isDir ? { recursive: true } : undefined);
      if (activeDir === node.path) setActiveDir(rootPath);
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
        {renamingPath === node.path ? (
          <div className="folder-item" style={{ marginLeft: depth * 14 }} data-testid="rename-row">
            <span style={{ width: 12, flexShrink: 0 }} />
            {node.isDir ? <IconFolder size={14} aria-hidden /> : <IconFile size={14} aria-hidden />}
            <input
              data-testid="rename-input"
              className="form-input"
              style={{ flex: 1, padding: "2px 6px" }}
              autoFocus
              value={renameValue}
              onClick={(e) => e.stopPropagation()}
              onChange={(e) => setRenameValue(e.currentTarget.value)}
              onKeyDown={(e) => {
                if (e.key === "Enter") confirmRename(node);
                if (e.key === "Escape") setRenamingPath(null);
              }}
              onBlur={() => confirmRename(node)}
            />
          </div>
        ) : (
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
            <span style={{ flex: 1, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>
              {node.name}
            </span>
            {confirmDeletePath === node.path ? (
              <>
                <span
                  data-testid="node-delete-yes"
                  className="conv-delete"
                  style={{ opacity: 1, color: "#ff8783" }}
                  onClick={(e) => {
                    e.stopPropagation();
                    deleteNode(node);
                  }}
                >
                  <IconCheck size={12} aria-hidden />
                </span>
                <span
                  data-testid="node-delete-no"
                  className="conv-delete"
                  style={{ opacity: 1 }}
                  onClick={(e) => {
                    e.stopPropagation();
                    setConfirmDeletePath(null);
                  }}
                >
                  <IconX size={12} aria-hidden />
                </span>
              </>
            ) : (
              <>
                <IconPencil
                  size={12}
                  className="conv-delete"
                  data-testid="node-rename-btn"
                  aria-hidden
                  onClick={(e) => {
                    e.stopPropagation();
                    startRename(node);
                  }}
                />
                <IconTrash
                  size={12}
                  className="conv-delete"
                  data-testid="node-delete-btn"
                  aria-hidden
                  onClick={(e) => {
                    e.stopPropagation();
                    deleteNode(node);
                  }}
                />
              </>
            )}
          </div>
        )}
        {node.isDir && node.expanded && node.children && renderTree(node.children, depth + 1)}
        {pendingCreate && pendingCreate.dir === node.path && renderCreateRow(depth + 1)}
      </div>
    ));
  }

  function renderCreateRow(depth: number) {
    if (!pendingCreate) return null;
    return (
      <div className="folder-item" style={{ marginLeft: depth * 14 }} data-testid="create-row">
        <span style={{ width: 12, flexShrink: 0 }} />
        {pendingCreate.type === "file" ? <IconFile size={14} aria-hidden /> : <IconFolder size={14} aria-hidden />}
        <input
          data-testid="create-input"
          className="form-input"
          style={{ flex: 1, padding: "2px 6px" }}
          autoFocus
          placeholder={pendingCreate.type === "file" ? "file name" : "folder name"}
          value={createValue}
          onChange={(e) => setCreateValue(e.currentTarget.value)}
          onKeyDown={(e) => {
            if (e.key === "Enter") confirmCreate();
            if (e.key === "Escape") setPendingCreate(null);
          }}
          onBlur={() => confirmCreate()}
        />
      </div>
    );
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
            {conversations.map((c) =>
              confirmDeleteId === c.id ? (
                <div key={c.id} className="conv active" data-testid="conv-delete-confirm">
                  <span className="label" style={{ color: "#ff8783" }}>
                    Delete "{c.title || "Untitled"}"?
                  </span>
                  <span
                    data-testid="conv-delete-yes"
                    className="conv-delete"
                    style={{ opacity: 1, color: "#ff8783" }}
                    onClick={(e) => {
                      e.stopPropagation();
                      setConfirmDeleteId(null);
                      onDelete(c.id);
                    }}
                  >
                    <IconCheck size={13} aria-hidden />
                  </span>
                  <span
                    data-testid="conv-delete-no"
                    className="conv-delete"
                    style={{ opacity: 1 }}
                    onClick={(e) => {
                      e.stopPropagation();
                      setConfirmDeleteId(null);
                    }}
                  >
                    <IconX size={13} aria-hidden />
                  </span>
                </div>
              ) : (
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
                    data-testid="conv-delete-btn"
                    aria-hidden
                    onClick={(e) => {
                      e.stopPropagation();
                      setConfirmDeleteId(c.id);
                    }}
                  />
                </div>
              ),
            )}
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
                  <IconButton icon={IconFolderOpen} iconSize={14} title="Open different folder" onClick={openFolder} />
                  <IconButton icon={IconFilePlus} iconSize={14} title="New file" onClick={startCreateFile} />
                  <IconButton icon={IconFolderPlus} iconSize={14} title="New folder" onClick={startCreateFolder} />
                  <IconButton icon={IconRefresh} iconSize={14} title="Refresh" onClick={refreshTree} />
                  <IconButton
                    icon={IconLayoutSidebarLeftCollapse}
                    iconSize={14}
                    title="Collapse all"
                    onClick={collapseAll}
                  />
                </div>
                <div className="folder-path" title={rootPath}>
                  {rootPath}
                </div>
                {folderError && (
                  <div style={{ color: "#ff8783", fontSize: 11, padding: "2px 6px 6px" }}>{folderError}</div>
                )}
                {renderTree(tree, 0)}
                {pendingCreate && pendingCreate.dir === rootPath && renderCreateRow(0)}
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
