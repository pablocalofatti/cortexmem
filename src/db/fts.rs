use anyhow::Result;

use super::Database;

#[derive(Debug, Clone)]
pub struct FtsResult {
    pub rowid: i64,
    pub rank: f64,
}

impl Database {
    pub fn sync_observation_to_fts(&self, id: i64) -> Result<()> {
        self.conn().execute(
            "INSERT INTO observations_fts(rowid, title, content, concepts, facts, type, project)
             SELECT id, title, content, COALESCE(concepts, ''), COALESCE(facts, ''), type, project
             FROM observations WHERE id = ?1",
            [id],
        )?;
        Ok(())
    }

    pub fn remove_from_fts(&self, id: i64) -> Result<()> {
        // For external content FTS5 tables, deletion uses the special
        // 'delete' command with the content values
        self.conn().execute(
            "INSERT INTO observations_fts(observations_fts, rowid, title, content, concepts, facts, type, project)
             SELECT 'delete', id, title, content, COALESCE(concepts, ''), COALESCE(facts, ''), type, project
             FROM observations WHERE id = ?1",
            [id],
        )?;
        Ok(())
    }

    pub fn search_fts(
        &self,
        query: &str,
        project: Option<&str>,
        limit: i64,
    ) -> Result<Vec<FtsResult>> {
        // Build FTS5 query — quote project to handle special chars (hyphens etc.)
        let fts_query = if let Some(proj) = project {
            format!("({{title content concepts facts}}: {query}) AND project:\"{proj}\"")
        } else {
            format!("{{title content concepts facts}}: {query}")
        };

        let mut stmt = self.conn().prepare(
            "SELECT rowid, rank
             FROM observations_fts
             WHERE observations_fts MATCH ?1
             ORDER BY rank
             LIMIT ?2",
        )?;

        let results = stmt
            .query_map(rusqlite::params![fts_query, limit], |row| {
                Ok(FtsResult {
                    rowid: row.get(0)?,
                    rank: row.get(1)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(results)
    }
}
