import { useEffect, useState } from "react";
import { ChevronDown, Copy, EyeOff, GripVertical, MoreHorizontal, Pencil, Pin, Tag } from "lucide-react";
import { api } from "@/lib/api";
import { useTauriEvent } from "@/lib/hooks";
import { useSettings } from "@/lib/settings";
import { appLabel, cn, fullDate, humanBytes, plural, timeAgo } from "@/lib/utils";
import { lineCount, wordCount } from "@/lib/smart";
import { COPY_FORMATS, clipMenu, isText, run } from "@/clip/actions";
import { AiButton, AiResult, useAi } from "@/clip/AiPanel";
import { ClipContent } from "@/clip/ClipContent";
import { ClipEditor } from "@/clip/ClipEditor";
import { KIND_LABEL } from "@/clip/kind";
import { Button, IconButton } from "@/ui/button";
import { Input } from "@/ui/form";
import { Menu, useMenu } from "@/ui/menu";
import { AppIcon, Badge, EmptyState, Logo } from "@/ui/misc";
import { toast } from "@/ui/toast";
import type { ClipItem, Collection } from "@/types";

interface Props {
  clip: ClipItem | null;
  collections: Collection[];
  editing: boolean;
  setEditing: (v: boolean) => void;
  onCopy: (plain: boolean) => void;
  onNewCollection: () => void;
  aiOnline: boolean;
}

export function PreviewPanel({ clip, collections, editing, setEditing, onCopy, onNewCollection, aiOnline }: Props) {
  const { settings, update } = useSettings();
  const [full, setFull] = useState<ClipItem | null>(null);
  const ai = useAi(clip);
  const formats = useMenu();
  const more = useMenu();

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
  useTauriEvent<number>("clip:ocr", (e) => {
    if (e.payload === id) api.get(e.payload).then(setFull);
  });

  if (!clip) {
    return (
      <EmptyState icon={<Logo />} title="Aucun élément sélectionné">
        Copiez quelque chose, ou ouvrez le collage rapide avec {settings?.shortcut_mode === "win_v" ? "Win+V" : settings?.shortcut || "votre raccourci"}.
      </EmptyState>
    );
  }
  const shown = full?.id === clip.id ? full : { ...clip, content: undefined };
  const text = isText(clip);
  const collection = collections.find((c) => c.id === clip.collection_id);
  const content = shown.content ?? "";
  const ignored = clip.source_app && settings?.ignore_apps.includes(clip.source_app);

  return (
    <section className="flex min-w-0 flex-1 flex-col" aria-label="Aperçu">
      <header className="border-b border-line px-6 pt-5 pb-4">
        <div className="flex flex-wrap items-center gap-x-2 gap-y-1 text-xs text-subtle-foreground">
          <Badge>{clip.language ? clip.language.toUpperCase() : KIND_LABEL[clip.kind]}</Badge>
          {clip.sensitive && (
            <Badge tone="warn">
              <EyeOff /> Sensible
            </Badge>
          )}
          {clip.has_rich && <Badge>Mise en forme</Badge>}
          {collection && <Badge>{collection.name}</Badge>}
          <span title={fullDate(clip.created_at)}>{timeAgo(clip.used_at)}</span>
          {clip.use_count > 1 && <span>· utilisé {clip.use_count} fois</span>}
          <span>· {humanBytes(clip.size_bytes)}</span>
          {text && content && !clip.sensitive && (
            <span>
              · {plural(wordCount(content), "mot")} · {plural(lineCount(content), "ligne")}
            </span>
          )}
        </div>

        <div className="mt-4 flex flex-wrap items-center gap-1.5">
          <Button variant="primary" size="sm" onClick={() => onCopy(false)}>
            <Copy /> Copier
          </Button>
          {text && (
            <Button size="sm" variant="ghost" onClick={(e) => formats.openBelow(e.currentTarget)}>
              Copier en… <ChevronDown />
            </Button>
          )}
          {text && !clip.sensitive && (
            <Button size="sm" variant="ghost" onClick={() => setEditing(!editing)} aria-pressed={editing}>
              <Pencil /> Modifier
            </Button>
          )}
          {text && (
            <AiButton
              ai={ai}
              disabled={!aiOnline}
              reason="Configurez l'IA dans les paramètres."
            />
          )}
          <div className="ml-auto flex items-center gap-1">
            {text && !clip.sensitive && content && (
              <span
                draggable
                onDragStart={(e) => {
                  e.dataTransfer.setData("text/plain", content);
                  e.dataTransfer.setData("application/x-clipper-clip", String(clip.id));
                }}
                title="Glisser le texte vers une autre application"
                className="flex size-8 cursor-grab items-center justify-center rounded-ctl text-subtle-foreground hover:bg-muted hover:text-foreground"
              >
                <GripVertical className="size-4" />
              </span>
            )}
            <IconButton
              label={clip.pinned ? "Désépingler" : "Épingler"}
              active={clip.pinned}
              onClick={() => run(() => api.setPinned([clip.id], !clip.pinned))}
            >
              <Pin className={cn(clip.pinned && "fill-current")} />
            </IconButton>
            <IconButton label="Plus d'actions" onClick={(e) => more.openBelow(e.currentTarget, "end")}>
              <MoreHorizontal />
            </IconButton>
          </div>
        </div>
      </header>

      <div className="flex min-h-0 flex-1 flex-col gap-4 overflow-auto px-6 py-5">
        {editing && full ? (
          <ClipEditor clip={full} inPopup={false} onDone={() => setEditing(false)} />
        ) : (
          <ClipContent clip={shown} />
        )}
        <AiResult ai={ai} inPopup={false} />
      </div>

      <footer className="flex flex-wrap items-center gap-x-5 gap-y-2 border-t border-line px-6 py-3 text-13">
        <TagsEditor key={clip.id} clip={clip} />
        {clip.source_app && (
          <span className="ml-auto flex items-center gap-2 text-subtle-foreground">
            <AppIcon dataDir={settings?.data_dir} app={clip.source_app} className="size-4" />
            {appLabel(clip.source_app)}
            {!ignored && (
              <Button
                size="xs"
                variant="ghost"
                title={`Ne plus enregistrer ce qui est copié depuis ${appLabel(clip.source_app)}`}
                onClick={async () => {
                  const error = await update({ ignore_apps: [...(settings?.ignore_apps ?? []), clip.source_app!] });
                  toast(error ?? `${appLabel(clip.source_app!)} est désormais ignoré.`, !!error);
                }}
              >
                Ignorer
              </Button>
            )}
          </span>
        )}
      </footer>

      {formats.anchor && (
        <Menu
          anchor={formats.anchor}
          onClose={formats.close}
          entries={[
            { label: "Texte brut", onSelect: () => onCopy(true) },
            { separator: true },
            ...COPY_FORMATS.map(([f, label]) => ({
              label,
              onSelect: () => run(() => api.copyTransformed(clip.id, f), "Copié."),
            })),
          ]}
        />
      )}
      {more.anchor && (
        <Menu
          anchor={more.anchor}
          onClose={more.close}
          entries={clipMenu({
            clips: [clip],
            collections,
            inPopup: false,
            onPaste: onCopy,
            onEdit: () => setEditing(true),
            onNewCollection,
          }).filter((e) => !("label" in e) || (e.label !== "Copier" && e.label !== "Copier en texte brut"))}
        />
      )}
    </section>
  );
}

function TagsEditor({ clip }: { clip: ClipItem }) {
  const [editing, setEditing] = useState(false);
  const [value, setValue] = useState(clip.tags.join(", "));
  const save = () => {
    setEditing(false);
    const tags = value.split(",").map((t) => t.trim()).filter(Boolean);
    if (tags.join() !== clip.tags.join()) run(() => api.updateTags(clip.id, tags));
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
          if (e.key === "Escape") setEditing(false);
        }}
        placeholder="tags séparés par des virgules"
        className="h-7 w-72 text-13"
        aria-label="Tags"
      />
    );
  }
  return (
    <button
      type="button"
      onClick={() => {
        setValue(clip.tags.join(", "));
        setEditing(true);
      }}
      className="flex items-center gap-1.5 rounded-ctl-sm text-subtle-foreground hover:text-foreground"
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
