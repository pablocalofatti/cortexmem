use anyhow::Result;
use serde::{Deserialize, Serialize};

use super::Database;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Prompt {
    pub id: i64,
    pub session_id: Option<i64>,
    pub content: String,
    pub project: Option<String>,
    pub created_at: String,
}

impl Database {
    pub fn insert_prompt(
        &self,
        session_id: Option<i64>,
        content: &str,
        project: Option<&str>,
    ) -> Result<i64> {
        self.conn().execute(
            "INSERT INTO user_prompts (session_id, content, project) VALUES (?1, ?2, ?3)",
            rusqlite::params![session_id, content, project],
        )?;
        let id = self.conn().last_insert_rowid();
        self.sync_prompt_to_fts(id)?;
        Ok(id)
    }

    fn sync_prompt_to_fts(&self, id: i64) -> Result<()> {
        self.conn().execute(
            "INSERT INTO prompts_fts(rowid, content, project)
             SELECT id, content, COALESCE(project, '') FROM user_prompts WHERE id = ?1",
            [id],
        )?;
        Ok(())
    }

    pub fn get_recent_prompts(&self, project: Option<&str>, limit: i64) -> Result<Vec<Prompt>> {
        match project {
            Some(p) => {
                let mut stmt = self.conn().prepare(
                    "SELECT id, session_id, content, project, created_at
                     FROM user_prompts WHERE project = ?1
                     ORDER BY created_at DESC, id DESC LIMIT ?2",
                )?;
                let rows = stmt
                    .query_map(rusqlite::params![p, limit], map_prompt_row)?
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(rows)
            }
            None => {
                let mut stmt = self.conn().prepare(
                    "SELECT id, session_id, content, project, created_at
                     FROM user_prompts ORDER BY created_at DESC, id DESC LIMIT ?1",
                )?;
                let rows = stmt
                    .query_map([limit], map_prompt_row)?
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(rows)
            }
        }
    }

    pub fn search_prompts(
        &self,
        query: &str,
        project: Option<&str>,
        limit: i64,
    ) -> Result<Vec<Prompt>> {
        let fts_query = if let Some(proj) = project {
            format!("content:\"{query}\" AND project:\"{proj}\"")
        } else {
            format!("content:\"{query}\"")
        };
        let mut stmt = self.conn().prepare(
            "SELECT p.id, p.session_id, p.content, p.project, p.created_at
             FROM prompts_fts f JOIN user_prompts p ON f.rowid = p.id
             WHERE prompts_fts MATCH ?1 ORDER BY rank LIMIT ?2",
        )?;
        let rows = stmt
            .query_map(rusqlite::params![fts_query, limit], map_prompt_row)?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }
}

fn map_prompt_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<Prompt> {
    Ok(Prompt {
        id: row.get(0)?,
        session_id: row.get(1)?,
        content: row.get(2)?,
        project: row.get(3)?,
        created_at: row.get(4)?,
    })
}
