import {
  Calendar,
  CalendarDays,
  Code2,
  File,
  Flame,
  History,
  Image,
  Inbox,
  Link2,
  MoreHorizontal,
  Pin,
  Plus,
  Star,
  Sun,
  Type,
  type LucideIcon,
} from "lucide-react";
import { cn } from "@/lib/utils";
import { ActivityHeatmap } from "./ActivityHeatmap";
import type { AiProvider, SortMode, Stats, TimeRange } from "@/types";

export type FilterKey = "all" | "pinned" | "favorites" | "text" | "code" | "url" | "file" | "image";

interface Props {
  filter: FilterKey;
  setFilter: (k: FilterKey) => void;
  category: string | null;
  setCategory: (c: string) => void;
  timeRange: TimeRange;
  setTimeRange: (t: TimeRange) => void;
  sort: SortMode;
  setSort: (s: SortMode) => void;
  stats: Stats | null;
  categories: string[];
  onManageCategories: () => void;
  aiOnline: boolean;
  aiProvider: AiProvider;
  paused: boolean;
}

const PROVIDER_LABEL: Record<AiProvider, string> = { ollama: "Ollama", openai: "OpenAI", anthropic: "Claude" };

export function Sidebar(p: Props) {
  const library: [FilterKey, string, LucideIcon, number | undefined][] = [
    ["all", "Tout", Inbox, p.stats?.total],
    ["pinned", "Épinglés", Pin, p.stats?.pinned],
    ["favorites", "Favoris", Star, p.stats?.favorites],
  ];
  const kinds: [FilterKey, string, LucideIcon, number | undefined][] = [
    ["text", "Texte", Type, p.stats?.text],
    ["code", "Code", Code2, p.stats?.code],
    ["url", "Liens", Link2, p.stats?.url],
    ["image", "Images", Image, p.stats?.image],
    ["file", "Fichiers", File, p.stats?.file],
  ];
  const ranges: [string, string, LucideIcon][] = [
    ["today", "Aujourd'hui", Sun],
    ["yesterday", "Hier", History],
    ["week", "7 derniers jours", Calendar],
    ["month", "30 derniers jours", CalendarDays],
  ];

  return (
    <aside className="flex w-[228px] shrink-0 flex-col border-r border-ink-700/60 bg-ink-900/40">
      <nav className="flex-1 space-y-5 overflow-y-auto px-3 py-4">
        <Section title="Bibliothèque">
          {library.map(([key, label, icon, count]) => (
            <NavButton
              key={key}
              icon={icon}
              label={label}
              count={count}
              active={p.filter === key && !p.category}
              onClick={() => p.setFilter(key)}
            />
          ))}
          <NavButton
            icon={Flame}
            label="Les plus utilisés"
            active={p.sort === "popular"}
            onClick={() => p.setSort(p.sort === "popular" ? "recent" : "popular")}
          />
        </Section>

        <Section title="Activité">
          <ActivityHeatmap
            selected={p.timeRange && /^\d{4}-\d{2}-\d{2}$/.test(p.timeRange) ? p.timeRange : null}
            onPick={p.setTimeRange}
            refreshKey={p.stats?.total ?? 0}
          />
          {ranges.map(([key, label, icon]) => (
            <NavButton
              key={key}
              icon={icon}
              label={label}
              active={p.timeRange === key}
              onClick={() => p.setTimeRange(p.timeRange === key ? null : key)}
            />
          ))}
        </Section>

        <Section title="Types">
          {kinds.map(([key, label, icon, count]) => (
            <NavButton
              key={key}
              icon={icon}
              label={label}
              count={count}
              active={p.filter === key && !p.category}
              onClick={() => p.setFilter(key)}
            />
          ))}
        </Section>

        <Section
          title="Catégories"
          action={
            <button
              onClick={p.onManageCategories}
              title="Gérer les catégories et les tags"
              aria-label="Gérer les catégories et les tags"
              className="rounded p-0.5 text-ink-400 hover:text-accent"
            >
              <MoreHorizontal size={14} />
            </button>
          }
        >
          {p.categories.length === 0 ? (
            <button onClick={p.onManageCategories} className="flex w-full items-center gap-2 px-3 py-1.5 text-[12px] text-ink-500 hover:text-accent">
              <Plus size={12} /> Aucune catégorie
            </button>
          ) : (
            p.categories.map((cat) => (
              <button
                key={cat}
                onClick={() => p.setCategory(cat)}
                className={cn(
                  "flex w-full items-center gap-3 rounded-lg px-3 py-1.5 text-left text-[13px]",
                  p.category === cat ? "bg-ink-700/60 text-ink-50" : "text-ink-300 hover:bg-ink-800/60 hover:text-ink-100",
                )}
              >
                <span className="h-1.5 w-1.5 shrink-0 rounded-full bg-accent" />
                <span className="truncate">{cat}</span>
              </button>
            ))
          )}
        </Section>
      </nav>

      <div className="flex items-center gap-2 border-t border-ink-700/60 px-5 py-2.5 text-[11px] text-ink-400">
        <span className={cn("h-1.5 w-1.5 rounded-full", p.aiOnline ? "bg-accent" : "bg-ink-500")} />
        IA : {PROVIDER_LABEL[p.aiProvider]} {p.aiOnline ? "prête" : "indisponible"}
        {p.paused && <span className="ml-auto font-medium uppercase tracking-wide text-amber-400">Pause</span>}
      </div>
    </aside>
  );
}

function Section({ title, action, children }: { title: string; action?: React.ReactNode; children: React.ReactNode }) {
  return (
    <div>
      <div className="mb-1 flex items-center justify-between px-3">
        <h3 className="text-[10.5px] font-semibold uppercase tracking-[0.08em] text-ink-500">{title}</h3>
        {action}
      </div>
      <div className="space-y-0.5">{children}</div>
    </div>
  );
}

function NavButton({
  icon: Icon,
  label,
  count,
  active,
  onClick,
}: {
  icon: LucideIcon;
  label: string;
  count?: number;
  active: boolean;
  onClick: () => void;
}) {
  return (
    <button
      onClick={onClick}
      aria-pressed={active}
      className={cn(
        "flex w-full items-center gap-3 rounded-lg px-3 py-1.5 text-left text-[13px]",
        active ? "bg-ink-700/60 text-ink-50" : "text-ink-300 hover:bg-ink-800/60 hover:text-ink-100",
      )}
    >
      <Icon size={14} className={active ? "text-accent" : undefined} />
      <span className="flex-1 truncate">{label}</span>
      {count !== undefined && <span className="font-mono text-[10.5px] tabular-nums text-ink-500">{count}</span>}
    </button>
  );
}
