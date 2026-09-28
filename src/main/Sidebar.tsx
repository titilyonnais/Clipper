import { useEffect, useRef, useState } from "react";
import {
  Clock,
  Folder,
  MoreHorizontal,
  PanelLeftClose,
  PanelLeftOpen,
  Pencil,
  Pin,
  Plus,
  Scissors,
  Settings as SettingsIcon,
  Trash2,
} from "lucide-react";
import { api } from "@/lib/api";
import { useSettings } from "@/lib/settings";
import { comboLabel, useKeymap } from "@/lib/shortcuts";
import { appLabel, cn } from "@/lib/utils";
import { run } from "@/clip/actions";
import { IconButton } from "@/ui/button";
import { useSlidingThumb } from "@/ui/form";
import { Menu, useMenu } from "@/ui/menu";
import { AppIcon } from "@/ui/misc";
import type { Collection, SourceApp, Stats } from "@/types";

export type View =
  | { kind: "history" }
  | { kind: "pinned" }
  | { kind: "snippets" }
  | { kind: "settings" }
  | { kind: "collection"; id: number }
  | { kind: "app"; name: string };

export function sameView(a: View, b: View) {
  if (a.kind !== b.kind) return false;
  if (a.kind === "collection" && b.kind === "collection") return a.id === b.id;
  if (a.kind === "app" && b.kind === "app") return a.name === b.name;
  return true;
}

/** A collection being created inline, optionally with clips to file into it. */
export type NewCollection = { assign?: number[] };

interface Props {
  view: View;
  onView: (v: View) => void;
  stats: Stats | null;
  collections: Collection[];
  apps: SourceApp[];
  snippetCount: number;
  width: number;
  collapsed: boolean;
  onToggleCollapsed: () => void;
  creating: NewCollection | null;
  onCreating: (c: NewCollection | null) => void;
}

export function Sidebar({
  view,
  onView,
  stats,
  collections,
  apps,
  snippetCount,
  width,
  collapsed,
  onToggleCollapsed,
  creating,
  onCreating,
}: Props) {
  const { settings } = useSettings();
  const keymap = useKeymap();
  const [dropTarget, setDropTarget] = useState<number | null>(null);
  const [showAllApps, setShowAllApps] = useState(false);
  const [renaming, setRenaming] = useState<number | null>(null);
  const menu = useMenu();
  const [menuFor, setMenuFor] = useState<Collection | null>(null);

  const visibleApps = showAllApps ? apps : apps.slice(0, 5);
  const navRef = useRef<HTMLElement>(null);
  const thumb = useSlidingThumb(
    navRef,
    `${JSON.stringify(view)}|${collections.length}|${visibleApps.length}|${collapsed}|${!!creating}|${renaming}`,
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

  const newCollectionLabel = `Nouvelle collection (${comboLabel(keymap.new_collection)})`;

  return (
    <aside
      className="flex shrink-0 flex-col bg-background transition-[width] duration-200 ease-out-soft"
      style={{ width: collapsed ? 60 : width }}
      aria-label="Navigation"
    >
      <nav ref={navRef} className={cn("relative min-h-0 flex-1 overflow-x-hidden overflow-y-auto py-3", collapsed ? "px-2" : "px-2.5")}>
        {thumb && (
          <span
            aria-hidden="true"
            className={cn(
              "absolute top-0 rounded-ctl bg-selected",
              collapsed ? "inset-x-2" : "inset-x-2.5",
              thumb.animate && "transition-[transform,height] duration-200 ease-out-soft",
            )}
            style={{ height: thumb.height, transform: `translateY(${thumb.top}px)` }}
          />
        )}
        <div className="space-y-px">
          {item({ kind: "history" }, "Historique", <Clock />, stats?.total)}
          {item({ kind: "pinned" }, "Épinglés", <Pin />, stats?.pinned)}
          {item({ kind: "snippets" }, "Snippets", <Scissors />, snippetCount)}
        </div>

        <Section
          title="Collections"
          collapsed={collapsed}
          action={
            <IconButton label={newCollectionLabel} size="sm" onClick={() => onCreating(creating ? null : {})}>
              <Plus />
            </IconButton>
          }
        >
          {collections.length === 0 && !creating && !collapsed && (
            <p className="px-2 py-1 text-xs leading-relaxed text-subtle-foreground">
              Créez une collection avec le bouton +, puis glissez-y des éléments.
            </p>
          )}
          {collections.map((c) =>
            renaming === c.id && !collapsed ? (
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
          {creating && !collapsed && (
            <NameInput placeholder="Nom de la collection" onSubmit={create} onCancel={() => onCreating(null)} />
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
            {apps.length > 5 && !collapsed && (
              <button
                type="button"
                onClick={() => setShowAllApps((v) => !v)}
                className="h-7 px-2 text-xs text-subtle-foreground transition-colors hover:text-foreground"
              >
                {showAllApps ? "Afficher moins" : `Afficher tout (${apps.length})`}
              </button>
            )}
          </Section>
        )}
      </nav>

      <div className={cn("flex gap-1 border-t border-line", collapsed ? "flex-col items-center p-2" : "items-center p-2.5")}>
        <div className={cn(!collapsed && "min-w-0 flex-1")}>
          <NavItem
            active={view.kind === "settings"}
            solid
            collapsed={collapsed}
            onClick={() => onView({ kind: "settings" })}
            icon={<SettingsIcon />}
            label="Paramètres"
          />
        </div>
        <IconButton
          label={`${collapsed ? "Déplier" : "Replier"} la barre latérale (${comboLabel(keymap.toggle_sidebar)})`}
          onClick={onToggleCollapsed}
        >
          {collapsed ? <PanelLeftOpen /> : <PanelLeftClose />}
        </IconButton>
      </div>

      {menu.anchor && menuFor && (
        <Menu
          anchor={menu.anchor}
          onClose={menu.close}
          entries={[
            {
              label: "Renommer",
              icon: <Pencil />,
              onSelect: () => {
                if (collapsed) onToggleCollapsed();
                setRenaming(menuFor.id);
              },
            },
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

function Section({
  title,
  action,
  collapsed,
  children,
}: {
  title: string;
  action?: React.ReactNode;
  collapsed: boolean;
  children: React.ReactNode;
}) {
  return (
    <div className="mt-5">
      {collapsed ? (
        <div className="mb-2 flex flex-col items-center gap-2">
          <div className="h-px w-6 bg-line" />
          {action}
        </div>
      ) : (
        <div className="mb-1 flex h-7 items-center justify-between pr-0.5 pl-2">
          <h3 className="text-xs font-normal text-subtle-foreground">{title}</h3>
          {action}
        </div>
      )}
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
  collapsed,
}: {
  active: boolean;
  onClick: () => void;
  icon: React.ReactNode;
  label: string;
  count?: number;
  trailing?: React.ReactNode;
  /** Draw its own highlight (outside the list with the sliding indicator). */
  solid?: boolean;
  collapsed?: boolean;
}) {
  return (
    <button
      type="button"
      onClick={onClick}
      title={collapsed ? (count !== undefined ? `${label} (${count})` : label) : undefined}
      aria-label={label}
      aria-current={active ? "page" : undefined}
      className={cn(
        "group relative flex h-8 items-center rounded-ctl text-left text-sm transition-colors duration-150",
        "[&_svg]:size-4 [&_svg]:shrink-0",
        collapsed ? "mx-auto w-9 justify-center" : "w-full gap-2.5 px-2",
        active ? cn("text-foreground", solid && "bg-selected") : "text-muted-foreground hover:bg-muted/60 hover:text-foreground",
      )}
    >
      <span className={cn("flex w-4 justify-center", active ? "text-foreground" : "text-subtle-foreground group-hover:text-foreground")}>
        {icon}
      </span>
      {!collapsed && (
        <>
          <span className="flex-1 truncate">{label}</span>
          {trailing}
          {count !== undefined && (
            <span className={cn("tabular text-xs text-subtle-foreground", !!trailing && "group-hover:hidden")}>{count}</span>
          )}
        </>
      )}
    </button>
  );
}
