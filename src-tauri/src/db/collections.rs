use super::{now, Db};
use crate::models::Collection;
use anyhow::{anyhow, Result};
use rusqlite::{params, OptionalExtension};

fn clean_name(name: &str) -> Result<String> {
    let name: String = name.trim().chars().take(48).collect();
    if name.is_empty() {
        return Err(anyhow!("Le nom ne peut pas être vide."));
    }
    Ok(name)
}

impl Db {
    pub fn collections(&self) -> Result<Vec<Collection>> {
        let c = self.conn.lock();
        let mut stmt = c.prepare_cached(
            "SELECT c.id, c.name, (SELECT COUNT(*) FROM clips WHERE collection_id = c.id)
             FROM collections c ORDER BY c.position, c.name",
        )?;
        let rows = stmt.query_map([], |r| {
            Ok(Collection {
                id: r.get(0)?,
                name: r.get(1)?,
                count: r.get(2)?,
            })
        })?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    pub fn create_collection(&self, name: &str) -> Result<i64> {
        let name = clean_name(name)?;
        let c = self.conn.lock();
        if let Some(id) = c
            .query_row(
                "SELECT id FROM collections WHERE name = ?1",
                params![name],
                |r| r.get(0),
            )
            .optional()?
        {
            return Ok(id);
        }
        c.execute(
            "INSERT INTO collections (name, position, created_at)
             VALUES (?1, (SELECT COALESCE(MAX(position), 0) + 1 FROM collections), ?2)",
            params![name, now()],
        )?;
        Ok(c.last_insert_rowid())
    }

    pub fn rename_collection(&self, id: i64, name: &str) -> Result<()> {
        let name = clean_name(name)?;
        self.conn
            .lock()
            .execute(
                "UPDATE collections SET name = ?1 WHERE id = ?2",
                params![name, id],
            )
            .map_err(|e| match e {
                rusqlite::Error::SqliteFailure(f, _)
                    if f.code == rusqlite::ErrorCode::ConstraintViolation =>
                {
                    anyhow!("Une collection porte déjà ce nom.")
                }
                e => e.into(),
            })?;
        Ok(())
    }

    /// Delete a collection. Its clips stay in the history as ordinary clips
    /// (`ON DELETE SET NULL`), subject to retention again unless pinned.
    pub fn delete_collection(&self, id: i64) -> Result<()> {
        self.conn
            .lock()
            .execute("DELETE FROM collections WHERE id = ?1", params![id])?;
        Ok(())
    }
}
