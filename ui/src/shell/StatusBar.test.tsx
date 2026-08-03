import { describe, it, expect, beforeEach } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import { useShellStore } from "../store/shellStore";
import StatusBar from "./StatusBar";
import type { LocalModelInfo, SystemStats } from "../types";

const stats: SystemStats = {
  cpuUsagePercent: 42,
  memoryUsedBytes: 4 * 1024 * 1024 * 1024,
  memoryTotalBytes: 16 * 1024 * 1024 * 1024,
  processMemoryBytes: 200 * 1024 * 1024,
  workspaceDiskBytes: 5 * 1024 * 1024 * 1024,
};

const loadedModel: LocalModelInfo = { name: "LFM2.5-350M", sizeBytes: 267060512, loaded: true, kind: "chat" };

function resetStore() {
  useShellStore.setState({ contextUsage: {}, activeTab: null, extraTabs: {} });
}

describe("StatusBar", () => {
  beforeEach(resetStore);

  it("shows placeholders while stats haven't loaded yet", () => {
    render(<StatusBar stats={null} loadedLocalModels={[]} />);
    expect(screen.getByText("No model loaded")).toBeInTheDocument();
  });

  it("shows real CPU/RAM/disk figures once stats arrive", () => {
    render(<StatusBar stats={stats} loadedLocalModels={[]} />);
    expect(screen.getByText("42%")).toBeInTheDocument();
    expect(screen.getByText("4.0 GB / 16.0 GB")).toBeInTheDocument();
    expect(screen.getByText("5.0 GB")).toBeInTheDocument();
  });

  it("shows the loaded model count and its detail in the dropdown", () => {
    render(<StatusBar stats={stats} loadedLocalModels={[loadedModel]} />);
    expect(screen.getByText("1 model(s) loaded")).toBeInTheDocument();

    fireEvent.click(screen.getByText("1 model(s) loaded"));
    expect(screen.getByText("LFM2.5-350M")).toBeInTheDocument();
  });

  it("clicking a statusbar item toggles its dropdown open and closed", () => {
    render(<StatusBar stats={stats} loadedLocalModels={[]} />);
    const cpuItem = screen.getByText("42%");

    fireEvent.click(cpuItem);
    expect(screen.getByText("CPU usage")).toBeInTheDocument();

    fireEvent.click(cpuItem);
    expect(screen.queryByText("CPU usage")).not.toBeInTheDocument();
  });

  it("opening a different item's dropdown closes the previously open one", () => {
    render(<StatusBar stats={stats} loadedLocalModels={[]} />);
    fireEvent.click(screen.getByText("42%"));
    expect(screen.getByText("CPU usage")).toBeInTheDocument();

    fireEvent.click(screen.getByText("4.0 GB / 16.0 GB"));
    expect(screen.queryByText("CPU usage")).not.toBeInTheDocument();
    expect(screen.getByText("System RAM")).toBeInTheDocument();
  });

  it("shows 'No active chat' and an empty dropdown when nothing is tracked", () => {
    render(<StatusBar stats={stats} loadedLocalModels={[]} />);
    expect(screen.getByTestId("statusbar-context")).toHaveTextContent("No active chat");

    fireEvent.click(screen.getByTestId("statusbar-context"));
    expect(screen.getByText("No token usage tracked yet")).toBeInTheDocument();
  });

  it("shows a real percentage for the active conversation's tracked usage", () => {
    useShellStore.setState({
      activeTab: "conv-1",
      extraTabs: {},
      contextUsage: { "conv-1": { usedTokens: 50, totalTokens: 200, modelLabel: "gpt-4o" } },
    });
    render(<StatusBar stats={stats} loadedLocalModels={[]} />);
    expect(screen.getByTestId("statusbar-context")).toHaveTextContent("25% context");
  });

  it("does not report context usage for a non-chat (extra) active tab", () => {
    useShellStore.setState({
      activeTab: "flow-1",
      extraTabs: { "flow-1": { id: "flow-1", type: "flow", title: "Flow editor" } },
      contextUsage: {},
    });
    render(<StatusBar stats={stats} loadedLocalModels={[]} />);
    expect(screen.getByTestId("statusbar-context")).toHaveTextContent("No active chat");
  });

  it("lists every tracked conversation's usage in the dropdown, not just the active one", () => {
    useShellStore.setState({
      activeTab: "conv-1",
      extraTabs: {},
      contextUsage: {
        "conv-1": { usedTokens: 10, totalTokens: 100, modelLabel: "gpt-4o" },
        "conv-2": { usedTokens: 20, totalTokens: 100, modelLabel: "claude-sonnet-5" },
      },
    });
    render(<StatusBar stats={stats} loadedLocalModels={[]} />);
    fireEvent.click(screen.getByTestId("statusbar-context"));

    expect(screen.getByText("gpt-4o")).toBeInTheDocument();
    expect(screen.getByText("claude-sonnet-5")).toBeInTheDocument();
  });
});
