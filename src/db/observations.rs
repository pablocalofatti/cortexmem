use anyhow::Result;
use sha2::{Digest, Sha256};

use super::Database;

#[derive(Debug, Clone)]
pub struct NewObservation {
    pub project: String,
    pub title: String,
    pub content: String,
    pub obs_type: String,
    pub concepts: Option<Vec<String>>,
    pub facts: Option<Vec<String>>,
    pub files: Option<Vec<String>>,
    pub topic_key: Option<String>,
    pub scope: String,
    pub session_id: Option<i64>,
}

#[derive(Debug, Clone)]
pub struct Observation {
    pub id: i64,
    pub session_id: Option<i64>,
    pub project: String,
    pub topic_key: Option<String>,
    pub obs_type: String,
    pub title: String,
    pub content: String,
    pub concepts: Option<Vec<String>>,
    pub facts: Option<Vec<String>>,
    pub files: Option<Vec<String>>,
    pub scope: String,
    pub tier: String,
    pub access_count: i64,
    pub revision_count: i64,
    pub content_hash: String,
    pub created_at: String,
    pub updated_at: String,
    pub deleted_at: Option<String>,
}

fn compute_content_hash(content: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    format!("{:x}", hasher.finalize())
}

fn vec_to_json(v: &Option<Vec<String>>) -> Option<String> {
    v.as_ref().map(|items| serde_json::to_string(items).unwrap_or_default())
}

fn json_to_vec(s: &Option<String>) -> Option<Vec<String>> {
    s.as_ref().and_then(|json| serde_json::from_str(json).ok())
}

impl Database {
    pub fn insert_observation(&self, obs: &NewObservation) -> Result<i64> {
        let hash = compute_content_hash(&obs.content);
        let concepts_json = vec_to_json(&obs.concepts);
        let facts_json = vec_to_json(&obs.facts);
        let files_json = vec_to_json(&obs.files);

        self.conn().execute(
            "INSERT INTO observations (session_id, project, topic_key, type, title, content,
             concepts, facts, files, scope, content_hash)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            rusqlite::params![
                obs.session_id,
                obs.project,
                obs.topic_key,
                obs.obs_type,
                obs.title,
                obs.content,
                concepts_json,
                facts_json,
                files_json,
                obs.scope,
                hash,
            ],
        )?;

        Ok(self.conn().last_insert_rowid())
    }

    pub fn get_observation(&self, id: i64) -> Result<Option<Observation>> {
        let result = self.conn().query_row(
            "SELECT id, session_id, project, topic_key, type, title, content,
                    concepts, facts, files, scope, tier, access_count,
                    revision_count, content_hash, created_at, updated_at, deleted_at
             FROM observations WHERE id = ?1",
            [id],
            |row| {
                let concepts_json: Option<String> = row.get(7)?;
                let facts_json: Option<String> = row.get(8)?;
                let files_json: Option<String> = row.get(9)?;

                Ok(Observation {
                    id: row.get(0)?,
                    session_id: row.get(1)?,
                    project: row.get(2)?,
                    topic_key: row.get(3)?,
                    obs_type: row.get(4)?,
                    title: row.get(5)?,
                    content: row.get(6)?,
                    concepts: json_to_vec(&concepts_json),
                    facts: json_to_vec(&facts_json),
                    files: json_to_vec(&files_json),
                    scope: row.get(10)?,
                    tier: row.get(11)?,
                    access_count: row.get(12)?,
                    revision_count: row.get(13)?,
                    content_hash: row.get(14)?,
                    created_at: row.get(15)?,
                    updated_at: row.get(16)?,
                    deleted_at: row.get(17)?,
                })
            },
        );

        match result {
            Ok(obs) => Ok(Some(obs)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    pub fn find_by_topic_key(
        &self,
        project: &str,
        topic_key: &str,
    ) -> Result<Option<Observation>> {
        let result = self.conn().query_row(
            "SELECT id FROM observations
             WHERE project = ?1 AND topic_key = ?2 AND deleted_at IS NULL
             ORDER BY updated_at DESC LIMIT 1",
            rusqlite::params![project, topic_key],
            |row| row.get::<_, i64>(0),
        );

        match result {
            Ok(id) => self.get_observation(id),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    pub fn find_by_content_hash(
        &self,
        hash: &str,
        within_minutes: i64,
    ) -> Result<Option<Observation>> {
        let result = self.conn().query_row(
            "SELECT id FROM observations
             WHERE content_hash = ?1
               AND deleted_at IS NULL
               AND datetime(created_at) >= datetime('now', ?2)
             ORDER BY created_at DESC LIMIT 1",
            rusqlite::params![hash, format!("-{within_minutes} minutes")],
            |row| row.get::<_, i64>(0),
        );

        match result {
            Ok(id) => self.get_observation(id),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    pub fn upsert_observation(&self, obs: &NewObservation) -> Result<i64> {
        if let Some(ref topic_key) = obs.topic_key {
            if let Some(existing) = self.find_by_topic_key(&obs.project, topic_key)? {
                let hash = compute_content_hash(&obs.content);
                let concepts_json = vec_to_json(&obs.concepts);
                let facts_json = vec_to_json(&obs.facts);
                let files_json = vec_to_json(&obs.files);

                self.conn().execute(
                    "UPDATE observations SET
                        title = ?1, content = ?2, concepts = ?3, facts = ?4,
                        files = ?5, content_hash = ?6,
                        revision_count = revision_count + 1,
                        updated_at = datetime('now')
                     WHERE id = ?7",
                    rusqlite::params![
                        obs.title,
                        obs.content,
                        concepts_json,
                        facts_json,
                        files_json,
                        hash,
                        existing.id,
                    ],
                )?;

                return Ok(existing.id);
            }
        }

        self.insert_observation(obs)
    }

    pub fn soft_delete(&self, id: i64) -> Result<()> {
        self.conn().execute(
            "UPDATE observations SET deleted_at = datetime('now') WHERE id = ?1",
            [id],
        )?;
        Ok(())
    }

    pub fn increment_access_count(&self, id: i64) -> Result<()> {
        self.conn().execute(
            "UPDATE observations SET access_count = access_count + 1, updated_at = datetime('now') WHERE id = ?1",
            [id],
        )?;
        Ok(())
    }

    pub fn list_observations(&self, project: &str, limit: i64) -> Result<Vec<Observation>> {
        let mut stmt = self.conn().prepare(
            "SELECT id FROM observations
             WHERE project = ?1 AND deleted_at IS NULL
             ORDER BY updated_at DESC LIMIT ?2",
        )?;

        let ids: Vec<i64> = stmt
            .query_map(rusqlite::params![project, limit], |row| row.get(0))?
            .collect::<Result<Vec<_>, _>>()?;

        let mut observations = Vec::with_capacity(ids.len());
        for id in ids {
            if let Some(obs) = self.get_observation(id)? {
                observations.push(obs);
            }
        }

        Ok(observations)
    }

    pub fn list_all_active_observations(&self) -> Result<Vec<Observation>> {
        let mut stmt = self.conn().prepare(
            "SELECT id FROM observations WHERE deleted_at IS NULL ORDER BY updated_at DESC",
        )?;

        let ids: Vec<i64> = stmt
            .query_map([], |row| row.get(0))?
            .collect::<Result<Vec<_>, _>>()?;

        let mut observations = Vec::with_capacity(ids.len());
        for id in ids {
            if let Some(obs) = self.get_observation(id)? {
                observations.push(obs);
            }
        }

        Ok(observations)
    }

    pub fn update_observation_fields(
        &self,
        id: i64,
        title: Option<&str>,
        content: Option<&str>,
        concepts: Option<&Vec<String>>,
        facts: Option<&Vec<String>>,
        files: Option<&Vec<String>>,
    ) -> Result<()> {
        let mut sets = Vec::new();
        let mut params: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

        if let Some(t) = title {
            sets.push("title = ?");
            params.push(Box::new(t.to_string()));
        }
        if let Some(c) = content {
            let hash = compute_content_hash(c);
            sets.push("content = ?");
            params.push(Box::new(c.to_string()));
            sets.push("content_hash = ?");
            params.push(Box::new(hash));
        }
        if let Some(c) = concepts {
            sets.push("concepts = ?");
            params.push(Box::new(serde_json::to_string(c).unwrap_or_default()));
        }
        if let Some(f) = facts {
            sets.push("facts = ?");
            params.push(Box::new(serde_json::to_string(f).unwrap_or_default()));
        }
        if let Some(f) = files {
            sets.push("files = ?");
            params.push(Box::new(serde_json::to_string(f).unwrap_or_default()));
        }

        if sets.is_empty() {
            return Ok(());
        }

        sets.push("revision_count = revision_count + 1");
        sets.push("updated_at = datetime('now')");

        let sql = format!("UPDATE observations SET {} WHERE id = ?", sets.join(", "));
        params.push(Box::new(id));

        let param_refs: Vec<&dyn rusqlite::types::ToSql> = params.iter().map(|p| p.as_ref()).collect();
        self.conn().execute(&sql, param_refs.as_slice())?;

        Ok(())
    }

    pub fn update_tier(&self, id: i64, tier: &str) -> Result<()> {
        self.conn().execute(
            "UPDATE observations SET tier = ?2 WHERE id = ?1",
            rusqlite::params![id, tier],
        )?;
        Ok(())
    }

    /// Backdate an observation's timestamps. Used for testing decay rules
    /// and manual time adjustments.
    pub fn backdate_observation(&self, id: i64, days_ago: i64) -> Result<()> {
        let offset = format!("-{days_ago} days");
        self.conn().execute(
            "UPDATE observations SET created_at = datetime('now', ?2), updated_at = datetime('now', ?2) WHERE id = ?1",
            rusqlite::params![id, offset],
        )?;
        Ok(())
    }
}
