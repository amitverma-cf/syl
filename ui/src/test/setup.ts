import { afterEach } from "vitest";
import { cleanup } from "@testing-library/react";
import "@testing-library/jest-dom/vitest";

// renderHook/render mount real React trees into jsdom; without an explicit
// unmount between tests, a previous test's component (and its effects,
// pending promise continuations, etc.) stays alive and can fire during a
// later, unrelated test.
afterEach(() => {
  cleanup();
});

// jsdom doesn't implement matchMedia, and several components/hooks probe it
// (or ResizeObserver) — stub the bits the test suite actually touches.
if (!window.matchMedia) {
  window.matchMedia = ((query: string) => ({
    matches: false,
    media: query,
    onchange: null,
    addListener: () => {},
    removeListener: () => {},
    addEventListener: () => {},
    removeEventListener: () => {},
    dispatchEvent: () => false,
  })) as unknown as typeof window.matchMedia;
}

if (!("ResizeObserver" in window)) {
  class ResizeObserverStub {
    observe() {}
    unobserve() {}
    disconnect() {}
  }
  // @ts-expect-error test-only stub
  window.ResizeObserver = ResizeObserverStub;
}
