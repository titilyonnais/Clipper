import { useEffect, useMemo, useState } from "react";
import { convertFileSrc } from "@tauri-apps/api/core";
import { AlertCircle, Eye, EyeOff, File, Folder, FolderOpen, ExternalLink, ScanText } from "lucide-react";
import { api, errorText } from "@/lib/api";
import { highlight } from "@/lib/highlight";
import { colorOf, prettyJson, urlParts } from "@/lib/smart";
import { cn, humanBytes, timeAgo } from "@/lib/utils";
import { Button } from "@/ui/button";
import { toast } from "@/ui/toast";
import type { ClipItem, FileInfo } from "@/types";

const DISPLAY_LIMIT = 150_000;
const HIGHLIGHT_LIMIT = 40_000;

/** The body of a clip: text, code, link, image or files. */
export function ClipContent({ clip, compact }: { clip: ClipItem; compact?: boolean }) {
  const [revealed, setRevealed] = useState(false);
  useEffect(() => setRevealed(false), [clip.id]);

  if (clip.sensitive && !revealed) {
    return (
      <div className="flex h-full flex-col items-center justify-center gap-3 text-center">
        <EyeOff className="size-5 text-subtle-foreground" />
        <div>
          <p className="text-sm text-foreground">Contenu sensible masqué</p>
          <p className="mt-1 text-13 text-muted-foreground">Mot de passe, clé ou jeton détecté. Il reste collable.</p>
        </div>
        <Button size="sm" variant="outline" onClick={() => setRevealed(true)}>
          <Eye /> Afficher
        </Button>
      </div>
    );
  }
  if (clip.kind === "image" && clip.image_path) return <ImageContent clip={clip} compact={compact} />;
  if (clip.kind === "file") return <FilesContent clipId={clip.id} />;
  if (clip.content === undefined) return null;
  return <TextContent clip={clip} text={clip.content} compact={compact} />;
}

function TextContent({ clip, text, compact }: { clip: ClipItem; text: string; compact?: boolean }) {
  const [showAll, setShowAll] = useState(false);
  const [formatted, setFormatted] = useState(true);
  useEffect(() => setShowAll(false), [clip.id]);

  const color = colorOf(text);
  const url = clip.kind === "url" ? urlParts(text) : null;
  const pretty = clip.language === "json" ? prettyJson(text) : null;
  const source = pretty && formatted ? pretty : text;
  const truncated = !showAll && source.length > DISPLAY_LIMIT;
  const shown = truncated ? source.slice(0, DISPLAY_LIMIT) : source;
  const html = useMemo(
    () => (shown.length <= HIGHLIGHT_LIMIT ? highlight(shown, clip.language) : null),
    [shown, clip.language],
  );

  return (
    <div className="space-y-3">
      {color && (
        <div className="flex items-center gap-3 rounded-card border border-line bg-surface p-3">
          <span className="size-10 rounded-ctl border border-border" style={{ background: color }} />
          <div className="text-13">
            <div className="font-mono text-foreground">{text.trim()}</div>
            <div className="text-muted-foreground">Couleur</div>
          </div>
        </div>
      )}
      {url && (
        <div className="flex items-center gap-3 rounded-card border border-line bg-surface p-3">
          <div className="min-w-0 flex-1 text-13">
            <div className="truncate text-foreground">{url.host}</div>
            {url.rest && <div className="truncate font-mono text-xs text-muted-foreground">{url.rest}</div>}
          </div>
          <Button size="sm" variant="outline" onClick={() => api.openUrl(clip.id).catch((e) => toast(errorText(e), true))}>
            <ExternalLink /> Ouvrir
          </Button>
        </div>
      )}
      {pretty && (
        <div className="flex gap-1 text-xs">
          <button
            onClick={() => setFormatted(true)}
            className={cn("rounded-ctl-sm px-2 py-0.5", formatted ? "bg-secondary text-foreground" : "text-muted-foreground")}
          >
            Formaté
          </button>
          <button
            onClick={() => setFormatted(false)}
            className={cn("rounded-ctl-sm px-2 py-0.5", !formatted ? "bg-secondary text-foreground" : "text-muted-foreground")}
          >
            Original
          </button>
        </div>
      )}
      <pre
        className={cn(
          "selectable rounded-card border border-line bg-surface leading-relaxed text-foreground",
          compact ? "p-3 text-[12.5px]" : "p-4 text-13",
          clip.kind === "text" ? "font-sans" : "font-mono text-[12.5px]",
        )}
      >
        {html !== null ? <code dangerouslySetInnerHTML={{ __html: html }} /> : <code className="font-[inherit]">{shown}</code>}
      </pre>
      {truncated && (
        <Button size="sm" variant="ghost" onClick={() => setShowAll(true)}>
          Afficher tout ({humanBytes(source.length)})
        </Button>
      )}
    </div>
  );
}

function ImageContent({ clip, compact }: { clip: ClipItem; compact?: boolean }) {
  const [broken, setBroken] = useState(false);
  useEffect(() => setBroken(false), [clip.id]);
  return (
    <div className="flex h-full flex-col gap-3">
      <div className="flex min-h-0 flex-1 items-center justify-center rounded-card border border-line bg-surface p-3">
        {broken ? (
          <p className="flex items-center gap-2 text-13 text-muted-foreground">
            <AlertCircle className="size-4" /> Fichier image introuvable.
          </p>
        ) : (
          <img
            src={convertFileSrc(clip.image_path!)}
            alt={`Image ${clip.preview}`}
            onError={() => setBroken(true)}
            className="max-h-full max-w-full rounded-[4px] object-contain"
          />
        )}
      </div>
      {!compact && clip.ocr_text && (
        <div className="rounded-card border border-line bg-surface p-3">
          <div className="mb-1.5 flex items-center gap-1.5 text-xs text-subtle-foreground">
            <ScanText className="size-3.5" /> Texte reconnu
          </div>
          <p className="selectable line-clamp-6 text-13 whitespace-pre-wrap text-muted-foreground">{clip.ocr_text}</p>
        </div>
      )}
    </div>
  );
}

function FilesContent({ clipId }: { clipId: number }) {
  const [infos, setInfos] = useState<FileInfo[] | null>(null);
  const [thumbs, setThumbs] = useState<Record<number, string>>({});

  useEffect(() => {
    let alive = true;
    const urls: string[] = [];
    setInfos(null);
    setThumbs({});
    api.fileInfos(clipId).then(async (list) => {
      if (!alive) return;
      setInfos(list);
      const images = list.map((f, i) => [f, i] as const).filter(([f]) => f.exists && f.is_image).slice(0, 4);
      for (const [, i] of images) {
        try {
          const bytes = await api.filePreview(clipId, i);
          if (!alive) return;
          const url = URL.createObjectURL(new Blob([bytes]));
          urls.push(url);
          setThumbs((t) => ({ ...t, [i]: url }));
        } catch {
          // Too large or unreadable: no thumbnail.
        }
      }
    });
    return () => {
      alive = false;
      urls.forEach(URL.revokeObjectURL);
    };
  }, [clipId]);

  if (!infos) return null;
  const act = (p: Promise<void>) => p.catch((e) => toast(errorText(e), true));

  return (
    <ul className="space-y-1.5">
      {infos.map((f, i) => {
        const name = f.path.split(/[\\/]/).filter(Boolean).pop() ?? f.path;
        const Icon = f.is_dir ? Folder : File;
        return (
          <li key={i} className="rounded-card border border-line bg-surface p-3">
            <div className="flex items-center gap-3">
              <span
                className={cn(
                  "flex size-8 shrink-0 items-center justify-center rounded-[5px] bg-muted",
                  f.exists ? "text-muted-foreground" : "text-danger",
                )}
              >
                {f.exists ? <Icon className="size-4" /> : <AlertCircle className="size-4" />}
              </span>
              <div className="min-w-0 flex-1">
                <div className="selectable truncate text-13 text-foreground">{name}</div>
                <div className="truncate text-xs text-subtle-foreground">
                  {!f.exists
                    ? "N'existe plus à cet emplacement"
                    : f.is_dir
                      ? "Dossier"
                      : `${humanBytes(f.size)}${f.modified ? ` · modifié ${timeAgo(f.modified)}` : ""}`}
                </div>
              </div>
              {f.exists && (
                <div className="flex gap-1">
                  {!f.is_dir && (
                    <Button size="xs" variant="ghost" onClick={() => act(api.openFile(clipId, i))}>
                      Ouvrir
                    </Button>
                  )}
                  <Button size="xs" variant="ghost" title="Afficher dans le dossier" onClick={() => act(api.revealFile(clipId, i))}>
                    <FolderOpen />
                  </Button>
                </div>
              )}
            </div>
            {thumbs[i] && <img src={thumbs[i]} alt={name} className="mt-3 max-h-48 max-w-full rounded-[4px]" />}
          </li>
        );
      })}
    </ul>
  );
}
