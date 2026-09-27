import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { SlidersHorizontal, X } from "lucide-react";
import { api } from "@/lib/api";
import { cn, hexToRgb, isEditable } from "@/lib/utils";
import type { AdvancedFilters as Filters, ClipItem, SettingsView, SortMode, Stats, TimeRange } from "@/types";
import { TitleBar } from "@/components/TitleBar";
import { Sidebar, type FilterKey } from "@/components/Sidebar";
import { SearchBar } from "@/components/SearchBar";
import { ClipList } from "@/components/ClipList";
import { Preview } from "@/components/Preview";
import { Settings } from "@/components/Settings";
import { AdvancedFilters, countActive } from "@/components/AdvancedFilters";
import { CategoriesManager } from "@/components/CategoriesManager";

const PAGE = 100;
const KIND_FILTERS = ["text", "code", "url", "file", "image"];
const RANGE_LABELS: Record<string, string> = {
  today: "Aujourd'hui",
  yesterday: "Hier",
  week: "7 derniers jours",
  month: "30 derniers jours",
  year: "12 derniers mois",
};

type Modal = "settings" | "filters" | "categories" | null;

export default function App() {
  const [settings, setSettings] = useState<SettingsView | null>(null);
  const [query, setQuery] = useState("");
  const [debouncedQuery, setDebouncedQuery] = useState("");
  const [filter, setFilter] = useState<FilterKey>("all");
  const [category, setCategory] = useState<string | null>(null);
  const [timeRange, setTimeRange] = useState<TimeRange>(null);
  const [sort, setSort] = useState<SortMode>("recent");
  const [advanced, setAdvanced] = useState<Filters>({});
  const [clips, setClips] = useState<ClipItem[]>([]);
  const [hasMore, setHasMore] = useState(false);
  const [selectedId, setSelectedId] = useState<number | null>(null);
  const [stats, setStats] = useState<Stats | null>(null);
  const [categories, setCategories] = useState<string[]>([]);
  const [paused, setPaused] = useState(false);
  const [aiOnline, setAiOnline] = useState(false);
  const [modal, setModal] = useState<Modal>(null);
  const searchRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    const t = setTimeout(() => setDebouncedQuery(query.trim()), 120);
    return () => clearTimeout(t);
  }, [query]);

  const params = useMemo(
    () => ({
      ...advanced,
      query: debouncedQuery || undefined,
      kinds: KIND_FILTERS.includes(filter) ? [filter as ClipItem["kind"]] : advanced.kinds,
      category,
      pinned_only: filter === "pinned",
      favorites_only: filter === "favorites",
      time_range: timeRange,
      sort,
    }),
    [debouncedQuery, filter, category, timeRange, sort, advanced],
  );

  // Every response is tagged so that a slow, outdated query never overwrites a newer one.
  const requestId = useRef(0);
  const loadedCount = useRef(0);
  const clipsRef = useRef(clips);
  clipsRef.current = clips;

  const load = useCallback(
    async (mode: "reset" | "refresh" | "more") => {
      const id = ++requestId.current;
      const offset = mode === "more" ? loadedCount.current : 0;
      const limit = mode === "refresh" ? Math.max(PAGE, loadedCount.current) : PAGE;
      const page = await api.list({ ...params, limit, offset });
      if (id !== requestId.current) return;
      const next = mode === "more" ? [...clipsRef.current, ...page] : page;
      loadedCount.current = next.length;
      setClips(next);
      setHasMore(page.length === limit);
      setSelectedId((cur) => (cur != null && next.some((c) => c.id === cur) ? cur : next[0]?.id ?? null));
    },
    [params],
  );

  const refreshMeta = useCallback(() => {
    api.stats().then(setStats);
    api.categories().then(setCategories);
  }, []);

  useEffect(() => {
    load("reset");
  }, [load]);

  // Backend events. Bursts (e.g. several copies in a row) are coalesced.
  const onChangedRef = useRef<() => void>(() => {});
  onChangedRef.current = () => {
    load("refresh");
    refreshMeta();
  };
  useEffect(() => {
    let timer: ReturnType<typeof setTimeout> | undefined;
    const unlisten = [
      listen("clips:changed", () => {
        clearTimeout(timer);
        timer = setTimeout(() => onChangedRef.current(), 60);
      }),
      listen("window:shown", () => {
        searchRef.current?.focus();
        searchRef.current?.select();
      }),
      listen<boolean>("monitor:paused", (e) => setPaused(e.payload)),
    ];
    return () => {
      clearTimeout(timer);
      unlisten.forEach((p) => p.then((f) => f()));
    };
  }, []);

  useEffect(() => {
    refreshMeta();
    api.getSettings().then((s) => {
      setSettings(s);
      setPaused(s.monitor_paused);
    });
  }, [refreshMeta]);

  // Appearance
  useEffect(() => {
    if (!settings) return;
    const root = document.documentElement;
    const apply = () => {
      const light =
        settings.theme === "light" ||
        (settings.theme === "auto" && window.matchMedia("(prefers-color-scheme: light)").matches);
      root.classList.toggle("light", light);
    };
    apply();
    const [r, g, b] = hexToRgb(settings.accent_color) ?? [163, 230, 53];
    root.style.setProperty("--accent", `${r} ${g} ${b}`);
    root.style.setProperty("--on-accent", 0.299 * r + 0.587 * g + 0.114 * b > 150 ? "10 10 12" : "255 255 255");
    root.dataset.density = settings.density;
    if (settings.theme !== "auto") return;
    const mq = window.matchMedia("(prefers-color-scheme: light)");
    mq.addEventListener("change", apply);
    return () => mq.removeEventListener("change", apply);
  }, [settings?.theme, settings?.accent_color, settings?.density]);

  // AI availability: re-checked when the provider changes, and periodically for Ollama.
  useEffect(() => {
    if (!settings) return;
    const check = () => api.aiHealth().then((r) => setAiOnline(r.ok)).catch(() => setAiOnline(false));
    check();
    if (settings.ai_provider !== "ollama") return;
    const t = setInterval(check, 60_000);
    return () => clearInterval(t);
  }, [
    settings?.ai_provider,
    settings?.ollama_url,
    settings?.ollama_model,
    settings?.openai_key_set,
    settings?.anthropic_key_set,
  ]);

  const selected = useMemo(() => clips.find((c) => c.id === selectedId) ?? null, [clips, selectedId]);

  const togglePause = async () => {
    const next = !paused;
    setPaused(next);
    await api.setPaused(next);
    setSettings((s) => (s ? { ...s, monitor_paused: next } : s));
  };

  const focusSearch = () => {
    searchRef.current?.focus();
    searchRef.current?.select();
  };

  // Keyboard navigation. Fields keep their own Enter / Escape handling.
  const keyState = useRef({ clips, selectedId, modal, query });
  keyState.current = { clips, selectedId, modal, query };
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      const { clips, selectedId, modal, query } = keyState.current;
      const target = document.activeElement;
      const inSearch = target === searchRef.current;
      const inField = isEditable(target) && !inSearch;

      if ((e.ctrlKey && e.key.toLowerCase() === "f") || (e.key === "/" && !isEditable(target) && !modal)) {
        e.preventDefault();
        focusSearch();
        return;
      }
      if (e.key === "Escape") {
        if (e.defaultPrevented || inField) return;
        if (modal) setModal(null);
        else if (query) setQuery("");
        else api.hideWindow();
        return;
      }
      if (modal || inField || e.defaultPrevented) return;

      if (e.key === "ArrowDown" || e.key === "ArrowUp") {
        e.preventDefault();
        if (!clips.length) return;
        const i = clips.findIndex((c) => c.id === selectedId);
        const next = clips[Math.max(0, Math.min(clips.length - 1, i + (e.key === "ArrowDown" ? 1 : -1)))];
        setSelectedId(next.id);
      } else if (e.key === "Enter" && selectedId != null && (inSearch || !isEditable(target))) {
        // Enter on a regular button activates it; on a list row it copies.
        if (target instanceof HTMLButtonElement && !target.dataset.row) return;
        e.preventDefault();
        api
          .copy(selectedId)
          .then(() => api.hideWindow())
          .catch(() => {});
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, []);

  const activeLabel = timeRange ? RANGE_LABELS[timeRange] ?? new Date(timeRange).toLocaleDateString("fr-FR") : null;
  const advancedCount = countActive(advanced);
  const hasFilters = !!activeLabel || advancedCount > 0 || sort !== "recent";

  return (
    <div className="flex h-screen flex-col overflow-hidden bg-ink-950">
      <TitleBar
        paused={paused}
        onTogglePause={togglePause}
        onOpenSettings={() => setModal("settings")}
        onSearch={focusSearch}
        count={stats?.total ?? 0}
      />
      <div className="flex min-h-0 flex-1">
        <Sidebar
          filter={filter}
          setFilter={(f) => {
            setFilter(f);
            setCategory(null);
          }}
          category={category}
          setCategory={(c) => {
            setCategory(c);
            setFilter("all");
          }}
          timeRange={timeRange}
          setTimeRange={setTimeRange}
          sort={sort}
          setSort={setSort}
          stats={stats}
          categories={categories}
          onManageCategories={() => setModal("categories")}
          aiOnline={aiOnline}
          aiProvider={settings?.ai_provider ?? "ollama"}
          paused={paused}
        />

        <section className="flex w-[380px] shrink-0 flex-col border-r border-ink-700/60 bg-ink-900/20">
          <SearchBar ref={searchRef} value={query} onChange={setQuery} count={clips.length} more={hasMore} />
          <div className="flex items-center gap-2 border-b border-ink-700/40 px-4 py-2">
            <button
              onClick={() => setModal("filters")}
              className={cn(
                "inline-flex h-7 items-center gap-1.5 rounded-md border px-2.5 text-[11.5px] font-medium",
                advancedCount > 0
                  ? "border-accent bg-accent/15 text-accent"
                  : "border-ink-700 bg-ink-800 text-ink-300 hover:text-ink-50",
              )}
            >
              <SlidersHorizontal size={11} />
              Filtres
              {advancedCount > 0 && (
                <span className="rounded bg-accent px-1.5 text-[10px] font-bold text-[rgb(var(--on-accent))]">
                  {advancedCount}
                </span>
              )}
            </button>
            {activeLabel && <Chip onClear={() => setTimeRange(null)}>{activeLabel}</Chip>}
            {sort === "popular" && <Chip onClear={() => setSort("recent")}>Populaires</Chip>}
            {hasFilters && (
              <button
                onClick={() => {
                  setTimeRange(null);
                  setSort("recent");
                  setAdvanced({});
                }}
                className="ml-auto text-[11px] text-ink-400 hover:text-accent"
              >
                Réinitialiser
              </button>
            )}
          </div>
          <ClipList
            clips={clips}
            selectedId={selectedId}
            onSelect={setSelectedId}
            hasMore={hasMore}
            onLoadMore={() => load("more")}
            emptyText={
              debouncedQuery
                ? `Aucun résultat pour « ${debouncedQuery} ».`
                : paused
                  ? "La capture est en pause."
                  : "Copiez quelque chose : il apparaîtra ici."
            }
          />
        </section>

        <Preview
          clip={selected}
          aiOnline={aiOnline}
          onIgnoreApp={async (app) => {
            if (!settings || settings.ignore_apps.includes(app)) return;
            setSettings(await api.setSettings({ ...settings, ignore_apps: [...settings.ignore_apps, app] }));
          }}
        />
      </div>

      {modal === "settings" && settings && (
        <Settings
          value={settings}
          stats={stats}
          onChange={(s) => {
            setSettings(s);
            setPaused(s.monitor_paused);
          }}
          onClose={() => setModal(null)}
        />
      )}
      {modal === "filters" && (
        <AdvancedFilters value={advanced} onChange={setAdvanced} onClose={() => setModal(null)} />
      )}
      {modal === "categories" && <CategoriesManager onClose={() => setModal(null)} />}
    </div>
  );
}

function Chip({ children, onClear }: { children: React.ReactNode; onClear: () => void }) {
  return (
    <span className="inline-flex h-7 items-center gap-1 rounded-md bg-ink-800 pl-2.5 pr-1 text-[11.5px] text-ink-100">
      {children}
      <button onClick={onClear} className="rounded p-0.5 text-ink-400 hover:text-ink-50" aria-label="Retirer">
        <X size={11} />
      </button>
    </span>
  );
}
