use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use rusqlite::{params, Connection, OptionalExtension, Transaction};
use serde::Serialize;

use super::migrations;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Text,
    Link,
    Image,
}

impl Kind {
    pub fn as_str(self) -> &'static str {
        match self {
            Kind::Text => "text",
            Kind::Link => "link",
            Kind::Image => "image",
        }
    }
}

#[derive(Debug, Clone)]
pub struct NewClip {
    pub kind: Kind,
    pub text: Option<String>,
    pub image_path: Option<String>,
    pub thumb_path: Option<String>,
    pub image_bytes: i64,
    pub hash: String,
}

#[derive(Debug, Clone, Copy)]
pub struct Limits {
    /// Maximum number of unpinned clips kept.
    pub max_items: i64,
    /// Total size budget for stored images (pinned included).
    pub max_image_bytes: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Clip {
    pub id: i64,
    pub kind: String,
    pub text: Option<String>,
    pub image_path: Option<String>,
    pub thumb_path: Option<String>,
    pub pinned: bool,
    pub created_at: i64,
    pub last_used_at: i64,
}

pub struct Db {
    conn: Connection,
}

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

fn row_to_clip(r: &rusqlite::Row) -> rusqlite::Result<Clip> {
    Ok(Clip {
        id: r.get("id")?,
        kind: r.get("kind")?,
        text: r.get("text")?,
        image_path: r.get("image_path")?,
        thumb_path: r.get("thumb_path")?,
        pinned: r.get::<_, i64>("pinned")? != 0,
        created_at: r.get("created_at")?,
        last_used_at: r.get("last_used_at")?,
    })
}

const COLUMNS: &str = "id, kind, text, image_path, thumb_path, pinned, created_at, last_used_at";

/// Deletes the rows and returns the image files that belonged to them.
fn delete_rows(tx: &Transaction, ids: &[i64]) -> rusqlite::Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    for id in ids {
        let paths: Option<(Option<String>, Option<String>)> = tx
            .query_row(
                "SELECT image_path, thumb_path FROM clips WHERE id = ?1",
                [id],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()?;
        if let Some((image, thumb)) = paths {
            files.extend(image.into_iter().chain(thumb).map(PathBuf::from));
        }
        tx.execute("DELETE FROM clips WHERE id = ?1", [id])?;
    }
    Ok(files)
}

fn evict(tx: &Transaction, limits: &Limits) -> rusqlite::Result<Vec<PathBuf>> {
    let mut removed = Vec::new();

    let over_count: Vec<i64> = {
        let mut stmt = tx.prepare(
            "SELECT id FROM clips WHERE pinned = 0
             ORDER BY created_at DESC, id DESC LIMIT -1 OFFSET ?1",
        )?;
        let rows = stmt.query_map([limits.max_items.max(0)], |r| r.get(0))?;
        rows.collect::<Result<_, _>>()?
    };
    removed.extend(delete_rows(tx, &over_count)?);

    loop {
        let total: i64 = tx.query_row(
            "SELECT IFNULL(SUM(image_bytes), 0) FROM clips",
            [],
            |r| r.get(0),
        )?;
        if total <= limits.max_image_bytes {
            break;
        }
        let oldest: Option<i64> = tx
            .query_row(
                "SELECT id FROM clips WHERE pinned = 0 AND kind = 'image'
                 ORDER BY created_at ASC, id ASC LIMIT 1",
                [],
                |r| r.get(0),
            )
            .optional()?;
        match oldest {
            Some(id) => removed.extend(delete_rows(tx, &[id])?),
            None => break,
        }
    }
    Ok(removed)
}

impl Db {
    pub fn open(path: &Path) -> rusqlite::Result<Self> {
        let mut conn = Connection::open(path)?;
        conn.execute_batch("PRAGMA journal_mode = WAL;")?;
        migrations::run(&mut conn)?;
        Ok(Self { conn })
    }

    #[cfg(test)]
    pub fn open_in_memory() -> rusqlite::Result<Self> {
        let mut conn = Connection::open_in_memory()?;
        migrations::run(&mut conn)?;
        Ok(Self { conn })
    }

    /// Inserts a clip, or moves an identical one (same hash) to the top.
    /// Returns image files that were evicted and should be deleted from disk.
    pub fn insert_or_touch(
        &mut self,
        clip: &NewClip,
        limits: &Limits,
    ) -> rusqlite::Result<Vec<PathBuf>> {
        let tx = self.conn.transaction()?;
        let top: i64 = tx.query_row(
            "SELECT IFNULL(MAX(created_at), 0) FROM clips",
            [],
            |r| r.get(0),
        )?;
        let ts = now_ms().max(top + 1);
        let updated = tx.execute(
            "UPDATE clips SET created_at = ?1, last_used_at = ?1 WHERE hash = ?2",
            params![ts, clip.hash],
        )?;
        if updated == 0 {
            tx.execute(
                "INSERT INTO clips
                   (kind, text, image_path, thumb_path, image_bytes, hash, created_at, last_used_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?7)",
                params![
                    clip.kind.as_str(),
                    clip.text,
                    clip.image_path,
                    clip.thumb_path,
                    clip.image_bytes,
                    clip.hash,
                    ts
                ],
            )?;
        }
        let removed = evict(&tx, limits)?;
        tx.commit()?;
        Ok(removed)
    }

    /// Re-applies the limits (e.g. after the user lowers them in settings).
    pub fn enforce_limits(&mut self, limits: &Limits) -> rusqlite::Result<Vec<PathBuf>> {
        let tx = self.conn.transaction()?;
        let removed = evict(&tx, limits)?;
        tx.commit()?;
        Ok(removed)
    }

    pub fn touch(&mut self, id: i64) -> rusqlite::Result<()> {
        let ts = now_ms();
        self.conn.execute(
            "UPDATE clips SET last_used_at = ?1,
               created_at = MAX(?1, (SELECT IFNULL(MAX(created_at), 0) + 1 FROM clips))
             WHERE id = ?2",
            params![ts, id],
        )?;
        Ok(())
    }

    pub fn get(&self, id: i64) -> rusqlite::Result<Option<Clip>> {
        self.conn
            .query_row(
                &format!("SELECT {COLUMNS} FROM clips WHERE id = ?1"),
                [id],
                row_to_clip,
            )
            .optional()
    }

    /// Full, untruncated text of a clip (the list only carries a preview).
    pub fn full_text(&self, id: i64) -> rusqlite::Result<Option<String>> {
        self.conn
            .query_row("SELECT text FROM clips WHERE id = ?1", [id], |r| r.get(0))
            .optional()
            .map(|o| o.flatten())
    }

    /// Pinned first, then newest first. The search is a Unicode-aware,
    /// case-insensitive substring match done in Rust (SQLite's LIKE only
    /// folds ASCII, which breaks Turkish characters).
    /// Long texts are cut to `preview_chars` for the list view.
    pub fn list(&self, query: Option<&str>, preview_chars: usize) -> rusqlite::Result<Vec<Clip>> {
        let mut stmt = self.conn.prepare(&format!(
            "SELECT {COLUMNS} FROM clips ORDER BY pinned DESC, created_at DESC, id DESC"
        ))?;
        let rows = stmt.query_map([], row_to_clip)?;
        let needle = query
            .map(|q| q.trim().to_lowercase())
            .filter(|q| !q.is_empty());

        let mut out = Vec::new();
        for row in rows {
            let mut clip = row?;
            if let Some(n) = &needle {
                let hit = clip
                    .text
                    .as_deref()
                    .is_some_and(|t| t.to_lowercase().contains(n.as_str()));
                if !hit {
                    continue;
                }
            }
            if let Some(t) = &clip.text {
                if t.chars().count() > preview_chars {
                    clip.text = Some(t.chars().take(preview_chars).collect());
                }
            }
            out.push(clip);
        }
        Ok(out)
    }

    pub fn set_pinned(&mut self, id: i64, pinned: bool) -> rusqlite::Result<()> {
        self.conn.execute(
            "UPDATE clips SET pinned = ?1 WHERE id = ?2",
            params![pinned as i64, id],
        )?;
        Ok(())
    }

    pub fn delete(&mut self, id: i64) -> rusqlite::Result<Vec<PathBuf>> {
        let tx = self.conn.transaction()?;
        let files = delete_rows(&tx, &[id])?;
        tx.commit()?;
        Ok(files)
    }

    pub fn clear(&mut self, include_pinned: bool) -> rusqlite::Result<Vec<PathBuf>> {
        let tx = self.conn.transaction()?;
        let ids: Vec<i64> = {
            let sql = if include_pinned {
                "SELECT id FROM clips"
            } else {
                "SELECT id FROM clips WHERE pinned = 0"
            };
            let mut stmt = tx.prepare(sql)?;
            let rows = stmt.query_map([], |r| r.get(0))?;
            rows.collect::<Result<_, _>>()?
        };
        let files = delete_rows(&tx, &ids)?;
        tx.commit()?;
        Ok(files)
    }

    /// Deletes only image rows (and their files); text clips are kept.
    pub fn clear_images(&mut self) -> rusqlite::Result<Vec<PathBuf>> {
        let tx = self.conn.transaction()?;
        let ids: Vec<i64> = {
            let mut stmt = tx.prepare("SELECT id FROM clips WHERE kind = 'image'")?;
            let rows = stmt.query_map([], |r| r.get(0))?;
            rows.collect::<Result<_, _>>()?
        };
        let files = delete_rows(&tx, &ids)?;
        tx.commit()?;
        Ok(files)
    }

    /// (total image bytes, image count)
    pub fn image_usage(&self) -> rusqlite::Result<(i64, i64)> {
        self.conn.query_row(
            "SELECT IFNULL(SUM(image_bytes), 0), COUNT(*) FROM clips WHERE kind = 'image'",
            [],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
    }

    pub fn referenced_files(&self) -> rusqlite::Result<HashSet<PathBuf>> {
        let mut stmt = self
            .conn
            .prepare("SELECT image_path, thumb_path FROM clips WHERE kind = 'image'")?;
        let rows = stmt.query_map([], |r| {
            Ok((r.get::<_, Option<String>>(0)?, r.get::<_, Option<String>>(1)?))
        })?;
        let mut set = HashSet::new();
        for row in rows {
            let (a, b) = row?;
            set.extend(a.into_iter().chain(b).map(PathBuf::from));
        }
        Ok(set)
    }

    pub fn get_setting(&self, key: &str) -> rusqlite::Result<Option<String>> {
        self.conn
            .query_row("SELECT value FROM settings WHERE key = ?1", [key], |r| {
                r.get(0)
            })
            .optional()
    }

    pub fn set_setting(&mut self, key: &str, value: &str) -> rusqlite::Result<()> {
        self.conn.execute(
            "INSERT INTO settings (key, value) VALUES (?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![key, value],
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const LIMITS: Limits = Limits {
        max_items: 500,
        max_image_bytes: 1_000,
    };

    fn text(t: &str) -> NewClip {
        NewClip {
            kind: Kind::Text,
            text: Some(t.to_string()),
            image_path: None,
            thumb_path: None,
            image_bytes: 0,
            hash: format!("h-{t}"),
        }
    }

    fn image(name: &str, bytes: i64) -> NewClip {
        NewClip {
            kind: Kind::Image,
            text: None,
            image_path: Some(format!("{name}.png")),
            thumb_path: Some(format!("{name}_t.png")),
            image_bytes: bytes,
            hash: format!("h-{name}"),
        }
    }

    fn texts(db: &Db, q: Option<&str>) -> Vec<String> {
        db.list(q, 1000)
            .unwrap()
            .into_iter()
            .map(|c| c.text.unwrap_or_else(|| c.image_path.unwrap()))
            .collect()
    }

    #[test]
    fn migrations_create_schema_and_survive_reopen() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("t.db");
        {
            let mut db = Db::open(&path).unwrap();
            db.insert_or_touch(&text("kalıcı"), &LIMITS).unwrap();
            assert_eq!(
                migrations::current_version(&db.conn).unwrap(),
                migrations::latest_version()
            );
        }
        let db = Db::open(&path).unwrap();
        assert_eq!(texts(&db, None), vec!["kalıcı"]);
    }

    #[test]
    fn newest_first_and_duplicates_move_to_top() {
        let mut db = Db::open_in_memory().unwrap();
        for t in ["a", "b", "c"] {
            db.insert_or_touch(&text(t), &LIMITS).unwrap();
        }
        assert_eq!(texts(&db, None), vec!["c", "b", "a"]);
        db.insert_or_touch(&text("a"), &LIMITS).unwrap();
        assert_eq!(texts(&db, None), vec!["a", "c", "b"]);
    }

    #[test]
    fn max_items_evicts_oldest_unpinned_only() {
        let mut db = Db::open_in_memory().unwrap();
        let limits = Limits {
            max_items: 2,
            ..LIMITS
        };
        db.insert_or_touch(&text("old-pinned"), &limits).unwrap();
        let id = db.list(None, 100).unwrap()[0].id;
        db.set_pinned(id, true).unwrap();
        for t in ["a", "b", "c"] {
            db.insert_or_touch(&text(t), &limits).unwrap();
        }
        assert_eq!(texts(&db, None), vec!["old-pinned", "c", "b"]);
    }

    #[test]
    fn pinned_items_stay_on_top() {
        let mut db = Db::open_in_memory().unwrap();
        db.insert_or_touch(&text("first"), &LIMITS).unwrap();
        db.insert_or_touch(&text("second"), &LIMITS).unwrap();
        let first = db.list(None, 100).unwrap()[1].id;
        db.set_pinned(first, true).unwrap();
        assert_eq!(texts(&db, None), vec!["first", "second"]);
    }

    #[test]
    fn search_is_case_insensitive_and_unicode_aware() {
        let mut db = Db::open_in_memory().unwrap();
        db.insert_or_touch(&text("Merhaba ÇOK güzel"), &LIMITS)
            .unwrap();
        db.insert_or_touch(&text("hello"), &LIMITS).unwrap();
        assert_eq!(texts(&db, Some("çok")), vec!["Merhaba ÇOK güzel"]);
        assert_eq!(texts(&db, Some("HEL")), vec!["hello"]);
        assert!(texts(&db, Some("zzz")).is_empty());
    }

    #[test]
    fn list_truncates_long_text_for_preview_only() {
        let mut db = Db::open_in_memory().unwrap();
        db.insert_or_touch(&text(&"x".repeat(50)), &LIMITS).unwrap();
        let clip = &db.list(None, 10).unwrap()[0];
        assert_eq!(clip.text.as_deref().unwrap().len(), 10);
        let full = db.get(clip.id).unwrap().unwrap();
        assert_eq!(full.text.unwrap().len(), 50);
    }

    #[test]
    fn image_quota_evicts_oldest_unpinned_images_and_returns_files() {
        let mut db = Db::open_in_memory().unwrap();
        db.insert_or_touch(&image("one", 600), &LIMITS).unwrap();
        db.insert_or_touch(&text("keep me"), &LIMITS).unwrap();
        let removed = db.insert_or_touch(&image("two", 600), &LIMITS).unwrap();
        assert_eq!(
            removed,
            vec![PathBuf::from("one.png"), PathBuf::from("one_t.png")]
        );
        assert_eq!(texts(&db, None), vec!["two.png", "keep me"]);
        assert_eq!(db.image_usage().unwrap(), (600, 1));
    }

    #[test]
    fn delete_and_clear_respect_pins() {
        let mut db = Db::open_in_memory().unwrap();
        db.insert_or_touch(&text("a"), &LIMITS).unwrap();
        db.insert_or_touch(&text("b"), &LIMITS).unwrap();
        db.insert_or_touch(&text("c"), &LIMITS).unwrap();
        let ids: Vec<i64> = db.list(None, 100).unwrap().iter().map(|c| c.id).collect();
        db.set_pinned(ids[2], true).unwrap(); // "a"
        db.delete(ids[0]).unwrap(); // "c"
        assert_eq!(texts(&db, None), vec!["a", "b"]);
        db.clear(false).unwrap();
        assert_eq!(texts(&db, None), vec!["a"]);
        db.clear(true).unwrap();
        assert!(texts(&db, None).is_empty());
    }

    #[test]
    fn settings_round_trip_and_upsert() {
        let mut db = Db::open_in_memory().unwrap();
        assert_eq!(db.get_setting("max_items").unwrap(), None);
        db.set_setting("max_items", "100").unwrap();
        db.set_setting("max_items", "250").unwrap();
        assert_eq!(db.get_setting("max_items").unwrap().as_deref(), Some("250"));
    }

    #[test]
    fn clear_images_keeps_text() {
        let mut db = Db::open_in_memory().unwrap();
        db.insert_or_touch(&text("t"), &LIMITS).unwrap();
        db.insert_or_touch(&image("img", 10), &LIMITS).unwrap();
        db.clear_images().unwrap();
        assert_eq!(texts(&db, None), vec!["t"]);
        assert!(db.referenced_files().unwrap().is_empty());
    }
}
