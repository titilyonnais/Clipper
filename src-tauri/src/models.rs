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
    pub category: Option<String>,
    pub tags: Vec<String>,
    pub pinned: bool,
    pub favorite: bool,
    pub source_app: Option<String>,
    pub size_bytes: i64,
    pub created_at: String,
    pub used_at: String,
    pub use_count: i64,
    /// Absolute path of the PNG file for image clips.
    pub image_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
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
    pub favorites: i64,
    /// Space used on disk (database + images).
    pub disk_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub theme: String,
    pub accent_color: String,
    pub density: String,
    pub shortcut: String,
    pub launch_at_startup: bool,
    pub monitor_paused: bool,
    /// Executable names (e.g. "keepass.exe") whose copies are never recorded.
    pub ignore_apps: Vec<String>,
    pub max_items: i64,
    pub auto_delete_days: i64,
    pub keep_favorites: bool,
    pub keep_pinned: bool,
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
            accent_color: "#a3e635".into(),
            density: "comfortable".into(),
            shortcut: "Ctrl+Shift+V".into(),
            launch_at_startup: false,
            monitor_paused: false,
            ignore_apps: vec![
                "keepass.exe".into(),
                "keepassxc.exe".into(),
                "1password.exe".into(),
                "bitwarden.exe".into(),
            ],
            max_items: 5000,
            auto_delete_days: 0,
            keep_favorites: true,
            keep_pinned: true,
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
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct ListParams {
    pub query: Option<String>,
    pub kinds: Option<Vec<String>>,
    pub category: Option<String>,
    pub tags: Option<Vec<String>>,
    pub pinned_only: bool,
    pub favorites_only: bool,
    pub has_category: Option<bool>,
    pub has_tags: Option<bool>,
    pub language: Option<String>,
    pub size_min: Option<i64>,
    pub size_max: Option<i64>,
    pub use_count_min: Option<i64>,
    pub created_from: Option<String>,
    pub created_to: Option<String>,
    pub time_range: Option<String>,
    pub sort: Option<String>,
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
