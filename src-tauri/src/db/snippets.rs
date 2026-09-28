use super::{now, Db};
use crate::models::Snippet;
use anyhow::{anyhow, Result};
use rusqlite::{params, OptionalExtension};

const COLUMNS: &str = "id, title, abbreviation, content, use_count, updated_at";

fn row(r: &rusqlite::Row) -> rusqlite::Result<Snippet> {
    Ok(Snippet {
        id: r.get(0)?,
        title: r.get(1)?,
        abbreviation: r.get(2)?,
        content: r.get(3)?,
        use_count: r.get(4)?,
        updated_at: r.get(5)?,
    })
}

/// Abbreviations are single words, e.g. ";sig".
fn clean_abbreviation(a: Option<&str>) -> Result<Option<String>> {
    let Some(a) = a.map(str::trim).filter(|a| !a.is_empty()) else {
        return Ok(None);
    };
    if a.chars().any(char::is_whitespace) || a.chars().count() > 24 {
        return Err(anyhow!(
            "L'abréviation doit tenir en un mot de 24 caractères au plus."
        ));
    }
    Ok(Some(a.to_string()))
}

impl Db {
    /// Snippets matching `query` (title, abbreviation or text), most used first.
    pub fn snippets(&self, query: Option<&str>) -> Result<Vec<Snippet>> {
        let c = self.conn.lock();
        let q = query.map(str::trim).filter(|q| !q.is_empty());
        let mut stmt = c.prepare_cached(&format!(
            "SELECT {COLUMNS} FROM snippets
             WHERE ?1 IS NULL
                OR abbreviation = ?1
                OR title LIKE '%' || ?1 || '%'
                OR content LIKE '%' || ?1 || '%'
             ORDER BY (abbreviation = ?1) DESC, use_count DESC, title"
        ))?;
        let rows = stmt.query_map(params![q], row)?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    pub fn snippet(&self, id: i64) -> Result<Snippet> {
        self.conn
            .lock()
            .query_row(
                &format!("SELECT {COLUMNS} FROM snippets WHERE id = ?1"),
                params![id],
                row,
            )
            .optional()?
            .ok_or_else(|| anyhow!("Snippet introuvable."))
    }

    /// Create (`id == 0`) or update a snippet. Returns its id.
    pub fn save_snippet(&self, s: &Snippet) -> Result<i64> {
        let title: String = s.title.trim().chars().take(80).collect();
        if title.is_empty() {
            return Err(anyhow!("Donnez un titre au snippet."));
        }
        if s.content.is_empty() {
            return Err(anyhow!("Le snippet est vide."));
        }
        let abbreviation = clean_abbreviation(s.abbreviation.as_deref())?;
        let c = self.conn.lock();
        let result = if s.id == 0 {
            c.execute(
                "INSERT INTO snippets (title, abbreviation, content, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?4)",
                params![title, abbreviation, s.content, now()],
            )
            .map(|_| c.last_insert_rowid())
        } else {
            c.execute(
                "UPDATE snippets SET title = ?1, abbreviation = ?2, content = ?3, updated_at = ?4
                 WHERE id = ?5",
                params![title, abbreviation, s.content, now(), s.id],
            )
            .map(|_| s.id)
        };
        result.map_err(|e| match e {
            rusqlite::Error::SqliteFailure(f, _)
                if f.code == rusqlite::ErrorCode::ConstraintViolation =>
            {
                anyhow!("Cette abréviation est déjà utilisée.")
            }
            e => e.into(),
        })
    }

    pub fn delete_snippet(&self, id: i64) -> Result<()> {
        self.conn
            .lock()
            .execute("DELETE FROM snippets WHERE id = ?1", params![id])?;
        Ok(())
    }

    pub fn bump_snippet(&self, id: i64) -> Result<()> {
        self.conn.lock().execute(
            "UPDATE snippets SET use_count = use_count + 1 WHERE id = ?1",
            params![id],
        )?;
        Ok(())
    }
}
