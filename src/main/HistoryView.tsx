import { forwardRef, useCallback, useEffect, useImperativeHandle, useMemo, useRef, useState } from "react";
import { CalendarDays, Clock, Code2, File, Image, Link2, ListOrdered, Pin, Search, Trash2, Type, X } from "lucide-react";
import { api } from "@/lib/api";
import { useClipList, useDebounced } from "@/lib/hooks";
import { useSettings } from "@/lib/settings";
import { actionFor, comboKeys, comboLabel, isTextField, overlayOpen, useKeymap } from "@/lib/shortcuts";
import { appLabel, cn, plural } from "@/lib/utils";
import { clipMenu, deleteClips, isText, run, togglePin } from "@/clip/actions";
import { ClipList, useSelection } from "@/clip/ClipList";
import { Button, IconButton } from "@/ui/button";
import { Dialog } from "@/ui/dialog";
import { Segmented } from "@/ui/form";
import { Menu, useMenu } from "@/ui/menu";
import { EmptyState, Kbd } from "@/ui/misc";
import { Splitter, type PanelWidth } from "@/ui/splitter";
import { PreviewPanel } from "./PreviewPanel";
import type { View } from "./Sidebar";
import type { ClipKind, Collection, ListParams, TimeRange } from "@/types";

const KINDS: { value: ClipKind | "all"; label: string; icon?: React.ReactNode }[] = [
  { value: "all", label: "Tout" },
  { value: "text", label: "Texte", icon: <Type /> },
  { value: "code", label: "Code", icon: <Code2 /> },
  { value: "url", label: "Liens", icon: <Link2 /> },
  { value: "image", label: "Images", icon: <Image /> },
  { value: "file", label: "Fichiers", icon: <File /> },
];

/** Type filter: words when the list is wide enough, icons (with tooltips) otherwise. */
const KIND_OPTIONS = KINDS.map((k) => ({
  value: k.value,
  title: k.label,
  label: k.icon ? (
    <>
      <span className="flex @[21.5rem]:hidden">{k.icon}</span>
      <span className="hidden @[21.5rem]:inline">{k.label}</span>
    </>
  ) : (
    k.label
  ),
}));

const RANGES: { value: TimeRange; label: string }[] = [
  { value: null, label: "Toutes les dates" },
  { value: "today", label: "Aujourd'hui" },
  { value: "yesterday", label: "Hier" },
  { value: "week", label: "7 derniers jours" },
  { value: "month", label: "30 derniers jours" },
];

export interface HistoryHandle {
  focusSearch: () => void;
  /** Run `fn` now, or after confirmation when an edit is unsaved. */
  guard: (fn: () => void) => void;
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
  const keymap = useKeymap();
  const [query, setQuery] = useState("");
  const [kind, setKind] = useState<ClipKind | "all">("all");
  const [range, setRange] = useState<TimeRange>(null);
  const [editing, setEditing] = useState(false);
  const [leaving, setLeaving] = useState<(() => void) | null>(null);
  const dirty = useRef(false);
  const onDirty = useCallback((d: boolean) => {
    dirty.current = d;
  }, []);
  const guard = useCallback((fn: () => void) => {
    if (dirty.current) setLeaving(() => fn);
    else fn();
  }, []);
  const q = useDebounced(query.trim(), 100);
  const searchRef = useRef<HTMLInputElement>(null);
  const toolbar = useRef<HTMLDivElement>(null);
  const toolbarHeight = useHeight(toolbar);
  const rangeMenu = useMenu();
  const context = useMenu();

  useImperativeHandle(ref, () => ({
    focusSearch: () => {
      searchRef.current?.focus();
      searchRef.current?.select();
    },
    guard,
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

  // Clicking another row while an edit is unsaved asks first. The callback
  // stays stable so the rows are not re-rendered.
  const pointerDown = useRef(sel.onPointerDown);
  pointerDown.current = sel.onPointerDown;
  const onPointerDown = useCallback(
    (id: number, e: React.MouseEvent) => guard(() => pointerDown.current(id, e)),
    [guard],
  );
  const selection = { ...sel, onPointerDown };

  const copy = (plain: boolean) => {
    if (!active) return;
    run(() => api.paste(active.id, plain), plain ? "Copié en texte brut." : "Copié.");
  };
  const canEdit = !!active && isText(active) && !active.sensitive;

  // Keys work wherever the focus is in the window (not only in the list).
  const onKey = useRef<(e: KeyboardEvent) => void>(() => {});
  onKey.current = (e) => {
    if (e.defaultPrevented || editing || overlayOpen()) return;
    const t = e.target as HTMLElement;
    const inSearch = t === searchRef.current;
    const inField = isTextField(t);
    const k = e.key;
    const handled = () => e.preventDefault();

    if (!inField || inSearch) {
      if (k === "ArrowDown" || k === "ArrowUp") {
        handled();
        return sel.move(k === "ArrowDown" ? 1 : -1, e.shiftKey);
      }
      if (k === "PageDown" || k === "PageUp") {
        handled();
        return sel.move(k === "PageDown" ? 10 : -10);
      }
      // Entrée on a focused button belongs to the button.
      if (k === "Enter" && active && t.tagName !== "BUTTON" && !e.ctrlKey && !e.altKey) {
        handled();
        if (sel.marked.size > 1) run(() => api.startQueue(sel.selected.filter(isText).map((c) => c.id)));
        else copy(e.shiftKey);
        return;
      }
      if (k === "Escape") {
        if (inSearch && query) {
          handled();
          return setQuery("");
        }
        if (sel.marked.size) {
          handled();
          return sel.clearMarks();
        }
      }
      if (!inField && (k === "Home" || k === "End")) {
        handled();
        return k === "Home" ? sel.first() : sel.last();
      }
    }

    const targets = sel.selected;
    switch (actionFor(keymap, e)) {
      case "copy":
        if (!active) return;
        copy(false);
        break;
      case "copy_plain":
        if (!active) return;
        copy(true);
        break;
      case "edit":
        if (!canEdit) return;
        setEditing(true);
        break;
      case "pin":
        if (!targets.length) return;
        togglePin(targets);
        break;
      case "delete":
        if (!targets.length) return;
        deleteClips(targets.map((c) => c.id));
        break;
      case "snippet":
        if (!canEdit) return;
        run(() => api.clipToSnippet(active!.id), "Snippet créé.");
        break;
      case "select_all":
        sel.selectAll();
        break;
      default:
        return;
    }
    handled();
  };
  useEffect(() => {
    const listener = (e: KeyboardEvent) => onKey.current(e);
    window.addEventListener("keydown", listener);
    return () => window.removeEventListener("keydown", listener);
  }, []);

  const menuClips = sel.marked.size > 1 && active && sel.marked.has(active.id) ? sel.selected : active ? [active] : [];

  return (
    <div className="flex min-w-0 flex-1 animate-in">
      <section className="flex shrink-0 flex-col" style={{ width: listPanel.width }} aria-label={title}>
        <div ref={toolbar} className="@container space-y-2.5 border-b border-line px-3.5 pt-3.5 pb-3">
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
                className={cn(
                  "h-9 w-full rounded-ctl border border-input/70 bg-muted/60 pl-8.5 text-sm text-ellipsis transition-[border-color,box-shadow] duration-150",
                  "placeholder:text-subtle-foreground hover:border-input focus:border-foreground/40 focus:ring-3 focus:ring-foreground/10",
                  query ? "pr-9" : "pr-3 @[20rem]:pr-[4.75rem]",
                )}
              />
              {query ? (
                <IconButton label="Effacer" size="xs" className="absolute top-1/2 right-1.5 -translate-y-1/2" onClick={() => setQuery("")}>
                  <X />
                </IconButton>
              ) : (
                keymap.search && (
                  <span className="pointer-events-none absolute top-1/2 right-2 hidden -translate-y-1/2 gap-1 @[20rem]:flex">
                    {comboKeys(keymap.search).map((k) => (
                      <Kbd key={k}>{k}</Kbd>
                    ))}
                  </span>
                )
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
          <Segmented size="sm" stretch label="Type" value={kind} onChange={setKind} options={KIND_OPTIONS} />
          {range && (
            <div className="flex h-5 animate-in items-center gap-2 text-xs text-muted-foreground">
              <CalendarDays className="size-3.5" />
              {RANGES.find((r) => r.value === range)?.label}
              <button
                type="button"
                className="ml-auto text-subtle-foreground transition-colors hover:text-foreground"
                onClick={() => setRange(null)}
              >
                Toutes les dates
              </button>
            </div>
          )}
        </div>

        {sel.marked.size > 1 && (
          <div className="flex animate-in items-center gap-1 border-b border-line bg-surface px-3.5 py-2 text-13">
            <span className="mr-auto truncate text-muted-foreground">{plural(sel.marked.size, "sélectionné")}</span>
            <Button
              size="xs"
              variant="ghost"
              title="Chaque Ctrl+V colle l'élément suivant"
              onClick={() => run(() => api.startQueue(sel.selected.filter(isText).map((c) => c.id)))}
            >
              <ListOrdered /> En série
            </Button>
            <IconButton label={`Épingler (${comboLabel(keymap.pin)})`} size="xs" onClick={() => togglePin(sel.selected)}>
              <Pin />
            </IconButton>
            <IconButton
              label={`Supprimer (${comboLabel(keymap.delete)})`}
              size="xs"
              className="hover:bg-danger/10 hover:text-danger"
              onClick={() => deleteClips(sel.selected.map((c) => c.id))}
            >
              <Trash2 />
            </IconButton>
            <IconButton label="Annuler la sélection (Échap)" size="xs" onClick={sel.clearMarks}>
              <X />
            </IconButton>
          </div>
        )}

        <div className="flex min-h-0 flex-1 flex-col">
          {clips.length ? (
            <ClipList
              clips={clips}
              selection={selection}
              hasMore={hasMore}
              onLoadMore={loadMore}
              onActivate={() => copy(false)}
              onContextMenu={(id, e) => {
                e.preventDefault();
                guard(() => {
                  if (!sel.marked.has(id)) sel.onPointerDown(id, e);
                  sel.setActive(id);
                  context.openAt(e.clientX, e.clientY);
                });
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
                    ? `Épinglez un élément (${comboLabel(keymap.pin)}) pour le garder toujours en tête.`
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
        onDirty={onDirty}
        onCopy={copy}
        onNewCollection={() => onNewCollection(active ? [active.id] : undefined)}
        aiOnline={aiOnline}
        emptyOffset={toolbarHeight}
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
            keymap,
          })}
        />
      )}
      {leaving && (
        <Dialog
          title="Abandonner les modifications ?"
          description="Le texte modifié n'a pas été enregistré."
          onClose={() => setLeaving(null)}
          footer={
            <>
              <Button onClick={() => setLeaving(null)}>Continuer à modifier</Button>
              <Button
                variant="danger"
                onClick={() => {
                  const next = leaving;
                  setLeaving(null);
                  dirty.current = false;
                  setEditing(false);
                  next();
                }}
              >
                Abandonner
              </Button>
            </>
          }
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


/** Height of an element, kept up to date. */
function useHeight(ref: React.RefObject<HTMLElement | null>) {
  const [height, setHeight] = useState(0);
  useEffect(() => {
    const el = ref.current;
    if (!el) return;
    const ro = new ResizeObserver(() => setHeight(el.offsetHeight));
    ro.observe(el);
    return () => ro.disconnect();
  }, [ref]);
  return height;
}
