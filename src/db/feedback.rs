use anyhow::Result;

use super::Database;

impl Database {
    /// Record that a user accessed an observation after a search query.
    pub fn record_search_feedback(
        &self,
        query_text: &str,
        observation_id: i64,
        session_id: Option<i64>,
    ) -> Result<()> {
        self.conn().execute(
            "INSERT INTO search_feedback (query_text, observation_id, session_id) VALUES (?1, ?2, ?3)",
            rusqlite::params![query_text, observation_id, session_id],
        )?;
        Ok(())
    }

    /// Get total feedback count for an observation (how many times it was accessed after searches).
    pub fn get_feedback_count(&self, observation_id: i64) -> Result<i64> {
        let count: i64 = self.conn().query_row(
            "SELECT COUNT(*) FROM search_feedback WHERE observation_id = ?1",
            [observation_id],
            |row| row.get(0),
        )?;
        Ok(count)
    }
}
