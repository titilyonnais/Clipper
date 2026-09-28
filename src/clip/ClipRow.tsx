import { memo } from "react";
import { convertFileSrc } from "@tauri-apps/api/core";
import { Pin, Type } from "lucide-react";
import { cn, timeAgo, appLabel } from "@/lib/utils";
import { colorOf } from "@/lib/smart";
import { AppIcon, Kbd } from "@/ui/misc";
import { KindIcon, clipTitle } from "./kind";
import type { ClipItem } from "@/types";

interface Props {
  clip: ClipItem;
  active: boolean;
  marked?: boolean;
  /** 1-9: paste shortcut shown in the popup. */
  number?: number;
  dataDir?: string;
  compact?: boolean;
  draggable?: boolean;
  onPointerDown: (id: number, e: React.MouseEvent) => void;
  onActivate: (id: number) => void;
  onContextMenu?: (id: number, e: React.MouseEvent) => void;
}

export const ClipRow = memo(function ClipRow({
  clip,
  active,
  marked,
  number,
  dataDir,
  compact,
  draggable,
  onPointerDown,
  onActivate,
  onContextMenu,
}: Props) {
  const color = clip.kind === "text" || clip.kind === "code" ? colorOf(clip.preview) : null;
  const title = clipTitle(clip);

  const onDragStart = (e: React.DragEvent) => {
    // Inside Clipper: drop on a collection. Outside: plain text, like any selection.
    e.dataTransfer.setData("application/x-clipper-clip", String(clip.id));
    // The preview is the full text unless it was shortened (then the preview
    // pane, which holds the whole content, is the place to drag from).
    if (!clip.sensitive && clip.kind !== "image" && clip.kind !== "file" && !clip.preview.endsWith("…")) {
      e.dataTransfer.setData("text/plain", clip.preview);
    }
    e.dataTransfer.effectAllowed = "copyMove";
  };

  return (
    <div
      role="option"
      aria-selected={active}
      data-id={clip.id}
      draggable={draggable}
      onDragStart={draggable ? onDragStart : undefined}
      onMouseDown={(e) => e.button === 0 && onPointerDown(clip.id, e)}
      onDoubleClick={() => onActivate(clip.id)}
      onContextMenu={onContextMenu ? (e) => onContextMenu(clip.id, e) : undefined}
      className={cn(
        "row-lazy group relative flex gap-3 rounded-ctl px-2.5",
        compact ? "items-center py-1.5" : "items-start py-2.5",
        active ? "bg-selected" : "hover:bg-muted/60",
        marked && "ring-1 ring-foreground/30 ring-inset",
      )}
    >
      {active && <span className="absolute top-2 bottom-2 left-0 w-0.5 rounded-full bg-foreground" />}
      {clip.kind === "image" && clip.image_path ? (
        <img
          src={convertFileSrc(clip.image_path)}
          alt=""
          loading="lazy"
          decoding="async"
          draggable={false}
          className={cn(
            "shrink-0 rounded-[5px] border border-line bg-muted object-cover",
            compact ? "size-7" : "mt-0.5 size-9",
          )}
        />
      ) : (
        <span
          className={cn(
            "flex shrink-0 items-center justify-center rounded-[5px] text-muted-foreground",
            compact ? "size-7" : "mt-0.5 size-9",
            active ? "bg-secondary text-foreground" : "bg-muted",
          )}
        >
          {color ? (
            <span className="size-4 rounded-full border border-border" style={{ background: color }} />
          ) : clip.kind === "text" && !clip.sensitive ? (
            <Type className="size-4" />
          ) : (
            <KindIcon clip={clip} className="size-4" />
          )}
        </span>
      )}

      <div className="min-w-0 flex-1">
        <p
          className={cn(
            "break-words text-13 leading-snug",
            compact ? "clamp-1" : "clamp-2",
            clip.sensitive ? "text-muted-foreground italic" : "text-foreground",
            (clip.kind === "code" || clip.kind === "url") && !clip.sensitive && "font-mono text-[12.5px]",
          )}
        >
          {title}
        </p>
        {!compact && (
          <div className="mt-1 flex min-w-0 items-center gap-1.5 text-xs text-subtle-foreground">
            {clip.pinned && <Pin className="size-3 shrink-0 text-muted-foreground" aria-label="Épinglé" />}
            {clip.source_app && (
              <>
                <AppIcon dataDir={dataDir} app={clip.source_app} className="size-3.5 shrink-0" />
                <span className="truncate">{appLabel(clip.source_app)}</span>
                <span aria-hidden="true">·</span>
              </>
            )}
            <span className="shrink-0">{timeAgo(clip.used_at)}</span>
            {clip.tags.length > 0 && <span className="truncate">· {clip.tags.map((t) => `#${t}`).join(" ")}</span>}
          </div>
        )}
      </div>

      {compact && (
        <div className="flex shrink-0 items-center gap-2 text-xs text-subtle-foreground">
          {clip.pinned && <Pin className="size-3" aria-label="Épinglé" />}
          {number ? <Kbd>{number}</Kbd> : <span className="w-12 text-right">{timeAgo(clip.used_at)}</span>}
        </div>
      )}
    </div>
  );
});
