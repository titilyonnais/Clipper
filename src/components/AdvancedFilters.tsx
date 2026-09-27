import { useEffect, useState } from "react";
import { RotateCcw } from "lucide-react";
import { api } from "@/lib/api";
import type { AdvancedFilters as Filters, ClipKind } from "@/types";
import { Chip, Dialog, Field, buttonClass, inputClass } from "./ui";
import { cn } from "@/lib/utils";

const KINDS: [ClipKind, string][] = [
  ["text", "Texte"],
  ["code", "Code"],
  ["url", "Lien"],
  ["image", "Image"],
  ["file", "Fichier"],
];

interface Props {
  value: Filters;
  onChange: (v: Filters) => void;
  onClose: () => void;
}

export function AdvancedFilters({ value, onChange, onClose }: Props) {
  const [draft, setDraft] = useState<Filters>(value);
  const [tags, setTags] = useState<string[]>([]);
  const [languages, setLanguages] = useState<string[]>([]);

  useEffect(() => {
    api.tags().then(setTags);
    api.languages().then(setLanguages);
  }, []);

  const set = (patch: Partial<Filters>) => setDraft((d) => ({ ...d, ...patch }));
  const toggle = <T,>(list: T[] | undefined, item: T) =>
    list?.includes(item) ? list.filter((x) => x !== item) : [...(list ?? []), item];
  const kb = (n: number | null | undefined) => (n == null ? "" : String(Math.round(n / 1024)));
  const fromKb = (v: string) => (v === "" ? null : Math.max(0, parseInt(v) || 0) * 1024);

  return (
    <Dialog
      title="Filtres avancés"
      onClose={onClose}
      footer={
        <>
          <button
            className={buttonClass}
            onClick={() => {
              setDraft({});
              onChange({});
            }}
          >
            <RotateCcw size={12} /> Réinitialiser
          </button>
          <div className="flex-1" />
          <button className="h-9 px-3 text-[12.5px] text-ink-300 hover:text-ink-50" onClick={onClose}>
            Annuler
          </button>
          <button
            className="h-9 rounded-md bg-accent px-4 text-[12.5px] font-semibold text-[rgb(var(--on-accent))] hover:opacity-90"
            onClick={() => {
              onChange(draft);
              onClose();
            }}
          >
            Appliquer
          </button>
        </>
      }
    >
      <div className="space-y-6 p-6">
        <Field label="Types de contenu">
          <div className="flex flex-wrap gap-1.5">
            {KINDS.map(([k, label]) => (
              <Chip key={k} active={!!draft.kinds?.includes(k)} onClick={() => set({ kinds: toggle(draft.kinds, k) })}>
                {label}
              </Chip>
            ))}
          </div>
        </Field>

        {languages.length > 0 && (
          <Field label="Langage">
            <div className="flex flex-wrap gap-1.5">
              <Chip active={!draft.language} onClick={() => set({ language: null })}>
                Tous
              </Chip>
              {languages.map((l) => (
                <Chip key={l} active={draft.language === l} onClick={() => set({ language: l })}>
                  {l}
                </Chip>
              ))}
            </div>
          </Field>
        )}

        <Field label="Date de capture">
          <div className="flex items-center gap-2">
            <input
              type="date"
              value={draft.created_from ?? ""}
              onChange={(e) => set({ created_from: e.target.value || null })}
              className={cn(inputClass, "w-44")}
            />
            <span className="text-ink-400">→</span>
            <input
              type="date"
              value={draft.created_to ?? ""}
              onChange={(e) => set({ created_to: e.target.value || null })}
              className={cn(inputClass, "w-44")}
            />
          </div>
        </Field>

        <div className="grid grid-cols-2 gap-6">
          <Field label="Taille (Ko)">
            <div className="flex items-center gap-2">
              <input
                type="number"
                min={0}
                placeholder="min"
                value={kb(draft.size_min)}
                onChange={(e) => set({ size_min: fromKb(e.target.value) })}
                className={inputClass}
              />
              <span className="text-ink-400">→</span>
              <input
                type="number"
                min={0}
                placeholder="max"
                value={kb(draft.size_max)}
                onChange={(e) => set({ size_max: fromKb(e.target.value) })}
                className={inputClass}
              />
            </div>
          </Field>
          <Field label="Utilisé au moins">
            <input
              type="number"
              min={0}
              placeholder="nombre de fois"
              value={draft.use_count_min ?? ""}
              onChange={(e) => set({ use_count_min: e.target.value === "" ? null : Math.max(0, parseInt(e.target.value) || 0) })}
              className={inputClass}
            />
          </Field>
        </div>

        <Field label="Catégorie">
          <div className="flex gap-1.5">
            <Chip active={draft.has_category == null} onClick={() => set({ has_category: null })}>
              Indifférent
            </Chip>
            <Chip active={draft.has_category === true} onClick={() => set({ has_category: true })}>
              Avec
            </Chip>
            <Chip active={draft.has_category === false} onClick={() => set({ has_category: false })}>
              Sans
            </Chip>
          </div>
        </Field>

        <Field label="Tags">
          <div className="mb-2 flex gap-1.5">
            <Chip active={draft.has_tags == null} onClick={() => set({ has_tags: null })}>
              Indifférent
            </Chip>
            <Chip active={draft.has_tags === true} onClick={() => set({ has_tags: true })}>
              Avec
            </Chip>
            <Chip active={draft.has_tags === false} onClick={() => set({ has_tags: false, tags: [] })}>
              Sans
            </Chip>
          </div>
          {tags.length > 0 && draft.has_tags !== false && (
            <div className="flex flex-wrap gap-1.5">
              {tags.map((t) => (
                <Chip
                  key={t}
                  active={!!draft.tags?.includes(t)}
                  onClick={() => set({ tags: toggle(draft.tags, t) })}
                  className="h-7 text-[11.5px]"
                >
                  #{t}
                </Chip>
              ))}
            </div>
          )}
        </Field>
      </div>
    </Dialog>
  );
}

export function countActive(f: Filters): number {
  return [
    f.kinds?.length,
    f.language,
    f.created_from || f.created_to,
    f.size_min != null || f.size_max != null,
    f.use_count_min != null,
    f.has_category != null,
    f.has_tags != null,
    f.tags?.length,
  ].filter(Boolean).length;
}
