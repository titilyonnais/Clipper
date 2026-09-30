import { useEffect, useRef, useState } from "react";
import { ChevronDown, Copy, EyeOff, GripVertical, MoreHorizontal, Pencil, Pin, Save, Sparkles, Tag, Trash2, Ban } from "lucide-react";
import { api } from "@/lib/api";
import { useTauriEvent } from "@/lib/hooks";
import { useSettings } from "@/lib/settings";
import { comboLabel, useKeymap } from "@/lib/shortcuts";
import { appLabel, cn, fullDate, humanBytes, plural, timeAgo } from "@/lib/utils";
import { lineCount, wordCount } from "@/lib/smart";
import { COPY_FORMATS, deleteClips, isText, organizeEntries, run, textEntries, togglePin } from "@/clip/actions";
import { AiResult, aiEntries, useAi } from "@/clip/AiPanel";
import { ClipContent } from "@/clip/ClipContent";
import { EditorArea } from "@/clip/ClipEditor";
import { KIND_LABEL } from "@/clip/kind";
import { Button, IconButton } from "@/ui/button";
import { Dialog } from "@/ui/dialog";
import { Input } from "@/ui/form";
import { Menu, useMenu, type MenuEntry } from "@/ui/menu";
import { AppIcon, Badge, EmptyState, Logo, Spinner } from "@/ui/misc";
import { toast } from "@/ui/toast";
import type { ClipItem, Collection } from "@/types";
import { shortcutLabel } from "./ShortcutCapture";

interface Props {
  clip: ClipItem | null;
  collections: Collection[];
  editing: boolean;
  setEditing: (v: boolean) => void;
  /** Unsaved changes in the editor, so the list can ask before leaving. */
  onDirty: (dirty: boolean) => void;
  onCopy: (plain: boolean) => void;
  onNewCollection: () => void;
  aiOnline: boolean;
  /** Height of the list toolbar: the empty state is centred on the same line as the list's. */
  emptyOffset: number;
}

export function PreviewPanel({ clip, collections, editing, setEditing, onDirty, onCopy, onNewCollection, aiOnline, emptyOffset }: Props) {
  const { settings, update } = useSettings();
  const keymap = useKeymap();
  const [full, setFull] = useState<ClipItem | null>(null);
  const [draft, setDraft] = useState("");
  const [confirm, setConfirm] = useState<"discard" | "ignore" | null>(null);
  const ai = useAi(clip);
  const formats = useMenu();
  const more = useMenu();
  const aiMenu = useMenu();
  const moreButton = useRef<HTMLButtonElement>(null);

  // The list carries no content: fetch the full clip when the selection changes.
  const id = clip?.id;
  const version = clip ? `${clip.used_at}|${clip.pinned}|${clip.sensitive}|${clip.collection_id}|${clip.tags.join()}` : "";
  useEffect(() => {
    if (id == null) return setFull(null);
    let alive = true;
    api.get(id).then((c) => alive && setFull(c));
    return () => {
      alive = false;
    };
  }, [id, version]);
  const currentId = useRef(id);
  currentId.current = id;
  useTauriEvent<number>("clip:ocr", (e) => {
    // Applied only if that clip is still the one shown.
    if (e.payload === id) api.get(e.payload).then((c) => currentId.current === e.payload && setFull(c));
  });

  // Entering the editor starts from the current text.
  useEffect(() => {
    if (editing && full) setDraft(full.content ?? "");
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [editing, full?.id]);
  const dirty = editing && !!full && draft !== (full.content ?? "");
  useEffect(() => onDirty(dirty), [dirty, onDirty]);

  if (!clip) {
    return (
      <section className="flex min-w-0 flex-1 flex-col" style={{ paddingTop: emptyOffset }} aria-label="Aperçu">
        <EmptyState icon={<Logo />} title="Aucun élément sélectionné">
          Copiez quelque chose, ou ouvrez le collage rapide avec{" "}
          {settings?.shortcut_mode === "win_v" ? "Win+V" : settings?.shortcut ? shortcutLabel(settings.shortcut) : "votre raccourci"}.
        </EmptyState>
      </section>
    );
  }

  const shown = full?.id === clip.id ? full : { ...clip, content: undefined };
  const text = isText(clip);
  const collection = collections.find((c) => c.id === clip.collection_id);
  const content = shown.content ?? "";
  const app = clip.source_app;
  const ignored = !!app && settings?.ignore_apps.includes(app);

  const save = () =>
    draft.trim() &&
    run(async () => {
      await api.updateText(clip.id, draft);
      setEditing(false);
    }, "Modification enregistrée.");
  const copyDraft = () => draft.trim() && run(() => api.pasteText(draft), "Texte modifié copié.");
  const cancel = () => (dirty ? setConfirm("discard") : setEditing(false));

  const moreEntries: MenuEntry[] = [
    ...organizeEntries([clip], collections, onNewCollection),
    ...(text ? [{ separator: true } as const, ...textEntries(clip, keymap)] : []),
    ...(text
      ? [
          { separator: true } as const,
          {
            label: aiOnline ? "Intelligence artificielle…" : "Intelligence artificielle (à configurer)",
            icon: <Sparkles />,
            disabled: !aiOnline || ai.busy,
            onSelect: () => moreButton.current && aiMenu.openBelow(moreButton.current, "end"),
          },
        ]
      : []),
    ...(app && !ignored
      ? [
          { separator: true } as const,
          { label: `Ne plus enregistrer depuis ${appLabel(app)}…`, icon: <Ban />, onSelect: () => setConfirm("ignore") },
        ]
      : []),
  ];

  return (
    <section className="@container flex min-w-0 flex-1 flex-col" aria-label="Aperçu">
      <header className="border-b border-line px-6 pt-5 pb-4">
        <div className="flex flex-wrap items-center gap-x-3 gap-y-1.5 text-xs text-subtle-foreground">
          <Badge>{clip.language ? clip.language.toUpperCase() : KIND_LABEL[clip.kind]}</Badge>
          {clip.sensitive && (
            <Badge tone="warn">
              <EyeOff /> Sensible
            </Badge>
          )}
          {clip.has_rich && <Badge>Mise en forme</Badge>}
          {collection && <Badge>{collection.name}</Badge>}
          {app && (
            <span className="flex items-center gap-1.5" title={app}>
              <AppIcon dataDir={settings?.data_dir} app={app} className="size-3.5" />
              Copié depuis {appLabel(app)}
            </span>
          )}
          <span title={fullDate(clip.created_at)}>{timeAgo(clip.used_at)}</span>
          {clip.use_count > 1 && <span>utilisé {clip.use_count} fois</span>}
          <span>{humanBytes(clip.size_bytes)}</span>
          {text && content && !clip.sensitive && (
            <span>
              {plural(wordCount(content), "mot")}, {plural(lineCount(content), "ligne")}
            </span>
          )}
        </div>

        {editing ? (
          <div className="mt-4 flex h-8 animate-in items-center gap-1.5">
            <Button variant="primary" size="sm" onClick={save} disabled={!draft.trim() || !dirty} title="Ctrl+S">
              <Save /> Enregistrer
            </Button>
            <Button size="sm" variant="ghost" onClick={copyDraft} disabled={!draft.trim()} title="Ctrl+Entrée">
              <Copy /> <span className="hidden @[30rem]:inline">Copier le texte modifié</span>
              <span className="@[30rem]:hidden">Copier</span>
            </Button>
            <Button size="sm" variant="ghost" onClick={cancel} className="-mr-2.5 ml-auto" title="Échap">
              Annuler
            </Button>
          </div>
        ) : (
          <div className="mt-4 flex h-8 items-center gap-1.5">
            <Button variant="primary" size="sm" onClick={() => onCopy(false)} title={comboLabel(keymap.copy)}>
              <Copy /> Copier
            </Button>
            {text && (
              <Button
                size="sm"
                variant="ghost"
                onClick={(e) => formats.openBelow(e.currentTarget)}
                aria-label="Copier dans un autre format"
                title="Copier dans un autre format"
              >
                <span className="hidden @[34rem]:inline">Copier en…</span>
                <ChevronDown />
              </Button>
            )}
            {text && !clip.sensitive && (
              <Button size="sm" variant="ghost" onClick={() => setEditing(true)} title={`Modifier (${comboLabel(keymap.edit)})`}>
                <Pencil /> <span className="hidden @[26rem]:inline">Modifier</span>
              </Button>
            )}
            {ai.busy && <Spinner className="ml-1" />}
            <div className="ml-auto flex items-center gap-0.5">
              {text && !clip.sensitive && content && (
                <span
                  draggable
                  onDragStart={(e) => {
                    e.dataTransfer.setData("text/plain", content);
                    e.dataTransfer.setData("application/x-clipper-clip", String(clip.id));
                  }}
                  title="Glisser le texte vers une autre application ou une collection"
                  className="hidden size-8 cursor-grab items-center justify-center rounded-ctl text-subtle-foreground transition-colors hover:bg-muted hover:text-foreground @[30rem]:flex"
                >
                  <GripVertical className="size-4" />
                </span>
              )}
              <IconButton
                label={`${clip.pinned ? "Désépingler" : "Épingler"} (${comboLabel(keymap.pin)})`}
                active={clip.pinned}
                onClick={() => togglePin([clip])}
              >
                <Pin className={cn(clip.pinned && "fill-current")} />
              </IconButton>
              <IconButton
                label={`Supprimer (${comboLabel(keymap.delete)})`}
                className="hover:bg-danger/10 hover:text-danger"
                onClick={() => deleteClips([clip.id])}
              >
                <Trash2 />
              </IconButton>
              <IconButton ref={moreButton} label="Plus d'actions" onClick={(e) => more.openBelow(e.currentTarget, "end")}>
                <MoreHorizontal />
              </IconButton>
            </div>
          </div>
        )}
      </header>

      <div className="flex min-h-0 flex-1 flex-col gap-4 overflow-auto px-6 py-5">
        {editing && full ? (
          <div className="flex min-h-0 flex-1 animate-in flex-col">
            <EditorArea value={draft} onChange={setDraft} onSave={save} onUse={copyDraft} onCancel={cancel} />
          </div>
        ) : (
          <div key={`${clip.id}:${shown.content !== undefined}`} className="flex min-h-0 flex-1 animate-rise flex-col">
            <ClipContent clip={shown} />
          </div>
        )}
        <AiResult ai={ai} inPopup={false} />
      </div>

      <footer className="flex min-h-12 items-center border-t border-line px-6 py-2 text-13">
        <TagsEditor key={clip.id} clip={clip} />
      </footer>

      {formats.anchor && (
        <Menu
          anchor={formats.anchor}
          onClose={formats.close}
          entries={[
            { label: "Texte brut", hint: comboLabel(keymap.copy_plain), onSelect: () => onCopy(true) },
            { separator: true },
            ...COPY_FORMATS.map(([f, label]) => ({
              label,
              onSelect: () => run(() => api.copyTransformed(clip.id, f), "Copié."),
            })),
          ]}
        />
      )}
      {more.anchor && <Menu anchor={more.anchor} onClose={more.close} entries={moreEntries} />}
      {aiMenu.anchor && <Menu anchor={aiMenu.anchor} onClose={aiMenu.close} entries={aiEntries(ai)} />}

      {confirm === "discard" && (
        <Dialog
          title="Abandonner les modifications ?"
          description="Le texte modifié n'a pas été enregistré."
          onClose={() => setConfirm(null)}
          footer={
            <>
              <Button onClick={() => setConfirm(null)}>Continuer à modifier</Button>
              <Button
                variant="danger"
                onClick={() => {
                  setConfirm(null);
                  setEditing(false);
                }}
              >
                Abandonner
              </Button>
            </>
          }
        />
      )}
      {confirm === "ignore" && app && (
        <Dialog
          title={`Ne plus enregistrer depuis ${appLabel(app)} ?`}
          description={`Ce que vous copierez dans ${appLabel(app)} ne sera plus ajouté à l'historique. Ce qui y est déjà reste. Réversible dans Paramètres › Capture.`}
          onClose={() => setConfirm(null)}
          footer={
            <>
              <Button onClick={() => setConfirm(null)}>Annuler</Button>
              <Button
                variant="primary"
                onClick={async () => {
                  setConfirm(null);
                  const error = await update({ ignore_apps: [...(settings?.ignore_apps ?? []), app] });
                  toast(error ?? `${appLabel(app)} est désormais ignoré.`, !!error);
                }}
              >
                Ne plus enregistrer
              </Button>
            </>
          }
        />
      )}
    </section>
  );
}

function TagsEditor({ clip }: { clip: ClipItem }) {
  const [editing, setEditing] = useState(false);
  const [value, setValue] = useState(clip.tags.join(", "));
  // Entrée or Échap end the edit once; the blur that follows is ignored.
  const settled = useRef(false);
  const save = () => {
    if (settled.current) return;
    settled.current = true;
    setEditing(false);
    const tags = value.split(",").map((t) => t.trim()).filter(Boolean);
    if (tags.join() !== clip.tags.join()) run(() => api.updateTags(clip.id, tags));
  };
  const cancel = () => {
    settled.current = true;
    setValue(clip.tags.join(", "));
    setEditing(false);
  };
  if (editing) {
    return (
      <Input
        autoFocus
        value={value}
        onChange={(e) => setValue(e.target.value)}
        onBlur={save}
        onKeyDown={(e) => {
          e.stopPropagation();
          if (e.key === "Enter") save();
          if (e.key === "Escape") cancel();
        }}
        placeholder="tags séparés par des virgules"
        className="h-8 w-full max-w-sm text-13"
        aria-label="Tags"
      />
    );
  }
  return (
    <button
      type="button"
      onClick={() => {
        settled.current = false;
        setValue(clip.tags.join(", "));
        setEditing(true);
      }}
      className="flex h-8 items-center gap-1.5 rounded-ctl-sm text-subtle-foreground transition-colors hover:text-foreground"
    >
      <Tag className="size-3.5" />
      {clip.tags.length ? (
        <span className="text-muted-foreground">{clip.tags.map((t) => `#${t}`).join("  ")}</span>
      ) : (
        "Ajouter des tags"
      )}
    </button>
  );
}
