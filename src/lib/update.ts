import { useSyncExternalStore } from "react";
import { listen } from "@tauri-apps/api/event";
import { api, errorText } from "./api";
import type { UpdateInfo } from "@/types";

export type UpdateStatus = "idle" | "checking" | "latest" | "available" | "downloading" | "installing" | "error";

export interface UpdateState {
  status: UpdateStatus;
  info: UpdateInfo | null;
  downloaded: number;
  total: number | null;
  error: string | null;
  /** The update window is shown. */
  open: boolean;
}

let state: UpdateState = { status: "idle", info: null, downloaded: 0, total: null, error: null, open: false };
const listeners = new Set<() => void>();

function set(patch: Partial<UpdateState>) {
  state = { ...state, ...patch };
  listeners.forEach((l) => l());
}

function subscribe(l: () => void) {
  listeners.add(l);
  return () => listeners.delete(l);
}

export function useUpdate() {
  return useSyncExternalStore(subscribe, () => state);
}

const busy = () => state.status === "downloading" || state.status === "installing";

// Found by the periodic check in the background.
listen<UpdateInfo>("update:available", (e) => {
  if (!busy()) set({ status: "available", info: e.payload, error: null });
});
listen<{ downloaded: number; total: number | null }>("update:progress", (e) => {
  if (state.status === "downloading") set({ downloaded: e.payload.downloaded, total: e.payload.total });
});
listen("update:installing", () => {
  if (state.status === "downloading") set({ status: "installing" });
});

/** Returns whether an update is ready to install. */
export async function checkForUpdate(): Promise<boolean> {
  if (busy() || state.status === "checking") return false;
  set({ status: "checking", error: null });
  try {
    const info = await api.checkUpdate();
    set(info ? { status: "available", info } : { status: "latest", info: null, open: false });
    return !!info;
  } catch (e) {
    set({ status: "error", error: errorText(e) });
    return false;
  }
}

export function openUpdate() {
  set({ open: true });
}

export function closeUpdate() {
  if (!busy()) set({ open: false });
}

/** Download and install; Clipper closes for the installer, which starts it again. */
export async function installUpdate() {
  if (busy()) return;
  set({ status: "downloading", downloaded: 0, total: null, error: null, open: true });
  try {
    await api.installUpdate();
  } catch (e) {
    // The pending update was consumed: a new check is needed to retry.
    set({ status: "error", error: errorText(e) });
  }
}

/** Release notes as blocks: headings, bullet points and paragraphs. */
export function noteBlocks(notes: string): { kind: "h" | "li" | "p"; text: string }[] {
  // The release body ends with download details after a rule.
  const body = notes.split(/\n-{3,}\s*\n/)[0];
  return body
    .split(/\r?\n/)
    .map((line) => line.trim())
    .filter(Boolean)
    .map((line) => {
      const clean = (s: string) => s.replace(/\*\*(.+?)\*\*/g, "$1").replace(/`([^`]+)`/g, "$1");
      if (/^#{1,6}\s/.test(line)) return { kind: "h" as const, text: clean(line.replace(/^#+\s*/, "")) };
      if (/^[-*]\s/.test(line)) return { kind: "li" as const, text: clean(line.slice(2)) };
      return { kind: "p" as const, text: clean(line) };
    });
}
