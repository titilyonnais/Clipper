import { useCallback, useEffect, useState } from "react";
import { Check, Folder, Pencil, Tag, Trash2 } from "lucide-react";
import { api, errorText } from "@/lib/api";
import { cn } from "@/lib/utils";
import { Dialog } from "./ui";

type Tab = "categories" | "tags";

export function CategoriesManager({ onClose }: { onClose: () => void }) {
  const [tab, setTab] = useState<Tab>("categories");
  const [items, setItems] = useState<[string, number][]>([]);
  const [editing, setEditing] = useState<string | null>(null);
  const [draft, setDraft] = useState("");
  const [confirm, setConfirm] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  const reload = useCallback(async () => {
    setItems(await (tab === "categories" ? api.categoryCounts() : api.tagCounts()));
  }, [tab]);

  useEffect(() => {
    setEditing(null);
    setConfirm(null);
    reload();
  }, [reload]);

  const act = async (fn: () => Promise<unknown>) => {
    try {
      await fn();
      setError(null);
    } catch (e) {
      setError(errorText(e));
    }
    setEditing(null);
    setConfirm(null);
    reload();
  };

  const rename = (old: string) => {
    const name = draft.trim();
    if (!name || name === old) return setEditing(null);
    act(() => (tab === "categories" ? api.renameCategory(old, name) : api.renameTag(old, name)));
  };

  const Icon = tab === "categories" ? Folder : Tag;

  return (
    <Dialog title="Catégories et tags" onClose={onClose}>
      <div className="flex gap-1 border-b border-ink-700/60 px-6 pt-2">
        {(
          [
            ["categories", "Catégories", Folder],
            ["tags", "Tags", Tag],
          ] as const
        ).map(([key, label, TabIcon]) => (
          <button
            key={key}
            onClick={() => setTab(key)}
            className={cn(
              "-mb-px inline-flex h-9 items-center gap-1.5 border-b-2 px-3 text-[12.5px] font-medium",
              tab === key ? "border-accent text-ink-50" : "border-transparent text-ink-400 hover:text-ink-100",
            )}
          >
            <TabIcon size={13} /> {label}
          </button>
        ))}
      </div>

      <div className="space-y-1.5 p-6">
        <p className="mb-3 text-[11.5px] leading-relaxed text-ink-500">
          Pour en créer, sélectionnez un élément et modifiez sa catégorie ou ses tags en bas de l'aperçu, ou utilisez
          l'action IA « Classer ».
        </p>
        {error && <p className="text-[12px] text-red-400">{error}</p>}
        {items.length === 0 && (
          <p className="py-8 text-center text-[12.5px] text-ink-400">
            {tab === "categories" ? "Aucune catégorie." : "Aucun tag."}
          </p>
        )}
        {items.map(([name, count]) => (
          <div key={name} className="flex items-center gap-2 rounded-lg border border-ink-700/40 bg-ink-800/40 px-3 py-2">
            <Icon size={13} className="shrink-0 text-accent" />
            {editing === name ? (
              <input
                autoFocus
                value={draft}
                onChange={(e) => setDraft(e.target.value)}
                onKeyDown={(e) => {
                  if (e.key === "Enter") rename(name);
                  if (e.key === "Escape") {
                    e.preventDefault();
                    setEditing(null);
                  }
                }}
                className="h-7 flex-1 rounded border border-ink-700 bg-ink-800 px-2 text-[13px] text-ink-50"
              />
            ) : (
              <span className="flex-1 truncate text-[13px] text-ink-50">{tab === "tags" ? `#${name}` : name}</span>
            )}
            <span className="text-[11px] tabular-nums text-ink-500">
              {count} élément{count > 1 ? "s" : ""}
            </span>
            {editing === name ? (
              <SmallButton title="Enregistrer" onClick={() => rename(name)}>
                <Check size={12} className="text-accent" />
              </SmallButton>
            ) : confirm === name ? (
              <button
                onClick={() => act(() => (tab === "categories" ? api.deleteCategory(name) : api.deleteTag(name)))}
                className="h-7 rounded-md bg-red-600 px-2 text-[11.5px] font-medium text-white hover:bg-red-700"
              >
                Retirer de {count} élément{count > 1 ? "s" : ""}
              </button>
            ) : (
              <>
                <SmallButton
                  title="Renommer"
                  onClick={() => {
                    setEditing(name);
                    setDraft(name);
                  }}
                >
                  <Pencil size={11} />
                </SmallButton>
                <SmallButton title="Supprimer (les éléments sont conservés)" onClick={() => setConfirm(name)} danger>
                  <Trash2 size={11} />
                </SmallButton>
              </>
            )}
          </div>
        ))}
      </div>
    </Dialog>
  );
}

function SmallButton({
  children,
  onClick,
  title,
  danger,
}: {
  children: React.ReactNode;
  onClick: () => void;
  title: string;
  danger?: boolean;
}) {
  return (
    <button
      onClick={onClick}
      title={title}
      aria-label={title}
      className={cn(
        "flex h-7 w-7 items-center justify-center rounded-md text-ink-400 hover:bg-ink-700 hover:text-ink-50",
        danger && "hover:bg-red-500/20 hover:text-red-400",
      )}
    >
      {children}
    </button>
  );
}
