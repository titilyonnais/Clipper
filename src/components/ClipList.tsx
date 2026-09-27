import { memo, useEffect, useRef } from "react";
import { convertFileSrc } from "@tauri-apps/api/core";
import { Code2, File, Image, Link2, Pin, Star, Type } from "lucide-react";
import { cn, timeAgo } from "@/lib/utils";
import type { ClipItem } from "@/types";

const KIND_ICON = { text: Type, code: Code2, url: Link2, file: File, image: Image };

interface Props {
  clips: ClipItem[];
  selectedId: number | null;
  onSelect: (id: number) => void;
  hasMore: boolean;
  onLoadMore: () => void;
  emptyText: string;
}

export function ClipList({ clips, selectedId, onSelect, hasMore, onLoadMore, emptyText }: Props) {
  const listRef = useRef<HTMLDivElement>(null);
  const sentinelRef = useRef<HTMLDivElement>(null);
  const loadMoreRef = useRef(onLoadMore);
  loadMoreRef.current = onLoadMore;

  // Infinite scroll: load the next page when the end of the list becomes visible.
  useEffect(() => {
    const el = sentinelRef.current;
    if (!el || !hasMore) return;
    const io = new IntersectionObserver(
      (entries) => entries[0].isIntersecting && loadMoreRef.current(),
      { root: listRef.current, rootMargin: "400px" },
    );
    io.observe(el);
    return () => io.disconnect();
  }, [hasMore, clips.length]);

  useEffect(() => {
    listRef.current?.querySelector(`[data-id="${selectedId}"]`)?.scrollIntoView({ block: "nearest" });
  }, [selectedId]);

  if (!clips.length) {
    return <div className="px-6 py-12 text-center text-[12.5px] leading-relaxed text-ink-400">{emptyText}</div>;
  }

  return (
    <div ref={listRef} className="min-h-0 flex-1 overflow-y-auto" role="listbox" aria-label="Historique">
      {clips.map((c) => (
        <Row key={c.id} clip={c} active={c.id === selectedId} onSelect={onSelect} />
      ))}
      <div ref={sentinelRef} className="h-px" />
    </div>
  );
}

const Row = memo(function Row({
  clip,
  active,
  onSelect,
}: {
  clip: ClipItem;
  active: boolean;
  onSelect: (id: number) => void;
}) {
  const Icon = KIND_ICON[clip.kind] ?? Type;
  return (
    <button
      data-id={clip.id}
      data-row="1"
      role="option"
      aria-selected={active}
      onClick={() => onSelect(clip.id)}
      className={cn(
        "clip-row relative flex w-full gap-3 border-b border-ink-700/40 px-4 text-left",
        active ? "bg-ink-700/40" : "hover:bg-ink-800/50",
      )}
    >
      {active && <span className="absolute bottom-2 left-0 top-2 w-[3px] rounded-r bg-accent" />}
      {clip.kind === "image" && clip.image_path ? (
        <img
          src={convertFileSrc(clip.image_path)}
          alt=""
          loading="lazy"
          decoding="async"
          className="mt-0.5 h-10 w-10 shrink-0 rounded-md border border-ink-700/60 bg-ink-800 object-cover"
        />
      ) : (
        <span
          className={cn(
            "mt-0.5 flex h-7 w-7 shrink-0 items-center justify-center rounded-md",
            active ? "bg-accent/15 text-accent" : "bg-ink-800 text-ink-400",
          )}
        >
          <Icon size={13} />
        </span>
      )}
      <span className="min-w-0 flex-1">
        {(clip.pinned || clip.favorite || clip.language || clip.category) && (
          <span className="mb-1 flex items-center gap-1.5">
            {clip.pinned && <Pin size={10} className="fill-accent text-accent" />}
            {clip.favorite && <Star size={10} className="fill-amber-400 text-amber-400" />}
            {clip.language && (
              <span className="rounded bg-ink-800 px-1.5 py-px font-mono text-[9.5px] uppercase text-ink-300">
                {clip.language}
              </span>
            )}
            {clip.category && <span className="truncate text-[10.5px] font-medium text-accent">{clip.category}</span>}
          </span>
        )}
        <span className="clamp break-words text-[13px] leading-snug text-ink-100">
          {clip.kind === "image" ? `Image ${clip.preview}` : clip.preview}
        </span>
        <span className="mt-1 flex items-center gap-1.5 text-[10.5px] text-ink-500">
          <span>{timeAgo(clip.used_at)}</span>
          {clip.use_count > 1 && <span>· {clip.use_count}×</span>}
          {clip.tags.length > 0 && <span className="truncate">· {clip.tags.map((t) => `#${t}`).join(" ")}</span>}
        </span>
      </span>
    </button>
  );
});
