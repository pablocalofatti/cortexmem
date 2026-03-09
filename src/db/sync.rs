use anyhow::Result;
use serde::{Deserialize, Serialize};

use super::Database;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncMutation {
    pub seq: i64,
    pub entity: String,
    pub entity_key: String,
    pub op: String,
    pub payload: String,
    pub project: String,
    pub occurred_at: String,
    pub acked_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncState {
    pub target_key: String,
    pub last_pushed_seq: i64,
    pub last_pulled_seq: i64,
    pub last_error: Option<String>,
    pub updated_at: String,
}

impl Database {
    pub fn insert_sync_mutation(
        &self,
        entity: &str,
        entity_key: &str,
        op: &str,
        payload: &str,
        project: &str,
    ) -> Result<i64> {
        self.conn().execute(
            "INSERT INTO sync_mutations (entity, entity_key, op, payload, project)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params![entity, entity_key, op, payload, project],
        )?;
        Ok(self.conn().last_insert_rowid())
    }

    pub fn list_unacked_mutations(&self, limit: i64) -> Result<Vec<SyncMutation>> {
        let mut stmt = self.conn().prepare(
            "SELECT seq, entity, entity_key, op, payload, project, occurred_at, acked_at
             FROM sync_mutations
             WHERE acked_at IS NULL
             ORDER BY seq ASC
             LIMIT ?1",
        )?;
        let rows = stmt
            .query_map(rusqlite::params![limit], row_to_sync_mutation)?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    pub fn ack_mutations(&self, up_to_seq: i64) -> Result<()> {
        self.conn().execute(
            "UPDATE sync_mutations SET acked_at = datetime('now')
             WHERE seq <= ?1 AND acked_at IS NULL",
            rusqlite::params![up_to_seq],
        )?;
        Ok(())
    }

    pub fn update_sync_state(
        &self,
        target_key: &str,
        last_pushed_seq: i64,
        last_pulled_seq: i64,
        last_error: Option<&str>,
    ) -> Result<()> {
        self.conn().execute(
            "INSERT INTO sync_state (target_key, last_pushed_seq, last_pulled_seq, last_error, updated_at)
             VALUES (?1, ?2, ?3, ?4, datetime('now'))
             ON CONFLICT(target_key) DO UPDATE SET
                last_pushed_seq = excluded.last_pushed_seq,
                last_pulled_seq = excluded.last_pulled_seq,
                last_error = excluded.last_error,
                updated_at = excluded.updated_at",
            rusqlite::params![target_key, last_pushed_seq, last_pulled_seq, last_error],
        )?;
        Ok(())
    }

    pub fn get_sync_state(&self, target_key: &str) -> Result<Option<SyncState>> {
        let result = self.conn().query_row(
            "SELECT target_key, last_pushed_seq, last_pulled_seq, last_error, updated_at
             FROM sync_state WHERE target_key = ?1",
            rusqlite::params![target_key],
            row_to_sync_state,
        );

        match result {
            Ok(s) => Ok(Some(s)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    pub fn record_sync_chunk(&self, chunk_id: &str) -> Result<bool> {
        let rows = self.conn().execute(
            "INSERT OR IGNORE INTO sync_chunks (chunk_id) VALUES (?1)",
            rusqlite::params![chunk_id],
        )?;
        Ok(rows > 0)
    }
}

fn row_to_sync_mutation(row: &rusqlite::Row<'_>) -> rusqlite::Result<SyncMutation> {
    Ok(SyncMutation {
        seq: row.get(0)?,
        entity: row.get(1)?,
        entity_key: row.get(2)?,
        op: row.get(3)?,
        payload: row.get(4)?,
        project: row.get(5)?,
        occurred_at: row.get(6)?,
        acked_at: row.get(7)?,
    })
}

fn row_to_sync_state(row: &rusqlite::Row<'_>) -> rusqlite::Result<SyncState> {
    Ok(SyncState {
        target_key: row.get(0)?,
        last_pushed_seq: row.get(1)?,
        last_pulled_seq: row.get(2)?,
        last_error: row.get(3)?,
        updated_at: row.get(4)?,
    })
}
