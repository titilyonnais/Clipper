use crate::models::{ClipItem, ListParams, Settings, Stats};
use anyhow::{anyhow, Result};
use base64::Engine;
use chrono::{SecondsFormat, Utc};
use parking_lot::Mutex;
use rusqlite::{params, params_from_iter, types::Value, Connection, OptionalExtension};
use std::path::{Path, PathBuf};

const SCHEMA_VERSION: i64 = 2;

const CLIP_COLUMNS: &str = "id, kind, preview, language, category, tags, pinned, favorite, \
     source_app, size_bytes, created_at, used_at, use_count, \
     CASE WHEN kind = 'image' THEN content END";

pub struct Db {
    conn: Mutex<Connection>,
    path: PathBuf,
    images_dir: PathBuf,
}

/// Row to insert, built by the clipboard monitor or the importer.
pub struct NewClip<'a> {
    pub kind: &'a str,
    pub content: &'a str,
    pub preview: &'a str,
    pub language: Option<&'a str>,
    pub source_app: Option<&'a str>,
    pub size_bytes: i64,
    pub hash: &'a str,
}

pub fn now() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true)
}

impl Db {
    pub fn open(data_dir: &Path) -> Result<Self> {
        let images_dir = data_dir.join("images");
        std::fs::create_dir_all(&images_dir)?;
        let path = data_dir.join("clipper.db");
        let conn = Connection::open(&path)?;
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "synchronous", "NORMAL")?;
        conn.pragma_update(None, "temp_store", "MEMORY")?;
        // Keep the write-ahead log small once its content is checkpointed.
        conn.pragma_update(None, "journal_size_limit", 4 * 1024 * 1024)?;
        let db = Self {
            conn: Mutex::new(conn),
            path,
            images_dir,
        };
        db.migrate()?;
        Ok(db)
    }

    pub fn data_dir(&self) -> &Path {
        self.path.parent().unwrap_or(Path::new("."))
    }

    fn migrate(&self) -> Result<()> {
        let mut c = self.conn.lock();
        let version: i64 = c.query_row("PRAGMA user_version", [], |r| r.get(0))?;
        if version >= SCHEMA_VERSION {
            return Ok(());
        }
        let has_clips: bool = c
            .query_row(
                "SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = 'clips'",
                [],
                |_| Ok(true),
            )
            .optional()?
            .unwrap_or(false);

        if !has_clips {
            // Fresh database: incremental auto-vacuum must be set before any table exists.
            c.pragma_update(None, "auto_vacuum", "INCREMENTAL")?;
            c.execute_batch(SCHEMA_TABLES)?;
            c.execute_batch(SCHEMA_FTS)?;
            c.pragma_update(None, "user_version", SCHEMA_VERSION)?;
            return Ok(());
        }

        // v1 -> v2: images move out of the database, the FTS index stops
        // indexing previews (and base64 image data), timestamps are normalised.
        let tx = c.transaction()?;
        tx.execute_batch(
            "DROP TRIGGER IF EXISTS clips_ai;
             DROP TRIGGER IF EXISTS clips_ad;
             DROP TRIGGER IF EXISTS clips_au;
             DROP TABLE IF EXISTS clips_fts;",
        )?;
        let images: Vec<(i64, String)> = {
            let mut stmt = tx.prepare("SELECT id, content FROM clips WHERE kind = 'image'")?;
            let rows = stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?;
            rows.collect::<rusqlite::Result<_>>()?
        };
        for (id, b64) in images {
            let stored = base64::engine::general_purpose::STANDARD
                .decode(b64.trim())
                .ok()
                .and_then(|png| self.store_image_file(&png).ok());
            match stored {
                Some(file) => {
                    tx.execute(
                        "UPDATE clips SET content = ?1 WHERE id = ?2",
                        params![file, id],
                    )?;
                }
                None => {
                    tx.execute("DELETE FROM clips WHERE id = ?1", params![id])?;
                }
            }
        }
        // Re-run the (stricter) type detection on text clips: v1 flagged plain
        // prose as code whenever it contained words like "interface".
        let texts: Vec<(i64, String, Option<String>, String)> = {
            let mut stmt = tx.prepare(
                "SELECT id, kind, language, content FROM clips WHERE kind IN ('text', 'code', 'url')",
            )?;
            let rows = stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)))?;
            rows.collect::<rusqlite::Result<_>>()?
        };
        for (id, kind, language, content) in texts {
            let (new_kind, new_language) = crate::clipboard::classify(&content);
            if new_kind != kind || new_language != language.as_deref() {
                tx.execute(
                    "UPDATE clips SET kind = ?1, language = ?2 WHERE id = ?3",
                    params![new_kind, new_language, id],
                )?;
            }
        }
        tx.execute_batch(
            "UPDATE clips SET
                created_at = strftime('%Y-%m-%dT%H:%M:%fZ', created_at),
                used_at = strftime('%Y-%m-%dT%H:%M:%fZ', used_at)
             WHERE created_at NOT LIKE '%Z';
             UPDATE clips SET preview = substr(preview, 1, instr(preview, ' ·') - 1)
             WHERE kind = 'image' AND instr(preview, ' ·') > 0;",
        )?;
        tx.execute_batch(SCHEMA_TABLES)?;
        tx.execute_batch(SCHEMA_FTS)?;
        tx.execute_batch("INSERT INTO clips_fts(clips_fts) VALUES('rebuild');")?;
        tx.pragma_update(None, "user_version", SCHEMA_VERSION)?;
        tx.commit()?;
        // Reclaim the space freed by the images and switch to incremental vacuum.
        c.pragma_update(None, "auto_vacuum", "INCREMENTAL")?;
        c.execute_batch("VACUUM;")?;
        checkpoint(&c);
        Ok(())
    }

    /// Write PNG bytes to the images directory (content-addressed) and
    /// return the file name.
    pub fn store_image_file(&self, png: &[u8]) -> Result<String> {
        use sha2::{Digest, Sha256};
        let name = format!("{:x}.png", Sha256::digest(png));
        let path = self.images_dir.join(&name);
        if !path.exists() {
            std::fs::write(&path, png)?;
        }
        Ok(name)
    }

    pub fn image_path(&self, file: &str) -> PathBuf {
        // `file` always comes from our own database, but never let it escape the folder.
        let name = Path::new(file).file_name().unwrap_or_default();
        self.images_dir.join(name)
    }

    /// Insert a clip, or bump it if an identical one already exists.
    pub fn upsert_clip(&self, clip: &NewClip) -> Result<i64> {
        let now = now();
        let c = self.conn.lock();
        let id = c.query_row(
            "INSERT INTO clips (kind, content, preview, language, source_app, size_bytes, hash, created_at, used_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?8)
             ON CONFLICT(hash) DO UPDATE SET
                used_at = excluded.used_at,
                use_count = use_count + 1,
                source_app = COALESCE(excluded.source_app, source_app)
             RETURNING id",
            params![
                clip.kind,
                clip.content,
                clip.preview,
                clip.language,
                clip.source_app,
                clip.size_bytes,
                clip.hash,
                now
            ],
            |r| r.get(0),
        )?;
        Ok(id)
    }

    pub fn list(&self, p: &ListParams) -> Result<Vec<ClipItem>> {
        let mut sql = format!("SELECT {CLIP_COLUMNS} FROM clips");
        let mut conds: Vec<String> = Vec::new();
        let mut args: Vec<Value> = Vec::new();

        if let Some(q) = p.query.as_deref().and_then(fts_query) {
            conds.push("id IN (SELECT rowid FROM clips_fts WHERE clips_fts MATCH ?)".into());
            args.push(q.into());
        }
        if let Some(kinds) = p.kinds.as_ref().filter(|v| !v.is_empty()) {
            conds.push(format!("kind IN ({})", vec!["?"; kinds.len()].join(",")));
            args.extend(kinds.iter().map(|k| Value::from(k.clone())));
        }
        if let Some(cat) = p.category.as_ref().filter(|s| !s.is_empty()) {
            conds.push("category = ?".into());
            args.push(cat.clone().into());
        }
        for tag in p.tags.iter().flatten() {
            conds.push("EXISTS (SELECT 1 FROM json_each(clips.tags) WHERE value = ?)".into());
            args.push(tag.clone().into());
        }
        if p.pinned_only {
            conds.push("pinned = 1".into());
        }
        if p.favorites_only {
            conds.push("favorite = 1".into());
        }
        match p.has_category {
            Some(true) => conds.push("COALESCE(category, '') <> ''".into()),
            Some(false) => conds.push("COALESCE(category, '') = ''".into()),
            None => {}
        }
        match p.has_tags {
            Some(true) => conds.push("tags <> '[]'".into()),
            Some(false) => conds.push("tags = '[]'".into()),
            None => {}
        }
        if let Some(lang) = p.language.as_ref().filter(|s| !s.is_empty()) {
            conds.push("language = ?".into());
            args.push(lang.clone().into());
        }
        if let Some(min) = p.size_min {
            conds.push("size_bytes >= ?".into());
            args.push(min.into());
        }
        if let Some(max) = p.size_max {
            conds.push("size_bytes <= ?".into());
            args.push(max.into());
        }
        if let Some(min) = p.use_count_min {
            conds.push("use_count >= ?".into());
            args.push(min.into());
        }
        if let Some((from, _)) = p.created_from.as_deref().and_then(local_day_bounds) {
            conds.push("created_at >= ?".into());
            args.push(from.into());
        }
        if let Some((_, to)) = p.created_to.as_deref().and_then(local_day_bounds) {
            conds.push("created_at < ?".into());
            args.push(to.into());
        }
        if let Some((from, to)) = p.time_range.as_deref().and_then(time_range_bounds) {
            conds.push("used_at >= ? AND used_at < ?".into());
            args.push(from.into());
            args.push(to.into());
        }

        if !conds.is_empty() {
            sql.push_str(" WHERE ");
            sql.push_str(&conds.join(" AND "));
        }
        sql.push_str(match p.sort.as_deref() {
            Some("popular") => " ORDER BY pinned DESC, use_count DESC, used_at DESC",
            Some("oldest") => " ORDER BY pinned DESC, used_at ASC",
            _ => " ORDER BY pinned DESC, used_at DESC",
        });
        sql.push_str(" LIMIT ? OFFSET ?");
        args.push(p.limit.unwrap_or(100).clamp(1, 1000).into());
        args.push(p.offset.unwrap_or(0).max(0).into());

        let c = self.conn.lock();
        let mut stmt = c.prepare_cached(&sql)?;
        let rows = stmt.query_map(params_from_iter(args), |r| self.row_to_clip(r))?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    pub fn get(&self, id: i64) -> Result<Option<ClipItem>> {
        let c = self.conn.lock();
        let mut stmt = c.prepare_cached(&format!(
            "SELECT {CLIP_COLUMNS}, content FROM clips WHERE id = ?1"
        ))?;
        let item = stmt
            .query_row(params![id], |r| {
                let mut item = self.row_to_clip(r)?;
                item.content = Some(r.get(14)?);
                Ok(item)
            })
            .optional()?;
        Ok(item)
    }

    /// Raw stored content (image file name for images).
    pub fn content(&self, id: i64) -> Result<(String, String)> {
        let c = self.conn.lock();
        c.query_row(
            "SELECT kind, content FROM clips WHERE id = ?1",
            params![id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()?
        .ok_or_else(|| anyhow!("Élément introuvable."))
    }

    fn row_to_clip(&self, r: &rusqlite::Row) -> rusqlite::Result<ClipItem> {
        let tags: String = r.get(5)?;
        let image_file: Option<String> = r.get(13)?;
        Ok(ClipItem {
            id: r.get(0)?,
            kind: r.get(1)?,
            preview: r.get(2)?,
            language: r.get(3)?,
            category: r.get(4)?,
            tags: serde_json::from_str(&tags).unwrap_or_default(),
            pinned: r.get(6)?,
            favorite: r.get(7)?,
            source_app: r.get(8)?,
            size_bytes: r.get(9)?,
            created_at: r.get(10)?,
            used_at: r.get(11)?,
            use_count: r.get(12)?,
            image_path: image_file.map(|f| self.image_path(&f).to_string_lossy().into_owned()),
            content: None,
        })
    }

    pub fn bump_used(&self, id: i64) -> Result<()> {
        self.conn.lock().execute(
            "UPDATE clips SET used_at = ?1, use_count = use_count + 1 WHERE id = ?2",
            params![now(), id],
        )?;
        Ok(())
    }

    pub fn toggle_pin(&self, id: i64) -> Result<()> {
        self.conn.lock().execute(
            "UPDATE clips SET pinned = 1 - pinned WHERE id = ?1",
            params![id],
        )?;
        Ok(())
    }

    pub fn toggle_favorite(&self, id: i64) -> Result<()> {
        self.conn.lock().execute(
            "UPDATE clips SET favorite = 1 - favorite WHERE id = ?1",
            params![id],
        )?;
        Ok(())
    }

    pub fn update_tags(&self, id: i64, tags: &[String]) -> Result<()> {
        let json = serde_json::to_string(&normalize_tags(tags))?;
        self.conn.lock().execute(
            "UPDATE clips SET tags = ?1 WHERE id = ?2",
            params![json, id],
        )?;
        Ok(())
    }

    pub fn update_category(&self, id: i64, category: Option<&str>) -> Result<()> {
        let category = category.map(str::trim).filter(|s| !s.is_empty());
        self.conn.lock().execute(
            "UPDATE clips SET category = ?1 WHERE id = ?2",
            params![category, id],
        )?;
        Ok(())
    }

    /// Delete rows matching `where_sql` and remove the image files they owned.
    fn delete_where(&self, where_sql: &str, args: &[&dyn rusqlite::ToSql]) -> Result<usize> {
        let files: Vec<Option<String>> = {
            let c = self.conn.lock();
            let mut stmt = c.prepare(&format!(
                "DELETE FROM clips WHERE {where_sql} RETURNING CASE WHEN kind = 'image' THEN content END"
            ))?;
            let rows = stmt.query_map(args, |r| r.get(0))?;
            rows.collect::<rusqlite::Result<_>>()?
        };
        for file in files.iter().flatten() {
            if !self.image_in_use(file)? {
                let _ = std::fs::remove_file(self.image_path(file));
            }
        }
        Ok(files.len())
    }

    fn image_in_use(&self, file: &str) -> Result<bool> {
        Ok(self
            .conn
            .lock()
            .query_row(
                "SELECT 1 FROM clips WHERE kind = 'image' AND content = ?1 LIMIT 1",
                params![file],
                |_| Ok(()),
            )
            .optional()?
            .is_some())
    }

    fn reclaim_space(&self) {
        let c = self.conn.lock();
        let _ = c.execute_batch("PRAGMA incremental_vacuum;");
        checkpoint(&c);
    }

    pub fn delete(&self, id: i64) -> Result<()> {
        self.delete_where("id = ?1", &[&id])?;
        Ok(())
    }

    pub fn clear_all(&self, keep_pinned: bool) -> Result<usize> {
        let n = if keep_pinned {
            self.delete_where("pinned = 0", &[])?
        } else {
            self.delete_where("1", &[])?
        };
        self.reclaim_space();
        Ok(n)
    }

    /// Keep at most `max` non-pinned clips (the most recently used ones).
    pub fn enforce_limit(&self, max: i64) -> Result<usize> {
        if max <= 0 {
            return Ok(0);
        }
        self.delete_where(
            "id IN (SELECT id FROM clips WHERE pinned = 0 ORDER BY used_at DESC LIMIT -1 OFFSET ?1)",
            &[&max],
        )
    }

    /// Delete clips not used for `days` days, honouring the exclusions.
    pub fn cleanup_expired(
        &self,
        days: i64,
        keep_favorites: bool,
        keep_pinned: bool,
    ) -> Result<usize> {
        if days <= 0 {
            return Ok(0);
        }
        let cutoff = (Utc::now() - chrono::Duration::days(days))
            .to_rfc3339_opts(SecondsFormat::Millis, true);
        let mut cond = String::from("used_at < ?1");
        if keep_pinned {
            cond.push_str(" AND pinned = 0");
        }
        if keep_favorites {
            cond.push_str(" AND favorite = 0");
        }
        let n = self.delete_where(&cond, &[&cutoff])?;
        if n > 0 {
            self.reclaim_space();
        }
        Ok(n)
    }

    /// Clips used per local calendar day over the last `days` days.
    pub fn histogram(&self, days: i64) -> Result<Vec<(String, i64)>> {
        let cutoff = (Utc::now() - chrono::Duration::days(days.clamp(1, 366)))
            .to_rfc3339_opts(SecondsFormat::Millis, true);
        let c = self.conn.lock();
        let mut stmt = c.prepare_cached(
            "SELECT date(used_at, 'localtime') AS day, COUNT(*) FROM clips
             WHERE used_at >= ?1 GROUP BY day ORDER BY day",
        )?;
        let rows = stmt.query_map(params![cutoff], |r| Ok((r.get(0)?, r.get(1)?)))?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    pub fn stats(&self) -> Result<Stats> {
        let mut s = self.conn.lock().query_row(
            "SELECT COUNT(*),
                    COALESCE(SUM(kind = 'text'), 0), COALESCE(SUM(kind = 'code'), 0),
                    COALESCE(SUM(kind = 'url'), 0), COALESCE(SUM(kind = 'file'), 0),
                    COALESCE(SUM(kind = 'image'), 0),
                    COALESCE(SUM(pinned), 0), COALESCE(SUM(favorite), 0)
             FROM clips",
            [],
            |r| {
                Ok(Stats {
                    total: r.get(0)?,
                    text: r.get(1)?,
                    code: r.get(2)?,
                    url: r.get(3)?,
                    file: r.get(4)?,
                    image: r.get(5)?,
                    pinned: r.get(6)?,
                    favorites: r.get(7)?,
                    disk_bytes: 0,
                })
            },
        )?;
        s.disk_bytes = self.disk_usage();
        Ok(s)
    }

    fn disk_usage(&self) -> u64 {
        let file_len = |p: &Path| std::fs::metadata(p).map(|m| m.len()).unwrap_or(0);
        let mut total = file_len(&self.path);
        for ext in ["db-wal", "db-shm"] {
            total += file_len(&self.path.with_extension(ext));
        }
        if let Ok(entries) = std::fs::read_dir(&self.images_dir) {
            total += entries
                .flatten()
                .filter_map(|e| e.metadata().ok())
                .map(|m| m.len())
                .sum::<u64>();
        }
        total
    }

    fn strings(&self, sql: &str) -> Result<Vec<String>> {
        let c = self.conn.lock();
        let mut stmt = c.prepare_cached(sql)?;
        let rows = stmt.query_map([], |r| r.get(0))?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    fn counts(&self, sql: &str) -> Result<Vec<(String, i64)>> {
        let c = self.conn.lock();
        let mut stmt = c.prepare_cached(sql)?;
        let rows = stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    pub fn categories(&self) -> Result<Vec<String>> {
        self.strings(
            "SELECT DISTINCT category FROM clips WHERE COALESCE(category, '') <> '' ORDER BY category",
        )
    }

    pub fn tags(&self) -> Result<Vec<String>> {
        self.strings("SELECT DISTINCT j.value FROM clips, json_each(clips.tags) j ORDER BY 1")
    }

    pub fn languages(&self) -> Result<Vec<String>> {
        self.strings(
            "SELECT DISTINCT language FROM clips WHERE COALESCE(language, '') <> '' ORDER BY language",
        )
    }

    pub fn category_counts(&self) -> Result<Vec<(String, i64)>> {
        self.counts(
            "SELECT category, COUNT(*) FROM clips WHERE COALESCE(category, '') <> ''
             GROUP BY category ORDER BY 2 DESC, 1",
        )
    }

    pub fn tag_counts(&self) -> Result<Vec<(String, i64)>> {
        self.counts(
            "SELECT j.value, COUNT(*) FROM clips, json_each(clips.tags) j
             GROUP BY j.value ORDER BY 2 DESC, 1",
        )
    }

    pub fn rename_category(&self, old: &str, new: &str) -> Result<usize> {
        let new = new.trim();
        if new.is_empty() {
            return Err(anyhow!("Le nom ne peut pas être vide."));
        }
        Ok(self.conn.lock().execute(
            "UPDATE clips SET category = ?1 WHERE category = ?2",
            params![new, old],
        )?)
    }

    pub fn delete_category(&self, name: &str) -> Result<usize> {
        Ok(self.conn.lock().execute(
            "UPDATE clips SET category = NULL WHERE category = ?1",
            params![name],
        )?)
    }

    /// Replace (`Some`) or remove (`None`) a tag on every clip carrying it.
    pub fn edit_tag(&self, old: &str, new: Option<&str>) -> Result<usize> {
        let mut c = self.conn.lock();
        let tx = c.transaction()?;
        let rows: Vec<(i64, String)> = {
            let mut stmt = tx.prepare(
                "SELECT id, tags FROM clips WHERE EXISTS (SELECT 1 FROM json_each(clips.tags) WHERE value = ?1)",
            )?;
            let rows = stmt.query_map(params![old], |r| Ok((r.get(0)?, r.get(1)?)))?;
            rows.collect::<rusqlite::Result<_>>()?
        };
        for (id, json) in &rows {
            let tags: Vec<String> = serde_json::from_str::<Vec<String>>(json)
                .unwrap_or_default()
                .into_iter()
                .filter_map(|t| {
                    if t == old {
                        new.map(str::to_string)
                    } else {
                        Some(t)
                    }
                })
                .collect();
            let json = serde_json::to_string(&normalize_tags(&tags))?;
            tx.execute(
                "UPDATE clips SET tags = ?1 WHERE id = ?2",
                params![json, id],
            )?;
        }
        tx.commit()?;
        Ok(rows.len())
    }

    /// Import a batch of clips inside one transaction. Returns how many were new.
    pub fn import(&self, clips: &[ImportedClip]) -> Result<usize> {
        let mut c = self.conn.lock();
        let tx = c.transaction()?;
        let mut added = 0;
        {
            let mut stmt = tx.prepare(
                "INSERT OR IGNORE INTO clips (kind, content, preview, language, category, tags, pinned,
                    favorite, source_app, size_bytes, hash, created_at, used_at, use_count)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
            )?;
            for clip in clips {
                added += stmt.execute(params![
                    clip.kind,
                    clip.content,
                    clip.preview,
                    clip.language,
                    clip.category,
                    serde_json::to_string(&clip.tags)?,
                    clip.pinned,
                    clip.favorite,
                    clip.source_app,
                    clip.size_bytes,
                    clip.hash,
                    clip.created_at,
                    clip.used_at,
                    clip.use_count
                ])?;
            }
        }
        tx.commit()?;
        Ok(added)
    }

    /// Every clip with its full content, for export.
    pub fn export_rows(&self) -> Result<Vec<ClipItem>> {
        let c = self.conn.lock();
        let mut stmt = c.prepare(&format!(
            "SELECT {CLIP_COLUMNS}, content FROM clips ORDER BY used_at DESC"
        ))?;
        let rows = stmt.query_map([], |r| {
            let mut item = self.row_to_clip(r)?;
            item.content = Some(r.get(14)?);
            Ok(item)
        })?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    // ---- Settings ----

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

pub struct ImportedClip {
    pub kind: String,
    pub content: String,
    pub preview: String,
    pub language: Option<String>,
    pub category: Option<String>,
    pub tags: Vec<String>,
    pub pinned: bool,
    pub favorite: bool,
    pub source_app: Option<String>,
    pub size_bytes: i64,
    pub hash: String,
    pub created_at: String,
    pub used_at: String,
    pub use_count: i64,
}

const SCHEMA_TABLES: &str = "
CREATE TABLE IF NOT EXISTS clips (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    kind TEXT NOT NULL,
    content TEXT NOT NULL,
    preview TEXT NOT NULL,
    language TEXT,
    category TEXT,
    tags TEXT NOT NULL DEFAULT '[]',
    pinned INTEGER NOT NULL DEFAULT 0,
    favorite INTEGER NOT NULL DEFAULT 0,
    source_app TEXT,
    size_bytes INTEGER NOT NULL DEFAULT 0,
    hash TEXT NOT NULL UNIQUE,
    created_at TEXT NOT NULL,
    used_at TEXT NOT NULL,
    use_count INTEGER NOT NULL DEFAULT 1
);
CREATE INDEX IF NOT EXISTS idx_clips_used ON clips(pinned DESC, used_at DESC);
CREATE INDEX IF NOT EXISTS idx_clips_used_at ON clips(used_at);
CREATE INDEX IF NOT EXISTS idx_clips_kind ON clips(kind);
CREATE INDEX IF NOT EXISTS idx_clips_category ON clips(category);
CREATE TABLE IF NOT EXISTS settings (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
);
";

// Only the searchable columns are indexed. The update trigger is limited to
// those columns so that bumping `used_at` does not rewrite the index.
const SCHEMA_FTS: &str = "
CREATE VIRTUAL TABLE IF NOT EXISTS clips_fts USING fts5(
    content, category, tags,
    content='clips', content_rowid='id',
    tokenize='unicode61 remove_diacritics 2'
);
CREATE TRIGGER IF NOT EXISTS clips_ai AFTER INSERT ON clips BEGIN
    INSERT INTO clips_fts(rowid, content, category, tags)
    VALUES (new.id, new.content, new.category, new.tags);
END;
CREATE TRIGGER IF NOT EXISTS clips_ad AFTER DELETE ON clips BEGIN
    INSERT INTO clips_fts(clips_fts, rowid, content, category, tags)
    VALUES ('delete', old.id, old.content, old.category, old.tags);
END;
CREATE TRIGGER IF NOT EXISTS clips_au AFTER UPDATE OF content, category, tags ON clips BEGIN
    INSERT INTO clips_fts(clips_fts, rowid, content, category, tags)
    VALUES ('delete', old.id, old.content, old.category, old.tags);
    INSERT INTO clips_fts(rowid, content, category, tags)
    VALUES (new.id, new.content, new.category, new.tags);
END;
";

/// Flush the write-ahead log into the database and shrink it.
fn checkpoint(c: &Connection) {
    let _ = c.query_row("PRAGMA wal_checkpoint(TRUNCATE)", [], |_| Ok(()));
}

pub fn normalize_tags(tags: &[String]) -> Vec<String> {
    let mut out: Vec<String> = tags
        .iter()
        .map(|t| {
            t.trim()
                .trim_start_matches('#')
                .trim()
                .chars()
                .take(48)
                .collect::<String>()
        })
        .filter(|t| !t.is_empty())
        .collect();
    out.sort();
    out.dedup();
    out.truncate(32);
    out
}

/// Turn free text into a safe FTS5 query: every word becomes a quoted prefix
/// term. Returns `None` when there is nothing searchable.
fn fts_query(q: &str) -> Option<String> {
    let terms: Vec<String> = q
        .split_whitespace()
        .filter(|w| w.chars().any(char::is_alphanumeric))
        .map(|w| format!("\"{}\"*", w.replace('"', "\"\"")))
        .collect();
    (!terms.is_empty()).then(|| terms.join(" "))
}

/// UTC bounds of a local calendar day given as YYYY-MM-DD.
fn local_day_bounds(day: &str) -> Option<(String, String)> {
    let d = chrono::NaiveDate::parse_from_str(day, "%Y-%m-%d").ok()?;
    Some((
        start_of_local_day(d),
        start_of_local_day(d + chrono::Duration::days(1)),
    ))
}

fn start_of_local_day(d: chrono::NaiveDate) -> String {
    use chrono::{Local, TimeZone};
    let midnight = d.and_hms_opt(0, 0, 0).expect("valid time");
    Local
        .from_local_datetime(&midnight)
        .earliest()
        .map(|dt| dt.with_timezone(&Utc))
        .unwrap_or_else(|| Utc.from_utc_datetime(&midnight))
        .to_rfc3339_opts(SecondsFormat::Millis, true)
}

/// Resolve a range token (today, yesterday, week, month, year or a
/// YYYY-MM-DD day) to UTC bounds based on local calendar days.
fn time_range_bounds(range: &str) -> Option<(String, String)> {
    use chrono::Duration;
    let today = chrono::Local::now().date_naive();
    let tomorrow = today + Duration::days(1);
    let (from, to) = match range {
        "today" => (today, tomorrow),
        "yesterday" => (today - Duration::days(1), today),
        "week" => (today - Duration::days(6), tomorrow),
        "month" => (today - Duration::days(29), tomorrow),
        "year" => (today - Duration::days(364), tomorrow),
        "" => return None,
        day => return local_day_bounds(day),
    };
    Some((start_of_local_day(from), start_of_local_day(to)))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_db() -> (Db, PathBuf) {
        let dir = std::env::temp_dir().join(format!("clipper-test-{}", rand_suffix()));
        std::fs::create_dir_all(&dir).unwrap();
        (Db::open(&dir).unwrap(), dir)
    }

    fn rand_suffix() -> String {
        format!("{}-{:?}", std::process::id(), std::thread::current().id()).replace(['(', ')'], "")
            + &Utc::now().timestamp_nanos_opt().unwrap_or(0).to_string()
    }

    fn text(db: &Db, s: &str) -> i64 {
        db.upsert_clip(&NewClip {
            kind: "text",
            content: s,
            preview: s,
            language: None,
            source_app: None,
            size_bytes: s.len() as i64,
            hash: &format!("t:{s}"),
        })
        .unwrap()
    }

    #[test]
    fn fts_query_escapes_and_skips_punctuation() {
        assert_eq!(fts_query("  "), None);
        assert_eq!(fts_query("*** ()"), None);
        assert_eq!(fts_query("foo \"bar"), Some("\"foo\"* \"\"\"bar\"*".into()));
    }

    #[test]
    fn upsert_dedupes_and_search_finds_accents() {
        let (db, dir) = temp_db();
        let a = text(&db, "Café crème à emporter");
        let b = text(&db, "Café crème à emporter");
        assert_eq!(a, b);
        assert_eq!(db.get(a).unwrap().unwrap().use_count, 2);
        let found = db
            .list(&ListParams {
                query: Some("cafe".into()),
                ..Default::default()
            })
            .unwrap();
        assert_eq!(found.len(), 1);
        assert!(found[0].content.is_none());
        let none = db
            .list(&ListParams {
                query: Some("\"(".into()),
                ..Default::default()
            })
            .unwrap();
        assert_eq!(none.len(), 1, "unsearchable query means no filter");
        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn tags_are_normalized_and_renamed() {
        let (db, dir) = temp_db();
        let id = text(&db, "hello");
        db.update_tags(
            id,
            &["#work".into(), "work".into(), " api ".into(), "".into()],
        )
        .unwrap();
        assert_eq!(db.get(id).unwrap().unwrap().tags, vec!["api", "work"]);
        db.edit_tag("work", Some("job")).unwrap();
        assert_eq!(db.tags().unwrap(), vec!["api", "job"]);
        db.edit_tag("api", None).unwrap();
        assert_eq!(db.tag_counts().unwrap(), vec![("job".to_string(), 1)]);
        let filtered = db
            .list(&ListParams {
                tags: Some(vec!["job".into()]),
                ..Default::default()
            })
            .unwrap();
        assert_eq!(filtered.len(), 1);
        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn migrates_v1_database() {
        let dir = std::env::temp_dir().join(format!("clipper-v1-{}", rand_suffix()));
        std::fs::create_dir_all(&dir).unwrap();
        {
            let c = Connection::open(dir.join("clipper.db")).unwrap();
            c.execute_batch(
                "CREATE TABLE clips (id INTEGER PRIMARY KEY AUTOINCREMENT, kind TEXT NOT NULL,
                    content TEXT NOT NULL, preview TEXT NOT NULL, language TEXT, category TEXT,
                    tags TEXT NOT NULL DEFAULT '[]', pinned INTEGER NOT NULL DEFAULT 0,
                    favorite INTEGER NOT NULL DEFAULT 0, source_app TEXT,
                    size_bytes INTEGER NOT NULL DEFAULT 0, hash TEXT NOT NULL UNIQUE,
                    created_at TEXT NOT NULL, used_at TEXT NOT NULL, use_count INTEGER NOT NULL DEFAULT 1);
                 CREATE VIRTUAL TABLE clips_fts USING fts5(content, preview, category, tags,
                    content='clips', content_rowid='id');
                 CREATE TABLE settings (key TEXT PRIMARY KEY, value TEXT NOT NULL);
                 INSERT INTO clips (kind, content, preview, hash, created_at, used_at) VALUES
                    ('text', 'bonjour le monde', 'bonjour', 't:1',
                     '2026-01-01T10:00:00.123456+00:00', '2026-01-02T10:00:00+00:00'),
                    ('image', 'iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNk+M9QDwADhgGAWjR9awAAAABJRU5ErkJggg==',
                     '1×1', 'i:1', '2026-01-01T10:00:00+00:00', '2026-01-01T10:00:00+00:00'),
                    ('image', '!!not base64!!', 'x', 'i:2', '2026-01-01T10:00:00+00:00', '2026-01-01T10:00:00+00:00');
                 INSERT INTO settings VALUES ('app', '{\"theme\":\"dark\",\"openai_api_key\":\"sk-test\",\"anthropic_api_key\":\"\"}');",
            )
            .unwrap();
        }
        let db = Db::open(&dir).unwrap();
        let clips = db.list(&ListParams::default()).unwrap();
        assert_eq!(clips.len(), 2, "undecodable image dropped");
        let img = clips.iter().find(|c| c.kind == "image").unwrap();
        assert!(Path::new(img.image_path.as_ref().unwrap()).exists());
        let text = clips.iter().find(|c| c.kind == "text").unwrap();
        assert_eq!(text.used_at, "2026-01-02T10:00:00.000Z");
        let found = db
            .list(&ListParams {
                query: Some("monde".into()),
                ..Default::default()
            })
            .unwrap();
        assert_eq!(found.len(), 1);
        assert_eq!(
            db.take_legacy_api_keys().unwrap(),
            vec![("openai", "sk-test".to_string())]
        );
        assert!(db.take_legacy_api_keys().unwrap().is_empty());
        assert_eq!(db.get_settings().unwrap().theme, "dark");
        drop(db);
        std::fs::remove_dir_all(dir).ok();
    }

    /// Migration of a real profile: `CLIPPER_TEST_DIR=<copy> cargo test --release -- --ignored`
    #[test]
    #[ignore]
    fn migrates_real_profile_copy() {
        let dir = PathBuf::from(std::env::var("CLIPPER_TEST_DIR").expect("CLIPPER_TEST_DIR"));
        let start = std::time::Instant::now();
        let db = Db::open(&dir).unwrap();
        println!("migration: {:?}", start.elapsed());
        let s = db.stats().unwrap();
        println!(
            "total={} text={} code={} url={} file={} image={} pinned={} disk={} MB",
            s.total,
            s.text,
            s.code,
            s.url,
            s.file,
            s.image,
            s.pinned,
            s.disk_bytes / 1_048_576
        );
        let start = std::time::Instant::now();
        let page = db.list(&ListParams::default()).unwrap();
        println!("first page ({} rows): {:?}", page.len(), start.elapsed());
        let start = std::time::Instant::now();
        let found = db
            .list(&ListParams {
                query: Some("http".into()),
                ..Default::default()
            })
            .unwrap();
        println!("search ({} rows): {:?}", found.len(), start.elapsed());
        let missing = page
            .iter()
            .filter_map(|c| c.image_path.as_ref())
            .filter(|p| !Path::new(p).exists())
            .count();
        assert_eq!(missing, 0);
    }

    #[test]
    fn limit_keeps_pinned_and_removes_image_files() {
        let (db, dir) = temp_db();
        let pinned = text(&db, "keep me");
        db.toggle_pin(pinned).unwrap();
        let file = db.store_image_file(b"not really a png").unwrap();
        db.upsert_clip(&NewClip {
            kind: "image",
            content: &file,
            preview: "img",
            language: None,
            source_app: None,
            size_bytes: 16,
            hash: "i:1",
        })
        .unwrap();
        std::thread::sleep(std::time::Duration::from_millis(5));
        text(&db, "newest");
        assert_eq!(db.enforce_limit(1).unwrap(), 1);
        assert!(!db.image_path(&file).exists());
        assert_eq!(db.stats().unwrap().total, 2);
        std::fs::remove_dir_all(dir).ok();
    }
}
