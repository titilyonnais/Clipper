use serde::{Deserialize, Serialize};

pub const KINDS: &[&str] = &["text", "code", "url", "file", "image"];

/// A clip as sent to the UI. `content` is only filled by `get_clip`; list
/// queries leave it out so that scrolling never ships megabytes over IPC.
#[derive(Debug, Clone, Serialize)]
pub struct ClipItem {
    pub id: i64,
    pub kind: String,
    pub preview: String,
    pub language: Option<String>,
    pub tags: Vec<String>,
    pub pinned: bool,
    pub collection_id: Option<i64>,
    pub sensitive: bool,
    pub has_rich: bool,
    pub source_app: Option<String>,
    pub size_bytes: i64,
    pub created_at: String,
    pub used_at: String,
    pub use_count: i64,
    /// Absolute path of the PNG file for image clips.
    pub image_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ocr_text: Option<String>,
}

#[derive(Debug, Default, Serialize)]
pub struct Stats {
    pub total: i64,
    pub text: i64,
    pub code: i64,
    pub url: i64,
    pub file: i64,
    pub image: i64,
    pub pinned: i64,
    pub sensitive: i64,
    /// Space used on disk (database + images).
    pub disk_bytes: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct Collection {
    pub id: i64,
    pub name: String,
    pub count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Snippet {
    #[serde(default)]
    pub id: i64,
    pub title: String,
    #[serde(default)]
    pub abbreviation: Option<String>,
    pub content: String,
    #[serde(default)]
    pub use_count: i64,
    #[serde(default)]
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct SourceApp {
    pub name: String,
    pub count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub theme: String,
    pub density: String,
    /// "win_v" (replaces Windows' clipboard history key) or "custom".
    pub shortcut_mode: String,
    pub shortcut: String,
    pub paste_directly: bool,
    pub always_plain_text: bool,
    pub launch_at_startup: bool,
    pub monitor_paused: bool,
    /// Executable names (e.g. "keepass.exe") whose copies are never recorded.
    pub ignore_apps: Vec<String>,
    pub detect_secrets: bool,
    pub ocr_enabled: bool,
    pub keep_rich_text: bool,
    pub max_items: i64,
    pub auto_delete_days: i64,
    pub backups_enabled: bool,
    pub onboarded: bool,
    pub ai_provider: String,
    pub ollama_url: String,
    pub ollama_model: String,
    pub openai_base_url: String,
    pub openai_model: String,
    pub anthropic_model: String,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            theme: "auto".into(),
            density: "comfortable".into(),
            shortcut_mode: "custom".into(),
            shortcut: "Ctrl+Shift+V".into(),
            paste_directly: true,
            always_plain_text: false,
            launch_at_startup: false,
            monitor_paused: false,
            ignore_apps: vec![
                "keepass.exe".into(),
                "keepassxc.exe".into(),
                "1password.exe".into(),
                "bitwarden.exe".into(),
            ],
            detect_secrets: true,
            ocr_enabled: true,
            keep_rich_text: true,
            max_items: 5000,
            auto_delete_days: 0,
            backups_enabled: true,
            onboarded: false,
            ai_provider: "ollama".into(),
            ollama_url: "http://localhost:11434".into(),
            ollama_model: "gemma3:4b".into(),
            openai_base_url: "https://api.openai.com".into(),
            openai_model: "gpt-5".into(),
            anthropic_model: "claude-opus-5".into(),
        }
    }
}

/// Settings plus read-only facts the UI needs. API keys themselves never
/// leave the backend.
#[derive(Debug, Serialize)]
pub struct SettingsView {
    #[serde(flatten)]
    pub settings: Settings,
    pub openai_key_set: bool,
    pub anthropic_key_set: bool,
    pub data_dir: String,
    /// Win+V is taken over and working (Explorer no longer owns it).
    pub win_v_active: bool,
    /// Incognito end time (RFC 3339), or "forever", or null when capturing.
    pub paused_until: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct ListParams {
    pub query: Option<String>,
    pub kinds: Option<Vec<String>>,
    pub collection_id: Option<i64>,
    pub source_app: Option<String>,
    pub tags: Option<Vec<String>>,
    pub pinned_only: bool,
    pub time_range: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct AiResponse {
    pub ok: bool,
    pub text: String,
    pub error: Option<String>,
}

impl AiResponse {
    pub fn ok(text: impl Into<String>) -> Self {
        Self {
            ok: true,
            text: text.into(),
            error: None,
        }
    }

    pub fn err(msg: impl Into<String>) -> Self {
        Self {
            ok: false,
            text: String::new(),
            error: Some(msg.into()),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct ImportResult {
    pub imported: usize,
    pub skipped: usize,
    pub total: usize,
}

#[derive(Debug, Serialize)]
pub struct BackupInfo {
    pub name: String,
    pub size: u64,
    pub created_at: String,
}
