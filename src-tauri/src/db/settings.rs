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
        let Some(raw) = self.raw_settings()? else {
            return Ok(Settings::default());
        };
        if let Ok(settings) = serde_json::from_value(raw.clone()) {
            return Ok(settings);
        }
        // A value no longer valid (older or newer version): keep every
        // other setting rather than starting again from the defaults.
        let mut merged = serde_json::to_value(Settings::default())?;
        if let Some(fields) = raw.as_object() {
            for (key, value) in fields {
                let mut candidate = merged.clone();
                candidate[key] = value.clone();
                if serde_json::from_value::<Settings>(candidate.clone()).is_ok() {
                    merged = candidate;
                } else {
                    log::warn!("setting {key} ignored: invalid value");
                }
            }
        }
        Ok(serde_json::from_value(merged).unwrap_or_default())
    }

    /// Read, change and write the settings in one step: two commands
    /// changing different settings at the same time keep both changes.
    pub fn update_settings(&self, change: impl FnOnce(&mut Settings)) -> Result<Settings> {
        let _guard = self.settings_lock.lock();
        let mut settings = self.get_settings()?;
        change(&mut settings);
        self.set_settings(&settings)?;
        Ok(settings)
    }

    /// Held while a command reads the settings, acts, then writes them.
    pub fn settings_guard(&self) -> parking_lot::MutexGuard<'_, ()> {
        self.settings_lock.lock()
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
