import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { flushSync } from "react-dom";
import { ArrowLeft, ChevronRight, EyeOff, Folder, Keyboard, Scissors, Search, X } from "lucide-react";
import { api, errorText } from "@/lib/api";
import { useClipList, useDebounced, useTauriEvent } from "@/lib/hooks";
import { useSettings } from "@/lib/settings";
import { appLabel, cn } from "@/lib/utils";
import { clipMenu, deleteClips, isText, run } from "@/clip/actions";
import { ClipContent } from "@/clip/ClipContent";
import { ClipEditor } from "@/clip/ClipEditor";
import { ClipList, useSelection } from "@/clip/ClipList";
import { useSnippets } from "@/main/SnippetsView";
import { ShortcutHelp } from "@/main/SettingsView";
import { IconButton } from "@/ui/button";
import { Segmented } from "@/ui/form";
import { Menu, useMenu } from "@/ui/menu";
import { EmptyState, Kbd } from "@/ui/misc";
import { toast } from "@/ui/toast";
import type { ClipItem, Collection, ListParams, Snippet } from "@/types";

type Tab = "history" | "snippets" | "collections";
/** Échap: play the exit animation, then hide the window. */
const CLOSE_EVENT = "clipper:close";
const TABS: Tab[] = ["history", "snippets", "collections"];

export function PopupApp() {
  const { settings } = useSettings();
  const [tab, setTab] = useState<Tab>("history");
  const [query, setQuery] = useState("");
  const [collection, setCollection] = useState<Collection | null>(null);
  const [target, setTarget] = useState<string | null>(null);
  const [editing, setEditing] = useState(false);
  const [help, setHelp] = useState(false);
  // "hidden" between two appearances, so each opening starts from nothing;
  // "closing" while the exit animation plays before the window hides.
  const [phase, setPhase] = useState<"open" | "closing" | "hidden">("open");
  // Incremented at each appearance: a hide requested before it is dropped.
  const shown = useRef(0);
  const searchRef = useRef<HTMLInputElement>(null);
  const q = useDebounced(query.trim(), 60);
  // The search field is in the header, outside the panes: keys are routed
  // from the root to whichever pane is shown.
  const keys = useRef<KeyHandler | null>(null);

  // Windows shows the last frame a window presented when it appears again:
  // the panel is made transparent and that frame presented before the window
  // hides, so the next opening starts from nothing instead of flashing the
  // previous content.
  const hide = useCallback(() => {
    const at = shown.current;
    flushSync(() => setPhase("hidden"));
    requestAnimationFrame(() =>
      requestAnimationFrame(() => {
        if (shown.current === at) api.hidePopup();
      }),
    );
  }, []);
  useTauriEvent("popup:dismiss", hide);
  useTauriEvent<string | null>("popup:shown", (e) => {
    shown.current++;
    setTarget(e.payload);
    setTab("history");
    setQuery("");
    setCollection(null);
    setEditing(false);
    setHelp(false);
    setPhase("open");
    requestAnimationFrame(() => searchRef.current?.focus());
  });
  // The window hides once the exit animation is over (also without animations).
  useEffect(() => {
    if (phase !== "closing") return;
    const t = setTimeout(hide, 110);
    return () => clearTimeout(t);
  }, [phase, hide]);
  useEffect(() => {
    const onBlur = () => setPhase("hidden");
    const onClose = () => setPhase((p) => (p === "open" ? "closing" : p));
    window.addEventListener("blur", onBlur);
    window.addEventListener(CLOSE_EVENT, onClose);
    return () => {
      window.removeEventListener("blur", onBlur);
      window.removeEventListener(CLOSE_EVENT, onClose);
    };
  }, []);
  useEffect(() => searchRef.current?.focus(), [tab, collection]);

  const switchTab = (t: Tab) => {
    setTab(t);
    setQuery("");
    setCollection(null);
    setEditing(false);
  };

  return (
    <div
      className={cn(
        "relative flex h-screen flex-col overflow-hidden",
        phase === "open" && "popup-open animate-popup-in",
        phase === "closing" && "pointer-events-none animate-popup-out",
        phase === "hidden" && "opacity-0",
      )}
      onKeyDown={(e) => {
        if (e.key === "F1" || (help && e.key === "Escape")) {
          e.preventDefault();
          setHelp((v) => !v && e.key === "F1");
          return;
        }
        keys.current?.(e);
      }}
    >
      <header className="flex h-14 shrink-0 items-center gap-3 border-b border-line pr-3 pl-4">
        {collection ? (
          <IconButton label="Retour aux collections" size="sm" onClick={() => setCollection(null)}>
            <ArrowLeft />
          </IconButton>
        ) : (
          <Search className="size-4 shrink-0 text-subtle-foreground" />
        )}
        <input
          ref={searchRef}
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          spellCheck={false}
          autoFocus
          aria-label="Rechercher"
          placeholder={
            collection
              ? `Rechercher dans ${collection.name}…`
              : tab === "snippets"
                ? "Rechercher un snippet…"
                : tab === "collections"
                  ? "Rechercher une collection…"
                  : "Rechercher, ou tapez l'abréviation d'un snippet…"
          }
          className="h-full min-w-0 flex-1 bg-transparent text-base text-foreground placeholder:text-subtle-foreground"
        />
        {query && (
          <IconButton label="Effacer" size="xs" onClick={() => setQuery("")}>
            <X />
          </IconButton>
        )}
        <Segmented
          size="sm"
          label="Vue"
          value={tab}
          onChange={switchTab}
          options={[
            { value: "history", label: "Historique" },
            { value: "snippets", label: "Snippets" },
            { value: "collections", label: "Collections" },
          ]}
        />
      </header>

      {tab === "snippets" ? (
        <SnippetsPane query={q} onTab={switchTab} clearQuery={query ? () => setQuery("") : undefined} keys={keys} />
      ) : tab === "collections" && !collection ? (
        <CollectionsPane
          query={q}
          onOpen={(c) => {
            setCollection(c);
            setQuery("");
          }}
          onTab={switchTab}
          keys={keys}
        />
      ) : (
        <HistoryPane
          params={{ query: q || undefined, collection_id: collection?.id }}
          rawQuery={query}
          onTab={switchTab}
          onBack={collection ? () => setCollection(null) : undefined}
          editing={editing}
          setEditing={setEditing}
          dataDir={settings?.data_dir}
          clearQuery={() => setQuery("")}
          keys={keys}
        />
      )}

      {help && (
        <div className="absolute inset-x-0 top-14 bottom-10 z-20 flex animate-in flex-col bg-background/95 px-8 py-6 backdrop-blur-sm">
          <div className="mb-2 flex items-center justify-between">
            <h2 className="text-sm text-foreground">Raccourcis clavier</h2>
            <IconButton label="Fermer" size="sm" onClick={() => setHelp(false)}>
              <X />
            </IconButton>
          </div>
          <ShortcutHelp />
        </div>
      )}

      <footer className="flex h-10 shrink-0 items-center gap-6 border-t border-line px-4 text-xs text-subtle-foreground">
        <Hint keys={["Entrée"]}>{settings?.paste_directly && target ? `Coller dans ${appLabel(target)}` : "Copier"}</Hint>
        <Hint keys={["Maj", "Entrée"]}>Texte brut</Hint>
        <span className="ml-auto flex items-center gap-4">
          {settings?.paused_until && (
            <span className="flex items-center gap-1.5 text-warn">
              <EyeOff className="size-3.5" /> Capture suspendue
            </span>
          )}
          <button
            type="button"
            aria-pressed={help}
            onClick={() => setHelp((v) => !v)}
            className="flex items-center gap-2 rounded-ctl-sm transition-colors hover:text-foreground"
          >
            <Keyboard className="size-3.5" /> Raccourcis <Kbd>F1</Kbd>
          </button>
        </span>
      </footer>
    </div>
  );
}

type KeyHandler = (e: React.KeyboardEvent) => void;
type KeysRef = React.RefObject<KeyHandler | null>;

function Hint({ keys, children }: { keys: string[]; children: React.ReactNode }) {
  return (
    <span className="flex items-center gap-2 whitespace-nowrap">
      <span className="flex gap-1">
        {keys.map((k) => (
          <Kbd key={k}>{k}</Kbd>
        ))}
      </span>
      {children}
    </span>
  );
}

/** Keys shared by every pane: tabs, main window, escape. */
function commonKeys(e: React.KeyboardEvent, onTab: (t: Tab) => void, current: Tab, onEscape: () => boolean) {
  if (e.key === "Tab") {
    e.preventDefault();
    const i = TABS.indexOf(current);
    onTab(TABS[(i + (e.shiftKey ? TABS.length - 1 : 1)) % TABS.length]);
    return true;
  }
  if (e.ctrlKey && e.key.toLowerCase() === "o") {
    e.preventDefault();
    api.showMain();
    return true;
  }
  if (e.key === "Escape") {
    e.preventDefault();
    if (!onEscape()) window.dispatchEvent(new Event(CLOSE_EVENT));
    return true;
  }
  return false;
}

async function paste(fn: () => Promise<unknown>) {
  try {
    const outcome = await fn();
    if (outcome === "copied") toast("Copié.");
  } catch (e) {
    toast(errorText(e), true);
  }
}

function HistoryPane({
  params,
  rawQuery,
  onTab,
  onBack,
  editing,
  setEditing,
  dataDir,
  clearQuery,
  keys,
}: {
  params: ListParams;
  rawQuery: string;
  onTab: (t: Tab) => void;
  onBack?: () => void;
  editing: boolean;
  setEditing: (v: boolean) => void;
  dataDir?: string;
  clearQuery: () => void;
  keys: KeysRef;
}) {
  const { clips, hasMore, loadMore, loaded } = useClipList(params);
  const sel = useSelection(clips);
  const active = clips.find((c) => c.id === sel.active) ?? null;
  const [full, setFull] = useState<ClipItem | null>(null);
  const [collections, setCollections] = useState<Collection[]>([]);
  const context = useMenu();

  // A typed abbreviation (e.g. ";sig") offers its snippet first.
  const abbreviation = rawQuery.trim();
  const [snippet, setSnippet] = useState<Snippet | null>(null);
  const [preferSnippet, setPreferSnippet] = useState(true);
  useEffect(() => {
    setPreferSnippet(true);
    if (!abbreviation || /\s/.test(abbreviation)) return setSnippet(null);
    let alive = true;
    api.snippets(abbreviation).then((list) => {
      if (alive) setSnippet(list.find((s) => s.abbreviation?.toLowerCase() === abbreviation.toLowerCase()) ?? null);
    });
    return () => {
      alive = false;
    };
  }, [abbreviation]);

  useEffect(() => {
    setEditing(false);
    if (!active) return setFull(null);
    // Short delay: scrolling through the list with the arrows stays fluid.
    const t = setTimeout(() => api.get(active.id).then(setFull), 60);
    return () => clearTimeout(t);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [active?.id, active?.used_at]);

  const pasteClip = (id: number, plain: boolean, shiftHeld = plain) => paste(() => api.paste(id, plain, shiftHeld));

  const onKeyDown = (e: React.KeyboardEvent) => {
    if (editing) return;
    if (
      commonKeys(e, onTab, params.collection_id ? "collections" : "history", () => {
        if (sel.marked.size) return sel.clearMarks(), true;
        if (rawQuery) return clearQuery(), true;
        if (onBack) return onBack(), true;
        return false;
      })
    )
      return;
    const k = e.key;
    if (k === "ArrowDown" || k === "ArrowUp") {
      e.preventDefault();
      setPreferSnippet(false);
      sel.move(k === "ArrowDown" ? 1 : -1, e.shiftKey);
    } else if (k === "PageDown" || k === "PageUp") {
      e.preventDefault();
      sel.move(k === "PageDown" ? 8 : -8);
    } else if (k === "Enter") {
      e.preventDefault();
      if (snippet && preferSnippet) paste(() => api.pasteSnippet(snippet.id, e.shiftKey));
      else if (sel.marked.size > 1) run(() => api.startQueue(sel.selected.filter(isText).map((c) => c.id)));
      else if (active) pasteClip(active.id, e.shiftKey);
    } else if (e.ctrlKey && /^[1-9]$/.test(k)) {
      e.preventDefault();
      const clip = clips[Number(k) - 1];
      if (clip) pasteClip(clip.id, false, false);
    } else if (e.ctrlKey && k.toLowerCase() === "e" && active && isText(active) && !active.sensitive) {
      e.preventDefault();
      setEditing(true);
    } else if (e.ctrlKey && k.toLowerCase() === "p" && sel.selected.length) {
      e.preventDefault();
      const pin = !sel.selected.every((c) => c.pinned);
      run(() => api.setPinned(sel.selected.map((c) => c.id), pin));
    } else if (e.ctrlKey && k === "Delete" && sel.selected.length) {
      e.preventDefault();
      deleteClips(sel.selected.map((c) => c.id));
    }
  };

  keys.current = onKeyDown;
  const menuClips = sel.marked.size > 1 && active && sel.marked.has(active.id) ? sel.selected : active ? [active] : [];

  return (
    <div className="flex min-h-0 flex-1">
      <div className="flex w-[360px] shrink-0 flex-col border-r border-line">
        {snippet && (
          <button
            type="button"
            onClick={() => paste(() => api.pasteSnippet(snippet.id))}
            className={cn(
              "mx-2 mt-2 flex items-center gap-3 rounded-ctl border px-2.5 py-2 text-left",
              preferSnippet ? "border-foreground/30 bg-selected" : "border-line hover:bg-muted/60",
            )}
          >
            <span className="flex size-7 items-center justify-center rounded-[5px] bg-secondary text-foreground">
              <Scissors className="size-4" />
            </span>
            <span className="min-w-0 flex-1">
              <span className="block truncate text-13 text-foreground">{snippet.title}</span>
              <span className="block truncate text-xs text-subtle-foreground">Snippet {snippet.abbreviation}</span>
            </span>
            {preferSnippet && <Kbd>↵</Kbd>}
          </button>
        )}
        {sel.marked.size > 1 && (
          <div className="mx-2 mt-2 flex items-center justify-between rounded-ctl bg-secondary px-3 py-1.5 text-xs">
            <span>{sel.marked.size} éléments · Entrée pour les coller en série</span>
            <button type="button" className="text-muted-foreground hover:text-foreground" onClick={sel.clearMarks}>
              Annuler
            </button>
          </div>
        )}
        {clips.length ? (
          <ClipList
            clips={clips}
            selection={sel}
            hasMore={hasMore}
            onLoadMore={loadMore}
            onActivate={(id) => pasteClip(id, false, false)}
            onContextMenu={(id, e) => {
              e.preventDefault();
              if (!sel.marked.has(id)) sel.onPointerDown(id, e);
              sel.setActive(id);
              api.collections().then(setCollections);
              context.openAt(e.clientX, e.clientY);
            }}
            dataDir={dataDir}
            compact
            numbered={!params.query}
            label="Historique"
          />
        ) : loaded ? (
          <EmptyState title={params.query ? "Aucun résultat" : "Rien ici pour l'instant"}>
            {params.query ? "Essayez d'autres mots." : "Ce que vous copiez apparaîtra ici."}
          </EmptyState>
        ) : null}
      </div>

      <div className="flex min-w-0 flex-1 flex-col overflow-auto p-4">
        {active && full?.id === active.id ? (
          editing ? (
            <div className="animate-in">
              <ClipEditor clip={full} onDone={() => setEditing(false)} />
            </div>
          ) : (
            <div key={full.id} className="flex min-h-0 flex-1 animate-in flex-col">
              <ClipContent clip={full} compact />
            </div>
          )
        ) : null}
      </div>

      {context.anchor && menuClips.length > 0 && (
        <Menu
          anchor={context.anchor}
          onClose={context.close}
          entries={clipMenu({
            clips: menuClips,
            collections,
            inPopup: true,
            onPaste: (plain) => active && pasteClip(active.id, plain),
            onEdit: () => setEditing(true),
            onNewCollection: () => api.showMain(),
          })}
        />
      )}
    </div>
  );
}

function SnippetsPane({
  query,
  onTab,
  clearQuery,
  keys,
}: {
  query: string;
  onTab: (t: Tab) => void;
  clearQuery?: () => void;
  keys: KeysRef;
}) {
  const snippets = useSnippets(query);
  const [index, setIndex] = useState(0);
  useEffect(() => setIndex(0), [query]);
  const current = snippets[Math.min(index, snippets.length - 1)];
  const listRef = useRef<HTMLDivElement>(null);
  useEffect(() => {
    listRef.current?.querySelector(`[data-index="${index}"]`)?.scrollIntoView({ block: "nearest" });
  }, [index]);

  const onKeyDown = (e: React.KeyboardEvent) => {
    if (commonKeys(e, onTab, "snippets", () => (clearQuery ? (clearQuery(), true) : false))) return;
    if (e.key === "ArrowDown" || e.key === "ArrowUp") {
      e.preventDefault();
      setIndex((i) => Math.max(0, Math.min(snippets.length - 1, i + (e.key === "ArrowDown" ? 1 : -1))));
    } else if (e.key === "Enter" && current) {
      e.preventDefault();
      paste(() => api.pasteSnippet(current.id, e.shiftKey));
    }
  };

  keys.current = onKeyDown;
  return (
    <div className="flex min-h-0 flex-1">
      <div ref={listRef} className="stagger w-[360px] shrink-0 overflow-y-auto border-r border-line px-2 py-1.5" role="listbox" aria-label="Snippets">
        {snippets.map((s, i) => (
          <button
            key={s.id}
            type="button"
            data-index={i}
            role="option"
            aria-selected={s === current}
            onMouseDown={() => setIndex(i)}
            onDoubleClick={() => paste(() => api.pasteSnippet(s.id))}
            className={cn(
              "flex w-full items-center gap-3 rounded-ctl px-2.5 py-2 text-left transition-colors duration-150",
              s === current ? "bg-selected" : "hover:bg-muted/60",
            )}
          >
            <span className="flex size-7 shrink-0 items-center justify-center rounded-[5px] bg-muted text-muted-foreground">
              <Scissors className="size-4" />
            </span>
            <span className="min-w-0 flex-1 truncate text-13">{s.title}</span>
            {s.abbreviation && <Kbd>{s.abbreviation}</Kbd>}
          </button>
        ))}
        {!snippets.length && (
          <EmptyState icon={<Scissors />} title={query ? "Aucun snippet trouvé" : "Aucun snippet"}>
            Créez vos textes réutilisables dans la grande fenêtre (Ctrl+O).
          </EmptyState>
        )}
      </div>
      <div className="min-w-0 flex-1 overflow-auto p-4">
        {current && (
          <pre className="selectable rounded-card border border-line bg-surface p-3 font-mono text-[12.5px] leading-relaxed whitespace-pre-wrap text-foreground">
            {current.content}
          </pre>
        )}
      </div>
    </div>
  );
}

function CollectionsPane({
  query,
  onOpen,
  onTab,
  keys,
}: {
  query: string;
  onOpen: (c: Collection) => void;
  onTab: (t: Tab) => void;
  keys: KeysRef;
}) {
  const [all, setAll] = useState<Collection[]>([]);
  const [index, setIndex] = useState(0);
  useEffect(() => {
    api.collections().then(setAll);
  }, []);
  const list = useMemo(
    () => all.filter((c) => c.name.toLowerCase().includes(query.toLowerCase())),
    [all, query],
  );
  useEffect(() => setIndex(0), [query]);

  const onKeyDown = (e: React.KeyboardEvent) => {
    if (commonKeys(e, onTab, "collections", () => false)) return;
    if (e.key === "ArrowDown" || e.key === "ArrowUp") {
      e.preventDefault();
      setIndex((i) => Math.max(0, Math.min(list.length - 1, i + (e.key === "ArrowDown" ? 1 : -1))));
    } else if (e.key === "Enter" && list[index]) {
      e.preventDefault();
      onOpen(list[index]);
    }
  };

  keys.current = onKeyDown;
  return (
    <div className="stagger min-h-0 flex-1 overflow-y-auto px-2 py-1.5" role="listbox" aria-label="Collections">
      {list.map((c, i) => (
        <button
          key={c.id}
          type="button"
          role="option"
          aria-selected={i === index}
          onMouseDown={() => setIndex(i)}
          onClick={() => onOpen(c)}
          className={cn("flex w-full items-center gap-3 rounded-ctl px-2.5 py-2.5 text-left transition-colors duration-150", i === index ? "bg-selected" : "hover:bg-muted/60")}
        >
          <Folder className="size-4 text-muted-foreground" />
          <span className="flex-1 truncate text-sm">{c.name}</span>
          <span className="tabular text-xs text-subtle-foreground">{c.count}</span>
          <ChevronRight className="size-4 text-subtle-foreground" />
        </button>
      ))}
      {!list.length && (
        <EmptyState icon={<Folder />} title={query ? "Aucune collection trouvée" : "Aucune collection"}>
          Créez des collections dans la grande fenêtre (Ctrl+O) pour ranger vos éléments.
        </EmptyState>
      )}
    </div>
  );
}
