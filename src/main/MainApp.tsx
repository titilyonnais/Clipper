import { useCallback, useEffect, useRef, useState } from "react";
import { api } from "@/lib/api";
import { useTauriEvent } from "@/lib/hooks";
import { useSettings } from "@/lib/settings";
import { useAiOnline } from "@/lib/ai";
import { actionFor, overlayOpen, useKeymap } from "@/lib/shortcuts";
import { undoDelete } from "@/clip/actions";
import { Splitter, usePanelWidth } from "@/ui/splitter";
import { TitleBar } from "./TitleBar";
import { FOLDED_WIDTH, Sidebar, type NewCollection, type View } from "./Sidebar";
import { HistoryView, viewTitle, type HistoryHandle } from "./HistoryView";
import { SnippetsView, useSnippets, type SnippetsHandle } from "./SnippetsView";
import { SettingsView } from "./SettingsView";
import { Onboarding } from "./Onboarding";
import { UpdateDialog } from "./UpdateDialog";
import { toast } from "@/ui/toast";
import type { Collection, SourceApp, Stats } from "@/types";

/** Below this window width the sidebar shows its icons only. */
const NARROW = 1000;
const COLLAPSED_KEY = "clipper.sidebar.collapsed";

export function MainApp() {
  const { settings } = useSettings();
  const keymap = useKeymap();
  const [view, setView] = useState<View>({ kind: "history" });
  const [stats, setStats] = useState<Stats | null>(null);
  const [collections, setCollections] = useState<Collection[]>([]);
  const [apps, setApps] = useState<SourceApp[]>([]);
  const [creating, setCreating] = useState<NewCollection | null>(null);
  const [naming, setNaming] = useState(false);
  const [onboardingDone, setOnboardingDone] = useState(false);
  const history = useRef<HistoryHandle>(null);
  const snippetsView = useRef<SnippetsHandle>(null);
  const snippets = useSnippets();
  const aiOnline = useAiOnline(settings);

  // Sidebar: folded by choice (remembered), or automatically when the
  // window is narrow, where Ctrl+B unfolds it for the moment.
  const windowWidth = useWindowWidth();
  const narrow = windowWidth < NARROW;
  const [folded, setFolded] = useState(() => readFlag(COLLAPSED_KEY));
  const [peek, setPeek] = useState(false);
  useEffect(() => setPeek(false), [narrow]);
  const collapsed = !creating && !naming && (narrow ? !peek : folded);
  const toggleSidebar = useCallback(() => {
    if (narrow) setPeek((p) => !p);
    else
      setFolded((f) => {
        writeFlag(COLLAPSED_KEY, !f);
        return !f;
      });
  }, [narrow]);

  const sidebar = usePanelWidth("sidebar", 224, 180, 320);
  const sidebarWidth = collapsed ? FOLDED_WIDTH : sidebar.width;
  // The preview keeps at least 360 px.
  const list = usePanelWidth("list", 380, 330, Math.max(330, Math.min(680, windowWidth - sidebarWidth - 360)));

  const refresh = useCallback(() => {
    api.stats().then(setStats);
    api.collections().then(setCollections);
    api.sourceApps().then(setApps);
  }, []);
  useEffect(refresh, [refresh]);
  // Restarted by an update installed from Clipper: say so once.
  useEffect(() => {
    api.takeUpdateNotice().then((previous) => {
      if (!previous) return;
      api.showMain();
      toast(`Clipper a été mis à jour (${previous} → ${__APP_VERSION__}).`);
    });
  }, []);
  const timer = useRef<ReturnType<typeof setTimeout>>(undefined);
  useTauriEvent("clips:changed", () => {
    clearTimeout(timer.current);
    timer.current = setTimeout(refresh, 120);
  });
  useTauriEvent("window:shown", () => {
    if (view.kind !== "settings" && view.kind !== "snippets") history.current?.focusSearch();
  });

  // Leaving a view with an unsaved edit asks first.
  const go = useCallback((v: View) => {
    const guard = history.current?.guard ?? snippetsView.current?.guard;
    if (guard) guard(() => setView(v));
    else setView(v);
  }, []);

  // A deleted collection: back to the history. The list is fetched again
  // first, as a collection just created may not be in it yet.
  useEffect(() => {
    if (view.kind !== "collection" || collections.some((c) => c.id === view.id)) return;
    let alive = true;
    api.collections().then((list) => {
      if (!alive) return;
      setCollections(list);
      if (!list.some((c) => c.id === view.id)) setView({ kind: "history" });
    });
    return () => {
      alive = false;
    };
  }, [collections, view]);

  const search = useCallback(() => {
    if (view.kind === "settings" || view.kind === "snippets") go({ kind: "history" });
    setTimeout(() => history.current?.focusSearch());
  }, [view, go]);

  // Window-wide shortcuts; the ones acting on items live in the history view.
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.defaultPrevented || overlayOpen()) return;
      switch (actionFor(keymap, e)) {
        case "search":
          search();
          break;
        case "new_collection":
          setCreating({});
          break;
        case "toggle_sidebar":
          toggleSidebar();
          break;
        case "settings":
          go({ kind: "settings" });
          break;
        case "undo":
          undoDelete();
          break;
        default:
          return;
      }
      e.preventDefault();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [keymap, search, toggleSidebar, go]);

  const showHistory = view.kind !== "settings" && view.kind !== "snippets";

  return (
    <div className="flex h-screen flex-col bg-background">
      <TitleBar sidebarFolded={collapsed} onToggleSidebar={toggleSidebar} />
      <div className="flex min-h-0 flex-1">
        <Sidebar
          view={view}
          onView={go}
          stats={stats}
          collections={collections}
          apps={apps}
          snippetCount={snippets.length}
          width={sidebar.width}
          collapsed={collapsed}
          creating={creating}
          onCreating={setCreating}
          onNaming={setNaming}
        />
        {collapsed ? <div className="w-px shrink-0 bg-line" /> : <Splitter panel={sidebar} label="Largeur de la barre latérale" />}
        {showHistory && (
          <HistoryView
            key={JSON.stringify(view)}
            ref={history}
            view={view}
            title={viewTitle(view, collections)}
            collections={collections}
            onNewCollection={(assign) => setCreating({ assign })}
            aiOnline={aiOnline}
            listPanel={list}
          />
        )}
        {view.kind === "snippets" && <SnippetsView ref={snippetsView} listPanel={list} />}
        {view.kind === "settings" && <SettingsView stats={stats} />}
      </div>

      {settings && !settings.onboarded && !onboardingDone && <Onboarding onDone={() => setOnboardingDone(true)} />}
      <UpdateDialog />
    </div>
  );
}

function readFlag(key: string) {
  try {
    return localStorage.getItem(key) === "1";
  } catch {
    return false;
  }
}

function writeFlag(key: string, value: boolean) {
  try {
    localStorage.setItem(key, value ? "1" : "0");
  } catch {
    // Not remembered, nothing else to do.
  }
}

function useWindowWidth() {
  const [width, setWidth] = useState(window.innerWidth);
  useEffect(() => {
    const onResize = () => setWidth(window.innerWidth);
    window.addEventListener("resize", onResize);
    return () => window.removeEventListener("resize", onResize);
  }, []);
  return width;
}
