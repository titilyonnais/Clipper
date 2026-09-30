import { useEffect, useLayoutEffect, useRef, useState } from "react";
import { createPortal } from "react-dom";
import { Check } from "lucide-react";
import { cn } from "@/lib/utils";

export type MenuEntry =
  | {
      label: string;
      icon?: React.ReactNode;
      hint?: string;
      checked?: boolean;
      danger?: boolean;
      disabled?: boolean;
      onSelect: () => void;
    }
  | { separator: true }
  | { heading: string };

/** `trigger`: the button that opened the menu; clicking it again closes it. */
type Anchor = { x: number; y: number; align?: "start" | "end"; trigger?: HTMLElement };

/**
 * Floating menu, positioned at a point (context menu) or under a button.
 * Keyboard: arrows, Home/End, Enter, Escape. Closes on outside click.
 */
export function Menu({ anchor, entries, onClose }: { anchor: Anchor; entries: MenuEntry[]; onClose: () => void }) {
  const ref = useRef<HTMLDivElement>(null);
  const [pos, setPos] = useState<{ left: number; top: number } | null>(null);
  const items = entries.flatMap((e, i) => ("onSelect" in e && !e.disabled ? [i] : []));
  const [active, setActive] = useState(-1);
  // The icon column exists only when an entry has something to show in it.
  const gutter = entries.some((e) => "onSelect" in e && (e.icon || e.checked !== undefined));

  useLayoutEffect(() => {
    const el = ref.current;
    if (!el) return;
    const { width, height } = el.getBoundingClientRect();
    let left = anchor.align === "end" ? anchor.x - width : anchor.x;
    let top = anchor.y;
    left = Math.max(6, Math.min(left, window.innerWidth - width - 6));
    if (top + height > window.innerHeight - 6) top = Math.max(6, anchor.y - height - 36);
    setPos({ left, top });
    el.focus();
  }, [anchor.x, anchor.y, anchor.align]);

  useEffect(() => {
    const onDown = (e: MouseEvent) => {
      const target = e.target as Node;
      // A press on the trigger is left to its click handler, which toggles.
      if (!ref.current?.contains(target) && !anchor.trigger?.contains(target)) onClose();
    };
    const onBlur = () => onClose();
    document.addEventListener("mousedown", onDown, true);
    window.addEventListener("blur", onBlur);
    return () => {
      document.removeEventListener("mousedown", onDown, true);
      window.removeEventListener("blur", onBlur);
    };
  }, [onClose, anchor.trigger]);

  const onKeyDown = (e: React.KeyboardEvent) => {
    e.stopPropagation();
    const at = items.indexOf(active);
    if (e.key === "Escape") {
      e.preventDefault();
      onClose();
    } else if (e.key === "ArrowDown") {
      e.preventDefault();
      setActive(items[(at + 1) % items.length]);
    } else if (e.key === "ArrowUp") {
      e.preventDefault();
      setActive(items[(at - 1 + items.length) % items.length]);
    } else if (e.key === "Home") {
      setActive(items[0]);
    } else if (e.key === "End") {
      setActive(items[items.length - 1]);
    } else if (e.key === "Enter" && active >= 0) {
      e.preventDefault();
      const entry = entries[active];
      if ("onSelect" in entry) {
        onClose();
        entry.onSelect();
      }
    }
  };

  return createPortal(
    <div
      ref={ref}
      role="menu"
      tabIndex={-1}
      onKeyDown={onKeyDown}
      style={{ left: pos?.left ?? anchor.x, top: pos?.top ?? anchor.y, visibility: pos ? "visible" : "hidden" }}
      className="fixed z-50 min-w-48 origin-top animate-pop rounded-dialog border border-border bg-popover p-1.5 shadow-lg shadow-black/30 outline-none"
    >
      {entries.map((e, i) => {
        if ("separator" in e) return <div key={i} className="mx-1 my-1.5 h-px bg-line" />;
        if ("heading" in e)
          return (
            <div key={i} className="px-2 pt-1.5 pb-1 text-xs text-subtle-foreground">
              {e.heading}
            </div>
          );
        return (
          <button
            key={i}
            type="button"
            role="menuitem"
            disabled={e.disabled}
            onMouseEnter={() => setActive(i)}
            onClick={() => {
              onClose();
              e.onSelect();
            }}
            className={cn(
              "flex h-8 w-full items-center gap-2.5 rounded-[5px] px-2 text-left text-13 transition-colors duration-100 disabled:opacity-40",
              "[&_svg]:size-4 [&_svg]:shrink-0",
              e.danger ? "text-danger" : "text-foreground",
              active === i && (e.danger ? "bg-danger/10" : "bg-secondary"),
            )}
          >
            {gutter && (
              <span className={cn("flex w-4 justify-center", !e.danger && "text-muted-foreground")}>
                {e.checked ? <Check /> : e.icon}
              </span>
            )}
            <span className="flex-1 truncate">{e.label}</span>
            {e.hint && <span className="pl-4 text-xs text-subtle-foreground">{e.hint}</span>}
          </button>
        );
      })}
    </div>,
    document.body,
  );
}

/** A trigger button that opens a menu below itself (and closes it when clicked again). */
export function useMenu() {
  const [anchor, setAnchor] = useState<Anchor | null>(null);
  return {
    open: anchor !== null,
    anchor,
    openAt: (x: number, y: number) => setAnchor({ x, y }),
    openBelow: (el: HTMLElement, align: "start" | "end" = "start") =>
      setAnchor((current) => {
        if (current?.trigger === el) return null;
        const r = el.getBoundingClientRect();
        return { x: align === "end" ? r.right : r.left, y: r.bottom + 4, align, trigger: el };
      }),
    close: () => setAnchor(null),
  };
}
