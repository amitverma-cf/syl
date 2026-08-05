import type { KeyboardEvent } from "react";

/// Spread onto a `div`/`span` standing in for a button so it's reachable and
/// activatable via keyboard (Tab to focus, Enter/Space to activate) — plain
/// `onClick` divs are otherwise invisible to keyboard-only navigation.
export function clickAsButtonProps(onActivate: () => void, ariaLabel?: string) {
  return {
    role: "button" as const,
    tabIndex: 0,
    "aria-label": ariaLabel,
    onClick: onActivate,
    onKeyDown: (e: KeyboardEvent) => {
      if (e.key === "Enter" || e.key === " ") {
        e.preventDefault();
        onActivate();
      }
    },
  };
}
