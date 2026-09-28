use super::Db;
use crate::models::Settings;
use anyhow::Result;
use rusqlite::{params, OptionalExtension};

impl Db {
    fn raw_settings(&self) -> Result<Option<serde_json::Value>> {
        let raw: Option<String> = self
            .conn
            .lock()
            .query_row("SELECT value FROM settings WHERE key = 'app'", [], |r| {
                r.get(0)
            })
            .optional()?;
        Ok(raw.and_then(|s| serde_json::from_str(&s).ok()))
    }

    pub fn get_settings(&self) -> Result<Settings> {
        Ok(self
            .raw_settings()?
            .and_then(|v| serde_json::from_value(v).ok())
            .unwrap_or_default())
    }

    pub fn set_settings(&self, s: &Settings) -> Result<()> {
        self.conn.lock().execute(
            "INSERT INTO settings(key, value) VALUES('app', ?1)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![serde_json::to_string(s)?],
        )?;
        Ok(())
    }

    /// Version 1 stored API keys in plain text inside the settings JSON.
    /// Returns them once so they can be moved to the OS credential store.
    pub fn take_legacy_api_keys(&self) -> Result<Vec<(&'static str, String)>> {
        let Some(raw) = self.raw_settings()? else {
            return Ok(vec![]);
        };
        let mut found = vec![];
        for (field, provider) in [
            ("openai_api_key", "openai"),
            ("anthropic_api_key", "anthropic"),
        ] {
            if let Some(key) = raw
                .get(field)
                .and_then(|v| v.as_str())
                .filter(|k| !k.trim().is_empty())
            {
                found.push((provider, key.trim().to_string()));
            }
        }
        if raw.get("openai_api_key").is_some() || raw.get("anthropic_api_key").is_some() {
            // Rewriting through the typed struct drops the legacy fields.
            self.set_settings(&self.get_settings()?)?;
        }
        Ok(found)
    }
}
