import { describe, it, expect } from "vitest";
import { joinPath, parentOf, updateNode, type TreeNode } from "./folderTree";

describe("joinPath", () => {
  it("joins with a forward slash when the base uses forward slashes", () => {
    expect(joinPath("/home/user/project", "src")).toBe("/home/user/project/src");
  });

  it("joins with a backslash when the base uses backslashes (Windows paths)", () => {
    expect(joinPath("C:\\Users\\amit\\project", "src")).toBe("C:\\Users\\amit\\project\\src");
  });

  it("does not double up the separator when the base already ends with one", () => {
    expect(joinPath("/home/user/", "src")).toBe("/home/user/src");
    expect(joinPath("C:\\Users\\amit\\", "src")).toBe("C:\\Users\\amit\\src");
  });
});

function node(overrides: Partial<TreeNode> = {}): TreeNode {
  return {
    name: "file.txt",
    path: "/root/file.txt",
    isDir: false,
    expanded: false,
    children: null,
    ...overrides,
  };
}

describe("parentOf", () => {
  it("strips exactly the trailing '/name' to recover the parent directory", () => {
    expect(parentOf(node({ path: "/root/sub/file.txt", name: "file.txt" }))).toBe("/root/sub");
  });

  it("works for a Windows-style path", () => {
    expect(parentOf(node({ path: "C:\\Users\\amit\\file.txt", name: "file.txt" }))).toBe("C:\\Users\\amit");
  });

  it("works when the node is directly under the root", () => {
    expect(parentOf(node({ path: "/root/file.txt", name: "file.txt" }))).toBe("/root");
  });
});

describe("updateNode", () => {
  it("applies the updater to the matching top-level node only", () => {
    const tree = [node({ path: "/a", name: "a" }), node({ path: "/b", name: "b" })];
    const result = updateNode(tree, "/a", (n) => ({ ...n, expanded: true }));
    expect(result[0].expanded).toBe(true);
    expect(result[1].expanded).toBe(false);
  });

  it("recurses into children to find a nested match", () => {
    const tree = [
      node({
        path: "/a",
        name: "a",
        isDir: true,
        children: [node({ path: "/a/b", name: "b" })],
      }),
    ];
    const result = updateNode(tree, "/a/b", (n) => ({ ...n, expanded: true }));
    expect(result[0].children![0].expanded).toBe(true);
    // the parent itself must be untouched apart from the new children array reference
    expect(result[0].expanded).toBe(false);
  });

  it("returns the tree unchanged (structurally) when the path isn't found anywhere", () => {
    const tree = [node({ path: "/a", name: "a" })];
    const result = updateNode(tree, "/does-not-exist", (n) => ({ ...n, expanded: true }));
    expect(result).toEqual(tree);
  });

  it("does not mutate the original array or nodes", () => {
    const original = [node({ path: "/a", name: "a" })];
    const result = updateNode(original, "/a", (n) => ({ ...n, expanded: true }));
    expect(original[0].expanded).toBe(false);
    expect(result).not.toBe(original);
  });
});
