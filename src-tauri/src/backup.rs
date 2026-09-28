//! Daily database backups (`backups/clipper-YYYY-MM-DD.db`), 7 kept.
//! Images are content-addressed files shared with the live database, so
//! only the database itself is copied.

use crate::db::Db;
use crate::models::BackupInfo;
use std::path::{Path, PathBuf};

const KEEP: usize = 7;
const PREFIX: &str = "clipper-";

pub fn dir(db: &Db) -> PathBuf {
    db.data_dir().join("backups")
}

fn is_backup_name(name: &str) -> bool {
    name.starts_with(PREFIX)
        && name.ends_with(".db")
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
    out.sort_by(|a, b| b.name.cmp(&a.name));
    out
}

/// Back up today's database unless it is already done, then prune.
pub fn run_daily(db: &Db) -> anyhow::Result<bool> {
    let dir = dir(db);
    std::fs::create_dir_all(&dir)?;
    let name = format!("{PREFIX}{}.db", chrono::Local::now().format("%Y-%m-%d"));
    let dest = dir.join(&name);
    if dest.exists() {
        return Ok(false);
    }
    db.backup_to(&dest)?;
    for old in list(db).into_iter().skip(KEEP) {
        let _ = std::fs::remove_file(dir.join(old.name));
    }
    Ok(true)
}

pub fn restore(db: &Db, name: &str) -> Result<(), String> {
    if !is_backup_name(name) {
        return Err("Sauvegarde invalide.".into());
    }
    let src: PathBuf = dir(db).join(name);
    if !Path::new(&src).exists() {
        return Err("Sauvegarde introuvable.".into());
    }
    // Keep the current state, in case the restore is a mistake.
    let safety = dir(db).join(format!(
        "{PREFIX}{}-avant-restauration.db",
        chrono::Local::now().format("%Y-%m-%d-%H%M%S")
    ));
    db.backup_to(&safety).map_err(|e| e.to_string())?;
    db.restore_from(&src).map_err(|e| e.to_string())
}
