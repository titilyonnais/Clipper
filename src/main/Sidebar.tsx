import { useState } from "react";
import { Clock, Folder, MoreHorizontal, Pencil, Pin, Plus, Scissors, Settings as SettingsIcon, Trash2 } from "lucide-react";
import { api } from "@/lib/api";
import { useSettings } from "@/lib/settings";
import { appLabel, cn } from "@/lib/utils";
import { run } from "@/clip/actions";
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

interface Props {
  view: View;
  onView: (v: View) => void;
  stats: Stats | null;
  collections: Collection[];
  apps: SourceApp[];
  snippetCount: number;
  onNewCollection: () => void;
  onRenameCollection: (c: Collection) => void;
}

export function Sidebar({ view, onView, stats, collections, apps, snippetCount, onNewCollection, onRenameCollection }: Props) {
  const { settings } = useSettings();
  const [dropTarget, setDropTarget] = useState<number | null>(null);
  const [showAllApps, setShowAllApps] = useState(false);
  const menu = useMenu();
  const [menuFor, setMenuFor] = useState<Collection | null>(null);

  const item = (v: View, label: string, icon: React.ReactNode, count?: number) => (
    <NavItem active={sameView(view, v)} onClick={() => onView(v)} icon={icon} label={label} count={count} />
  );

  const onDrop = (e: React.DragEvent, collectionId: number) => {
    e.preventDefault();
    setDropTarget(null);
    const id = Number(e.dataTransfer.getData("application/x-clipper-clip"));
    if (id) run(() => api.setCollection([id], collectionId), "Rangé.");
  };

  const visibleApps = showAllApps ? apps : apps.slice(0, 5);

  return (
    <aside className="flex w-56 shrink-0 flex-col border-r border-line bg-background">
      <nav className="min-h-0 flex-1 overflow-y-auto px-2.5 py-3">
        <div className="space-y-px">
          {item({ kind: "history" }, "Historique", <Clock />, stats?.total)}
          {item({ kind: "pinned" }, "Épinglés", <Pin />, stats?.pinned)}
          {item({ kind: "snippets" }, "Snippets", <Scissors />, snippetCount)}
        </div>

        <Section
          title="Collections"
          action={
            <button
              type="button"
              title="Nouvelle collection"
              aria-label="Nouvelle collection"
              onClick={onNewCollection}
              className="rounded-ctl-sm p-0.5 text-subtle-foreground hover:bg-muted hover:text-foreground"
            >
              <Plus className="size-3.5" />
            </button>
          }
        >
          {collections.length === 0 && (
            <p className="px-2 py-1 text-xs leading-relaxed text-subtle-foreground">
              Glissez un élément ici après avoir créé une collection.
            </p>
          )}
          {collections.map((c) => (
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
              onContextMenu={(e) => {
                e.preventDefault();
                setMenuFor(c);
                menu.openAt(e.clientX, e.clientY);
              }}
              className={cn("rounded-ctl", dropTarget === c.id && "ring-1 ring-foreground/40")}
            >
              <NavItem
                active={view.kind === "collection" && view.id === c.id}
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
                    className="hidden rounded-ctl-sm p-0.5 text-subtle-foreground group-hover:block hover:bg-secondary hover:text-foreground"
                  >
                    <MoreHorizontal className="size-3.5" />
                  </span>
                }
              />
            </div>
          ))}
        </Section>

        {apps.length > 0 && (
          <Section title="Applications">
            {visibleApps.map((a) => (
              <NavItem
                key={a.name}
                active={view.kind === "app" && view.name === a.name}
                onClick={() => onView({ kind: "app", name: a.name })}
                icon={<AppIcon dataDir={settings?.data_dir} app={a.name} className="size-4" />}
                label={appLabel(a.name)}
                count={a.count}
              />
            ))}
            {apps.length > 5 && (
              <button
                type="button"
                onClick={() => setShowAllApps((v) => !v)}
                className="px-2 py-1 text-xs text-subtle-foreground hover:text-foreground"
              >
                {showAllApps ? "Afficher moins" : `Afficher tout (${apps.length})`}
              </button>
            )}
          </Section>
        )}
      </nav>

      <div className="border-t border-line p-2.5">
        {item({ kind: "settings" }, "Paramètres", <SettingsIcon />)}
      </div>

      {menu.anchor && menuFor && (
        <Menu
          anchor={menu.anchor}
          onClose={menu.close}
          entries={[
            { label: "Renommer…", icon: <Pencil />, onSelect: () => onRenameCollection(menuFor) },
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

function Section({ title, action, children }: { title: string; action?: React.ReactNode; children: React.ReactNode }) {
  return (
    <div className="mt-5">
      <div className="mb-1 flex h-6 items-center justify-between px-2">
        <h3 className="text-xs font-normal text-subtle-foreground">{title}</h3>
        {action}
      </div>
      <div className="space-y-px">{children}</div>
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
}: {
  active: boolean;
  onClick: () => void;
  icon: React.ReactNode;
  label: string;
  count?: number;
  trailing?: React.ReactNode;
}) {
  return (
    <button
      type="button"
      onClick={onClick}
      aria-current={active ? "page" : undefined}
      className={cn(
        "group flex h-8 w-full items-center gap-2.5 rounded-ctl px-2 text-left text-sm transition-colors duration-150",
        "[&_svg]:size-4 [&_svg]:shrink-0",
        active ? "bg-selected text-foreground" : "text-muted-foreground hover:bg-muted/60 hover:text-foreground",
      )}
    >
      <span className={cn("flex w-4 justify-center", active ? "text-foreground" : "text-subtle-foreground")}>{icon}</span>
      <span className="flex-1 truncate">{label}</span>
      {trailing}
      {count !== undefined && (
        <span className={cn("tabular text-xs text-subtle-foreground", !!trailing && "group-hover:hidden")}>{count}</span>
      )}
    </button>
  );
}
