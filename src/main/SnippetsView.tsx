import { forwardRef, useCallback, useEffect, useImperativeHandle, useMemo, useRef, useState } from "react";
import { Copy, Plus, Scissors, Search, Trash2 } from "lucide-react";
import { api } from "@/lib/api";
import { useDebounced, useTauriEvent } from "@/lib/hooks";
import { cn } from "@/lib/utils";
import { run } from "@/clip/actions";
import { Button } from "@/ui/button";
import { Dialog } from "@/ui/dialog";
import { Input, Textarea } from "@/ui/form";
import { EmptyState, Kbd } from "@/ui/misc";
import { Splitter, type PanelWidth } from "@/ui/splitter";
import type { Snippet } from "@/types";

const VARIABLES: [string, string][] = [
  ["{date}", "Date du jour"],
  ["{heure}", "Heure"],
  ["{jour}", "Jour de la semaine"],
  ["{presse-papiers}", "Contenu du presse-papiers"],
];

const EMPTY: Snippet = { id: 0, title: "", abbreviation: "", content: "", use_count: 0, updated_at: "" };

export function useSnippets(query = "") {
  const [snippets, setSnippets] = useState<Snippet[]>([]);
  const reload = () => api.snippets(query).then(setSnippets);
  // eslint-disable-next-line react-hooks/exhaustive-deps
  useEffect(() => void reload(), [query]);
  useTauriEvent("snippets:changed", reload);
  return snippets;
}

export interface SnippetsHandle {
  /** Run `fn` now, or once unsaved changes are saved or discarded. */
  guard: (fn: () => void) => void;
}

export const SnippetsView = forwardRef<SnippetsHandle, { listPanel: PanelWidth }>(function SnippetsView(
  { listPanel },
  ref,
) {
  const [query, setQuery] = useState("");
  const q = useDebounced(query.trim(), 100);
  const snippets = useSnippets(q);
  const [selected, setSelected] = useState<number | null>(null);
  const [draft, setDraft] = useState<Snippet | null>(null);
  const contentRef = useRef<HTMLTextAreaElement>(null);

  const current = useMemo(() => snippets.find((s) => s.id === selected) ?? null, [snippets, selected]);
  useEffect(() => {
    if (selected === null && snippets.length && !draft) setSelected(snippets[0].id);
  }, [snippets, selected, draft]);
  useEffect(() => {
    if (current) setDraft({ ...current, abbreviation: current.abbreviation ?? "" });
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [current?.id, current?.updated_at]);

  const dirty =
    draft !== null &&
    (!current ||
      draft.title !== current.title ||
      (draft.abbreviation ?? "") !== (current.abbreviation ?? "") ||
      draft.content !== current.content);

  const save = () =>
    draft
      ? run(async () => {
          const id = await api.saveSnippet({ ...draft, abbreviation: draft.abbreviation?.trim() || null });
          // A second save updates this snippet instead of creating another.
          setDraft((d) => (d ? { ...d, id } : d));
          setSelected(id);
          return true;
        }, "Snippet enregistré.")
      : Promise.resolve(undefined);

  const [leaving, setLeaving] = useState<(() => void) | null>(null);
  const dirtyRef = useRef(dirty);
  dirtyRef.current = dirty;
  const guard = useCallback((fn: () => void) => {
    if (dirtyRef.current) setLeaving(() => fn);
    else fn();
  }, []);
  useImperativeHandle(ref, () => ({ guard }), [guard]);

  const insertVariable = (v: string) => {
    const el = contentRef.current;
    if (!el || !draft) return;
    const start = el.selectionStart;
    const next = draft.content.slice(0, start) + v + draft.content.slice(el.selectionEnd);
    setDraft({ ...draft, content: next });
    requestAnimationFrame(() => {
      el.focus();
      el.setSelectionRange(start + v.length, start + v.length);
    });
  };

  return (
    <div className="flex min-w-0 flex-1 animate-in">
      <section className="flex shrink-0 flex-col" style={{ width: listPanel.width }} aria-label="Snippets">
        <div className="flex gap-2 border-b border-line px-3.5 pt-3.5 pb-3">
          <div className="relative flex-1">
            <Search className="pointer-events-none absolute top-1/2 left-2.5 size-4 -translate-y-1/2 text-subtle-foreground" />
            <Input value={query} onChange={(e) => setQuery(e.target.value)} placeholder="Rechercher un snippet…" className="h-9 pl-8.5" />
          </div>
          <Button
            size="lg"
            variant="primary"
            onClick={() =>
              guard(() => {
                setSelected(null);
                setDraft({ ...EMPTY });
              })
            }
          >
            <Plus /> Nouveau
          </Button>
        </div>
        <div className="stagger min-h-0 flex-1 overflow-y-auto px-2 py-1.5" role="listbox" aria-label="Snippets">
          {snippets.map((s) => (
            <button
              key={s.id}
              type="button"
              role="option"
              aria-selected={s.id === selected}
              onClick={() => s.id !== selected && guard(() => setSelected(s.id))}
              className={cn(
                "flex w-full flex-col gap-1 rounded-ctl px-3 py-2.5 text-left transition-colors duration-150",
                s.id === selected ? "bg-selected" : "hover:bg-muted/60",
              )}
            >
              <span className="flex w-full items-center gap-2">
                <span className="flex-1 truncate text-13 text-foreground">{s.title}</span>
                {s.abbreviation && <Kbd>{s.abbreviation}</Kbd>}
              </span>
              <span className="clamp-1 text-xs text-subtle-foreground">{s.content}</span>
            </button>
          ))}
          {!snippets.length && (
            <EmptyState icon={<Scissors />} title={q ? "Aucun snippet trouvé" : "Aucun snippet"}>
              Les snippets sont des textes permanents à coller en deux touches : signature, adresse, réponses types…
            </EmptyState>
          )}
        </div>
      </section>
      <Splitter panel={listPanel} label="Largeur de la liste" />

      <section className="flex min-w-0 flex-1 flex-col" aria-label="Édition du snippet">
        {draft ? (
          <form
            className="flex min-h-0 flex-1 flex-col gap-4 overflow-auto px-6 py-5"
            onSubmit={(e) => {
              e.preventDefault();
              save();
            }}
            onKeyDown={(e) => {
              if (e.ctrlKey && e.key.toLowerCase() === "s") {
                e.preventDefault();
                save();
              }
            }}
          >
            <div className="grid grid-cols-[1fr_180px] gap-3">
              <label className="space-y-1.5">
                <span className="text-13 text-muted-foreground">Titre</span>
                <Input
                  value={draft.title}
                  onChange={(e) => setDraft({ ...draft, title: e.target.value })}
                  placeholder="Signature"
                  autoFocus={!current}
                  className="h-9"
                />
              </label>
              <label className="space-y-1.5">
                <span className="text-13 text-muted-foreground">Abréviation</span>
                <Input
                  value={draft.abbreviation ?? ""}
                  onChange={(e) => setDraft({ ...draft, abbreviation: e.target.value })}
                  placeholder=";sig"
                  className="h-9 font-mono"
                />
              </label>
            </div>
            <label className="flex min-h-0 flex-1 flex-col gap-1.5">
              <span className="text-13 text-muted-foreground">Texte</span>
              <Textarea
                ref={contentRef}
                value={draft.content}
                onChange={(e) => setDraft({ ...draft, content: e.target.value })}
                className="min-h-48 flex-1 font-mono text-[12.5px]"
              />
            </label>
            <div className="flex flex-wrap items-center gap-1.5">
              <span className="mr-1 text-xs text-subtle-foreground">Insérer</span>
              {VARIABLES.map(([v, label]) => (
                <Button key={v} size="xs" variant="outline" title={label} onClick={() => insertVariable(v)}>
                  <span className="font-mono">{v}</span>
                </Button>
              ))}
            </div>
            <p className="text-13 leading-relaxed text-muted-foreground">
              Dans le collage rapide, tapez l'abréviation puis Entrée pour coller le snippet. Les variables sont remplacées au moment du collage.
            </p>
            <div className="flex items-center gap-2 border-t border-line pt-4">
              <Button type="submit" variant="primary" disabled={!dirty || !draft.title.trim() || !draft.content}>
                Enregistrer <Kbd className="border-brand-foreground/30 text-brand-foreground/70">Ctrl S</Kbd>
              </Button>
              {current && (
                <>
                  <Button onClick={() => run(() => api.pasteSnippet(current.id), "Copié.")}>
                    <Copy /> Copier
                  </Button>
                  <Button
                    variant="ghost"
                    className="ml-auto hover:text-danger"
                    onClick={() =>
                      run(async () => {
                        await api.deleteSnippet(current.id);
                        setSelected(null);
                        setDraft(null);
                      }, "Snippet supprimé.")
                    }
                  >
                    <Trash2 /> Supprimer
                  </Button>
                </>
              )}
            </div>
          </form>
        ) : (
          <EmptyState icon={<Scissors />} title="Aucun snippet sélectionné" />
        )}
      </section>
      {leaving && (
        <Dialog
          title="Enregistrer les modifications ?"
          description="Ce snippet a été modifié et n'est pas encore enregistré."
          onClose={() => setLeaving(null)}
          footer={
            <>
              <Button
                variant="ghost"
                onClick={() => {
                  const next = leaving;
                  setLeaving(null);
                  setDraft(null);
                  next();
                }}
              >
                Abandonner
              </Button>
              <Button onClick={() => setLeaving(null)}>Continuer à modifier</Button>
              <Button
                variant="primary"
                data-autofocus
                onClick={async () => {
                  const next = leaving;
                  setLeaving(null);
                  if (await save()) next();
                }}
              >
                Enregistrer
              </Button>
            </>
          }
        />
      )}
    </div>
  );
});
