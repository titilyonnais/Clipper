import { forwardRef, useEffect, useImperativeHandle, useMemo, useRef, useState } from "react";
import { CalendarDays, Clock, ListOrdered, Pin, Search, Trash2, X } from "lucide-react";
import { api } from "@/lib/api";
import { useClipList, useDebounced } from "@/lib/hooks";
import { useSettings } from "@/lib/settings";
import { appLabel, cn, isEditable, plural } from "@/lib/utils";
import { clipMenu, isText, run } from "@/clip/actions";
import { ClipList, useSelection } from "@/clip/ClipList";
import { Button, IconButton } from "@/ui/button";
import { Segmented } from "@/ui/form";
import { Menu, useMenu } from "@/ui/menu";
import { EmptyState, Kbd } from "@/ui/misc";
import { toast } from "@/ui/toast";
import { Splitter, type PanelWidth } from "@/ui/splitter";
import { PreviewPanel } from "./PreviewPanel";
import type { View } from "./Sidebar";
import type { ClipKind, Collection, ListParams, TimeRange } from "@/types";

const KINDS: { value: ClipKind | "all"; label: string }[] = [
  { value: "all", label: "Tout" },
  { value: "text", label: "Texte" },
  { value: "code", label: "Code" },
  { value: "url", label: "Liens" },
  { value: "image", label: "Images" },
  { value: "file", label: "Fichiers" },
];

const RANGES: { value: TimeRange; label: string }[] = [
  { value: null, label: "Toutes les dates" },
  { value: "today", label: "Aujourd'hui" },
  { value: "yesterday", label: "Hier" },
  { value: "week", label: "7 derniers jours" },
  { value: "month", label: "30 derniers jours" },
];

export interface HistoryHandle {
  focusSearch: () => void;
}

interface Props {
  view: View;
  title: string;
  collections: Collection[];
  onNewCollection: (assign?: number[]) => void;
  aiOnline: boolean;
  listPanel: PanelWidth;
}

export const HistoryView = forwardRef<HistoryHandle, Props>(function HistoryView(
  { view, title, collections, onNewCollection, aiOnline, listPanel },
  ref,
) {
  const { settings } = useSettings();
  const [query, setQuery] = useState("");
  const [kind, setKind] = useState<ClipKind | "all">("all");
  const [range, setRange] = useState<TimeRange>(null);
  const [editing, setEditing] = useState(false);
  const q = useDebounced(query.trim(), 100);
  const searchRef = useRef<HTMLInputElement>(null);
  const rangeMenu = useMenu();
  const context = useMenu();

  useImperativeHandle(ref, () => ({
    focusSearch: () => {
      searchRef.current?.focus();
      searchRef.current?.select();
    },
  }));

  const params = useMemo<ListParams>(
    () => ({
      query: q || undefined,
      kinds: kind === "all" ? undefined : [kind],
      time_range: range,
      pinned_only: view.kind === "pinned",
      collection_id: view.kind === "collection" ? view.id : undefined,
      source_app: view.kind === "app" ? view.name : undefined,
    }),
    [q, kind, range, view],
  );
  const { clips, hasMore, loaded, loadMore } = useClipList(params);
  const sel = useSelection(clips);
  const active = clips.find((c) => c.id === sel.active) ?? null;
  useEffect(() => setEditing(false), [sel.active]);

  const copy = (plain: boolean) => {
    if (!active) return;
    run(() => api.paste(active.id, plain), plain ? "Copié en texte brut." : "Copié.");
  };

  const onKeyDown = (e: React.KeyboardEvent) => {
    const t = e.target as HTMLElement;
    const inSearch = t === searchRef.current;
    if (isEditable(t) && !inSearch) return;
    const k = e.key;
    if (k === "ArrowDown" || k === "ArrowUp") {
      e.preventDefault();
      sel.move(k === "ArrowDown" ? 1 : -1, e.shiftKey);
    } else if (k === "PageDown" || k === "PageUp") {
      e.preventDefault();
      sel.move(k === "PageDown" ? 10 : -10);
    } else if (k === "Enter" && active && !editing) {
      e.preventDefault();
      if (sel.marked.size > 1) run(() => api.startQueue(sel.selected.filter(isText).map((c) => c.id)));
      else copy(e.shiftKey);
    } else if (inSearch) {
      if (k === "Escape" && query) {
        e.preventDefault();
        setQuery("");
      }
    } else if (k === "Delete" && sel.selected.length) {
      e.preventDefault();
      const ids = sel.selected.map((c) => c.id);
      run(() => api.remove(ids), ids.length > 1 ? `${ids.length} éléments supprimés.` : "Supprimé.");
    } else if (e.ctrlKey && k.toLowerCase() === "a") {
      e.preventDefault();
      sel.selectAll();
    } else if (e.ctrlKey && k.toLowerCase() === "p" && sel.selected.length) {
      e.preventDefault();
      const pin = !sel.selected.every((c) => c.pinned);
      run(() => api.setPinned(sel.selected.map((c) => c.id), pin));
    } else if (e.ctrlKey && k.toLowerCase() === "e" && active && isText(active) && !active.sensitive) {
      e.preventDefault();
      setEditing(true);
    } else if (k === "Escape" && sel.marked.size) {
      e.preventDefault();
      sel.clearMarks();
    } else if (k === "Home" || k === "End") {
      e.preventDefault();
      if (k === "Home") sel.first();
      else sel.last();
    }
  };

  const menuClips = sel.marked.size > 1 && active && sel.marked.has(active.id) ? sel.selected : active ? [active] : [];

  return (
    <div className="flex min-w-0 flex-1 animate-in" onKeyDown={onKeyDown}>
      <section className="flex shrink-0 flex-col" style={{ width: listPanel.width }} aria-label={title}>
        <div className="space-y-2.5 border-b border-line px-3.5 pt-3.5 pb-3">
          <div className="flex gap-2">
          <div className="relative min-w-0 flex-1">
            <Search className="pointer-events-none absolute top-1/2 left-2.5 size-4 -translate-y-1/2 text-subtle-foreground" />
            <input
              ref={searchRef}
              type="text"
              value={query}
              onChange={(e) => setQuery(e.target.value)}
              placeholder={searchPlaceholder(view, title)}
              spellCheck={false}
              aria-label="Rechercher"
              className="h-9 w-full rounded-ctl border border-input/70 bg-muted/60 pr-16 pl-8.5 text-sm placeholder:text-subtle-foreground hover:border-input focus:border-foreground/40 focus:ring-3 focus:ring-foreground/10"
            />
            {query ? (
              <IconButton label="Effacer" size="xs" className="absolute top-1/2 right-1.5 -translate-y-1/2" onClick={() => setQuery("")}>
                <X />
              </IconButton>
            ) : (
              <Kbd className="absolute top-1/2 right-2 -translate-y-1/2">Ctrl F</Kbd>
            )}
          </div>
            <IconButton
              label={range ? `Période : ${RANGES.find((r) => r.value === range)?.label}` : "Filtrer par date"}
              size="lg"
              variant={range ? "secondary" : "outline"}
              active={!!range}
              onClick={(e) => rangeMenu.openBelow(e.currentTarget, "end")}
              className={cn("relative", !range && "border-input/70 bg-muted/60")}
            >
              <CalendarDays />
              {range && <span className="absolute top-1.5 right-1.5 size-1.5 rounded-full bg-foreground" />}
            </IconButton>
          </div>
          <Segmented size="sm" stretch label="Type" value={kind} onChange={setKind} options={KINDS} />
          {range && (
            <div className="flex animate-in items-center gap-2 text-xs text-muted-foreground">
              <CalendarDays className="size-3.5" />
              {RANGES.find((r) => r.value === range)?.label}
              <button type="button" className="ml-auto text-subtle-foreground hover:text-foreground" onClick={() => setRange(null)}>
                Toutes les dates
              </button>
            </div>
          )}
        </div>

        {sel.marked.size > 1 && (
          <div className="flex animate-in items-center gap-1 border-b border-line bg-surface px-3.5 py-2 text-13">
            <span className="mr-auto text-muted-foreground">{plural(sel.marked.size, "sélectionné")}</span>
            <Button size="xs" variant="ghost" title="Chaque Ctrl+V colle l'élément suivant" onClick={() => run(() => api.startQueue(sel.selected.filter(isText).map((c) => c.id)))}>
              <ListOrdered /> En série
            </Button>
            <Button size="xs" variant="ghost" onClick={() => run(() => api.setPinned(sel.selected.map((c) => c.id), true))}>
              <Pin />
            </Button>
            <Button
              size="xs"
              variant="ghost"
              className="hover:text-danger"
              onClick={() => run(() => api.remove(sel.selected.map((c) => c.id)), "Supprimés.")}
            >
              <Trash2 />
            </Button>
            <IconButton label="Annuler la sélection" size="xs" onClick={sel.clearMarks}>
              <X />
            </IconButton>
          </div>
        )}

        <div className="flex min-h-0 flex-1 flex-col">
          {clips.length ? (
            <ClipList
              clips={clips}
              selection={sel}
              hasMore={hasMore}
              onLoadMore={loadMore}
              onActivate={() => copy(false)}
              onContextMenu={(id, e) => {
                e.preventDefault();
                if (!sel.marked.has(id)) sel.onPointerDown(id, e);
                sel.setActive(id);
                context.openAt(e.clientX, e.clientY);
              }}
              dataDir={settings?.data_dir}
              compact={settings?.density === "compact"}
              draggable
              label={title}
            />
          ) : loaded ? (
            <EmptyState icon={<Clock />} title={q ? "Aucun résultat" : "Rien ici pour l'instant"}>
              {q
                ? `Rien ne correspond à « ${q} ».`
                : view.kind === "collection"
                  ? "Glissez des éléments sur la collection dans la barre latérale."
                  : view.kind === "pinned"
                    ? "Épinglez un élément (Ctrl+P) pour le garder toujours en tête."
                    : "Ce que vous copiez apparaîtra ici."}
            </EmptyState>
          ) : null}
        </div>
      </section>
      <Splitter panel={listPanel} label="Largeur de la liste" />

      <PreviewPanel
        clip={active}
        collections={collections}
        editing={editing}
        setEditing={setEditing}
        onCopy={copy}
        onNewCollection={() => onNewCollection(active ? [active.id] : undefined)}
        aiOnline={aiOnline}
      />

      {rangeMenu.anchor && (
        <Menu
          anchor={rangeMenu.anchor}
          onClose={rangeMenu.close}
          entries={RANGES.map((r) => ({ label: r.label, checked: r.value === range, onSelect: () => setRange(r.value) }))}
        />
      )}
      {context.anchor && menuClips.length > 0 && (
        <Menu
          anchor={context.anchor}
          onClose={context.close}
          entries={clipMenu({
            clips: menuClips,
            collections,
            inPopup: false,
            onPaste: copy,
            onEdit: () => setEditing(true),
            onNewCollection: () => onNewCollection(menuClips.map((c) => c.id)),
            onDeleted: () => toast("Supprimé."),
          })}
        />
      )}
    </div>
  );
});

function searchPlaceholder(view: View, title: string) {
  switch (view.kind) {
    case "pinned":
      return "Rechercher dans les épinglés…";
    case "collection":
      return `Rechercher dans ${title}…`;
    case "app":
      return `Rechercher dans ce qui vient de ${title}…`;
    default:
      return "Rechercher dans l'historique…";
  }
}

export function viewTitle(view: View, collections: Collection[]) {
  switch (view.kind) {
    case "pinned":
      return "Épinglés";
    case "collection":
      return collections.find((c) => c.id === view.id)?.name ?? "Collection";
    case "app":
      return appLabel(view.name);
    default:
      return "Historique";
  }
}

