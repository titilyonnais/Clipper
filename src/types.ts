export type ClipKind = "text" | "code" | "url" | "file" | "image";

export interface ClipItem {
  id: number;
  kind: ClipKind;
  preview: string;
  language: string | null;
  tags: string[];
  pinned: boolean;
  collection_id: number | null;
  sensitive: boolean;
  has_rich: boolean;
  source_app: string | null;
  size_bytes: number;
  created_at: string;
  used_at: string;
  use_count: number;
  image_path: string | null;
  /** Small square version for the lists, once made. */
  thumb_path: string | null;
  /** Only present when fetched with `api.get`. */
  content?: string;
  ocr_text?: string | null;
}

export interface Stats {
  total: number;
  text: number;
  code: number;
  url: number;
  file: number;
  image: number;
  pinned: number;
  sensitive: number;
  disk_bytes: number;
}

export interface Collection {
  id: number;
  name: string;
  count: number;
}

export interface Snippet {
  id: number;
  title: string;
  abbreviation: string | null;
  content: string;
  use_count: number;
  updated_at: string;
}

export interface SourceApp {
  name: string;
  count: number;
}

export type AiProvider = "ollama" | "openai" | "anthropic";
export type Theme = "auto" | "dark" | "light";
export type Density = "comfortable" | "compact";

export interface Settings {
  theme: Theme;
  density: Density;
  shortcut_mode: "win_v" | "custom";
  shortcut: string;
  paste_directly: boolean;
  always_plain_text: boolean;
  launch_at_startup: boolean;
  monitor_paused: boolean;
  ignore_apps: string[];
  detect_secrets: boolean;
  ocr_enabled: boolean;
  keep_rich_text: boolean;
  max_items: number;
  auto_delete_days: number;
  backups_enabled: boolean;
  check_updates: boolean;
  onboarded: boolean;
  ai_provider: AiProvider;
  ollama_url: string;
  ollama_model: string;
  openai_base_url: string;
  openai_model: string;
  anthropic_model: string;
  /** Main-window shortcuts changed by the user: action id -> "Ctrl+P". */
  shortcuts: Partial<Record<string, string>>;
}

export interface SettingsView extends Settings {
  openai_key_set: boolean;
  anthropic_key_set: boolean;
  data_dir: string;
  win_v_active: boolean;
  /** RFC 3339 end of a timed pause, "forever", or null when capturing. */
  paused_until: string | null;
}

export interface AiResponse {
  ok: boolean;
  text: string;
  error: string | null;
}

export type AiAction = "summarize" | "explain" | "rephrase" | "fix" | "translate";
export type CopyFormat = "trim" | "one_line" | "lowercase" | "uppercase" | "json_escape" | "url_encode" | "base64";
/** A period (`today`, `week`, `month`, `3m`, `6m`, `1y`), a `YYYY-MM-DD` day or a `from..to` span. */
export type TimeRange = string | null;

export interface FileInfo {
  path: string;
  exists: boolean;
  is_dir: boolean;
  size: number;
  modified: string | null;
  is_image: boolean;
}

export interface ListParams {
  query?: string;
  kinds?: ClipKind[];
  collection_id?: number | null;
  source_app?: string | null;
  tags?: string[];
  pinned_only?: boolean;
  sensitive_only?: boolean;
  time_range?: TimeRange;
  limit?: number;
  offset?: number;
}

export interface ImportResult {
  imported: number;
  skipped: number;
  total: number;
}

export interface BackupInfo {
  name: string;
  size: number;
  created_at: string;
}

export interface QueueStatus {
  active: boolean;
  position: number;
  total: number;
}

export type PasteOutcome = "pasted" | "copied";

export interface UpdateInfo {
  version: string;
  current: string;
  notes: string | null;
  date: string | null;
}

/** An application of this computer, for the ignore list. */
export interface InstalledApp {
  /** Executable file name, lowercase: what the ignore list holds. */
  exe: string;
  name: string;
  path: string | null;
  /** Why it is worth ignoring (password manager…), when it is. */
  category: string | null;
  score: number;
  /** Clips in the history that came from it. */
  copies: number;
  last_used: string | null;
  running: boolean;
}
