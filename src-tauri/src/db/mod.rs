//! SQLite storage: schema, migrations and shared helpers. Queries live in
//! the sub-modules, grouped by what they manage.

mod clips;
mod collections;
mod settings;
mod snippets;
#[cfg(test)]
mod tests;

pub use clips::{ImportedClip, NewClip, RichFormats};

use anyhow::Result;
use base64::Engine;
use chrono::{SecondsFormat, Utc};
use parking_lot::Mutex;
use rusqlite::{params, Connection, OptionalExtension};
use std::path::{Path, PathBuf};

const SCHEMA_VERSION: i64 = 4;

pub struct Db {
    conn: Mutex<Connection>,
    path: PathBuf,
    images_dir: PathBuf,
    /// When the deletion that can still be undone happened.
    undo_since: Mutex<Option<std::time::Instant>>,
}

pub fn now() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true)
}

fn open_connection(path: &Path) -> Result<Connection> {
    let conn = Connection::open(path)?;
    conn.pragma_update(None, "journal_mode", "WAL")?;
    conn.pragma_update(None, "synchronous", "NORMAL")?;
    conn.pragma_update(None, "temp_store", "MEMORY")?;
    // Keep the write-ahead log small once its content is checkpointed.
    conn.pragma_update(None, "journal_size_limit", 4 * 1024 * 1024)?;
    // Deleted clips (secrets included) are overwritten, not left in free pages.
    conn.pragma_update(None, "secure_delete", "FAST")?;
    Ok(conn)
}

impl Db {
    pub fn open(data_dir: &Path) -> Result<Self> {
        let images_dir = data_dir.join("images");
        std::fs::create_dir_all(&images_dir)?;
        let path = data_dir.join("clipper.db");
        let db = Self {
            conn: Mutex::new(open_connection(&path)?),
            path,
            images_dir,
            undo_since: Mutex::new(None),
        };
        db.migrate()?;
        db.conn.lock().pragma_update(None, "foreign_keys", "ON")?;
        db.tidy_images()?;
        Ok(db)
    }

    pub fn data_dir(&self) -> &Path {
        self.path.parent().unwrap_or(Path::new("."))
    }

    /// Write a consistent copy of the database to `dest` (used by backups).
    /// A second connection reads the database (WAL allows it), so capture and
    /// the interface are not blocked while the copy is written.
    pub fn backup_to(&self, dest: &Path) -> Result<()> {
        let _ = std::fs::remove_file(dest);
        Connection::open(&self.path)?.execute(
            "VACUUM INTO ?1",
            params![dest.to_string_lossy().into_owned()],
        )?;
        Ok(())
    }

    /// Image files are stored beside the database. Remove the ones no clip
    /// uses any more (left by a crash before a deletion became final), and
    /// the image clips whose file is gone (a backup restored from before
    /// the file was deleted).
    fn tidy_images(&self) -> Result<()> {
        let used: std::collections::HashSet<String> = {
            let c = self.conn.lock();
            let mut stmt = c.prepare("SELECT content FROM clips WHERE kind = 'image'")?;
            let rows = stmt.query_map([], |r| r.get(0))?;
            rows.collect::<rusqlite::Result<_>>()?
        };
        let mut present = std::collections::HashSet::new();
        for entry in std::fs::read_dir(&self.images_dir)?.flatten() {
            let name = entry.file_name().to_string_lossy().into_owned();
            if used.contains(&name) {
                present.insert(name);
            } else if name.ends_with(".png") {
                let _ = std::fs::remove_file(entry.path());
            }
        }
        let missing: Vec<&String> = used.iter().filter(|f| !present.contains(*f)).collect();
        if !missing.is_empty() {
            let c = self.conn.lock();
            for file in missing {
                c.execute(
                    "DELETE FROM clips WHERE kind = 'image' AND content = ?1",
                    params![file],
                )?;
            }
        }
        Ok(())
    }

    /// Replace the database with the file at `src` (a backup) and reopen it.
    pub fn restore_from(&self, src: &Path) -> Result<()> {
        self.forget_undo()?;
        let mut conn = self.conn.lock();
        // Release the file before overwriting it.
        *conn = Connection::open_in_memory()?;
        for ext in ["db-wal", "db-shm"] {
            let _ = std::fs::remove_file(self.path.with_extension(ext));
        }
        let copied = std::fs::copy(src, &self.path);
        *conn = open_connection(&self.path)?;
        copied?;
        drop(conn);
        self.migrate()?;
        self.conn.lock().pragma_update(None, "foreign_keys", "ON")?;
        self.tidy_images()?;
        Ok(())
    }

    fn migrate(&self) -> Result<()> {
        let mut c = self.conn.lock();
        let version: i64 = c.query_row("PRAGMA user_version", [], |r| r.get(0))?;
        if version >= SCHEMA_VERSION {
            return Ok(());
        }
        let has_clips = c
            .query_row(
                "SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = 'clips'",
                [],
                |_| Ok(()),
            )
            .optional()?
            .is_some();

        if !has_clips {
            // Fresh database: incremental auto-vacuum must be set before any table exists.
            c.pragma_update(None, "auto_vacuum", "INCREMENTAL")?;
            let tx = c.transaction()?;
            tx.execute_batch(SCHEMA)?;
            tx.execute_batch(SCHEMA_FTS)?;
            tx.pragma_update(None, "user_version", SCHEMA_VERSION)?;
            tx.commit()?;
            return Ok(());
        }

        let tx = c.transaction()?;
        if version < 3 {
            tx.execute_batch(
                "DROP TRIGGER IF EXISTS clips_ai;
                 DROP TRIGGER IF EXISTS clips_ad;
                 DROP TRIGGER IF EXISTS clips_au;
                 DROP TABLE IF EXISTS clips_fts;",
            )?;
            if version < 2 {
                self.migrate_v1_data(&tx)?;
            }
            migrate_v2_to_v3(&tx)?;
        }
        fix_file_clips(&tx)?;
        tx.pragma_update(None, "user_version", SCHEMA_VERSION)?;
        tx.commit()?;
        if version < 3 {
            // Reclaim the space freed by the migration and use incremental vacuum from now on.
            c.pragma_update(None, "auto_vacuum", "INCREMENTAL")?;
            c.execute_batch("VACUUM;")?;
        }
        checkpoint(&c);
        Ok(())
    }

    /// v1 stored images as base64 inside the database and misclassified prose
    /// as code; timestamps used a different format.
    fn migrate_v1_data(&self, tx: &rusqlite::Transaction) -> Result<()> {
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
                Some(file) => tx.execute(
                    "UPDATE clips SET content = ?1 WHERE id = ?2",
                    params![file, id],
                )?,
                None => tx.execute("DELETE FROM clips WHERE id = ?1", params![id])?,
            };
        }
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

    fn reclaim_space(&self) {
        let c = self.conn.lock();
        let _ = c.execute_batch("PRAGMA incremental_vacuum;");
        checkpoint(&c);
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
}

/// v2 -> v3: favourites become pins, categories become collections, new
/// columns and tables, and a search index that never contains secrets.
fn migrate_v2_to_v3(tx: &rusqlite::Transaction) -> Result<()> {
    let now = now();
    tx.execute_batch(SCHEMA_COLLECTIONS)?;
    tx.execute(
        "INSERT OR IGNORE INTO collections (name, position, created_at)
         SELECT name, ROW_NUMBER() OVER (ORDER BY name), ?1
         FROM (SELECT DISTINCT trim(category) AS name FROM clips WHERE COALESCE(trim(category), '') <> '')",
        params![now],
    )?;
    tx.execute_batch(
        "CREATE TABLE clips_v3 (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            kind TEXT NOT NULL,
            content TEXT NOT NULL,
            preview TEXT NOT NULL,
            language TEXT,
            tags TEXT NOT NULL DEFAULT '[]',
            pinned INTEGER NOT NULL DEFAULT 0,
            collection_id INTEGER REFERENCES collections(id) ON DELETE SET NULL,
            sensitive INTEGER NOT NULL DEFAULT 0,
            has_rich INTEGER NOT NULL DEFAULT 0,
            ocr_text TEXT,
            source_app TEXT,
            size_bytes INTEGER NOT NULL DEFAULT 0,
            hash TEXT NOT NULL UNIQUE,
            created_at TEXT NOT NULL,
            used_at TEXT NOT NULL,
            use_count INTEGER NOT NULL DEFAULT 1
         );
         INSERT INTO clips_v3 (id, kind, content, preview, language, tags, pinned, collection_id,
                source_app, size_bytes, hash, created_at, used_at, use_count)
         SELECT id, kind, content, preview, language, tags, (pinned OR favorite),
                (SELECT c.id FROM collections c WHERE c.name = trim(clips.category)),
                source_app, size_bytes, hash, created_at, used_at, use_count
         FROM clips;
         DROP TABLE clips;
         ALTER TABLE clips_v3 RENAME TO clips;",
    )?;
    tx.execute_batch(SCHEMA)?;

    let texts: Vec<(i64, String)> = {
        let mut stmt =
            tx.prepare("SELECT id, content FROM clips WHERE kind IN ('text', 'code', 'url')")?;
        let rows = stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?;
        rows.collect::<rusqlite::Result<_>>()?
    };
    for (id, content) in texts {
        if crate::sensitive::detect(&content).is_some() {
            tx.execute("UPDATE clips SET sensitive = 1 WHERE id = ?1", params![id])?;
        }
    }
    tx.execute_batch(SCHEMA_FTS)?;
    tx.execute_batch(&format!(
        "INSERT INTO clips_fts (rowid, content, ocr_text, tags)
         SELECT id, {FTS_CONTENT_OF_ROW}, COALESCE(ocr_text, ''), tags FROM clips;"
    ))?;
    Ok(())
}

/// v3 -> v4: version 1 filed copied paths (plain text such as `C:\dossier`)
/// as files, and prefixed file previews with an emoji. Paths become text
/// again and file previews are rebuilt.
fn fix_file_clips(tx: &rusqlite::Transaction) -> Result<()> {
    use crate::clipboard::{classify, files_preview, hash_text, make_preview};
    let rows: Vec<(i64, String, String)> = {
        let mut stmt = tx.prepare("SELECT id, content, preview FROM clips WHERE kind = 'file'")?;
        let rows = stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?;
        rows.collect::<rusqlite::Result<_>>()?
    };
    for (id, content, preview) in rows {
        if let Ok(paths) = serde_json::from_str::<Vec<String>>(&content) {
            let fixed = files_preview(&paths);
            if fixed != preview {
                tx.execute(
                    "UPDATE clips SET preview = ?1 WHERE id = ?2",
                    params![fixed, id],
                )?;
            }
            continue;
        }
        let hash = hash_text(&content);
        let duplicate = tx
            .query_row(
                "SELECT 1 FROM clips WHERE hash = ?1 AND id <> ?2",
                params![hash, id],
                |_| Ok(()),
            )
            .optional()?
            .is_some();
        if duplicate {
            tx.execute("DELETE FROM clips WHERE id = ?1", params![id])?;
            continue;
        }
        let (kind, language) = classify(&content);
        tx.execute(
            "UPDATE clips SET kind = ?1, language = ?2, preview = ?3, hash = ?4, sensitive = ?5
             WHERE id = ?6",
            params![
                kind,
                language,
                make_preview(&content),
                hash,
                crate::sensitive::detect(&content).is_some(),
                id
            ],
        )?;
    }
    Ok(())
}

/// The collections table on its own: the v2 -> v3 migration needs it before
/// the clips table is rebuilt.
macro_rules! collections_table {
    () => {
        "
CREATE TABLE IF NOT EXISTS collections (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL UNIQUE COLLATE NOCASE,
    position INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL
);
"
    };
}

const SCHEMA_COLLECTIONS: &str = collections_table!();

const SCHEMA: &str = concat!(
    collections_table!(),
    "
CREATE TABLE IF NOT EXISTS clips (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    kind TEXT NOT NULL,
    content TEXT NOT NULL,
    preview TEXT NOT NULL,
    language TEXT,
    tags TEXT NOT NULL DEFAULT '[]',
    pinned INTEGER NOT NULL DEFAULT 0,
    collection_id INTEGER REFERENCES collections(id) ON DELETE SET NULL,
    sensitive INTEGER NOT NULL DEFAULT 0,
    has_rich INTEGER NOT NULL DEFAULT 0,
    ocr_text TEXT,
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
CREATE INDEX IF NOT EXISTS idx_clips_collection ON clips(collection_id);
CREATE INDEX IF NOT EXISTS idx_clips_app ON clips(source_app);
CREATE TABLE IF NOT EXISTS clip_formats (
    clip_id INTEGER NOT NULL REFERENCES clips(id) ON DELETE CASCADE,
    format TEXT NOT NULL,
    data BLOB NOT NULL,
    PRIMARY KEY (clip_id, format)
) WITHOUT ROWID;
CREATE TABLE IF NOT EXISTS snippets (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    title TEXT NOT NULL,
    abbreviation TEXT UNIQUE COLLATE NOCASE,
    content TEXT NOT NULL,
    use_count INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS settings (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
);
"
);

/// What the search index stores for a row: never the text of a secret, never
/// an image file name.
const FTS_CONTENT_OF_ROW: &str = "CASE WHEN sensitive OR kind = 'image' THEN '' ELSE content END";

// Contentless index: it keeps only tokens, so it cannot leak a secret even
// if rebuilt, and rows are deleted by id.
const SCHEMA_FTS: &str = "
CREATE VIRTUAL TABLE IF NOT EXISTS clips_fts USING fts5(
    content, ocr_text, tags,
    content='', contentless_delete=1,
    tokenize='unicode61 remove_diacritics 2'
);
CREATE TRIGGER IF NOT EXISTS clips_ai AFTER INSERT ON clips BEGIN
    INSERT INTO clips_fts (rowid, content, ocr_text, tags) VALUES (new.id,
        CASE WHEN new.sensitive OR new.kind = 'image' THEN '' ELSE new.content END,
        COALESCE(new.ocr_text, ''), new.tags);
END;
CREATE TRIGGER IF NOT EXISTS clips_ad AFTER DELETE ON clips BEGIN
    DELETE FROM clips_fts WHERE rowid = old.id;
END;
CREATE TRIGGER IF NOT EXISTS clips_au AFTER UPDATE OF kind, content, ocr_text, tags, sensitive ON clips BEGIN
    DELETE FROM clips_fts WHERE rowid = old.id;
    INSERT INTO clips_fts (rowid, content, ocr_text, tags) VALUES (new.id,
        CASE WHEN new.sensitive OR new.kind = 'image' THEN '' ELSE new.content END,
        COALESCE(new.ocr_text, ''), new.tags);
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

/// Resolve a range token (today, yesterday, week, month or a YYYY-MM-DD
/// day) to UTC bounds based on local calendar days.
fn time_range_bounds(range: &str) -> Option<(String, String)> {
    use chrono::Duration;
    let today = chrono::Local::now().date_naive();
    let tomorrow = today + Duration::days(1);
    let (from, to) = match range {
        "today" => (today, tomorrow),
        "yesterday" => (today - Duration::days(1), today),
        "week" => (today - Duration::days(6), tomorrow),
        "month" => (today - Duration::days(29), tomorrow),
        "" => return None,
        day => {
            let d = chrono::NaiveDate::parse_from_str(day, "%Y-%m-%d").ok()?;
            (d, d + Duration::days(1))
        }
    };
    Some((start_of_local_day(from), start_of_local_day(to)))
}
