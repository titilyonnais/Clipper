import { invoke } from "@tauri-apps/api/core";
import type {
  AiAction,
  AiProvider,
  AiResponse,
  ClipItem,
  CopyFormat,
  FileInfo,
  ImportResult,
  ListParams,
  Settings,
  SettingsView,
  Stats,
} from "@/types";

export const api = {
  list: (params: ListParams) => invoke<ClipItem[]>("list_clips", { params }),
  get: (id: number) => invoke<ClipItem | null>("get_clip", { id }),
  copy: (id: number, format?: CopyFormat) => invoke<void>("copy_clip", { id, format: format ?? null }),

  togglePin: (id: number) => invoke<void>("toggle_pin", { id }),
  toggleFavorite: (id: number) => invoke<void>("toggle_favorite", { id }),
  updateTags: (id: number, tags: string[]) => invoke<void>("update_tags", { id, tags }),
  updateCategory: (id: number, category: string | null) => invoke<void>("update_category", { id, category }),
  remove: (id: number) => invoke<void>("delete_clip", { id }),
  clearHistory: () => invoke<number>("clear_history"),
  cleanupNow: () => invoke<number>("cleanup_now"),

  fileInfos: (id: number) => invoke<FileInfo[]>("file_infos", { id }),
  filePreview: (id: number, index: number) => invoke<ArrayBuffer>("file_preview", { id, index }),
  openFile: (id: number, index: number) => invoke<void>("open_file", { id, index }),
  revealFile: (id: number, index: number) => invoke<void>("reveal_file", { id, index }),
  openUrl: (id: number) => invoke<void>("open_url", { id }),

  exportHistory: () => invoke<string | null>("export_history"),
  importHistory: () => invoke<ImportResult | null>("import_history"),

  stats: () => invoke<Stats>("get_stats"),
  histogram: (days: number) => invoke<[string, number][]>("get_histogram", { days }),
  categories: () => invoke<string[]>("list_categories"),
  tags: () => invoke<string[]>("list_tags"),
  languages: () => invoke<string[]>("list_languages"),
  categoryCounts: () => invoke<[string, number][]>("category_counts"),
  tagCounts: () => invoke<[string, number][]>("tag_counts"),
  renameCategory: (old: string, name: string) => invoke<number>("rename_category", { old, new: name }),
  deleteCategory: (name: string) => invoke<number>("delete_category", { name }),
  renameTag: (old: string, name: string) => invoke<number>("rename_tag", { old, new: name }),
  deleteTag: (name: string) => invoke<number>("delete_tag", { name }),

  getSettings: () => invoke<SettingsView>("get_settings"),
  setSettings: (settings: Settings) => invoke<SettingsView>("set_settings", { settings }),
  setApiKey: (provider: Exclude<AiProvider, "ollama">, key: string) =>
    invoke<boolean>("set_api_key", { provider, key }),
  setPaused: (paused: boolean) => invoke<void>("set_paused", { paused }),
  hideWindow: () => invoke<void>("hide_window"),

  aiHealth: () => invoke<AiResponse>("ai_health"),
  aiRun: (id: number, action: AiAction, lang?: string) =>
    invoke<AiResponse>("ai_run", { id, action, lang: lang ?? null }),
  aiSmartTag: (id: number) => invoke<AiResponse>("ai_smart_tag", { id }),
};

/** Tauri rejects commands with a plain string; normalise anything else. */
export function errorText(e: unknown): string {
  return typeof e === "string" ? e : e instanceof Error ? e.message : String(e);
}
