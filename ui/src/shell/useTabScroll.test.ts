import { describe, it, expect, vi } from "vitest";
import { act, renderHook } from "@testing-library/react";
import { useTabScroll } from "./useTabScroll";

function mockMetrics(el: HTMLDivElement, { scrollLeft = 0, clientWidth = 100, scrollWidth = 100 } = {}) {
  Object.defineProperty(el, "scrollLeft", { value: scrollLeft, configurable: true });
  Object.defineProperty(el, "clientWidth", { value: clientWidth, configurable: true });
  Object.defineProperty(el, "scrollWidth", { value: scrollWidth, configurable: true });
}

function renderWithElement(itemCount: number, el: HTMLDivElement) {
  return renderHook(
    ({ count }) => {
      const tab = useTabScroll(count);
      tab.scrollRef.current = el;
      return tab;
    },
    { initialProps: { count: itemCount } },
  );
}

describe("useTabScroll", () => {
  it("shows neither chevron when all tabs already fit", () => {
    const el = document.createElement("div");
    mockMetrics(el, { scrollLeft: 0, clientWidth: 500, scrollWidth: 500 });

    const { result, rerender } = renderWithElement(3, el);
    rerender({ count: 3 });

    expect(result.current.showLeft).toBe(false);
    expect(result.current.showRight).toBe(false);
  });

  it("shows the right chevron when content overflows to the right", () => {
    const el = document.createElement("div");
    mockMetrics(el, { scrollLeft: 0, clientWidth: 300, scrollWidth: 800 });

    const { result, rerender } = renderWithElement(5, el);
    rerender({ count: 5 });

    expect(result.current.showRight).toBe(true);
    expect(result.current.showLeft).toBe(false);
  });

  it("shows the left chevron once scrolled away from the start", () => {
    const el = document.createElement("div");
    mockMetrics(el, { scrollLeft: 50, clientWidth: 300, scrollWidth: 800 });

    const { result, rerender } = renderWithElement(5, el);
    rerender({ count: 5 });

    expect(result.current.showLeft).toBe(true);
  });

  it("re-checks chevron visibility on a native scroll event", () => {
    const el = document.createElement("div");
    mockMetrics(el, { scrollLeft: 0, clientWidth: 300, scrollWidth: 800 });

    const { result, rerender } = renderWithElement(5, el);
    rerender({ count: 5 });
    expect(result.current.showLeft).toBe(false);

    mockMetrics(el, { scrollLeft: 100, clientWidth: 300, scrollWidth: 800 });
    act(() => {
      el.dispatchEvent(new Event("scroll"));
    });

    expect(result.current.showLeft).toBe(true);
  });

  it("scrollByStep scrolls right by a fixed positive amount", () => {
    const el = document.createElement("div");
    mockMetrics(el);
    el.scrollBy = vi.fn();

    const { result, rerender } = renderWithElement(3, el);
    rerender({ count: 3 });

    act(() => result.current.scrollByStep(1));
    expect(el.scrollBy).toHaveBeenCalledWith({ left: 140, behavior: "smooth" });
  });

  it("scrollByStep scrolls left by a fixed negative amount", () => {
    const el = document.createElement("div");
    mockMetrics(el);
    el.scrollBy = vi.fn();

    const { result, rerender } = renderWithElement(3, el);
    rerender({ count: 3 });

    act(() => result.current.scrollByStep(-1));
    expect(el.scrollBy).toHaveBeenCalledWith({ left: -140, behavior: "smooth" });
  });

  it("scrollByStep is a no-op when the ref isn't attached", () => {
    const { result } = renderHook(({ count }) => useTabScroll(count), { initialProps: { count: 0 } });
    expect(() => act(() => result.current.scrollByStep(1))).not.toThrow();
  });
});
