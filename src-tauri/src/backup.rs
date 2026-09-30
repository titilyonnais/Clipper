//! Daily database backups (`backups/clipper-YYYY-MM-DD.db.gz`), 7 kept.
//! Images are content-addressed files shared with the live database, so
//! only the database itself is copied. Backups are only read to restore
//! one: they are stored compressed (about a third of the size).

use crate::db::Db;
use crate::models::BackupInfo;
use flate2::{read::GzDecoder, write::GzEncoder, Compression};
use std::fs::File;
use std::io::{BufReader, BufWriter, Write};
use std::path::{Path, PathBuf};

const KEEP: usize = 7;
const PREFIX: &str = "clipper-";
const PACKED: &str = ".db.gz";
/// Uncompressed backups written by versions before 3.4.
const PLAIN: &str = ".db";

pub fn dir(db: &Db) -> PathBuf {
    db.data_dir().join("backups")
}

fn is_backup_name(name: &str) -> bool {
    name.starts_with(PREFIX)
        && (name.ends_with(PACKED) || name.ends_with(PLAIN))
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '.')
}

pub fn list(db: &Db) -> Vec<BackupInfo> {
    let mut out: Vec<BackupInfo> = std::fs::read_dir(dir(db))
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().into_owned();
            let meta = e.metadata().ok()?;
            is_backup_name(&name).then(|| BackupInfo {
                created_at: meta
                    .modified()
                    .ok()
                    .map(|t| chrono::DateTime::<chrono::Utc>::from(t).to_rfc3339())
                    .unwrap_or_default(),
                size: meta.len(),
                name,
            })
        })
        .collect();
    out.sort_by(|a, b| b.created_at.cmp(&a.created_at).then_with(|| b.name.cmp(&a.name)));
    out
}

/// Back up today's database unless it is already done.
pub fn run_daily(db: &Db) -> anyhow::Result<bool> {
    compress_old(db);
    let stem = format!("{PREFIX}{}", chrono::Local::now().format("%Y-%m-%d"));
    let dir = dir(db);
    if dir.join(format!("{stem}{PACKED}")).exists() || dir.join(format!("{stem}{PLAIN}")).exists() {
        return Ok(false);
    }
    write(db, &stem)?;
    Ok(true)
}

/// Back up now (from the settings), whatever the day's backup.
pub fn run_now(db: &Db) -> anyhow::Result<()> {
    write(db, &format!("{PREFIX}{}", chrono::Local::now().format("%Y-%m-%d-%H%M%S")))
}

/// Write a compressed backup named `stem`, then keep only the most recent ones.
fn write(db: &Db, stem: &str) -> anyhow::Result<()> {
    let dir = dir(db);
    std::fs::create_dir_all(&dir)?;
    let copy = dir.join(format!("{stem}.tmp"));
    db.backup_to(&copy)?;
    let packed = gzip(&copy, &dir.join(format!("{stem}{PACKED}")));
    let _ = std::fs::remove_file(&copy);
    packed?;
    for old in list(db).into_iter().skip(KEEP) {
        let _ = std::fs::remove_file(dir.join(old.name));
    }
    Ok(())
}

/// Compress `src` into `dest`, which appears only once complete.
fn gzip(src: &Path, dest: &Path) -> std::io::Result<()> {
    let part = dest.with_extension("part");
    let result = (|| {
        let mut input = BufReader::new(File::open(src)?);
        let mut out = GzEncoder::new(BufWriter::new(File::create(&part)?), Compression::best());
        std::io::copy(&mut input, &mut out)?;
        out.finish()?.flush()?;
        std::fs::rename(&part, dest)
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(&part);
    }
    result
}

/// Compress the backups of earlier versions, keeping their date.
fn compress_old(db: &Db) {
    let dir = dir(db);
    for b in list(db).into_iter().filter(|b| !b.name.ends_with(PACKED)) {
        let src = dir.join(&b.name);
        let dest = dir.join(format!("{}{PACKED}", b.name.trim_end_matches(PLAIN)));
        let modified = std::fs::metadata(&src).and_then(|m| m.modified());
        if gzip(&src, &dest).is_ok() {
            if let (Ok(time), Ok(file)) = (modified, File::options().write(true).open(&dest)) {
                let _ = file.set_modified(time);
            }
            let _ = std::fs::remove_file(&src);
        }
    }
}

pub fn restore(db: &Db, name: &str) -> Result<(), String> {
    if !is_backup_name(name) {
        return Err("Sauvegarde invalide.".into());
    }
    let dir = dir(db);
    let src = dir.join(name);
    if !src.exists() {
        return Err("Sauvegarde introuvable.".into());
    }
    // Keep the current state, in case the restore is a mistake.
    write(
        db,
        &format!("{PREFIX}{}-avant-restauration", chrono::Local::now().format("%Y-%m-%d-%H%M%S")),
    )
    .map_err(|e| e.to_string())?;
    if !name.ends_with(PACKED) {
        return db.restore_from(&src).map_err(|e| e.to_string());
    }
    let plain = dir.join("restauration.tmp");
    let unpacked = (|| {
        let mut input = GzDecoder::new(BufReader::new(File::open(&src)?));
        let mut out = BufWriter::new(File::create(&plain)?);
        std::io::copy(&mut input, &mut out)?;
        out.flush()
    })();
    let result = unpacked
        .map_err(|e| format!("Sauvegarde illisible : {e}"))
        .and_then(|()| db.restore_from(&plain).map_err(|e| e.to_string()));
    let _ = std::fs::remove_file(&plain);
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compressed_backup_restores() {
        let data = std::env::temp_dir().join(format!("clipper-backup-{}", std::process::id()));
        let db = Db::open(&data).unwrap();
        let mut settings = db.get_settings().unwrap();
        settings.max_items = 1234;
        db.set_settings(&settings).unwrap();
        // A backup from an earlier version, uncompressed.
        std::fs::create_dir_all(dir(&db)).unwrap();
        let old = dir(&db).join("clipper-2020-01-01.db");
        db.backup_to(&old).unwrap();
        let date = std::time::SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(1_577_880_000);
        File::options().write(true).open(&old).unwrap().set_modified(date).unwrap();
        run_now(&db).unwrap();
        compress_old(&db);
        let names: Vec<String> = list(&db).into_iter().map(|b| b.name).collect();
        assert_eq!(names.len(), 2);
        assert!(names.iter().all(|n| n.ends_with(PACKED)), "{names:?}");
        let kept = std::fs::metadata(dir(&db).join("clipper-2020-01-01.db.gz")).unwrap();
        assert_eq!(kept.modified().unwrap(), date);

        settings.max_items = 99;
        db.set_settings(&settings).unwrap();
        restore(&db, &names[0]).unwrap();
        assert_eq!(db.get_settings().unwrap().max_items, 1234);
        assert!(!dir(&db).join("restauration.tmp").exists());
        drop(db);
        let _ = std::fs::remove_dir_all(&data);
    }
}
