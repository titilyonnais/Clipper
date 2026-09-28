use super::*;
use crate::models::{ListParams, Snippet};

fn temp_dir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "clipper-{tag}-{}-{}",
        std::process::id(),
        Utc::now().timestamp_nanos_opt().unwrap_or(0)
    ));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn temp_db() -> (Db, PathBuf) {
    let dir = temp_dir("test");
    (Db::open(&dir).unwrap(), dir)
}

fn text(db: &Db, s: &str) -> i64 {
    add(db, s, false, &RichFormats::default())
}

fn add(db: &Db, s: &str, sensitive: bool, rich: &RichFormats) -> i64 {
    db.upsert_clip(&NewClip {
        kind: "text",
        content: s,
        preview: s,
        language: None,
        source_app: Some("notepad.exe"),
        size_bytes: s.len() as i64,
        hash: &crate::clipboard::hash_text(s),
        sensitive,
        rich,
    })
    .unwrap()
}

fn search(db: &Db, q: &str) -> Vec<i64> {
    db.list(&ListParams {
        query: Some(q.into()),
        ..Default::default()
    })
    .unwrap()
    .into_iter()
    .map(|c| c.id)
    .collect()
}

#[test]
fn fts_query_escapes_and_skips_punctuation() {
    assert_eq!(fts_query("  "), None);
    assert_eq!(fts_query("*** ()"), None);
    assert_eq!(fts_query("foo \"bar"), Some("\"foo\"* \"\"\"bar\"*".into()));
}

#[test]
fn upsert_dedupes_and_search_ignores_accents() {
    let (db, dir) = temp_db();
    let a = text(&db, "Café crème à emporter");
    let b = text(&db, "Café crème à emporter");
    assert_eq!(a, b);
    assert_eq!(db.get(a).unwrap().unwrap().use_count, 2);
    assert_eq!(search(&db, "cafe"), vec![a]);
    assert_eq!(
        search(&db, "\"(").len(),
        1,
        "unsearchable query means no filter"
    );
    assert!(db.list(&ListParams::default()).unwrap()[0]
        .content
        .is_none());
    std::fs::remove_dir_all(dir).ok();
}

#[test]
fn secrets_are_never_indexed() {
    let (db, dir) = temp_db();
    let id = add(&db, "motdepasse=Hunter2xyz", true, &RichFormats::default());
    assert!(search(&db, "Hunter2xyz").is_empty());
    db.set_sensitive(id, false).unwrap();
    assert_eq!(search(&db, "Hunter2xyz"), vec![id]);
    db.set_sensitive(id, true).unwrap();
    assert!(search(&db, "Hunter2xyz").is_empty());
    std::fs::remove_dir_all(dir).ok();
}

#[test]
fn ocr_text_is_searchable() {
    let (db, dir) = temp_db();
    let file = db.store_image_file(b"png").unwrap();
    let id = db
        .upsert_clip(&NewClip {
            kind: "image",
            content: &file,
            preview: "1×1",
            language: None,
            source_app: None,
            size_bytes: 3,
            hash: "i:x",
            sensitive: false,
            rich: &RichFormats::default(),
        })
        .unwrap();
    assert_eq!(db.images_without_ocr(10).unwrap().len(), 1);
    assert!(
        search(&db, "facture").is_empty(),
        "file name is not indexed"
    );
    db.set_ocr_text(id, "Facture n° 42").unwrap();
    assert_eq!(search(&db, "facture"), vec![id]);
    assert!(db.images_without_ocr(10).unwrap().is_empty());
    std::fs::remove_dir_all(dir).ok();
}

#[test]
fn rich_formats_follow_their_clip() {
    let (db, dir) = temp_db();
    let rich = RichFormats {
        html: Some(b"<b>gras</b>".to_vec()),
        rtf: None,
    };
    let id = add(&db, "gras", false, &rich);
    assert!(db.get(id).unwrap().unwrap().has_rich);
    assert_eq!(
        db.rich_formats(id).unwrap().html.as_deref(),
        Some(&b"<b>gras</b>"[..])
    );
    db.delete(&[id]).unwrap();
    let left: i64 = db
        .conn
        .lock()
        .query_row("SELECT COUNT(*) FROM clip_formats", [], |r| r.get(0))
        .unwrap();
    assert_eq!(left, 0, "deleted with the clip");
    std::fs::remove_dir_all(dir).ok();
}

#[test]
fn editing_text_merges_duplicates() {
    let (db, dir) = temp_db();
    let a = add(
        &db,
        "brouillon",
        false,
        &RichFormats {
            html: Some(b"x".to_vec()),
            rtf: None,
        },
    );
    let b = text(&db, "final");
    assert_eq!(db.update_text(a, "brouillon v2", false).unwrap(), a);
    assert!(
        !db.get(a).unwrap().unwrap().has_rich,
        "stale formatting dropped"
    );
    db.set_pinned(a, true).unwrap();
    db.update_tags(a, &["travail".to_string()]).unwrap();
    db.update_tags(b, &["perso".to_string()]).unwrap();
    assert_eq!(db.update_text(a, "final", false).unwrap(), b);
    assert!(db.get(a).unwrap().is_none());
    let merged = db.get(b).unwrap().unwrap();
    assert_eq!(merged.use_count, 2);
    assert!(merged.pinned, "the pin of the edited clip is kept");
    let mut tags = merged.tags.clone();
    tags.sort();
    assert_eq!(tags, vec!["perso", "travail"]);
    std::fs::remove_dir_all(dir).ok();
}

#[test]
fn collections_group_and_protect_clips() {
    let (db, dir) = temp_db();
    let work = db.create_collection("Travail").unwrap();
    assert_eq!(
        db.create_collection(" travail ").unwrap(),
        work,
        "names are case-insensitive"
    );
    assert!(db.create_collection("   ").is_err());
    let filed = text(&db, "rangé");
    db.set_collection(filed, Some(work)).unwrap();
    std::thread::sleep(std::time::Duration::from_millis(5));
    text(&db, "récent");
    assert_eq!(db.enforce_limit(1).unwrap(), 0, "filed clips are kept");
    assert_eq!(db.collections().unwrap()[0].count, 1);
    let listed = db
        .list(&ListParams {
            collection_id: Some(work),
            ..Default::default()
        })
        .unwrap();
    assert_eq!(listed.len(), 1);
    let other = db.create_collection("Perso").unwrap();
    assert!(db.rename_collection(other, "TRAVAIL").is_err());
    db.delete_collection(work).unwrap();
    assert_eq!(db.get(filed).unwrap().unwrap().collection_id, None);
    assert_eq!(db.clear_history().unwrap(), 2);
    std::fs::remove_dir_all(dir).ok();
}

#[test]
fn snippets_crud_and_search() {
    let (db, dir) = temp_db();
    let mut s = Snippet {
        id: 0,
        title: "Signature".into(),
        abbreviation: Some(";sig".into()),
        content: "Cordialement,\nJean".into(),
        use_count: 0,
        updated_at: String::new(),
    };
    let id = db.save_snippet(&s).unwrap();
    assert!(db
        .save_snippet(&Snippet {
            id: 0,
            title: "Autre".into(),
            abbreviation: Some(";SIG".into()),
            content: "x".into(),
            use_count: 0,
            updated_at: String::new(),
        })
        .is_err());
    assert!(db
        .save_snippet(&Snippet {
            abbreviation: Some("deux mots".into()),
            ..s.clone()
        })
        .is_err());
    assert_eq!(db.snippets(Some(";sig")).unwrap()[0].id, id);
    assert_eq!(db.snippets(Some("cordialement")).unwrap().len(), 1);
    assert_eq!(db.snippets(None).unwrap().len(), 1);
    s.id = id;
    s.title = "Signature pro".into();
    db.save_snippet(&s).unwrap();
    assert_eq!(db.snippet(id).unwrap().title, "Signature pro");
    db.bump_snippet(id).unwrap();
    db.delete_snippet(id).unwrap();
    assert!(db.snippets(None).unwrap().is_empty());
    std::fs::remove_dir_all(dir).ok();
}

#[test]
fn tags_are_normalized() {
    let (db, dir) = temp_db();
    let id = text(&db, "hello");
    db.update_tags(
        id,
        &["#work".into(), "work".into(), " api ".into(), "".into()],
    )
    .unwrap();
    assert_eq!(db.get(id).unwrap().unwrap().tags, vec!["api", "work"]);
    std::fs::remove_dir_all(dir).ok();
}

#[test]
fn limit_keeps_pinned_and_removes_image_files() {
    let (db, dir) = temp_db();
    let pinned = text(&db, "keep me");
    db.set_pinned(pinned, true).unwrap();
    let file = db.store_image_file(b"not really a png").unwrap();
    db.upsert_clip(&NewClip {
        kind: "image",
        content: &file,
        preview: "img",
        language: None,
        source_app: None,
        size_bytes: 16,
        hash: "i:1",
        sensitive: false,
        rich: &RichFormats::default(),
    })
    .unwrap();
    std::thread::sleep(std::time::Duration::from_millis(5));
    text(&db, "newest");
    assert_eq!(db.enforce_limit(1).unwrap(), 1);
    assert!(!db.image_path(&file).exists());
    assert_eq!(db.stats().unwrap().total, 2);
    assert_eq!(db.source_apps().unwrap()[0].name, "notepad.exe");
    std::fs::remove_dir_all(dir).ok();
}

#[test]
fn last_deletion_can_be_undone() {
    let (db, dir) = temp_db();
    let rich = RichFormats {
        html: Some(b"<i>x</i>".to_vec()),
        rtf: None,
    };
    let a = add(&db, "premier", false, &rich);
    let b = text(&db, "second");
    db.set_pinned(a, true).unwrap();
    assert_eq!(db.delete(&[a, b]).unwrap(), 2);
    assert!(db.list(&ListParams::default()).unwrap().is_empty());
    assert!(search(&db, "premier").is_empty());

    assert_eq!(db.undo_delete().unwrap(), 2);
    let back = db.get(a).unwrap().unwrap();
    assert!(back.pinned && back.has_rich, "restored as it was");
    assert!(db.rich_formats(a).unwrap().html.is_some());
    assert_eq!(search(&db, "premier"), vec![a]);
    assert_eq!(db.undo_delete().unwrap(), 0, "only once");

    // A new deletion makes the previous one final.
    db.delete(&[a]).unwrap();
    db.delete(&[b]).unwrap();
    assert_eq!(db.undo_delete().unwrap(), 1);
    assert!(db.get(a).unwrap().is_none());
    std::fs::remove_dir_all(dir).ok();
}

#[test]
fn backup_and_restore_round_trip() {
    let (db, dir) = temp_db();
    let keep = text(&db, "avant");
    let backup = dir.join("b.db");
    db.backup_to(&backup).unwrap();
    text(&db, "après");
    db.delete(&[keep]).unwrap();
    db.restore_from(&backup).unwrap();
    let texts: Vec<String> = db
        .list(&ListParams::default())
        .unwrap()
        .into_iter()
        .map(|c| c.preview)
        .collect();
    assert_eq!(texts, vec!["avant"]);
    std::fs::remove_dir_all(dir).ok();
}

const V2_SCHEMA: &str = "
CREATE TABLE clips (id INTEGER PRIMARY KEY AUTOINCREMENT, kind TEXT NOT NULL,
    content TEXT NOT NULL, preview TEXT NOT NULL, language TEXT, category TEXT,
    tags TEXT NOT NULL DEFAULT '[]', pinned INTEGER NOT NULL DEFAULT 0,
    favorite INTEGER NOT NULL DEFAULT 0, source_app TEXT,
    size_bytes INTEGER NOT NULL DEFAULT 0, hash TEXT NOT NULL UNIQUE,
    created_at TEXT NOT NULL, used_at TEXT NOT NULL, use_count INTEGER NOT NULL DEFAULT 1);
CREATE INDEX idx_clips_category ON clips(category);
CREATE VIRTUAL TABLE clips_fts USING fts5(content, category, tags, content='clips', content_rowid='id');
CREATE TABLE settings (key TEXT PRIMARY KEY, value TEXT NOT NULL);
";

#[test]
fn migrates_v2_database() {
    let dir = temp_dir("v2");
    {
        let c = Connection::open(dir.join("clipper.db")).unwrap();
        c.execute_batch(V2_SCHEMA).unwrap();
        c.execute_batch(
            "INSERT INTO clips (kind, content, preview, category, favorite, hash, created_at, used_at) VALUES
                ('text', 'rapport trimestriel', 'rapport', ' Travail ', 1, 't:1', '2026-01-01T10:00:00.000Z', '2026-01-02T10:00:00.000Z'),
                ('text', 'AKIAIOSFODNN7EXAMPLE', 'AKIA', NULL, 0, 't:2', '2026-01-01T10:00:00.000Z', '2026-01-01T10:00:00.000Z'),
                ('code', 'SELECT 1 FROM t', 'SELECT', 'Travail', 0, 't:3', '2026-01-01T10:00:00.000Z', '2026-01-01T10:00:00.000Z');
             INSERT INTO settings VALUES ('app', '{\"theme\":\"dark\",\"accent_color\":\"#f59e0b\",\"keep_favorites\":true}');
             PRAGMA user_version = 2;",
        )
        .unwrap();
    }
    let db = Db::open(&dir).unwrap();
    let cols = db.collections().unwrap();
    assert_eq!(cols.len(), 1);
    assert_eq!((cols[0].name.as_str(), cols[0].count), ("Travail", 2));
    let all = db.list(&ListParams::default()).unwrap();
    assert_eq!(all.len(), 3);
    let report = all.iter().find(|c| c.preview == "rapport").unwrap();
    assert!(report.pinned, "favourites become pins");
    assert!(all.iter().find(|c| c.preview == "AKIA").unwrap().sensitive);
    assert!(search(&db, "AKIAIOSFODNN7EXAMPLE").is_empty());
    assert_eq!(search(&db, "trimestriel"), vec![report.id]);
    assert_eq!(db.get_settings().unwrap().theme, "dark", "settings survive");
    let version: i64 = db
        .conn
        .lock()
        .query_row("PRAGMA user_version", [], |r| r.get(0))
        .unwrap();
    assert_eq!(version, SCHEMA_VERSION);
    drop(db);
    std::fs::remove_dir_all(dir).ok();
}

#[test]
fn migrates_v1_database() {
    let dir = temp_dir("v1");
    {
        let c = Connection::open(dir.join("clipper.db")).unwrap();
        c.execute_batch(V2_SCHEMA).unwrap();
        c.execute_batch(
            "INSERT INTO clips (kind, content, preview, hash, created_at, used_at) VALUES
                ('text', 'bonjour le monde', 'bonjour', 't:1',
                 '2026-01-01T10:00:00.123456+00:00', '2026-01-02T10:00:00+00:00'),
                ('image', 'iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNk+M9QDwADhgGAWjR9awAAAABJRU5ErkJggg==',
                 '1×1 · 1 KB', 'i:1', '2026-01-01T10:00:00+00:00', '2026-01-01T10:00:00+00:00'),
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
    assert_eq!(img.preview, "1×1");
    let text = clips.iter().find(|c| c.kind == "text").unwrap();
    assert_eq!(text.used_at, "2026-01-02T10:00:00.000Z");
    assert_eq!(search(&db, "monde").len(), 1);
    assert_eq!(
        db.take_legacy_api_keys().unwrap(),
        vec![("openai", "sk-test".to_string())]
    );
    assert!(db.take_legacy_api_keys().unwrap().is_empty());
    drop(db);
    std::fs::remove_dir_all(dir).ok();
}

#[test]
fn legacy_paths_become_text() {
    let dir = temp_dir("v3");
    {
        let db = Db::open(&dir).unwrap();
        text(&db, "D:\\déjà là");
        db.conn
            .lock()
            .execute_batch(
                "INSERT INTO clips (kind, content, preview, hash, created_at, used_at) VALUES
                    ('file', 'C:\\Users\\moi\\Projet', 'C:\\Users\\moi\\Projet', 'f:1', '2026-01-01T10:00:00.000Z', '2026-01-01T10:00:00.000Z'),
                    ('file', 'D:\\déjà là', 'D:\\déjà là', 'f:2', '2026-01-01T10:00:00.000Z', '2026-01-01T10:00:00.000Z'),
                    ('file', '[\"C:\\\\a\\\\rapport.pdf\"]', '📄 rapport.pdf', 'f:3', '2026-01-01T10:00:00.000Z', '2026-01-01T10:00:00.000Z');
                 PRAGMA user_version = 3;",
            )
            .unwrap();
    }
    let db = Db::open(&dir).unwrap();
    let clips = db.list(&ListParams::default()).unwrap();
    assert_eq!(
        clips.len(),
        3,
        "a path already in the history is not duplicated"
    );
    let path = clips
        .iter()
        .find(|c| c.preview == "C:\\Users\\moi\\Projet")
        .unwrap();
    assert_eq!(path.kind, "text");
    assert_eq!(search(&db, "Projet"), vec![path.id]);
    let file = clips.iter().find(|c| c.kind == "file").unwrap();
    assert_eq!(file.preview, "rapport.pdf");
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
        "total={} text={} code={} url={} file={} image={} pinned={} sensitive={} disk={} MB",
        s.total,
        s.text,
        s.code,
        s.url,
        s.file,
        s.image,
        s.pinned,
        s.sensitive,
        s.disk_bytes / 1_048_576
    );
    println!("collections: {:?}", db.collections().unwrap());
    let start = std::time::Instant::now();
    let page = db.list(&ListParams::default()).unwrap();
    println!("first page ({} rows): {:?}", page.len(), start.elapsed());
    let start = std::time::Instant::now();
    let found = search(&db, "http");
    println!("search ({} rows): {:?}", found.len(), start.elapsed());
}
