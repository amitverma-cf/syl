export interface TreeNode {
  name: string;
  path: string;
  isDir: boolean;
  expanded: boolean;
  children: TreeNode[] | null;
}

export function joinPath(base: string, name: string): string {
  const sep = base.includes("\\") && !base.includes("/") ? "\\" : "/";
  return base.endsWith(sep) ? `${base}${name}` : `${base}${sep}${name}`;
}

export function parentOf(node: Pick<TreeNode, "path" | "name">): string {
  return node.path.slice(0, node.path.length - node.name.length - 1);
}

export function updateNode(nodes: TreeNode[], path: string, fn: (n: TreeNode) => TreeNode): TreeNode[] {
  return nodes.map((n) => {
    if (n.path === path) return fn(n);
    if (n.children) return { ...n, children: updateNode(n.children, path, fn) };
    return n;
  });
}
