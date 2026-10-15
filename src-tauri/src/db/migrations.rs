use rusqlite::Connection;

/// Append-only list of schema migrations. Index + 1 is the schema version
/// stored in `PRAGMA user_version`. Never edit an entry that has shipped;
/// add a new one instead so existing user data is upgraded in place.
const MIGRATIONS: &[&str] = &[
    // v1: clips + settings
    "CREATE TABLE clips (
        id            INTEGER PRIMARY KEY AUTOINCREMENT,
        kind          TEXT NOT NULL CHECK (kind IN ('text', 'link', 'image')),
        text          TEXT,
        image_path    TEXT,
        thumb_path    TEXT,
        image_bytes   INTEGER NOT NULL DEFAULT 0,
        hash          TEXT NOT NULL UNIQUE,
        pinned        INTEGER NOT NULL DEFAULT 0,
        created_at    INTEGER NOT NULL,
        last_used_at  INTEGER NOT NULL
    );
    CREATE INDEX idx_clips_order ON clips (pinned DESC, created_at DESC);
    CREATE TABLE settings (
        key   TEXT PRIMARY KEY,
        value TEXT NOT NULL
    );",
];

#[cfg(test)]
pub fn latest_version() -> usize {
    MIGRATIONS.len()
}

pub fn current_version(conn: &Connection) -> rusqlite::Result<usize> {
    conn.query_row("PRAGMA user_version", [], |r| r.get::<_, i64>(0))
        .map(|v| v as usize)
}

pub fn run(conn: &mut Connection) -> rusqlite::Result<()> {
    let current = current_version(conn)?;
    for (i, sql) in MIGRATIONS.iter().enumerate().skip(current) {
        let tx = conn.transaction()?;
        tx.execute_batch(sql)?;
        tx.execute_batch(&format!("PRAGMA user_version = {}", i + 1))?;
        tx.commit()?;
    }
    Ok(())
}
