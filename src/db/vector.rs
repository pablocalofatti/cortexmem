use anyhow::Result;

use super::Database;

#[derive(Debug, Clone)]
pub struct VecResult {
    pub rowid: i64,
    pub distance: f64,
}

impl Database {
    pub fn insert_vector(&self, rowid: i64, embedding: &[f32]) -> Result<()> {
        let blob = embedding_to_blob(embedding);
        self.conn().execute(
            "INSERT INTO vec_observations(rowid, embedding) VALUES (?1, ?2)",
            rusqlite::params![rowid, blob],
        )?;
        Ok(())
    }

    pub fn delete_vector(&self, rowid: i64) -> Result<()> {
        self.conn()
            .execute("DELETE FROM vec_observations WHERE rowid = ?1", [rowid])?;
        Ok(())
    }

    pub fn search_vector(&self, query_embedding: &[f32], limit: i64) -> Result<Vec<VecResult>> {
        let blob = embedding_to_blob(query_embedding);
        let mut stmt = self.conn().prepare(
            "SELECT rowid, distance
             FROM vec_observations
             WHERE embedding MATCH ?1
             ORDER BY distance
             LIMIT ?2",
        )?;

        let results = stmt
            .query_map(rusqlite::params![blob, limit], |row| {
                Ok(VecResult {
                    rowid: row.get(0)?,
                    distance: row.get(1)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(results)
    }
}

fn embedding_to_blob(embedding: &[f32]) -> Vec<u8> {
    embedding.iter().flat_map(|f| f.to_le_bytes()).collect()
}
