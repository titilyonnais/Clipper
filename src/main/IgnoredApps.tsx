import { useEffect, useMemo, useRef, useState } from "react";
import { Check, FolderOpen, Plus, Search, ShieldAlert, X } from "lucide-react";
import { api, errorText } from "@/lib/api";
import { useSettings } from "@/lib/settings";
import { appLabel, cn, plural, timeAgo } from "@/lib/utils";
import { Button, IconButton } from "@/ui/button";
import { Dialog } from "@/ui/dialog";
import { Input } from "@/ui/form";
import { AppIcon } from "@/ui/misc";
import { toast } from "@/ui/toast";
import type { InstalledApp } from "@/types";

/** Kept between openings of the settings: shown at once, refreshed behind. */
let known: InstalledApp[] | null = null;

/** Lowercase without accents, for searching. */
const fold = (s: string) =>
  s
    .normalize("NFD")
    .replace(/\p{M}/gu, "")
    .toLowerCase();

/**
 * Applications whose copies are never recorded: the list, suggestions
 * detected on this computer, and a picker with search.
 */
export function IgnoredApps() {
  const { settings, update } = useSettings();
  const [apps, setApps] = useState<InstalledApp[] | null>(known);
  const [picking, setPicking] = useState(false);

  useEffect(() => {
    api
      .installedApps()
      .then((list) => {
        // Executables chosen by hand stay known.
        const picked = (known ?? []).filter((a) => !list.some((b) => b.exe === a.exe));
        known = [...list, ...picked];
        setApps(known);
      })
      .catch(() => setApps((a) => a ?? []));
  }, []);

  if (!settings) return null;
  const dataDir = settings.data_dir;
  const ignored = settings.ignore_apps;
  const byExe = new Map((apps ?? []).map((a) => [a.exe, a]));

  const save = async (next: string[]) => {
    const error = await update({ ignore_apps: next });
    if (error) toast(error, true);
  };
  const toggle = (exe: string) => save(ignored.includes(exe) ? ignored.filter((a) => a !== exe) : [...ignored, exe]);

  const browse = async () => {
    try {
      const app = await api.pickApp();
      if (!app) return;
      if (!known?.some((a) => a.exe === app.exe)) {
        known = [...(known ?? []), app];
        setApps(known);
      }
      if (ignored.includes(app.exe)) toast(`${app.name} est déjà ignorée.`);
      else await save([...ignored, app.exe]);
    } catch (e) {
      toast(errorText(e), true);
    }
  };

  const suggestions = (apps ?? [])
    .filter((a) => a.score > 0 && !ignored.includes(a.exe))
    .sort((a, b) => b.score - a.score)
    .slice(0, 4);

  return (
    <section className="mb-8">
      <h2 className="mb-1 text-sm text-muted-foreground">Applications ignorées</h2>
      <div className="rounded-card border border-line bg-surface">
        <div className="flex items-center justify-between gap-6 px-4 py-3">
          <p className="text-13 leading-relaxed text-muted-foreground">
            Rien de ce qui est copié depuis ces applications n'est enregistré. Les gestionnaires de mots de passe qui le
            signalent à Windows sont ignorés d'office.
          </p>
          <Button className="shrink-0" onClick={() => setPicking(true)}>
            <Plus className="size-3.5" />
            Ajouter
          </Button>
        </div>

        {ignored.length > 0 && (
          <ul className="divide-y divide-line border-t border-line px-4">
            {ignored.map((exe) => {
              const app = byExe.get(exe);
              return (
                <li key={exe} className="flex h-12 items-center gap-3">
                  <AppIcon dataDir={dataDir} app={exe} className="size-5 shrink-0" />
                  <span className="min-w-0 truncate text-sm text-foreground">{app?.name ?? appLabel(exe)}</span>
                  <span className="min-w-0 flex-1 truncate font-mono text-xs text-subtle-foreground">{exe}</span>
                  <IconButton label={`Ne plus ignorer ${app?.name ?? appLabel(exe)}`} size="sm" onClick={() => toggle(exe)}>
                    <X />
                  </IconButton>
                </li>
              );
            })}
          </ul>
        )}

        {suggestions.length > 0 && (
          <div className="flex flex-wrap items-center gap-1.5 border-t border-line px-4 py-3">
            <span className="mr-1 text-xs text-subtle-foreground">Suggestions</span>
            {suggestions.map((a) => (
              <button
                key={a.exe}
                type="button"
                title={`${a.category} : ignorer ${a.name}`}
                onClick={() => toggle(a.exe)}
                className="flex h-7 items-center gap-1.5 rounded-full border border-border pr-2.5 pl-1.5 text-13 text-muted-foreground transition-colors hover:border-foreground/25 hover:text-foreground"
              >
                <AppIcon dataDir={dataDir} app={a.exe} className="size-4" />
                {a.name}
                <Plus className="size-3" />
              </button>
            ))}
          </div>
        )}
      </div>

      {picking && (
        <AppPicker
          apps={apps}
          ignored={ignored}
          dataDir={dataDir}
          onToggle={toggle}
          onBrowse={browse}
          onClose={() => setPicking(false)}
        />
      )}
    </section>
  );
}

type Entry = { app: InstalledApp; detail: string };
type Block = { title: string; entries: Entry[] };

function AppPicker({
  apps,
  ignored,
  dataDir,
  onToggle,
  onBrowse,
  onClose,
}: {
  apps: InstalledApp[] | null;
  ignored: string[];
  dataDir: string;
  onToggle: (exe: string) => void;
  onBrowse: () => void;
  onClose: () => void;
}) {
  const [query, setQuery] = useState("");
  const [cursor, setCursor] = useState(0);
  const listRef = useRef<HTMLDivElement>(null);

  // Sections are frozen while the dialog is open: ticking an app does not
  // move it under the pointer.
  const ignoredAtOpen = useRef(ignored).current;

  const blocks = useMemo<Block[]>(() => {
    if (!apps) return [];
    const q = fold(query.trim());
    const describe = (a: InstalledApp) =>
      [a.exe, a.running ? "ouverte" : null].filter(Boolean).join(" · ");
    if (q) {
      // A word of the name starting with the query first, then its initials
      // ("vsc" for Visual Studio Code), then anywhere in the name, the
      // executable, and last the kind of application ("mots de passe").
      const rank = (a: InstalledApp) => {
        const name = fold(a.name);
        const words = name.split(/[\s\-_.()]+/).filter(Boolean);
        if (words.some((w) => w.startsWith(q))) return 0;
        if (q.length > 1 && words.map((w) => w[0]).join("").startsWith(q)) return 1;
        if (name.includes(q)) return 2;
        if (a.exe.includes(q)) return 3;
        if (a.category && fold(a.category).includes(q)) return 4;
        return -1;
      };
      const found = apps
        .map((a) => ({ a, r: rank(a) }))
        .filter((x) => x.r >= 0)
        .sort((x, y) => x.r - y.r || fold(x.a.name).localeCompare(fold(y.a.name)))
        .map(({ a }) => ({ app: a, detail: a.category ?? describe(a) }));
      return found.length ? [{ title: plural(found.length, "résultat"), entries: found }] : [];
    }
    const suggested = apps
      .filter((a) => a.score > 0)
      .sort((a, b) => b.score - a.score)
      .map((a) => ({ app: a, detail: a.category! }));
    const recent = apps
      .filter((a) => a.copies > 0 && a.last_used && !a.score)
      .sort((a, b) => (b.last_used ?? "").localeCompare(a.last_used ?? ""))
      .slice(0, 8)
      .map((a) => ({ app: a, detail: `${plural(a.copies, "copie")} · ${timeAgo(a.last_used!)}` }));
    const shown = new Set([...suggested, ...recent].map((e) => e.app.exe));
    // Ignored apps not found on this computer (typed before, or uninstalled).
    const others = ignoredAtOpen
      .filter((exe) => !apps.some((a) => a.exe === exe))
      .map((exe) => ({
        app: { exe, name: appLabel(exe), path: null, category: null, score: 0, copies: 0, last_used: null, running: false },
        detail: `${exe} · introuvable sur cet ordinateur`,
      }));
    const all = apps.filter((a) => !shown.has(a.exe)).map((a) => ({ app: a, detail: describe(a) }));
    return [
      { title: "Suggestions", entries: suggested },
      { title: "Utilisées récemment", entries: recent },
      { title: "Toutes les applications", entries: [...others, ...all] },
    ].filter((b) => b.entries.length);
  }, [apps, query, ignoredAtOpen]);

  const flat = blocks.flatMap((b) => b.entries);
  useEffect(() => setCursor(0), [query]);
  useEffect(() => {
    listRef.current?.querySelector(`[data-index="${cursor}"]`)?.scrollIntoView({ block: "nearest" });
  }, [cursor]);

  const onKey = (e: React.KeyboardEvent) => {
    if (e.key === "ArrowDown" || e.key === "ArrowUp") {
      e.preventDefault();
      const n = flat.length;
      if (n) setCursor((c) => (c + (e.key === "ArrowDown" ? 1 : n - 1)) % n);
    } else if (e.key === "Enter" && flat[cursor]) {
      e.preventDefault();
      onToggle(flat[cursor].app.exe);
    }
  };

  let index = 0;
  return (
    <Dialog
      title="Applications ignorées"
      description="Cochez celles dont les copies ne doivent jamais être enregistrées."
      onClose={onClose}
      className="max-w-lg"
      footer={
        <>
          <Button variant="ghost" className="mr-auto" onClick={onBrowse}>
            <FolderOpen className="size-3.5" />
            Parcourir…
          </Button>
          <Button variant="primary" onClick={onClose}>
            Terminé
          </Button>
        </>
      }
    >
      <div className="relative">
        <Search className="pointer-events-none absolute top-1/2 left-2.5 size-4 -translate-y-1/2 text-subtle-foreground" />
        <Input
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          onKeyDown={onKey}
          placeholder={apps ? `Rechercher parmi ${plural(apps.length, "application")}…` : "Recherche des applications…"}
          aria-label="Rechercher une application"
          aria-controls="app-picker-list"
          className="pl-8"
        />
      </div>
      <div
        ref={listRef}
        id="app-picker-list"
        role="listbox"
        aria-label="Applications"
        aria-multiselectable="true"
        className="-mx-2 mt-3 h-[min(420px,55vh)] overflow-y-auto px-2 pb-1"
      >
        {!apps &&
          Array.from({ length: 7 }, (_, i) => (
            <div key={i} className="flex h-11 items-center gap-3 px-2.5">
              <span className="size-6 animate-pulse rounded-[5px] bg-secondary" />
              <span className="h-3 animate-pulse rounded bg-secondary" style={{ width: `${30 + ((i * 37) % 40)}%` }} />
            </div>
          ))}
        {apps && !flat.length && (
          <p className="px-2.5 py-8 text-center text-13 text-muted-foreground">
            Aucune application trouvée. « Parcourir… » permet de choisir un exécutable.
          </p>
        )}
        {blocks.map((block) => (
          <div key={block.title} role="group" aria-label={block.title}>
            <div className="sticky top-0 z-10 bg-surface px-2.5 pt-3 pb-1.5 text-xs text-subtle-foreground">{block.title}</div>
            {block.entries.map(({ app, detail }) => {
              const i = index++;
              const on = ignored.includes(app.exe);
              return (
                <button
                  key={`${block.title}-${app.exe}`}
                  type="button"
                  role="option"
                  aria-selected={on}
                  data-index={i}
                  tabIndex={-1}
                  onMouseMove={() => cursor !== i && setCursor(i)}
                  onClick={() => onToggle(app.exe)}
                  className={cn(
                    "flex h-11 w-full items-center gap-3 rounded-ctl px-2.5 text-left transition-colors duration-100",
                    i === cursor && "bg-muted/70",
                  )}
                >
                  <AppIcon dataDir={dataDir} app={app.exe} className="size-6 shrink-0" />
                  <span className="min-w-0 flex-1">
                    <span className="flex items-center gap-1.5 truncate text-sm text-foreground">
                      {app.name}
                      {app.score > 0 && <ShieldAlert className="size-3.5 shrink-0 text-subtle-foreground" aria-hidden="true" />}
                    </span>
                    <span className="block truncate text-xs text-subtle-foreground">{detail}</span>
                  </span>
                  <span
                    aria-hidden="true"
                    className={cn(
                      "flex size-[18px] shrink-0 items-center justify-center rounded-full border transition-colors duration-150",
                      on ? "border-transparent bg-brand text-brand-foreground" : "border-border",
                    )}
                  >
                    {on && <Check className="size-3" strokeWidth={3} />}
                  </span>
                </button>
              );
            })}
          </div>
        ))}
      </div>
    </Dialog>
  );
}
