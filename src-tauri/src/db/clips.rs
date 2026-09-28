use super::{checkpoint, fts_query, normalize_tags, now, time_range_bounds, Db};
use crate::models::{ClipItem, ListParams, SourceApp, Stats};
use anyhow::{anyhow, Result};
use chrono::{SecondsFormat, Utc};
use rusqlite::{params, params_from_iter, types::Value, OptionalExtension};

const CLIP_COLUMNS: &str = "id, kind, preview, language, tags, pinned, collection_id, sensitive, \
     has_rich, source_app, size_bytes, created_at, used_at, use_count, \
     CASE WHEN kind = 'image' THEN content END";
const N_COLUMNS: usize = 15;

/// Rows kept by retention and the item limit: pinned and filed ones.
const KEPT: &str = "(pinned = 1 OR collection_id IS NOT NULL)";

/// Row to insert, built by the clipboard monitor.
pub struct NewClip<'a> {
    pub kind: &'a str,
    pub content: &'a str,
    pub preview: &'a str,
    pub language: Option<&'a str>,
    pub source_app: Option<&'a str>,
    pub size_bytes: i64,
    pub hash: &'a str,
    pub sensitive: bool,
    pub rich: &'a RichFormats,
}

/// Formatted versions of a text clip, pasted back when rich text is kept.
#[derive(Default, Clone)]
pub struct RichFormats {
    pub html: Option<Vec<u8>>,
    pub rtf: Option<Vec<u8>>,
}

impl RichFormats {
    pub fn is_empty(&self) -> bool {
        self.html.is_none() && self.rtf.is_none()
    }
}

pub struct ImportedClip {
    pub kind: String,
    pub content: String,
    pub preview: String,
    pub language: Option<String>,
    pub tags: Vec<String>,
    pub pinned: bool,
    pub collection: Option<String>,
    pub source_app: Option<String>,
    pub size_bytes: i64,
    pub hash: String,
    pub sensitive: bool,
    pub created_at: String,
    pub used_at: String,
    pub use_count: i64,
}

impl Db {
    /// Insert a clip, or bump it if an identical one already exists.
    pub fn upsert_clip(&self, clip: &NewClip) -> Result<i64> {
        let now = now();
        let mut c = self.conn.lock();
        let tx = c.transaction()?;
        let id: i64 = tx.query_row(
            "INSERT INTO clips (kind, content, preview, language, source_app, size_bytes, hash,
                    sensitive, has_rich, created_at, used_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?10)
             ON CONFLICT(hash) DO UPDATE SET
                used_at = excluded.used_at,
                use_count = use_count + 1,
                source_app = COALESCE(excluded.source_app, source_app),
                has_rich = MAX(has_rich, excluded.has_rich)
             RETURNING id",
            params![
                clip.kind,
                clip.content,
                clip.preview,
                clip.language,
                clip.source_app,
                clip.size_bytes,
                clip.hash,
                clip.sensitive,
                !clip.rich.is_empty(),
                now
            ],
            |r| r.get(0),
        )?;
        for (format, data) in [("html", &clip.rich.html), ("rtf", &clip.rich.rtf)] {
            if let Some(data) = data {
                tx.execute(
                    "INSERT OR REPLACE INTO clip_formats (clip_id, format, data) VALUES (?1, ?2, ?3)",
                    params![id, format, data],
                )?;
            }
        }
        tx.commit()?;
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
        if let Some(id) = p.collection_id {
            conds.push("collection_id = ?".into());
            args.push(id.into());
        }
        if let Some(app) = p.source_app.as_ref().filter(|s| !s.is_empty()) {
            conds.push("source_app = ?".into());
            args.push(app.clone().into());
        }
        for tag in p.tags.iter().flatten() {
            conds.push("EXISTS (SELECT 1 FROM json_each(clips.tags) WHERE value = ?)".into());
            args.push(tag.clone().into());
        }
        if p.pinned_only {
            conds.push("pinned = 1".into());
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
        sql.push_str(" ORDER BY pinned DESC, used_at DESC LIMIT ? OFFSET ?");
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
            "SELECT {CLIP_COLUMNS}, content, ocr_text FROM clips WHERE id = ?1"
        ))?;
        let item = stmt
            .query_row(params![id], |r| {
                let mut item = self.row_to_clip(r)?;
                item.content = Some(r.get(N_COLUMNS)?);
                item.ocr_text = r.get(N_COLUMNS + 1)?;
                Ok(item)
            })
            .optional()?;
        Ok(item)
    }

    /// Raw stored content (image file name for images).
    pub fn content(&self, id: i64) -> Result<(String, String)> {
        self.conn
            .lock()
            .query_row(
                "SELECT kind, content FROM clips WHERE id = ?1",
                params![id],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()?
            .ok_or_else(|| anyhow!("Élément introuvable."))
    }

    pub fn rich_formats(&self, id: i64) -> Result<RichFormats> {
        let c = self.conn.lock();
        let mut stmt =
            c.prepare_cached("SELECT format, data FROM clip_formats WHERE clip_id = ?1")?;
        let mut rich = RichFormats::default();
        let rows = stmt.query_map(params![id], |r| Ok((r.get::<_, String>(0)?, r.get(1)?)))?;
        for row in rows {
            let (format, data) = row?;
            match format.as_str() {
                "html" => rich.html = Some(data),
                "rtf" => rich.rtf = Some(data),
                _ => {}
            }
        }
        Ok(rich)
    }

    fn row_to_clip(&self, r: &rusqlite::Row) -> rusqlite::Result<ClipItem> {
        let tags: String = r.get(4)?;
        let image_file: Option<String> = r.get(14)?;
        Ok(ClipItem {
            id: r.get(0)?,
            kind: r.get(1)?,
            preview: r.get(2)?,
            language: r.get(3)?,
            tags: serde_json::from_str(&tags).unwrap_or_default(),
            pinned: r.get(5)?,
            collection_id: r.get(6)?,
            sensitive: r.get(7)?,
            has_rich: r.get(8)?,
            source_app: r.get(9)?,
            size_bytes: r.get(10)?,
            created_at: r.get(11)?,
            used_at: r.get(12)?,
            use_count: r.get(13)?,
            image_path: image_file.map(|f| self.image_path(&f).to_string_lossy().into_owned()),
            content: None,
            ocr_text: None,
        })
    }

    pub fn bump_used(&self, id: i64) -> Result<()> {
        self.conn.lock().execute(
            "UPDATE clips SET used_at = ?1, use_count = use_count + 1 WHERE id = ?2",
            params![now(), id],
        )?;
        Ok(())
    }

    pub fn set_pinned(&self, id: i64, pinned: bool) -> Result<()> {
        self.conn.lock().execute(
            "UPDATE clips SET pinned = ?1 WHERE id = ?2",
            params![pinned, id],
        )?;
        Ok(())
    }

    pub fn set_sensitive(&self, id: i64, sensitive: bool) -> Result<()> {
        self.conn.lock().execute(
            "UPDATE clips SET sensitive = ?1 WHERE id = ?2",
            params![sensitive, id],
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

    pub fn set_collection(&self, id: i64, collection_id: Option<i64>) -> Result<()> {
        self.conn.lock().execute(
            "UPDATE clips SET collection_id = ?1 WHERE id = ?2",
            params![collection_id, id],
        )?;
        Ok(())
    }

    /// Replace the text of a clip (edit before paste). If the new text already
    /// exists as another clip, the two are merged into that one.
    pub fn update_text(&self, id: i64, text: &str, sensitive: bool) -> Result<i64> {
        let (kind, language) = crate::clipboard::classify(text);
        let preview = crate::clipboard::make_preview(text);
        let hash = crate::clipboard::hash_text(text);
        let mut c = self.conn.lock();
        let tx = c.transaction()?;
        let existing: Option<i64> = tx
            .query_row(
                "SELECT id FROM clips WHERE hash = ?1 AND id <> ?2",
                params![hash, id],
                |r| r.get(0),
            )
            .optional()?;
        let target = match existing {
            Some(other) => {
                tx.execute(
                    "UPDATE clips SET used_at = ?1, use_count = use_count + 1 WHERE id = ?2",
                    params![now(), other],
                )?;
                tx.execute("DELETE FROM clips WHERE id = ?1", params![id])?;
                other
            }
            None => {
                tx.execute(
                    "UPDATE clips SET content = ?1, kind = ?2, language = ?3, preview = ?4, hash = ?5,
                        sensitive = ?6, size_bytes = ?7, has_rich = 0, used_at = ?8
                     WHERE id = ?9",
                    params![
                        text,
                        kind,
                        language,
                        preview,
                        hash,
                        sensitive,
                        text.len() as i64,
                        now(),
                        id
                    ],
                )?;
                // Formatted versions no longer match the edited text.
                tx.execute("DELETE FROM clip_formats WHERE clip_id = ?1", params![id])?;
                id
            }
        };
        tx.commit()?;
        Ok(target)
    }

    pub fn set_ocr_text(&self, id: i64, text: &str) -> Result<()> {
        self.conn.lock().execute(
            "UPDATE clips SET ocr_text = ?1 WHERE id = ?2",
            params![text, id],
        )?;
        Ok(())
    }

    /// Image clips not yet processed by OCR, most recent first.
    pub fn images_without_ocr(&self, limit: i64) -> Result<Vec<(i64, String)>> {
        let c = self.conn.lock();
        let mut stmt = c.prepare_cached(
            "SELECT id, content FROM clips WHERE kind = 'image' AND ocr_text IS NULL
             ORDER BY used_at DESC LIMIT ?1",
        )?;
        let rows = stmt.query_map(params![limit], |r| Ok((r.get(0)?, r.get(1)?)))?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
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

    /// Delete clips chosen by the user. They are kept aside (in temporary
    /// tables, gone when Clipper quits) until the next deletion, so the last
    /// one can be undone.
    pub fn delete(&self, ids: &[i64]) -> Result<usize> {
        self.forget_undo()?;
        let ids = serde_json::to_string(ids)?;
        let c = self.conn.lock();
        c.execute_batch(
            "CREATE TEMP TABLE IF NOT EXISTS undo_clips AS SELECT * FROM clips WHERE 0;
             CREATE TEMP TABLE IF NOT EXISTS undo_formats AS SELECT * FROM clip_formats WHERE 0;",
        )?;
        c.execute(
            "INSERT INTO undo_clips SELECT * FROM clips WHERE id IN (SELECT value FROM json_each(?1))",
            params![ids],
        )?;
        c.execute(
            "INSERT INTO undo_formats SELECT * FROM clip_formats
             WHERE clip_id IN (SELECT value FROM json_each(?1))",
            params![ids],
        )?;
        Ok(c.execute(
            "DELETE FROM clips WHERE id IN (SELECT value FROM json_each(?1))",
            params![ids],
        )?)
    }

    /// Put back the clips removed by the last `delete`. A clip copied again
    /// in the meantime is not duplicated.
    pub fn undo_delete(&self) -> Result<usize> {
        let mut c = self.conn.lock();
        if !has_undo(&c)? {
            return Ok(0);
        }
        let tx = c.transaction()?;
        let n = tx.execute("INSERT OR IGNORE INTO clips SELECT * FROM undo_clips", [])?;
        tx.execute(
            "INSERT OR IGNORE INTO clip_formats SELECT * FROM undo_formats
             WHERE clip_id IN (SELECT id FROM clips)",
            [],
        )?;
        tx.execute_batch("DELETE FROM undo_clips; DELETE FROM undo_formats;")?;
        tx.commit()?;
        Ok(n)
    }

    /// Make the last deletion final: drop the kept rows and their image files.
    pub fn forget_undo(&self) -> Result<()> {
        let files: Vec<String> = {
            let c = self.conn.lock();
            if !has_undo(&c)? {
                return Ok(());
            }
            let files = {
                let mut stmt = c.prepare("SELECT content FROM undo_clips WHERE kind = 'image'")?;
                let rows = stmt.query_map([], |r| r.get(0))?;
                rows.collect::<rusqlite::Result<_>>()?
            };
            c.execute_batch("DELETE FROM undo_clips; DELETE FROM undo_formats;")?;
            files
        };
        for file in files {
            if !self.image_in_use(&file)? {
                let _ = std::fs::remove_file(self.image_path(&file));
            }
        }
        Ok(())
    }

    /// Delete the history, keeping pinned clips and clips filed in a collection.
    pub fn clear_history(&self) -> Result<usize> {
        let n = self.delete_where(&format!("NOT {KEPT}"), &[])?;
        self.reclaim_space();
        Ok(n)
    }

    /// Keep at most `max` unpinned, unfiled clips (the most recently used ones).
    pub fn enforce_limit(&self, max: i64) -> Result<usize> {
        if max <= 0 {
            return Ok(0);
        }
        self.delete_where(
            &format!(
                "id IN (SELECT id FROM clips WHERE NOT {KEPT} ORDER BY used_at DESC LIMIT -1 OFFSET ?1)"
            ),
            &[&max],
        )
    }

    /// Delete unpinned, unfiled clips not used for `days` days.
    pub fn cleanup_expired(&self, days: i64) -> Result<usize> {
        if days <= 0 {
            return Ok(0);
        }
        let cutoff = (Utc::now() - chrono::Duration::days(days))
            .to_rfc3339_opts(SecondsFormat::Millis, true);
        let n = self.delete_where(&format!("used_at < ?1 AND NOT {KEPT}"), &[&cutoff])?;
        if n > 0 {
            self.reclaim_space();
        }
        Ok(n)
    }

    pub fn stats(&self) -> Result<Stats> {
        let mut s = self.conn.lock().query_row(
            "SELECT COUNT(*),
                    COALESCE(SUM(kind = 'text'), 0), COALESCE(SUM(kind = 'code'), 0),
                    COALESCE(SUM(kind = 'url'), 0), COALESCE(SUM(kind = 'file'), 0),
                    COALESCE(SUM(kind = 'image'), 0),
                    COALESCE(SUM(pinned), 0), COALESCE(SUM(sensitive), 0)
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
                    sensitive: r.get(7)?,
                    disk_bytes: 0,
                })
            },
        )?;
        s.disk_bytes = self.disk_usage();
        Ok(s)
    }

    pub fn tags(&self) -> Result<Vec<(String, i64)>> {
        let c = self.conn.lock();
        let mut stmt = c.prepare_cached(
            "SELECT j.value, COUNT(*) FROM clips, json_each(clips.tags) j
             GROUP BY j.value ORDER BY 2 DESC, 1",
        )?;
        let rows = stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    /// Applications clips were copied from, most frequent first.
    pub fn source_apps(&self) -> Result<Vec<SourceApp>> {
        let c = self.conn.lock();
        let mut stmt = c.prepare_cached(
            "SELECT source_app, COUNT(*) FROM clips WHERE COALESCE(source_app, '') <> ''
             GROUP BY source_app ORDER BY 2 DESC, 1 LIMIT 30",
        )?;
        let rows = stmt.query_map([], |r| {
            Ok(SourceApp {
                name: r.get(0)?,
                count: r.get(1)?,
            })
        })?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
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
        let now = now();
        {
            let mut add_collection = tx.prepare(
                "INSERT OR IGNORE INTO collections (name, position, created_at)
                 VALUES (?1, (SELECT COALESCE(MAX(position), 0) + 1 FROM collections), ?2)",
            )?;
            let mut find_collection = tx.prepare("SELECT id FROM collections WHERE name = ?1")?;
            let mut stmt = tx.prepare(
                "INSERT OR IGNORE INTO clips (kind, content, preview, language, tags, pinned,
                    source_app, size_bytes, hash, sensitive, created_at, used_at, use_count,
                    collection_id)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
            )?;
            for clip in clips {
                let collection_id: Option<i64> = match &clip.collection {
                    Some(name) => {
                        add_collection.execute(params![name, now])?;
                        find_collection
                            .query_row(params![name], |r| r.get(0))
                            .optional()?
                    }
                    None => None,
                };
                added += stmt.execute(params![
                    clip.kind,
                    clip.content,
                    clip.preview,
                    clip.language,
                    serde_json::to_string(&clip.tags)?,
                    clip.pinned,
                    clip.source_app,
                    clip.size_bytes,
                    clip.hash,
                    clip.sensitive,
                    clip.created_at,
                    clip.used_at,
                    clip.use_count,
                    collection_id
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
            "SELECT {CLIP_COLUMNS}, content, ocr_text FROM clips ORDER BY used_at DESC"
        ))?;
        let rows = stmt.query_map([], |r| {
            let mut item = self.row_to_clip(r)?;
            item.content = Some(r.get(N_COLUMNS)?);
            item.ocr_text = r.get(N_COLUMNS + 1)?;
            Ok(item)
        })?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    /// Flush pending writes (before a backup or on exit).
    pub fn checkpoint(&self) {
        checkpoint(&self.conn.lock());
    }
}

fn has_undo(c: &rusqlite::Connection) -> Result<bool> {
    Ok(c
        .query_row(
            "SELECT 1 FROM temp.sqlite_master WHERE name = 'undo_clips'",
            [],
            |_| Ok(()),
        )
        .optional()?
        .is_some())
}
