import { invoke } from "@tauri-apps/api/core";
import type {
  AiAction,
  AiProvider,
  AiResponse,
  BackupInfo,
  ClipItem,
  Collection,
  CopyFormat,
  FileInfo,
  ImportResult,
  ListParams,
  PasteOutcome,
  QueueStatus,
  Settings,
  SettingsView,
  Snippet,
  SourceApp,
  Stats,
  UpdateInfo,
} from "@/types";

export const api = {
  list: (params: ListParams) => invoke<ClipItem[]>("list_clips", { params }),
  get: (id: number) => invoke<ClipItem | null>("get_clip", { id }),
  stats: () => invoke<Stats>("get_stats"),
  sourceApps: () => invoke<SourceApp[]>("source_apps"),

  paste: (id: number, plain = false, shiftHeld = false) =>
    invoke<PasteOutcome>("paste_clip", { id, plain, shiftHeld }),
  pasteText: (text: string, shiftHeld = false) => invoke<PasteOutcome>("paste_text", { text, shiftHeld }),
  copyTransformed: (id: number, format: CopyFormat) => invoke<void>("copy_transformed", { id, format }),
  updateText: (id: number, text: string) => invoke<number>("update_clip_text", { id, text }),
  setPinned: (ids: number[], pinned: boolean) => invoke<void>("set_pinned", { ids, pinned }),
  setSensitive: (id: number, sensitive: boolean) => invoke<void>("set_sensitive", { id, sensitive }),
  remove: (ids: number[]) => invoke<number>("delete_clips", { ids }),
  undoDelete: () => invoke<number>("undo_delete"),
  clearHistory: () => invoke<number>("clear_history"),

  startQueue: (ids: number[]) => invoke<void>("start_queue", { ids }),
  stopQueue: () => invoke<void>("stop_queue"),
  queueStatus: () => invoke<QueueStatus>("queue_status"),

  collections: () => invoke<Collection[]>("list_collections"),
  createCollection: (name: string) => invoke<number>("create_collection", { name }),
  renameCollection: (id: number, name: string) => invoke<void>("rename_collection", { id, name }),
  deleteCollection: (id: number) => invoke<void>("delete_collection", { id }),
  setCollection: (ids: number[], collectionId: number | null) =>
    invoke<void>("set_collection", { ids, collectionId }),

  updateTags: (id: number, tags: string[]) => invoke<void>("update_tags", { id, tags }),

  snippets: (query?: string) => invoke<Snippet[]>("list_snippets", { query: query || null }),
  saveSnippet: (snippet: Snippet) => invoke<number>("save_snippet", { snippet }),
  deleteSnippet: (id: number) => invoke<void>("delete_snippet", { id }),
  clipToSnippet: (id: number) => invoke<number>("clip_to_snippet", { id }),
  pasteSnippet: (id: number, shiftHeld = false) => invoke<PasteOutcome>("paste_snippet", { id, shiftHeld }),

  fileInfos: (id: number) => invoke<FileInfo[]>("file_infos", { id }),
  filePreview: (id: number, index: number) => invoke<ArrayBuffer>("file_preview", { id, index }),
  openFile: (id: number, index: number) => invoke<void>("open_file", { id, index }),
  revealFile: (id: number, index: number) => invoke<void>("reveal_file", { id, index }),
  openUrl: (id: number) => invoke<void>("open_url", { id }),
  exportHistory: () => invoke<string | null>("export_history"),
  importHistory: () => invoke<ImportResult | null>("import_history"),

  getSettings: () => invoke<SettingsView>("get_settings"),
  setSettings: (settings: Settings) => invoke<SettingsView>("set_settings", { settings }),
  setApiKey: (provider: Exclude<AiProvider, "ollama">, key: string) =>
    invoke<boolean>("set_api_key", { provider, key }),
  enableWinV: (restartExplorer: boolean) => invoke<boolean>("enable_win_v", { restartExplorer }),
  disableWinV: (restartExplorer: boolean) => invoke<void>("disable_win_v", { restartExplorer }),
  /** null resumes, 0 pauses until resumed, n pauses for n minutes. */
  setIncognito: (minutes: number | null) => invoke<void>("set_incognito", { minutes }),
  setFrameTheme: (light: boolean) => invoke<void>("set_frame_theme", { light }),
  completeOnboarding: () => invoke<SettingsView>("complete_onboarding"),

  hidePopup: () => invoke<void>("hide_popup"),
  showMain: () => invoke<void>("show_main"),
  hideMain: () => invoke<void>("hide_main"),

  backups: () => invoke<BackupInfo[]>("list_backups"),
  backupNow: () => invoke<BackupInfo[]>("backup_now"),
  restoreBackup: (name: string) => invoke<void>("restore_backup", { name }),
  openDataFolder: () => invoke<void>("open_data_folder"),

  checkUpdate: () => invoke<UpdateInfo | null>("check_update"),
  /** Downloads, then closes Clipper for the installer, which restarts it. */
  installUpdate: () => invoke<void>("install_update"),
  /** The version Clipper was updated from, once, after an update. */
  takeUpdateNotice: () => invoke<string | null>("take_update_notice"),

  aiHealth: () => invoke<AiResponse>("ai_health"),
  aiRun: (id: number, action: AiAction, lang?: string) =>
    invoke<AiResponse>("ai_run", { id, action, lang: lang ?? null }),
  aiSmartTag: (id: number) => invoke<AiResponse>("ai_smart_tag", { id }),
};

/** Tauri rejects commands with a plain string; normalise anything else. */
export function errorText(e: unknown): string {
  return typeof e === "string" ? e : e instanceof Error ? e.message : String(e);
}
