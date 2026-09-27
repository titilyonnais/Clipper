import { useEffect, useMemo, useRef, useState } from "react";
import { convertFileSrc } from "@tauri-apps/api/core";
import {
  AlertCircle,
  Check,
  ChevronDown,
  Copy,
  ExternalLink,
  Eye,
  EyeOff,
  File as FileIcon,
  Folder,
  FolderOpen,
  Image as ImageIcon,
  Languages,
  Pin,
  Sparkles,
  Star,
  Tag,
  Trash2,
} from "lucide-react";
import { api, errorText } from "@/lib/api";
import { highlight } from "@/lib/highlight";
import { cn, humanBytes, timeAgo } from "@/lib/utils";
import type { AiAction, AiResponse, ClipItem, CopyFormat, FileInfo } from "@/types";

const DISPLAY_LIMIT = 200_000;
const HIGHLIGHT_LIMIT = 60_000;

const FORMATS: [CopyFormat, string][] = [
  ["trim", "Sans espaces"],
  ["lowercase", "minuscules"],
  ["uppercase", "MAJUSCULES"],
  ["json_escape", "Chaîne JSON"],
  ["url_encode", "Encodé URL"],
  ["base64", "Base64"],
];

const AI_ACTIONS: [AiAction, string][] = [
  ["summarize", "Résumer"],
  ["explain", "Expliquer"],
  ["rephrase", "Reformuler"],
  ["fix", "Corriger"],
];

const LANGUAGES: [string, string][] = [
  ["français", "Français"],
  ["anglais", "English"],
  ["espagnol", "Español"],
  ["allemand", "Deutsch"],
  ["italien", "Italiano"],
  ["portugais", "Português"],
  ["néerlandais", "Nederlands"],
  ["japonais", "日本語"],
  ["chinois", "中文"],
];

interface Props {
  clip: ClipItem | null;
  aiOnline: boolean;
  onIgnoreApp: (app: string) => void;
}

export function Preview({ clip, aiOnline, onIgnoreApp }: Props) {
  const [content, setContent] = useState<string | null>(null);
  const [showAll, setShowAll] = useState(false);
  const [notice, setNotice] = useState<{ text: string; error?: boolean } | null>(null);
  const [ai, setAi] = useState<AiResponse | null>(null);
  const [aiBusy, setAiBusy] = useState(false);

  const id = clip?.id;
  useEffect(() => {
    setContent(null);
    setShowAll(false);
    setAi(null);
    setNotice(null);
    if (id == null) return;
    let alive = true;
    api.get(id).then((c) => alive && setContent(c?.content ?? ""));
    return () => {
      alive = false;
    };
  }, [id]);

  const flash = (text: string, error = false) => {
    setNotice({ text, error });
    window.setTimeout(() => setNotice((n) => (n?.text === text ? null : n)), error ? 5000 : 1500);
  };

  const run = (fn: () => Promise<unknown>, done?: string) =>
    fn()
      .then(() => done && flash(done))
      .catch((e) => flash(errorText(e), true));

  if (!clip) {
    return (
      <div className="flex flex-1 items-center justify-center px-10 text-center">
        <div className="max-w-xs">
          <h2 className="mb-2 font-display text-[18px] font-semibold text-ink-50">Aucune sélection</h2>
          <p className="text-[12.5px] leading-relaxed text-ink-400">
            Choisissez un élément pour le prévisualiser, le copier ou le transformer. <kbd>Entrée</kbd> copie
            l'élément sélectionné, <kbd>Échap</kbd> masque la fenêtre.
          </p>
        </div>
      </div>
    );
  }

  const isText = clip.kind === "text" || clip.kind === "code" || clip.kind === "url";

  const askAi = async (action: AiAction | "tag", lang?: string) => {
    setAiBusy(true);
    setAi(null);
    try {
      setAi(action === "tag" ? await api.aiSmartTag(clip.id) : await api.aiRun(clip.id, action, lang));
    } catch (e) {
      setAi({ ok: false, text: "", error: errorText(e) });
    } finally {
      setAiBusy(false);
    }
  };

  return (
    <div className="flex min-w-0 flex-1 flex-col bg-ink-900/30">
      <header className="border-b border-ink-700/60 px-6 pb-4 pt-5">
        <div className="flex items-start justify-between gap-4">
          <div className="min-w-0 flex-1">
            <div className="mb-1 flex flex-wrap items-center gap-x-2 text-[11px] text-ink-400">
              <span className="font-medium uppercase tracking-wide text-accent">{KIND_LABEL[clip.kind]}</span>
              {clip.language && <span>· {clip.language}</span>}
              <span>· {humanBytes(clip.size_bytes)}</span>
              <span>· {timeAgo(clip.used_at)}</span>
              {clip.use_count > 1 && <span>· utilisé {clip.use_count} fois</span>}
            </div>
            <h2 className="truncate font-display text-[16px] font-semibold text-ink-50">
              {clip.kind === "image" ? `Image ${clip.preview}` : clip.preview.slice(0, 120)}
            </h2>
          </div>
          <div className="flex shrink-0 items-center gap-1">
            <IconButton
              title={clip.pinned ? "Désépingler" : "Épingler (jamais supprimé automatiquement)"}
              active={clip.pinned}
              onClick={() => run(() => api.togglePin(clip.id))}
            >
              <Pin size={14} className={clip.pinned ? "fill-accent" : undefined} />
            </IconButton>
            <IconButton
              title={clip.favorite ? "Retirer des favoris" : "Ajouter aux favoris"}
              active={clip.favorite}
              onClick={() => run(() => api.toggleFavorite(clip.id))}
            >
              <Star size={14} className={clip.favorite ? "fill-amber-400 text-amber-400" : undefined} />
            </IconButton>
            <ConfirmButton key={clip.id} onConfirm={() => run(() => api.remove(clip.id))} />
          </div>
        </div>

        <div className="mt-4 flex flex-wrap items-center gap-1.5">
          <ActionButton primary onClick={() => run(() => api.copy(clip.id), "Copié")}>
            <Copy size={12} /> Copier
          </ActionButton>
          {isText && <FormatMenu onPick={(f) => run(() => api.copy(clip.id, f), "Copié")} />}
          {clip.kind === "url" && (
            <ActionButton onClick={() => run(() => api.openUrl(clip.id))}>
              <ExternalLink size={12} /> Ouvrir
            </ActionButton>
          )}
          {notice && (
            <span className={cn("inline-flex items-center gap-1 text-[11.5px]", notice.error ? "text-red-400" : "text-accent")}>
              {notice.error ? <AlertCircle size={12} /> : <Check size={12} />} {notice.text}
            </span>
          )}
        </div>

        {isText && (
          <div className="mt-2 flex flex-wrap items-center gap-1.5">
            <span className="mr-1 text-[10.5px] font-semibold uppercase tracking-wide text-ink-500">IA</span>
            {AI_ACTIONS.map(([action, label]) => (
              <ActionButton key={action} disabled={aiBusy || !aiOnline} onClick={() => askAi(action)}>
                {label}
              </ActionButton>
            ))}
            <TranslateMenu disabled={aiBusy || !aiOnline} onPick={(lang) => askAi("translate", lang)} />
            <ActionButton disabled={aiBusy || !aiOnline} onClick={() => askAi("tag")}>
              <Sparkles size={12} /> Classer
            </ActionButton>
            {!aiOnline && <span className="text-[11px] text-ink-500">Configurez l'IA dans les paramètres.</span>}
          </div>
        )}
      </header>

      <div className="min-h-0 flex-1 overflow-auto px-6 py-5">
        {clip.kind === "image" && clip.image_path ? (
          <img
            src={convertFileSrc(clip.image_path)}
            alt={`Image ${clip.preview}`}
            className="mx-auto max-h-full max-w-full rounded-lg border border-ink-700/60"
          />
        ) : clip.kind === "file" ? (
          <FileList clipId={clip.id} />
        ) : content === null ? null : (
          <TextBody content={content} language={clip.language} showAll={showAll} onShowAll={() => setShowAll(true)} />
        )}

        {aiBusy && <p className="mt-5 animate-pulse text-[12px] text-ink-400">Génération en cours…</p>}
        {ai && <AiResult result={ai} />}
      </div>

      <footer className="flex flex-wrap items-center gap-x-5 gap-y-2 border-t border-ink-700/60 px-6 py-3 text-[12px]">
        <MetaEditor
          key={`cat-${clip.id}`}
          icon={<Folder size={11} />}
          label="Catégorie"
          value={clip.category ?? ""}
          display={clip.category}
          placeholder="ex. Travail"
          onSave={(v) => run(() => api.updateCategory(clip.id, v.trim() || null))}
        />
        <MetaEditor
          key={`tags-${clip.id}`}
          icon={<Tag size={11} />}
          label="Tags"
          value={clip.tags.join(", ")}
          display={clip.tags.length ? clip.tags.map((t) => `#${t}`).join(" ") : null}
          placeholder="ex. api, doc"
          onSave={(v) => run(() => api.updateTags(clip.id, v.split(",")))}
        />
        {clip.source_app && (
          <span className="ml-auto inline-flex items-center gap-2 text-[11px] text-ink-500">
            depuis {clip.source_app}
            <button
              onClick={() => onIgnoreApp(clip.source_app!)}
              title={`Ne plus enregistrer ce qui est copié depuis ${clip.source_app}`}
              className="inline-flex items-center gap-1 rounded px-1.5 py-0.5 hover:bg-ink-800 hover:text-ink-100"
            >
              <EyeOff size={11} /> Ignorer
            </button>
          </span>
        )}
      </footer>
    </div>
  );
}

const KIND_LABEL: Record<ClipItem["kind"], string> = {
  text: "Texte",
  code: "Code",
  url: "Lien",
  file: "Fichiers",
  image: "Image",
};

function TextBody({
  content,
  language,
  showAll,
  onShowAll,
}: {
  content: string;
  language: string | null;
  showAll: boolean;
  onShowAll: () => void;
}) {
  const truncated = !showAll && content.length > DISPLAY_LIMIT;
  const shown = truncated ? content.slice(0, DISPLAY_LIMIT) : content;
  const html = useMemo(
    () => (shown.length <= HIGHLIGHT_LIMIT ? highlight(shown, language) : null),
    [shown, language],
  );
  return (
    <>
      <pre className="selectable rounded-xl border border-ink-700/60 bg-ink-900/60 p-5 leading-relaxed text-ink-100">
        {html !== null ? <code dangerouslySetInnerHTML={{ __html: html }} /> : <code>{shown}</code>}
      </pre>
      {truncated && (
        <button onClick={onShowAll} className="mt-2 text-[12px] text-accent hover:underline">
          Afficher tout ({humanBytes(content.length)})
        </button>
      )}
    </>
  );
}

function AiResult({ result }: { result: AiResponse }) {
  const [copied, setCopied] = useState(false);
  return (
    <div className="mt-5 rounded-xl border border-accent/30 bg-accent/[0.05] p-4">
      <div className="mb-2 flex items-center gap-2 text-[11px] font-semibold uppercase tracking-wide text-accent">
        <Sparkles size={11} /> Réponse de l'IA
        {result.ok && (
          <button
            onClick={() =>
              navigator.clipboard.writeText(result.text).then(() => {
                setCopied(true);
                setTimeout(() => setCopied(false), 1500);
              })
            }
            className="ml-auto inline-flex items-center gap-1 rounded px-1.5 py-0.5 normal-case tracking-normal text-ink-300 hover:bg-ink-800 hover:text-ink-50"
          >
            {copied ? <Check size={11} /> : <Copy size={11} />} Copier
          </button>
        )}
      </div>
      {result.ok ? (
        <p className="selectable whitespace-pre-wrap text-[13px] leading-relaxed text-ink-100">{result.text}</p>
      ) : (
        <p className="text-[12.5px] text-red-400">{result.error}</p>
      )}
    </div>
  );
}

function FileList({ clipId }: { clipId: number }) {
  const [infos, setInfos] = useState<FileInfo[] | null>(null);
  const [previews, setPreviews] = useState<Record<number, string>>({});
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let alive = true;
    const urls: string[] = [];
    setInfos(null);
    setPreviews({});
    setError(null);
    api.fileInfos(clipId).then(async (list) => {
      if (!alive) return;
      setInfos(list);
      // Thumbnails for the first few images only.
      const images = list.map((f, i) => [f, i] as const).filter(([f]) => f.exists && f.is_image).slice(0, 6);
      for (const [, i] of images) {
        try {
          const bytes = await api.filePreview(clipId, i);
          if (!alive) return;
          const url = URL.createObjectURL(new Blob([bytes]));
          urls.push(url);
          setPreviews((p) => ({ ...p, [i]: url }));
        } catch {
          // Too large or unreadable: the row simply shows no thumbnail.
        }
      }
    });
    return () => {
      alive = false;
      urls.forEach(URL.revokeObjectURL);
    };
  }, [clipId]);

  if (!infos) return null;

  const act = (fn: () => Promise<void>) => {
    setError(null);
    fn().catch((e) => setError(errorText(e)));
  };

  return (
    <div className="space-y-2">
      {error && (
        <div className="rounded-md border border-red-500/40 bg-red-500/10 px-3 py-2 text-[12px] text-red-400">{error}</div>
      )}
      {infos.map((f, i) => {
        const name = f.path.split(/[\\/]/).filter(Boolean).pop() ?? f.path;
        const Icon = !f.exists ? AlertCircle : f.is_dir ? FolderOpen : f.is_image ? ImageIcon : FileIcon;
        return (
          <div
            key={i}
            className={cn(
              "selectable flex items-start gap-3 rounded-xl border bg-ink-900/60 px-4 py-3",
              f.exists ? "border-ink-700/60" : "border-red-500/40",
            )}
          >
            <span
              className={cn(
                "flex h-9 w-9 shrink-0 items-center justify-center rounded-md",
                f.exists ? "bg-ink-800 text-ink-300" : "bg-red-500/15 text-red-400",
              )}
            >
              <Icon size={15} />
            </span>
            <div className="min-w-0 flex-1">
              <div className="flex flex-wrap items-center gap-2">
                <span className="truncate text-[13px] font-medium text-ink-50">{name}</span>
                <span className="text-[10.5px] text-ink-500">
                  {!f.exists ? "introuvable" : f.is_dir ? "dossier" : humanBytes(f.size)}
                  {f.exists && f.modified && ` · modifié ${timeAgo(f.modified)}`}
                </span>
              </div>
              <div className="truncate font-mono text-[11px] text-ink-400">{f.path}</div>
              {f.exists && (
                <div className="mt-2 flex gap-1.5">
                  {!f.is_dir && (
                    <ActionButton onClick={() => act(() => api.openFile(clipId, i))}>
                      <ExternalLink size={11} /> Ouvrir
                    </ActionButton>
                  )}
                  <ActionButton onClick={() => act(() => api.revealFile(clipId, i))}>
                    <FolderOpen size={11} /> Localiser
                  </ActionButton>
                </div>
              )}
              {previews[i] && (
                <img src={previews[i]} alt={name} className="mt-3 max-h-56 max-w-full rounded-lg border border-ink-700/60" />
              )}
              {f.is_image && f.exists && !previews[i] && (
                <span className="mt-2 inline-flex items-center gap-1 text-[11px] text-ink-500">
                  <Eye size={11} /> Aperçu non disponible
                </span>
              )}
            </div>
          </div>
        );
      })}
    </div>
  );
}

function MetaEditor({
  icon,
  label,
  value,
  display,
  placeholder,
  onSave,
}: {
  icon: React.ReactNode;
  label: string;
  value: string;
  display: string | null;
  placeholder: string;
  onSave: (v: string) => void;
}) {
  const [editing, setEditing] = useState(false);
  const [draft, setDraft] = useState(value);
  const save = () => {
    setEditing(false);
    if (draft !== value) onSave(draft);
  };
  return (
    <div className="flex items-center gap-2">
      <span className="inline-flex items-center gap-1 text-[11px] text-ink-500">
        {icon} {label}
      </span>
      {editing ? (
        <input
          autoFocus
          value={draft}
          placeholder={placeholder}
          onChange={(e) => setDraft(e.target.value)}
          onBlur={save}
          onKeyDown={(e) => {
            if (e.key === "Enter") save();
            if (e.key === "Escape") {
              e.preventDefault();
              setDraft(value);
              setEditing(false);
            }
          }}
          className="h-6 w-44 rounded border border-ink-700 bg-ink-800 px-2 text-[12px] text-ink-50"
        />
      ) : (
        <button
          onClick={() => {
            setDraft(value);
            setEditing(true);
          }}
          className="text-ink-100 hover:text-accent"
        >
          {display || <span className="italic text-ink-500">aucun</span>}
        </button>
      )}
    </div>
  );
}

function ConfirmButton({ onConfirm }: { onConfirm: () => void }) {
  const [armed, setArmed] = useState(false);
  useEffect(() => {
    if (!armed) return;
    const t = setTimeout(() => setArmed(false), 3000);
    return () => clearTimeout(t);
  }, [armed]);
  return (
    <button
      onClick={() => (armed ? onConfirm() : setArmed(true))}
      title="Supprimer"
      aria-label={armed ? "Confirmer la suppression" : "Supprimer"}
      className={cn(
        "flex h-8 items-center justify-center gap-1 rounded-lg px-2 text-[11.5px]",
        armed ? "bg-red-600 font-medium text-white" : "text-ink-300 hover:bg-red-500/15 hover:text-red-400",
      )}
    >
      <Trash2 size={14} />
      {armed && "Supprimer ?"}
    </button>
  );
}

function IconButton({
  children,
  onClick,
  active,
  title,
}: {
  children: React.ReactNode;
  onClick: () => void;
  active?: boolean;
  title: string;
}) {
  return (
    <button
      onClick={onClick}
      title={title}
      aria-label={title}
      aria-pressed={active}
      className={cn(
        "flex h-8 w-8 items-center justify-center rounded-lg",
        active ? "bg-accent/15 text-accent" : "text-ink-300 hover:bg-ink-800 hover:text-ink-50",
      )}
    >
      {children}
    </button>
  );
}

function ActionButton({
  children,
  onClick,
  primary,
  disabled,
}: {
  children: React.ReactNode;
  onClick: () => void;
  primary?: boolean;
  disabled?: boolean;
}) {
  return (
    <button
      onClick={onClick}
      disabled={disabled}
      className={cn(
        "inline-flex h-7 items-center gap-1.5 rounded-md border px-2.5 text-[11.5px] font-medium disabled:cursor-not-allowed disabled:opacity-40",
        primary
          ? "border-accent bg-accent text-[rgb(var(--on-accent))] hover:opacity-90"
          : "border-ink-700 bg-ink-800/70 text-ink-200 hover:bg-ink-700/70 hover:text-ink-50",
      )}
    >
      {children}
    </button>
  );
}

function useDismiss(open: boolean, close: () => void) {
  const ref = useRef<HTMLDivElement>(null);
  useEffect(() => {
    if (!open) return;
    const onDown = (e: MouseEvent) => {
      if (!ref.current?.contains(e.target as Node)) close();
    };
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        e.preventDefault();
        close();
      }
    };
    document.addEventListener("mousedown", onDown);
    window.addEventListener("keydown", onKey, true);
    return () => {
      document.removeEventListener("mousedown", onDown);
      window.removeEventListener("keydown", onKey, true);
    };
  }, [open, close]);
  return ref;
}

function Menu({
  label,
  disabled,
  items,
  onPick,
}: {
  label: React.ReactNode;
  disabled?: boolean;
  items: [string, string][];
  onPick: (value: string) => void;
}) {
  const [open, setOpen] = useState(false);
  const ref = useDismiss(open, () => setOpen(false));
  return (
    <div className="relative" ref={ref}>
      <ActionButton disabled={disabled} onClick={() => setOpen((v) => !v)}>
        {label} <ChevronDown size={10} />
      </ActionButton>
      {open && (
        <div role="menu" className="absolute left-0 top-full z-20 mt-1 min-w-[160px] rounded-md border border-ink-700 bg-ink-900 py-1 shadow-xl">
          {items.map(([value, text]) => (
            <button
              key={value}
              role="menuitem"
              onClick={() => {
                setOpen(false);
                onPick(value);
              }}
              className="w-full px-3 py-1.5 text-left text-[12px] text-ink-200 hover:bg-ink-800 hover:text-ink-50"
            >
              {text}
            </button>
          ))}
        </div>
      )}
    </div>
  );
}

function FormatMenu({ onPick }: { onPick: (f: CopyFormat) => void }) {
  return <Menu label="Copier en…" items={FORMATS} onPick={(v) => onPick(v as CopyFormat)} />;
}

function TranslateMenu({ disabled, onPick }: { disabled: boolean; onPick: (lang: string) => void }) {
  return (
    <Menu
      label={
        <>
          <Languages size={12} /> Traduire
        </>
      }
      disabled={disabled}
      items={LANGUAGES}
      onPick={onPick}
    />
  );
}
