use anyhow::Result;
use sha2::{Digest, Sha256};

use crate::db::{Database, NewObservation};

const DEDUP_WINDOW_MINUTES: i64 = 15;

#[derive(Debug, Clone)]
pub enum DedupResult {
    NewContent,
    HashMatch(i64),
    TopicKeyUpsert(i64),
}

pub fn check_dedup(db: &Database, obs: &NewObservation) -> Result<DedupResult> {
    // 1. Hash check — exact duplicate within time window
    let hash = compute_content_hash(&obs.content);
    if let Some(existing) = db.find_by_content_hash(&hash, DEDUP_WINDOW_MINUTES)? {
        return Ok(DedupResult::HashMatch(existing.id));
    }

    // 2. Topic key check — same project + topic_key = upsert
    if let Some(ref topic_key) = obs.topic_key
        && let Some(existing) = db.find_by_topic_key(&obs.project, topic_key)?
    {
        return Ok(DedupResult::TopicKeyUpsert(existing.id));
    }

    Ok(DedupResult::NewContent)
}

fn compute_content_hash(content: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    format!("{:x}", hasher.finalize())
}
