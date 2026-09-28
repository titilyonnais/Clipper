import { useCallback, useEffect, useRef, useState } from "react";
import { cn } from "@/lib/utils";

const read = (key: string) => {
  try {
    const v = Number(localStorage.getItem(key));
    return Number.isFinite(v) && v > 0 ? v : null;
  } catch {
    return null;
  }
};

const write = (key: string, value: number) => {
  try {
    localStorage.setItem(key, String(Math.round(value)));
  } catch {
    // Storage unavailable: the width simply isn't remembered.
  }
};

export type PanelWidth = ReturnType<typeof usePanelWidth>;

/** A panel width the user can drag, remembered between sessions. */
export function usePanelWidth(key: string, initial: number, min: number, max: number) {
  const storageKey = `clipper.width.${key}`;
  const [width, setWidth] = useState(() => read(storageKey) ?? initial);
  const clamp = useCallback((w: number) => Math.round(Math.max(min, Math.min(max, w))), [min, max]);
  const set = useCallback(
    (w: number, persist = true) => {
      const v = clamp(w);
      setWidth(v);
      if (persist) write(storageKey, v);
    },
    [clamp, storageKey],
  );
  return { width: clamp(width), set, reset: () => set(initial), min, max };
}

/**
 * Vertical drag handle on the right edge of a panel. Double-click restores
 * the default width; arrows resize from the keyboard.
 */
export function Splitter({ panel, label }: { panel: PanelWidth; label: string }) {
  const [dragging, setDragging] = useState(false);
  const start = useRef({ x: 0, width: 0 });
  const frame = useRef(0);

  useEffect(() => {
    if (!dragging) return;
    // No transitions while dragging: panels follow the pointer exactly.
    document.body.classList.add("resizing");
    return () => document.body.classList.remove("resizing");
  }, [dragging]);

  return (
    <div
      role="separator"
      aria-orientation="vertical"
      aria-label={label}
      aria-valuenow={panel.width}
      aria-valuemin={panel.min}
      aria-valuemax={panel.max}
      tabIndex={0}
      title="Glisser pour redimensionner · double-clic pour rétablir"
      onPointerDown={(e) => {
        if (e.button !== 0) return;
        e.preventDefault();
        e.currentTarget.setPointerCapture(e.pointerId);
        start.current = { x: e.clientX, width: panel.width };
        setDragging(true);
      }}
      onPointerMove={(e) => {
        if (!dragging) return;
        const x = e.clientX;
        cancelAnimationFrame(frame.current);
        frame.current = requestAnimationFrame(() => panel.set(start.current.width + x - start.current.x, false));
      }}
      onPointerUp={(e) => {
        if (!dragging) return;
        setDragging(false);
        panel.set(start.current.width + e.clientX - start.current.x);
      }}
      onDoubleClick={panel.reset}
      onKeyDown={(e) => {
        if (e.key === "ArrowLeft" || e.key === "ArrowRight") {
          e.preventDefault();
          panel.set(panel.width + (e.key === "ArrowRight" ? 16 : -16));
        }
      }}
      className="group relative z-10 -mx-[3px] w-[7px] shrink-0 cursor-col-resize touch-none outline-none"
    >
      <span
        className={cn(
          "absolute inset-y-0 left-[3px] w-px bg-line transition-[background-color,box-shadow] duration-150",
          "group-hover:bg-foreground/25 group-focus-visible:bg-foreground/40",
          dragging && "bg-foreground/40 shadow-[0_0_0_1px_color-mix(in_srgb,var(--foreground)_12%,transparent)]",
        )}
      />
    </div>
  );
}
