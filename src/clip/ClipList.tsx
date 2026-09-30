import { useCallback, useEffect, useRef, useState } from "react";
import { ClipRow } from "./ClipRow";
import type { ClipItem } from "@/types";

/**
 * Selection model shared by the popup and the main window: one active row,
 * plus an optional multi-selection (Ctrl+click, Maj+click, Maj+flèches).
 */
export function useSelection(clips: ClipItem[]) {
  const [active, setActive] = useState<number | null>(null);
  const [marked, setMarked] = useState<Set<number>>(new Set());
  const anchor = useRef<number | null>(null);
  const ids = clips.map((c) => c.id);
  const key = ids.join(",");
  const previous = useRef<number[]>([]);

  useEffect(() => {
    // The active row gone (deleted, merged): the one that took its place,
    // so a second Suppr removes the next item, not the newest.
    setActive((a) => {
      if (a !== null && ids.includes(a)) return a;
      const at = a === null ? -1 : previous.current.indexOf(a);
      // Only for removals: a new list (another search) starts at the top.
      const removal = ids.every((id) => previous.current.includes(id));
      if (at < 0 || !removal) return ids[0] ?? null;
      const next = previous.current.slice(at + 1).find((id) => ids.includes(id));
      return next ?? ids[ids.length - 1] ?? null;
    });
    previous.current = ids;
    setMarked((m) => {
      const kept = new Set([...m].filter((id) => ids.includes(id)));
      return kept.size === m.size ? m : kept;
    });
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [key]);

  const range = (from: number, to: number) => {
    const a = ids.indexOf(from);
    const b = ids.indexOf(to);
    if (a < 0 || b < 0) return new Set([to]);
    return new Set(ids.slice(Math.min(a, b), Math.max(a, b) + 1));
  };

  const onPointerDown = useCallback(
    (id: number, e: React.MouseEvent) => {
      if (e.ctrlKey) {
        setMarked((m) => {
          const next = new Set(m.size ? m : active !== null ? [active] : []);
          if (next.has(id)) next.delete(id);
          else next.add(id);
          return next;
        });
        setActive(id);
      } else if (e.shiftKey && anchor.current !== null) {
        setMarked(range(anchor.current, id));
        setActive(id);
      } else {
        anchor.current = id;
        setMarked(new Set());
        setActive(id);
      }
    },
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [key, active],
  );

  const move = (delta: number, extend = false) => {
    if (!ids.length) return;
    const i = active === null ? -1 : ids.indexOf(active);
    const next = ids[Math.max(0, Math.min(ids.length - 1, i + delta))];
    if (extend) {
      if (anchor.current === null) anchor.current = active ?? next;
      setMarked(range(anchor.current, next));
    } else {
      anchor.current = next;
      setMarked(new Set());
    }
    setActive(next);
  };

  const selected = marked.size ? clips.filter((c) => marked.has(c.id)) : clips.filter((c) => c.id === active);

  return {
    active,
    setActive,
    marked,
    selected,
    onPointerDown,
    move,
    first: () => move(-ids.length),
    last: () => move(ids.length),
    clearMarks: () => setMarked(new Set()),
    selectAll: () => setMarked(new Set(ids)),
  };
}

interface ListProps {
  clips: ClipItem[];
  selection: ReturnType<typeof useSelection>;
  hasMore: boolean;
  onLoadMore: () => void;
  onActivate: (id: number) => void;
  onContextMenu?: (id: number, e: React.MouseEvent) => void;
  dataDir?: string;
  compact?: boolean;
  numbered?: boolean;
  draggable?: boolean;
  label: string;
}

export function ClipList({
  clips,
  selection,
  hasMore,
  onLoadMore,
  onActivate,
  onContextMenu,
  dataDir,
  compact,
  numbered,
  draggable,
  label,
}: ListProps) {
  const listRef = useRef<HTMLDivElement>(null);
  const sentinelRef = useRef<HTMLDivElement>(null);
  const loadMore = useRef(onLoadMore);
  loadMore.current = onLoadMore;

  // Infinite scroll: load the next page before the end becomes visible.
  useEffect(() => {
    const el = sentinelRef.current;
    if (!el || !hasMore) return;
    const io = new IntersectionObserver((entries) => entries[0].isIntersecting && loadMore.current(), {
      root: listRef.current,
      rootMargin: "600px",
    });
    io.observe(el);
    return () => io.disconnect();
  }, [hasMore, clips.length]);

  useEffect(() => {
    listRef.current
      ?.querySelector(`[data-id="${selection.active}"]`)
      ?.scrollIntoView({ block: "nearest" });
  }, [selection.active]);

  return (
    <div
      ref={listRef}
      role="listbox"
      aria-label={label}
      aria-multiselectable="true"
      className="stagger min-h-0 flex-1 overflow-y-auto px-2 py-1.5"
    >
      {clips.map((c, i) => (
        <ClipRow
          key={c.id}
          clip={c}
          active={c.id === selection.active}
          marked={selection.marked.has(c.id)}
          number={numbered && i < 9 ? i + 1 : undefined}
          dataDir={dataDir}
          compact={compact}
          draggable={draggable}
          onPointerDown={selection.onPointerDown}
          onActivate={onActivate}
          onContextMenu={onContextMenu}
        />
      ))}
      <div ref={sentinelRef} className="h-px" />
    </div>
  );
}
