export type ClipKind = "text" | "code" | "url" | "file" | "image";

export interface ClipItem {
  id: number;
  kind: ClipKind;
  preview: string;
  language: string | null;
  category: string | null;
  tags: string[];
  pinned: boolean;
  favorite: boolean;
  source_app: string | null;
  size_bytes: number;
  created_at: string;
  used_at: string;
  use_count: number;
  image_path: string | null;
  /** Only present when fetched with `api.get`. */
  content?: string;
}

export interface Stats {
  total: number;
  text: number;
  code: number;
  url: number;
  file: number;
  image: number;
  pinned: number;
  favorites: number;
  disk_bytes: number;
}

export type AiProvider = "ollama" | "openai" | "anthropic";
export type Theme = "auto" | "dark" | "light";
export type Density = "comfortable" | "compact";

export interface Settings {
  theme: Theme;
  accent_color: string;
  density: Density;
  shortcut: string;
  launch_at_startup: boolean;
  monitor_paused: boolean;
  ignore_apps: string[];
  max_items: number;
  auto_delete_days: number;
  keep_favorites: boolean;
  keep_pinned: boolean;
  ai_provider: AiProvider;
  ollama_url: string;
  ollama_model: string;
  openai_base_url: string;
  openai_model: string;
  anthropic_model: string;
}

export interface SettingsView extends Settings {
  openai_key_set: boolean;
  anthropic_key_set: boolean;
  data_dir: string;
}

export interface AiResponse {
  ok: boolean;
  text: string;
  error: string | null;
}

export type AiAction = "summarize" | "explain" | "rephrase" | "fix" | "translate";
export type CopyFormat = "trim" | "lowercase" | "uppercase" | "json_escape" | "url_encode" | "base64";
export type SortMode = "recent" | "popular" | "oldest";
/** A named range or a local day (YYYY-MM-DD). */
export type TimeRange = "today" | "yesterday" | "week" | "month" | "year" | (string & {}) | null;

export interface FileInfo {
  path: string;
  exists: boolean;
  is_dir: boolean;
  size: number;
  modified: string | null;
  is_image: boolean;
}

export interface AdvancedFilters {
  kinds?: ClipKind[];
  language?: string | null;
  size_min?: number | null;
  size_max?: number | null;
  use_count_min?: number | null;
  created_from?: string | null;
  created_to?: string | null;
  has_category?: boolean | null;
  has_tags?: boolean | null;
  tags?: string[];
}

export interface ListParams extends AdvancedFilters {
  query?: string;
  category?: string | null;
  pinned_only?: boolean;
  favorites_only?: boolean;
  time_range?: TimeRange;
  sort?: SortMode;
  limit?: number;
  offset?: number;
}

export interface ImportResult {
  imported: number;
  skipped: number;
  total: number;
}
