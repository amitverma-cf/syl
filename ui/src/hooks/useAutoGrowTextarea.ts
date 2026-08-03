import { useLayoutEffect, useRef } from "react";

/**
 * Grows a textarea's height to fit its content (up to the CSS max-height,
 * which clips into a scrollbar beyond that), re-measuring whenever `value`
 * changes so pasted text or programmatic updates resize it too.
 */
export function useAutoGrowTextarea<T extends HTMLTextAreaElement>(value: string) {
  const ref = useRef<T>(null);

  useLayoutEffect(() => {
    const el = ref.current;
    if (!el) return;
    el.style.height = "auto";
    el.style.height = `${el.scrollHeight}px`;
  }, [value]);

  return ref;
}
