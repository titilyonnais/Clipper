import { useEffect, useRef, useState } from "react";
import { ChevronDown, Clock, Download, Folder, MoreHorizontal, Pencil, Pin, Plus, Scissors, Settings as SettingsIcon, ShieldAlert, Trash2 } from "lucide-react";
import { api } from "@/lib/api";
import { useSettings } from "@/lib/settings";
import { openUpdate, useUpdate } from "@/lib/update";
import { comboLabel, useKeymap } from "@/lib/shortcuts";
import { appLabel, cn } from "@/lib/utils";
import { run } from "@/clip/actions";
import { useSlidingThumb } from "@/ui/form";
import { Menu, useMenu } from "@/ui/menu";
import { AppIcon } from "@/ui/misc";
import type { Collection, SourceApp, Stats } from "@/types";

export type View =
  | { kind: "history" }
  | { kind: "pinned" }
  | { kind: "sensitive" }
  | { kind: "snippets" }
  | { kind: "settings" }
  | { kind: "collection"; id: number }
  | { kind: "app"; name: string };

function sameView(a: View, b: View) {
  if (a.kind !== b.kind) return false;
  if (a.kind === "collection" && b.kind === "collection") return a.id === b.id;
  if (a.kind === "app" && b.kind === "app") return a.name === b.name;
  return true;
}

/** A collection being created inline, optionally with clips to file into it. */
export type NewCollection = { assign?: number[] };

/**
 * Folded width: the 10 px side padding plus a 32 px row. Every row keeps its
 * icon at the same place whether the sidebar is folded or not, so folding
 * only hides the labels.
 */
export const FOLDED_WIDTH = 52;

interface Props {
  view: View;
  onView: (v: View) => void;
  stats: Stats | null;
  collections: Collection[];
  apps: SourceApp[];
  snippetCount: number;
  width: number;
  collapsed: boolean;
  creating: NewCollection | null;
  onCreating: (c: NewCollection | null) => void;
  /** A name is being typed: the sidebar must stay unfolded. */
  onNaming: (naming: boolean) => void;
}

export function Sidebar({ view, onView, stats, collections, apps, snippetCount, width, collapsed, creating, onCreating, onNaming }: Props) {
  const { settings } = useSettings();
  const update = useUpdate();
  const keymap = useKeymap();
  const [dropTarget, setDropTarget] = useState<number | null>(null);
  const [showAllApps, setShowAllApps] = useState(false);
  const [renaming, setRenaming] = useState<number | null>(null);
  const menu = useMenu();
  const [menuFor, setMenuFor] = useState<Collection | null>(null);
  useEffect(() => onNaming(renaming !== null), [renaming, onNaming]);

  const visibleApps = showAllApps ? apps : apps.slice(0, 5);
  const navRef = useRef<HTMLElement>(null);
  const thumb = useSlidingThumb(
    navRef,
    `${JSON.stringify(view)}|${collections.length}|${visibleApps.length}|${!!creating}|${renaming}`,
    '[aria-current="page"]',
  );

  const item = (v: View, label: string, icon: React.ReactNode, count?: number) => (
    <NavItem active={sameView(view, v)} collapsed={collapsed} onClick={() => onView(v)} icon={icon} label={label} count={count} />
  );

  const onDrop = (e: React.DragEvent, collectionId: number) => {
    e.preventDefault();
    setDropTarget(null);
    const id = Number(e.dataTransfer.getData("application/x-clipper-clip"));
    if (id) run(() => api.setCollection([id], collectionId), "Rangé.");
  };

  const create = (name: string) =>
    run(async () => {
      const id = await api.createCollection(name);
      if (creating?.assign?.length) await api.setCollection(creating.assign, id);
      onCreating(null);
      onView({ kind: "collection", id });
    });

  return (
    <aside
      className="flex shrink-0 flex-col overflow-hidden bg-background transition-[width] duration-200 ease-out-soft"
      style={{ width: collapsed ? FOLDED_WIDTH : width }}
      aria-label="Navigation"
    >
      <nav ref={navRef} className="relative min-h-0 flex-1 overflow-x-hidden overflow-y-auto px-2.5 py-3">
        {thumb && (
          <span
            aria-hidden="true"
            className={cn(
              "absolute inset-x-2.5 top-0 rounded-ctl bg-selected",
              thumb.animate && "transition-[transform,height] duration-200 ease-out-soft",
            )}
            style={{ height: thumb.height, transform: `translateY(${thumb.top}px)` }}
          />
        )}
        <div className="space-y-px">
          {item({ kind: "history" }, "Historique", <Clock />, stats?.total)}
          {item({ kind: "pinned" }, "Épinglés", <Pin />, stats?.pinned)}
          {item({ kind: "sensitive" }, "Sensibles", <ShieldAlert />, stats?.sensitive)}
          {item({ kind: "snippets" }, "Snippets", <Scissors />, snippetCount)}
        </div>

        <Section title="Collections" collapsed={collapsed}>
          {collections.map((c) =>
            renaming === c.id ? (
              <NameInput
                key={c.id}
                initial={c.name}
                onSubmit={(name) =>
                  run(async () => {
                    if (name !== c.name) await api.renameCollection(c.id, name);
                    setRenaming(null);
                  })
                }
                onCancel={() => setRenaming(null)}
              />
            ) : (
              <div
                key={c.id}
                onDragOver={(e) => {
                  if (e.dataTransfer.types.includes("application/x-clipper-clip")) {
                    e.preventDefault();
                    setDropTarget(c.id);
                  }
                }}
                onDragLeave={() => setDropTarget((t) => (t === c.id ? null : t))}
                onDrop={(e) => onDrop(e, c.id)}
                onDoubleClick={() => !collapsed && setRenaming(c.id)}
                onContextMenu={(e) => {
                  e.preventDefault();
                  setMenuFor(c);
                  menu.openAt(e.clientX, e.clientY);
                }}
                className={cn("rounded-ctl transition-shadow", dropTarget === c.id && "ring-1 ring-foreground/40")}
              >
                <NavItem
                  active={view.kind === "collection" && view.id === c.id}
                  collapsed={collapsed}
                  onClick={() => onView({ kind: "collection", id: c.id })}
                  icon={<Folder />}
                  label={c.name}
                  count={c.count}
                  trailing={
                    <span
                      role="button"
                      tabIndex={-1}
                      aria-label={`Options de ${c.name}`}
                      onClick={(e) => {
                        e.stopPropagation();
                        setMenuFor(c);
                        menu.openBelow(e.currentTarget as HTMLElement, "end");
                      }}
                      className="hidden size-6 items-center justify-center rounded-ctl-sm text-subtle-foreground group-hover:flex hover:bg-secondary hover:text-foreground"
                    >
                      <MoreHorizontal className="size-4" />
                    </span>
                  }
                />
              </div>
            ),
          )}
          {creating ? (
            <NameInput placeholder="Nom de la collection" onSubmit={create} onCancel={() => onCreating(null)} />
          ) : (
            <NavItem
              active={false}
              collapsed={collapsed}
              muted
              onClick={() => onCreating({})}
              icon={<Plus />}
              label="Nouvelle collection"
              title={`Nouvelle collection (${comboLabel(keymap.new_collection)})`}
            />
          )}
        </Section>

        {apps.length > 0 && (
          <Section title="Applications" collapsed={collapsed}>
            {visibleApps.map((a) => (
              <NavItem
                key={a.name}
                active={view.kind === "app" && view.name === a.name}
                collapsed={collapsed}
                onClick={() => onView({ kind: "app", name: a.name })}
                icon={<AppIcon dataDir={settings?.data_dir} app={a.name} className="size-4" />}
                label={appLabel(a.name)}
                count={a.count}
              />
            ))}
            {apps.length > 5 && (
              <NavItem
                active={false}
                collapsed={collapsed}
                muted
                onClick={() => setShowAllApps((v) => !v)}
                icon={<ChevronDown className={cn("transition-transform duration-200", showAllApps && "rotate-180")} />}
                label={showAllApps ? "Afficher moins" : `Afficher tout (${apps.length})`}
              />
            )}
          </Section>
        )}
      </nav>

      <div className="space-y-px border-t border-line px-2.5 py-2.5">
        {update.info && update.status === "available" && (
          <NavItem
            active={false}
            collapsed={collapsed}
            onClick={openUpdate}
            icon={<Download className="text-brand" />}
            label="Mise à jour disponible"
            title={`Clipper ${update.info.version} est disponible`}
          />
        )}
        <NavItem
          active={view.kind === "settings"}
          solid
          collapsed={collapsed}
          onClick={() => onView({ kind: "settings" })}
          icon={<SettingsIcon />}
          label="Paramètres"
          title={`Paramètres (${comboLabel(keymap.settings)})`}
        />
      </div>

      {menu.anchor && menuFor && (
        <Menu
          anchor={menu.anchor}
          onClose={menu.close}
          entries={[
            { label: "Renommer", icon: <Pencil />, onSelect: () => setRenaming(menuFor.id) },
            {
              label: "Supprimer la collection",
              icon: <Trash2 />,
              danger: true,
              onSelect: () =>
                run(async () => {
                  await api.deleteCollection(menuFor.id);
                  if (view.kind === "collection" && view.id === menuFor.id) onView({ kind: "history" });
                }, "Collection supprimée, ses éléments restent dans l'historique."),
            },
          ]}
        />
      )}
    </aside>
  );
}

/** Section heading; folded, it becomes a short rule of the same height. */
function Section({ title, collapsed, children }: { title: string; collapsed: boolean; children: React.ReactNode }) {
  return (
    <div className="mt-4">
      <div className="relative mb-1 flex h-7 items-center px-2">
        <h3
          className={cn(
            "text-xs font-normal whitespace-nowrap text-subtle-foreground transition-opacity duration-150",
            collapsed && "opacity-0",
          )}
        >
          {title}
        </h3>
        <span
          aria-hidden="true"
          className={cn(
            "absolute top-1/2 left-2 h-px w-4 bg-border transition-opacity duration-150",
            collapsed ? "opacity-100" : "opacity-0",
          )}
        />
      </div>
      <div className="space-y-px">{children}</div>
    </div>
  );
}

/** Inline name field: Entrée validates, Échap or an empty field cancels. */
function NameInput({
  initial = "",
  placeholder,
  onSubmit,
  onCancel,
}: {
  initial?: string;
  placeholder?: string;
  onSubmit: (name: string) => void;
  onCancel: () => void;
}) {
  const [value, setValue] = useState(initial);
  const ref = useRef<HTMLInputElement>(null);
  useEffect(() => {
    ref.current?.focus();
    ref.current?.select();
  }, []);
  const done = () => (value.trim() ? onSubmit(value.trim()) : onCancel());
  return (
    <div className="flex h-8 animate-in items-center gap-2.5 rounded-ctl bg-muted px-2 ring-1 ring-foreground/25">
      <Folder className="size-4 shrink-0 text-subtle-foreground" />
      <input
        ref={ref}
        value={value}
        maxLength={48}
        placeholder={placeholder}
        spellCheck={false}
        aria-label="Nom de la collection"
        onChange={(e) => setValue(e.target.value)}
        onBlur={done}
        onKeyDown={(e) => {
          e.stopPropagation();
          if (e.key === "Enter") {
            e.preventDefault();
            done();
          } else if (e.key === "Escape") {
            e.preventDefault();
            onCancel();
          }
        }}
        className="min-w-0 flex-1 bg-transparent text-sm text-foreground placeholder:text-subtle-foreground"
      />
    </div>
  );
}

function NavItem({
  active,
  onClick,
  icon,
  label,
  count,
  trailing,
  solid,
  muted,
  collapsed,
  title,
}: {
  active: boolean;
  onClick: () => void;
  icon: React.ReactNode;
  label: string;
  count?: number;
  trailing?: React.ReactNode;
  /** Draw its own highlight (outside the list with the sliding indicator). */
  solid?: boolean;
  /** A secondary action rather than a place. */
  muted?: boolean;
  collapsed: boolean;
  title?: string;
}) {
  const tooltip = title ?? (count !== undefined ? `${label} (${count})` : label);
  return (
    <button
      type="button"
      onClick={onClick}
      title={collapsed || title ? tooltip : undefined}
      aria-label={label}
      aria-current={active ? "page" : undefined}
      className={cn(
        "group relative flex h-8 w-full items-center gap-2.5 overflow-hidden rounded-ctl px-2 text-left text-sm whitespace-nowrap transition-colors duration-150",
        "[&_svg]:size-4 [&_svg]:shrink-0",
        active
          ? cn("text-foreground", solid && "bg-selected")
          : cn("hover:bg-muted/60 hover:text-foreground", muted ? "text-subtle-foreground" : "text-muted-foreground"),
      )}
    >
      <span className={cn("flex w-4 shrink-0 justify-center", active ? "text-foreground" : "text-subtle-foreground group-hover:text-foreground")}>
        {icon}
      </span>
      <span className={cn("flex min-w-0 flex-1 items-center gap-2.5 transition-opacity duration-150", collapsed && "opacity-0")}>
        <span className="min-w-0 flex-1 truncate">{label}</span>
        {!collapsed && trailing}
        {count !== undefined && (
          <span className={cn("tabular text-xs text-subtle-foreground", !!trailing && !collapsed && "group-hover:hidden")}>{count}</span>
        )}
      </span>
    </button>
  );
}
