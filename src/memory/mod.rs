pub mod autotag;
mod compact;
mod decay;
mod dedup;

pub use compact::{CompactionStats, run_compaction};
pub use dedup::DedupResult;

use anyhow::Result;

use crate::db::{Database, NewObservation};
use crate::embed::EmbeddingManager;

#[derive(Debug)]
pub struct SaveResult {
    pub id: i64,
    pub dedup_status: DedupResult,
    pub was_embedded: bool,
}

pub struct MemoryManager {
    db: Database,
    embed_mgr: Option<EmbeddingManager>,
}

impl MemoryManager {
    pub fn new(db: Database, embed_mgr: Option<EmbeddingManager>) -> Self {
        Self { db, embed_mgr }
    }

    pub fn db(&self) -> &Database {
        &self.db
    }

    pub fn embed_mgr(&self) -> Option<&EmbeddingManager> {
        self.embed_mgr.as_ref()
    }

    pub fn save_observation(&self, obs: &NewObservation) -> Result<SaveResult> {
        let obs = self.maybe_autotag(obs);
        let dedup_status = dedup::check_dedup(&self.db, &obs)?;

        match &dedup_status {
            DedupResult::HashMatch(existing_id) => {
                // Exact duplicate within window — skip, return existing
                Ok(SaveResult {
                    id: *existing_id,
                    dedup_status,
                    was_embedded: false,
                })
            }
            DedupResult::TopicKeyUpsert(_) => {
                // Upsert: update existing observation
                let id = self.db.upsert_observation(&obs)?;

                // Sync FTS
                self.db.remove_from_fts(id).ok(); // may not exist yet
                self.db.sync_observation_to_fts(id)?;

                // Re-embed if model available
                let was_embedded = self.try_embed_observation(id, &obs);

                Ok(SaveResult {
                    id,
                    dedup_status,
                    was_embedded,
                })
            }
            DedupResult::NewContent => {
                // Insert new observation
                let id = self.db.insert_observation(&obs)?;

                // Sync FTS
                self.db.sync_observation_to_fts(id)?;

                // Embed if model available
                let was_embedded = self.try_embed_observation(id, &obs);

                Ok(SaveResult {
                    id,
                    dedup_status,
                    was_embedded,
                })
            }
        }
    }

    fn maybe_autotag(&self, obs: &NewObservation) -> NewObservation {
        let mut obs = obs.clone();

        if obs.concepts.is_none() || obs.concepts.as_ref().is_some_and(|c| c.is_empty()) {
            let text = format!("{} {}", obs.title, obs.content);
            let keywords = autotag::extract_keywords(&text, 6);
            if !keywords.is_empty() {
                obs.concepts = Some(keywords);
            }
        }

        if obs.facts.is_none() || obs.facts.as_ref().is_some_and(|f| f.is_empty()) {
            let facts = autotag::extract_facts(&obs.content, 3);
            if !facts.is_empty() {
                obs.facts = Some(facts);
            }
        }

        obs
    }

    fn try_embed_observation(&self, id: i64, obs: &NewObservation) -> bool {
        let Some(ref mgr) = self.embed_mgr else {
            return false;
        };

        if !mgr.is_model_available() {
            tracing::info!("Embedding model not loaded — downloading on first use...");
            if mgr.download_model().is_err() {
                return false;
            }
        }

        let search_text = crate::embed::build_search_text(
            &obs.title,
            &obs.content,
            &obs.concepts
                .as_deref()
                .unwrap_or_default()
                .iter()
                .map(String::as_str)
                .collect::<Vec<_>>(),
            &obs.facts
                .as_deref()
                .unwrap_or_default()
                .iter()
                .map(String::as_str)
                .collect::<Vec<_>>(),
        );

        match mgr.embed(&search_text) {
            Ok(embedding) => {
                // Delete existing vector (if upsert) then insert new
                self.db.delete_vector(id).ok();
                self.db.insert_vector(id, &embedding).is_ok()
            }
            Err(_) => false,
        }
    }
}
