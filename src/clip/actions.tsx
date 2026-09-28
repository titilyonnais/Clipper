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
import { comboLabel, type ActionId, type Keymap } from "@/lib/shortcuts";
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

/** Delete now, with "Annuler" in the notification for a few seconds. */
export function deleteClips(ids: number[]) {
  return run(async () => {
    const n = await api.remove(ids);
    toast(n > 1 ? `${n} éléments supprimés.` : "Supprimé.", false, {
      label: "Annuler",
      run: () => run(() => api.undoDelete()),
    });
  });
}

export function togglePin(clips: ClipItem[]) {
  const pin = !clips.every((c) => c.pinned);
  return run(() => api.setPinned(clips.map((c) => c.id), pin));
}

/** Hint shown next to a menu entry: the current shortcut of an action. */
const hint = (keymap: Keymap | undefined, id: ActionId) => (keymap?.[id] ? comboLabel(keymap[id]) : undefined);

/** Collections: file, create, remove. */
export function organizeEntries(clips: ClipItem[], collections: Collection[], onNewCollection: () => void): MenuEntry[] {
  const ids = clips.map((c) => c.id);
  const entries: MenuEntry[] = [{ heading: "Ranger dans" }];
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
  return entries;
}

/** Snippet and masking, for a single text clip. */
export function textEntries(clip: ClipItem, keymap?: Keymap): MenuEntry[] {
  if (!isText(clip)) return [];
  const entries: MenuEntry[] = [];
  if (!clip.sensitive) {
    entries.push({
      label: "Enregistrer comme snippet",
      icon: <Scissors />,
      hint: hint(keymap, "snippet"),
      onSelect: () => run(() => api.clipToSnippet(clip.id), "Snippet créé."),
    });
  }
  entries.push(
    clip.sensitive
      ? { label: "Ne plus masquer", icon: <Eye />, onSelect: () => run(() => api.setSensitive(clip.id, false)) }
      : { label: "Masquer (contenu sensible)", icon: <EyeOff />, onSelect: () => run(() => api.setSensitive(clip.id, true)) },
  );
  return entries;
}

interface MenuOptions {
  /** Clips the menu acts on (the right-clicked one, or the multi-selection). */
  clips: ClipItem[];
  collections: Collection[];
  inPopup: boolean;
  onPaste: (plain: boolean) => void;
  onEdit?: () => void;
  onNewCollection: () => void;
  keymap?: Keymap;
}

/** Full context menu of the list (right click). */
export function clipMenu({ clips, collections, inPopup, onPaste, onEdit, onNewCollection, keymap }: MenuOptions): MenuEntry[] {
  const ids = clips.map((c) => c.id);
  const one = clips.length === 1 ? clips[0] : null;
  const allPinned = clips.every((c) => c.pinned);
  const texts = clips.filter(isText);
  const entries: MenuEntry[] = [];

  if (one) {
    entries.push({
      label: inPopup ? "Coller" : "Copier",
      icon: inPopup ? <ClipboardPaste /> : <Copy />,
      hint: inPopup ? "Entrée" : hint(keymap, "copy"),
      onSelect: () => onPaste(false),
    });
    if (isText(one)) {
      entries.push({
        label: inPopup ? "Coller en texte brut" : "Copier en texte brut",
        icon: <Type />,
        hint: inPopup ? "Maj+Entrée" : hint(keymap, "copy_plain"),
        onSelect: () => onPaste(true),
      });
      if (onEdit && !one.sensitive) {
        entries.push({ label: "Modifier", icon: <Pencil />, hint: inPopup ? "Ctrl+E" : hint(keymap, "edit"), onSelect: onEdit });
      }
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
    hint: inPopup ? "Ctrl+P" : hint(keymap, "pin"),
    onSelect: () => togglePin(clips),
  });
  entries.push(...organizeEntries(clips, collections, onNewCollection));
  if (one && isText(one)) entries.push({ separator: true }, ...textEntries(one, keymap));

  entries.push({ separator: true });
  entries.push({
    label: clips.length > 1 ? `Supprimer (${clips.length})` : "Supprimer",
    icon: <Trash2 />,
    hint: inPopup ? "Ctrl+Suppr" : hint(keymap, "delete"),
    danger: true,
    onSelect: () => deleteClips(ids),
  });
  return entries;
}
