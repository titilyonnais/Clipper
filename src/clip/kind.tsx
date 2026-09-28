import { Code2, FileText, Files, Image, Link2, Lock, Type } from "lucide-react";
import type { ClipItem, ClipKind } from "@/types";

export const KIND_LABEL: Record<ClipKind, string> = {
  text: "Texte",
  code: "Code",
  url: "Lien",
  file: "Fichiers",
  image: "Image",
};

export function KindIcon({ clip, className }: { clip: Pick<ClipItem, "kind" | "sensitive" | "preview">; className?: string }) {
  if (clip.sensitive) return <Lock className={className} />;
  switch (clip.kind) {
    case "code":
      return <Code2 className={className} />;
    case "url":
      return <Link2 className={className} />;
    case "image":
      return <Image className={className} />;
    case "file":
      return clip.preview.includes(",") ? <Files className={className} /> : <FileText className={className} />;
    default:
      return <Type className={className} />;
  }
}

/** One-line label for a clip, secrets masked. */
export function clipTitle(clip: ClipItem): string {
  if (clip.sensitive) return "Contenu sensible masqué";
  if (clip.kind === "image") return `Image ${clip.preview}`;
  return clip.preview;
}
