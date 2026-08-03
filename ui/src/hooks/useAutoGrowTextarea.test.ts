import { describe, it, expect, vi } from "vitest";
import { renderHook } from "@testing-library/react";
import { useAutoGrowTextarea } from "./useAutoGrowTextarea";

function renderWithElement(initialValue: string, el: HTMLTextAreaElement) {
  return renderHook(
    ({ value }) => {
      const ref = useAutoGrowTextarea<HTMLTextAreaElement>(value);
      ref.current = el;
      return ref;
    },
    { initialProps: { value: initialValue } },
  );
}

describe("useAutoGrowTextarea", () => {
  it("sets the textarea's height to its scrollHeight", () => {
    const el = document.createElement("textarea");
    Object.defineProperty(el, "scrollHeight", { value: 42, configurable: true });

    const { rerender } = renderWithElement("hello", el);
    rerender({ value: "hello\nworld" });

    expect(el.style.height).toBe("42px");
  });

  it("re-measures (resets to auto first) whenever the value changes, so it can shrink back down", () => {
    const el = document.createElement("textarea");
    let height = 100;
    Object.defineProperty(el, "scrollHeight", { get: () => height });
    const setHeightSpy = vi.spyOn(el.style, "height", "set");

    const { rerender } = renderWithElement("a\nb\nc\nd", el);

    height = 20;
    rerender({ value: "a" });

    // it must set "auto" before re-reading scrollHeight, otherwise a shrink
    // would never be observed (scrollHeight only grows while height stays large)
    expect(setHeightSpy.mock.calls.some(([v]) => v === "auto")).toBe(true);
    expect(el.style.height).toBe("20px");
  });

  it("does nothing (no throw) when the ref isn't attached to any element yet", () => {
    expect(() => {
      renderHook(({ value }) => useAutoGrowTextarea<HTMLTextAreaElement>(value), {
        initialProps: { value: "x" },
      });
    }).not.toThrow();
  });
});
