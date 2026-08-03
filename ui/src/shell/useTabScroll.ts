import { useEffect, useRef, useState } from "react";

export function useTabScroll(itemCount: number) {
  const scrollRef = useRef<HTMLDivElement>(null);
  const [showLeft, setShowLeft] = useState(false);
  const [showRight, setShowRight] = useState(false);

  function update() {
    const el = scrollRef.current;
    if (!el) return;
    setShowLeft(el.scrollLeft > 2);
    setShowRight(el.scrollLeft + el.clientWidth < el.scrollWidth - 2);
  }

  useEffect(() => {
    update();
    const el = scrollRef.current;
    if (!el) return;
    el.addEventListener("scroll", update);
    const ro = new ResizeObserver(update);
    ro.observe(el);
    return () => {
      el.removeEventListener("scroll", update);
      ro.disconnect();
    };
  }, [itemCount]);

  function scrollByStep(dir: 1 | -1) {
    scrollRef.current?.scrollBy({ left: dir * 140, behavior: "smooth" });
  }

  return { scrollRef, showLeft, showRight, scrollByStep };
}
