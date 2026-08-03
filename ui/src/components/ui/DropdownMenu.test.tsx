import { useState } from "react";
import { describe, it, expect, vi } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import DropdownMenu, { type DropdownMenuGroup } from "./DropdownMenu";

function Wrapper({ groups, emptyLabel }: { groups: DropdownMenuGroup[]; emptyLabel?: string }) {
  const [open, setOpen] = useState(false);
  return (
    <DropdownMenu
      trigger={
        <div className="dropdown" onClick={() => setOpen((v) => !v)}>
          Pick a model
        </div>
      }
      groups={groups}
      open={open}
      onOpenChange={setOpen}
      emptyLabel={emptyLabel}
    />
  );
}

describe("DropdownMenu", () => {
  it("stays closed until the trigger is clicked", () => {
    render(<Wrapper groups={[{ items: [{ key: "a", label: "Model A" }] }]} />);
    expect(screen.queryByText("Model A")).not.toBeInTheDocument();
    fireEvent.click(screen.getByText("Pick a model"));
    expect(screen.getByText("Model A")).toBeInTheDocument();
  });

  it("renders group labels and calls onSelect then closes for a leaf item", () => {
    const onSelect = vi.fn();
    render(
      <Wrapper
        groups={[
          { label: "LOCAL", items: [{ key: "a", label: "Model A", onSelect }] },
          { label: "OPENAI", items: [{ key: "b", label: "Model B" }] },
        ]}
      />,
    );
    fireEvent.click(screen.getByText("Pick a model"));
    expect(screen.getByText("LOCAL")).toBeInTheDocument();
    expect(screen.getByText("OPENAI")).toBeInTheDocument();

    fireEvent.click(screen.getByText("Model A"));
    expect(onSelect).toHaveBeenCalledTimes(1);
    expect(screen.queryByText("Model A")).not.toBeInTheDocument();
  });

  it("expands nested children on click instead of selecting, and selects a leaf child on a second click", () => {
    const onParentSelect = vi.fn();
    const onChildSelect = vi.fn();
    render(
      <Wrapper
        groups={[
          {
            items: [
              {
                key: "gpt",
                label: "gpt-5",
                onSelect: onParentSelect,
                children: [
                  { key: "gpt-low", label: "low effort", onSelect: onChildSelect },
                  { key: "gpt-high", label: "high effort" },
                ],
              },
            ],
          },
        ]}
      />,
    );
    fireEvent.click(screen.getByText("Pick a model"));
    expect(screen.queryByText("low effort")).not.toBeInTheDocument();

    fireEvent.click(screen.getByText("gpt-5"));
    expect(onParentSelect).not.toHaveBeenCalled();
    expect(screen.getByText("low effort")).toBeInTheDocument();
    expect(screen.getByText("high effort")).toBeInTheDocument();

    fireEvent.click(screen.getByText("low effort"));
    expect(onChildSelect).toHaveBeenCalledTimes(1);
    expect(screen.queryByText("low effort")).not.toBeInTheDocument();
  });

  it("marks the selected item and shows a sublabel", () => {
    render(
      <Wrapper groups={[{ items: [{ key: "a", label: "Model A", selected: true, sublabel: "loaded" }] }]} />,
    );
    fireEvent.click(screen.getByText("Pick a model"));
    expect(screen.getByText("Model A").closest(".option-item")!.className).toContain("sel");
    expect(screen.getByText("loaded")).toBeInTheDocument();
  });

  it("shows the empty label when every group has zero items", () => {
    render(<Wrapper groups={[{ items: [] }]} emptyLabel="No models found" />);
    fireEvent.click(screen.getByText("Pick a model"));
    expect(screen.getByText("No models found")).toBeInTheDocument();
  });

  it("closes when the mouse leaves the open menu", () => {
    render(<Wrapper groups={[{ items: [{ key: "a", label: "Model A" }] }]} />);
    fireEvent.click(screen.getByText("Pick a model"));
    fireEvent.mouseLeave(screen.getByText("Model A").closest(".option-menu")!);
    expect(screen.queryByText("Model A")).not.toBeInTheDocument();
  });
});
