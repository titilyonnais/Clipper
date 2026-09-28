import { useMemo } from "react";
import { useSettings } from "./settings";

/**
 * Keyboard shortcuts of the main window. Every action has a default the user
 * can change in Settings › Raccourcis; only changes are stored. Navigation
 * (arrows, Entrée, Échap, Début/Fin) is fixed.
 */
export const ACTIONS = [
  { id: "copy", label: "Copier l'élément", group: "Élément", default: "Ctrl+C" },
  { id: "copy_plain", label: "Copier en texte brut", group: "Élément", default: "Ctrl+Shift+C" },
  { id: "edit", label: "Modifier", group: "Élément", default: "Ctrl+E" },
  { id: "pin", label: "Épingler / désépingler", group: "Élément", default: "Ctrl+P" },
  { id: "delete", label: "Supprimer", group: "Élément", default: "Delete" },
  { id: "undo", label: "Annuler la suppression", group: "Élément", default: "Ctrl+Z" },
  { id: "snippet", label: "Enregistrer comme snippet", group: "Élément", default: "Ctrl+Shift+S" },
  { id: "select_all", label: "Tout sélectionner", group: "Élément", default: "Ctrl+A" },
  { id: "search", label: "Rechercher", group: "Fenêtre", default: "Ctrl+F" },
  { id: "new_collection", label: "Nouvelle collection", group: "Fenêtre", default: "Ctrl+N" },
  { id: "toggle_sidebar", label: "Afficher / masquer la barre latérale", group: "Fenêtre", default: "Ctrl+B" },
  { id: "settings", label: "Paramètres", group: "Fenêtre", default: "Ctrl+," },
] as const;

export type ActionId = (typeof ACTIONS)[number]["id"];
export type Keymap = Record<ActionId, string>;

/** Combinations that stay with text fields (the search box, the editor). */
const TEXT_EDITING = new Set(["Ctrl+C", "Ctrl+X", "Ctrl+V", "Ctrl+A", "Ctrl+Z", "Ctrl+Y", "Delete", "Backspace"]);
/** Keys reserved for navigation, never assignable. */
export const RESERVED = new Set(["Enter", "Shift+Enter", "Escape", "ArrowUp", "ArrowDown", "Home", "End", "PageUp", "PageDown", "Tab"]);

const NAMED: Record<string, string> = { " ": "Space", Del: "Delete", Esc: "Escape" };
const MODIFIER_KEYS = new Set(["Control", "Shift", "Alt", "Meta", "AltGraph", "CapsLock"]);

/** "Ctrl+Shift+P" for a key event, or null for a lone modifier. */
export function comboOf(e: KeyboardEvent | React.KeyboardEvent): string | null {
  if (MODIFIER_KEYS.has(e.key)) return null;
  let key: string;
  if (/^Key[A-Z]$/.test(e.code) && e.key.length === 1 && /[a-z]/i.test(e.key)) key = e.key.toUpperCase();
  else if (/^Digit\d$/.test(e.code)) key = e.code.slice(5);
  else key = NAMED[e.key] ?? (e.key.length === 1 ? e.key.toUpperCase() : e.key);
  const mods = [e.ctrlKey && "Ctrl", e.altKey && "Alt", e.shiftKey && "Shift"].filter(Boolean);
  return [...mods, key].join("+");
}

const LABELS: Record<string, string> = {
  Shift: "Maj",
  Delete: "Suppr",
  Backspace: "Retour arrière",
  Enter: "Entrée",
  Escape: "Échap",
  Space: "Espace",
  Insert: "Inser",
  ArrowUp: "↑",
  ArrowDown: "↓",
  ArrowLeft: "←",
  ArrowRight: "→",
  PageUp: "Page préc.",
  PageDown: "Page suiv.",
  Home: "Début",
  End: "Fin",
};

/** Keys of a combination as shown to the user: ["Ctrl", "Maj", "P"]. */
export function comboKeys(combo: string): string[] {
  if (!combo) return [];
  // "Ctrl++" : the last key can itself be "+".
  const parts = combo.endsWith("++") ? [...combo.slice(0, -2).split("+"), "+"] : combo.split("+");
  return parts.map((p) => LABELS[p] ?? p);
}

export function comboLabel(combo: string) {
  return comboKeys(combo).join("+");
}

export function useKeymap(): Keymap {
  const { settings } = useSettings();
  const custom = settings?.shortcuts;
  return useMemo(
    () => Object.fromEntries(ACTIONS.map((a) => [a.id, custom?.[a.id] ?? a.default])) as Keymap,
    [custom],
  );
}

/** Action bound to this event, if any. */
export function actionFor(keymap: Keymap, e: KeyboardEvent): ActionId | null {
  const combo = comboOf(e);
  if (!combo) return null;
  const inText = isTextField(e.target);
  // Inside a text field the usual editing keys keep their meaning.
  if (inText && (TEXT_EDITING.has(combo) || !/^(Ctrl|Alt)\+/.test(combo))) return null;
  // Ctrl+C on selected text copies the text, not the item.
  if (combo === keymap.copy && window.getSelection()?.toString()) return null;
  return (Object.keys(keymap) as ActionId[]).find((id) => keymap[id] === combo) ?? null;
}

export function isTextField(target: EventTarget | null) {
  const el = target as HTMLElement | null;
  return !!el && (el.tagName === "INPUT" || el.tagName === "TEXTAREA" || el.isContentEditable);
}

/** A menu, a dialog or the onboarding has the keyboard. */
export function overlayOpen() {
  return !!document.querySelector('[role="menu"], [role="dialog"]');
}
