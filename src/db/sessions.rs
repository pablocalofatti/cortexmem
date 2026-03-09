use anyhow::Result;

use super::Database;

#[derive(Debug, Clone)]
pub struct Session {
    pub id: i64,
    pub project: String,
    pub directory: String,
    pub summary: Option<String>,
    pub started_at: String,
    pub ended_at: Option<String>,
}

impl Database {
    pub fn create_session(&self, project: &str, directory: &str) -> Result<i64> {
        self.conn().execute(
            "INSERT INTO sessions (project, directory) VALUES (?1, ?2)",
            rusqlite::params![project, directory],
        )?;
        Ok(self.conn().last_insert_rowid())
    }

    pub fn end_session(&self, id: i64, summary: Option<&str>) -> Result<()> {
        self.conn().execute(
            "UPDATE sessions SET ended_at = datetime('now'), summary = COALESCE(?2, summary) WHERE id = ?1",
            rusqlite::params![id, summary],
        )?;
        Ok(())
    }

    pub fn get_session(&self, id: i64) -> Result<Option<Session>> {
        let result = self.conn().query_row(
            "SELECT id, project, directory, summary, started_at, ended_at
             FROM sessions WHERE id = ?1",
            [id],
            row_to_session,
        );

        match result {
            Ok(s) => Ok(Some(s)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    pub fn get_latest_session(&self, project: &str) -> Result<Option<Session>> {
        let result = self.conn().query_row(
            "SELECT id, project, directory, summary, started_at, ended_at
             FROM sessions WHERE project = ?1
             ORDER BY id DESC LIMIT 1",
            [project],
            row_to_session,
        );

        match result {
            Ok(s) => Ok(Some(s)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    pub fn set_session_summary(&self, id: i64, summary: &str) -> Result<()> {
        self.conn().execute(
            "UPDATE sessions SET summary = ?2 WHERE id = ?1",
            rusqlite::params![id, summary],
        )?;
        Ok(())
    }
}

fn row_to_session(row: &rusqlite::Row<'_>) -> rusqlite::Result<Session> {
    Ok(Session {
        id: row.get(0)?,
        project: row.get(1)?,
        directory: row.get(2)?,
        summary: row.get(3)?,
        started_at: row.get(4)?,
        ended_at: row.get(5)?,
    })
}
