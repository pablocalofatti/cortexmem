mod fts;
mod observations;
mod prompts;
pub(crate) mod schema;
mod sessions;
mod sync;

pub use sync::{SyncMutation, SyncState};
mod vector;

pub use fts::FtsResult;
pub use observations::{NewObservation, Observation};
pub use prompts::Prompt;
pub use sessions::Session;
pub use vector::VecResult;

use std::path::Path;
use std::sync::Once;

use anyhow::Result;
use rusqlite::Connection;
use rusqlite::ffi::sqlite3_auto_extension;

static VEC_INIT: Once = Once::new();

fn register_vec_extension() {
    VEC_INIT.call_once(|| {
        #[allow(clippy::missing_transmute_annotations)]
        unsafe {
            // SAFETY: Registering sqlite-vec as an auto-extension before any
            // connections are created. `call_once` guarantees this runs exactly
            // once, and sqlite3_auto_extension is thread-safe per SQLite docs.
            sqlite3_auto_extension(Some(std::mem::transmute(
                sqlite_vec::sqlite3_vec_init as *const (),
            )));
        }
    });
}

pub struct Database {
    conn: Connection,
}

impl Database {
    pub fn open(path: &Path) -> Result<Self> {
        register_vec_extension();
        let conn = Connection::open(path)?;
        let db = Self { conn };
        db.configure_pragmas()?;
        schema::migrate(&db.conn)?;
        Ok(db)
    }

    pub fn open_in_memory() -> Result<Self> {
        register_vec_extension();
        let conn = Connection::open_in_memory()?;
        let db = Self { conn };
        db.configure_pragmas()?;
        schema::migrate(&db.conn)?;
        Ok(db)
    }

    fn configure_pragmas(&self) -> Result<()> {
        self.conn.pragma_update(None, "journal_mode", "wal")?;
        self.conn.pragma_update(None, "foreign_keys", "ON")?;
        self.conn.pragma_update(None, "busy_timeout", 5000)?;
        Ok(())
    }

    pub fn schema_version(&self) -> Result<i64> {
        let version: String = self.conn.query_row(
            "SELECT value FROM meta WHERE key = 'schema_version'",
            [],
            |row| row.get(0),
        )?;
        Ok(version.parse()?)
    }

    pub fn journal_mode(&self) -> Result<String> {
        let mode: String = self
            .conn
            .pragma_query_value(None, "journal_mode", |row| row.get(0))?;
        Ok(mode)
    }

    pub fn has_vec_extension(&self) -> Result<bool> {
        let version: String = self
            .conn
            .query_row("SELECT vec_version()", [], |row| row.get(0))?;
        Ok(!version.is_empty())
    }

    pub(crate) fn conn(&self) -> &Connection {
        &self.conn
    }

    /// Read a value from the `meta` key-value table.
    pub fn get_meta(&self, key: &str) -> Option<String> {
        self.conn
            .query_row("SELECT value FROM meta WHERE key = ?1", [key], |row| {
                row.get(0)
            })
            .ok()
    }

    /// Write a value to the `meta` key-value table.
    pub fn set_meta(&self, key: &str, value: &str) -> Result<()> {
        self.conn.execute(
            "INSERT OR REPLACE INTO meta (key, value) VALUES (?1, ?2)",
            [key, value],
        )?;
        Ok(())
    }

    /// Delete all vector embeddings (used before reindex with a different model).
    pub fn delete_all_vectors(&self) -> Result<()> {
        self.conn.execute("DELETE FROM vec_observations", [])?;
        Ok(())
    }

    /// List IDs of all non-deleted observations.
    pub fn list_all_observation_ids(&self) -> Result<Vec<i64>> {
        let mut stmt = self
            .conn
            .prepare("SELECT id FROM observations WHERE deleted_at IS NULL ORDER BY id")?;
        let ids = stmt
            .query_map([], |row| row.get(0))?
            .collect::<std::result::Result<Vec<i64>, _>>()?;
        Ok(ids)
    }

    /// Count rows in the FTS5 index table.
    pub fn count_fts_entries(&self) -> Result<i64> {
        let count: i64 =
            self.conn
                .query_row("SELECT COUNT(*) FROM observations_fts", [], |row| {
                    row.get(0)
                })?;
        Ok(count)
    }

    /// Count rows in the vector index table.
    pub fn count_vector_entries(&self) -> Result<i64> {
        let count: i64 =
            self.conn
                .query_row("SELECT COUNT(*) FROM vec_observations", [], |row| {
                    row.get(0)
                })?;
        Ok(count)
    }
}
