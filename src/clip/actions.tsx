import {
  ClipboardPaste,
  Copy,
  Eye,
  EyeOff,
  FolderInput,
  FolderMinus,
  ListOrdered,
  Pencil,
  Pin,
  PinOff,
  Plus,
  Scissors,
  Trash2,
  Type,
} from "lucide-react";
import { api, errorText } from "@/lib/api";
import { toast } from "@/ui/toast";
import type { MenuEntry } from "@/ui/menu";
import type { ClipItem, Collection, CopyFormat } from "@/types";

export const COPY_FORMATS: [CopyFormat, string][] = [
  ["trim", "Sans espaces superflus"],
  ["one_line", "Sur une seule ligne"],
  ["lowercase", "en minuscules"],
  ["uppercase", "EN MAJUSCULES"],
  ["json_escape", "Chaîne JSON"],
  ["url_encode", "Encodé pour URL"],
  ["base64", "Base64"],
];

/** Run an action and report failures as a toast. */
export async function run<T>(fn: () => Promise<T>, done?: string): Promise<T | undefined> {
  try {
    const result = await fn();
    if (done) toast(done);
    return result;
  } catch (e) {
    toast(errorText(e), true);
    return undefined;
  }
}

export function isText(clip: ClipItem) {
  return clip.kind !== "image" && clip.kind !== "file";
}

interface MenuOptions {
  /** Clips the menu acts on (the right-clicked one, or the multi-selection). */
  clips: ClipItem[];
  collections: Collection[];
  inPopup: boolean;
  onPaste: (plain: boolean) => void;
  onEdit?: () => void;
  onNewCollection: () => void;
  onDeleted?: () => void;
}

export function clipMenu({
  clips,
  collections,
  inPopup,
  onPaste,
  onEdit,
  onNewCollection,
  onDeleted,
}: MenuOptions): MenuEntry[] {
  const ids = clips.map((c) => c.id);
  const one = clips.length === 1 ? clips[0] : null;
  const allPinned = clips.every((c) => c.pinned);
  const texts = clips.filter(isText);
  const entries: MenuEntry[] = [];

  if (one) {
    entries.push(
      { label: inPopup ? "Coller" : "Copier", icon: inPopup ? <ClipboardPaste /> : <Copy />, hint: "Entrée", onSelect: () => onPaste(false) },
    );
    if (isText(one)) {
      entries.push({ label: inPopup ? "Coller en texte brut" : "Copier en texte brut", icon: <Type />, hint: "Maj+Entrée", onSelect: () => onPaste(true) });
      if (onEdit && !one.sensitive) entries.push({ label: "Modifier…", icon: <Pencil />, hint: "Ctrl+E", onSelect: onEdit });
    }
  } else if (texts.length > 1) {
    entries.push({
      label: `Coller en série (${texts.length})`,
      icon: <ListOrdered />,
      onSelect: () => run(() => api.startQueue(texts.map((c) => c.id))),
    });
  }

  entries.push({ separator: true });
  entries.push({
    label: allPinned ? "Désépingler" : "Épingler",
    icon: allPinned ? <PinOff /> : <Pin />,
    hint: "Ctrl+P",
    onSelect: () => run(() => api.setPinned(ids, !allPinned)),
  });

  entries.push({ heading: "Ranger dans" });
  for (const c of collections) {
    const inside = clips.every((x) => x.collection_id === c.id);
    entries.push({
      label: c.name,
      icon: <FolderInput />,
      checked: inside,
      onSelect: () => run(() => api.setCollection(ids, inside ? null : c.id)),
    });
  }
  entries.push({ label: "Nouvelle collection…", icon: <Plus />, onSelect: onNewCollection });
  if (clips.some((c) => c.collection_id !== null)) {
    entries.push({ label: "Retirer de la collection", icon: <FolderMinus />, onSelect: () => run(() => api.setCollection(ids, null)) });
  }

  if (one && isText(one)) {
    entries.push({ separator: true });
    if (!one.sensitive) {
      entries.push({ label: "Enregistrer comme snippet", icon: <Scissors />, onSelect: () => run(() => api.clipToSnippet(one.id), "Snippet créé.") });
    }
    entries.push(
      one.sensitive
        ? { label: "Ne plus masquer", icon: <Eye />, onSelect: () => run(() => api.setSensitive(one.id, false)) }
        : { label: "Masquer (contenu sensible)", icon: <EyeOff />, onSelect: () => run(() => api.setSensitive(one.id, true)) },
    );
  }

  entries.push({ separator: true });
  entries.push({
    label: clips.length > 1 ? `Supprimer (${clips.length})` : "Supprimer",
    icon: <Trash2 />,
    hint: "Suppr",
    danger: true,
    onSelect: () =>
      run(async () => {
        await api.remove(ids);
        onDeleted?.();
      }),
  });
  return entries;
}
