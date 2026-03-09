use anyhow::Result;
use rusqlite::Connection;

const CURRENT_VERSION: i64 = 1;

pub fn migrate(conn: &Connection) -> Result<()> {
    let version = get_schema_version(conn);

    if version >= CURRENT_VERSION {
        return Ok(());
    }

    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS meta (
            key   TEXT PRIMARY KEY,
            value TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS sessions (
            id          INTEGER PRIMARY KEY AUTOINCREMENT,
            project     TEXT NOT NULL,
            directory   TEXT NOT NULL,
            summary     TEXT,
            started_at  TEXT DEFAULT (datetime('now')),
            ended_at    TEXT
        );

        CREATE TABLE IF NOT EXISTS observations (
            id              INTEGER PRIMARY KEY AUTOINCREMENT,
            session_id      INTEGER REFERENCES sessions(id),
            project         TEXT NOT NULL,
            topic_key       TEXT,
            type            TEXT NOT NULL,
            title           TEXT NOT NULL,
            content         TEXT NOT NULL,
            concepts        TEXT,
            facts           TEXT,
            files           TEXT,
            scope           TEXT DEFAULT 'project',
            tier            TEXT DEFAULT 'buffer',
            access_count    INTEGER DEFAULT 0,
            revision_count  INTEGER DEFAULT 1,
            content_hash    TEXT NOT NULL,
            embedding       BLOB,
            created_at      TEXT DEFAULT (datetime('now')),
            updated_at      TEXT DEFAULT (datetime('now')),
            deleted_at      TEXT
        );

        CREATE INDEX IF NOT EXISTS idx_observations_project
            ON observations(project);
        CREATE INDEX IF NOT EXISTS idx_observations_topic_key
            ON observations(project, topic_key);
        CREATE INDEX IF NOT EXISTS idx_observations_content_hash
            ON observations(content_hash);
        CREATE INDEX IF NOT EXISTS idx_observations_type
            ON observations(type);
        CREATE INDEX IF NOT EXISTS idx_observations_tier
            ON observations(tier);

        CREATE VIRTUAL TABLE IF NOT EXISTS observations_fts USING fts5(
            title, content, concepts, facts, type, project,
            content=observations,
            content_rowid=id,
            tokenize='porter unicode61'
        );

        CREATE VIRTUAL TABLE IF NOT EXISTS vec_observations USING vec0(
            embedding float[384]
        );
        ",
    )?;

    conn.execute(
        "INSERT OR REPLACE INTO meta (key, value) VALUES ('schema_version', ?1)",
        [CURRENT_VERSION.to_string()],
    )?;

    Ok(())
}

fn get_schema_version(conn: &Connection) -> i64 {
    conn.query_row(
        "SELECT value FROM meta WHERE key = 'schema_version'",
        [],
        |row| {
            let val: String = row.get(0)?;
            Ok(val.parse::<i64>().unwrap_or(0))
        },
    )
    .unwrap_or(0)
}
