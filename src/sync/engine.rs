// Sync engine will be wired into CLI/MCP commands in a follow-up task.
#![allow(dead_code)]

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::db::{Database, Observation};

const PUSH_BATCH_LIMIT: i64 = 100;
const SYNC_STATE_KEY: &str = "cloud";

#[derive(Debug, Clone)]
pub struct SyncConfig {
    pub server_url: String,
    pub api_key: String,
}

#[derive(Debug, Serialize)]
struct PushPayload {
    mutations: Vec<MutationPayload>,
}

#[derive(Debug, Serialize)]
struct MutationPayload {
    entity: String,
    entity_key: String,
    op: String,
    payload: serde_json::Value,
    project: String,
    occurred_at: String,
}

#[derive(Debug, Deserialize)]
struct PushResponse {
    accepted: i64,
    last_seq: i64,
}

#[derive(Debug, Deserialize)]
struct PullResponse {
    mutations: Vec<PullMutation>,
}

#[derive(Debug, Deserialize)]
struct PullMutation {
    seq: i64,
    entity: String,
    entity_key: String,
    op: String,
    payload: serde_json::Value,
    #[allow(dead_code)]
    project: String,
    #[allow(dead_code)]
    occurred_at: String,
}

/// Push unacked local mutations to the cloud server.
/// Returns the number of mutations accepted by the server.
pub async fn push(db: &Database, config: &SyncConfig) -> Result<i64> {
    let mutations = db.list_unacked_mutations(PUSH_BATCH_LIMIT)?;
    if mutations.is_empty() {
        return Ok(0);
    }

    let last_local_seq = mutations.last().map(|m| m.seq).unwrap_or(0);

    let payload = PushPayload {
        mutations: mutations
            .into_iter()
            .map(|m| {
                let payload_value: serde_json::Value =
                    serde_json::from_str(&m.payload).unwrap_or_default();
                MutationPayload {
                    entity: m.entity,
                    entity_key: m.entity_key,
                    op: m.op,
                    payload: payload_value,
                    project: m.project,
                    occurred_at: m.occurred_at,
                }
            })
            .collect(),
    };

    let client = reqwest::Client::new();
    let url = format!("{}/sync/push", config.server_url.trim_end_matches('/'));

    let response = client
        .post(&url)
        .bearer_auth(&config.api_key)
        .json(&payload)
        .send()
        .await
        .context("failed to connect to sync server")?;

    let status = response.status();
    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        anyhow::bail!("push failed with status {status}: {body}");
    }

    let push_resp: PushResponse = response
        .json()
        .await
        .context("failed to parse push response")?;

    db.ack_mutations(last_local_seq)?;

    let state = db.get_sync_state(SYNC_STATE_KEY)?;
    let last_pulled = state.as_ref().map(|s| s.last_pulled_seq).unwrap_or(0);
    db.update_sync_state(SYNC_STATE_KEY, push_resp.last_seq, last_pulled, None)?;

    Ok(push_resp.accepted)
}

/// Pull remote mutations from the cloud server and apply them locally.
/// Returns the number of mutations applied.
pub async fn pull(db: &Database, config: &SyncConfig) -> Result<i64> {
    let state = db.get_sync_state(SYNC_STATE_KEY)?;
    let since_seq = state.as_ref().map(|s| s.last_pulled_seq).unwrap_or(0);

    let client = reqwest::Client::new();
    let url = format!(
        "{}/sync/pull?since_seq={since_seq}",
        config.server_url.trim_end_matches('/')
    );

    let response = client
        .get(&url)
        .bearer_auth(&config.api_key)
        .send()
        .await
        .context("failed to connect to sync server")?;

    let status = response.status();
    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        anyhow::bail!("pull failed with status {status}: {body}");
    }

    let pull_resp: PullResponse = response
        .json()
        .await
        .context("failed to parse pull response")?;

    let count = pull_resp.mutations.len() as i64;
    let mut max_seq = since_seq;

    for m in &pull_resp.mutations {
        if m.seq > max_seq {
            max_seq = m.seq;
        }
        if let Err(e) = apply_remote_mutation(db, m) {
            tracing::warn!(
                seq = m.seq,
                entity = %m.entity,
                op = %m.op,
                "failed to apply remote mutation: {e}"
            );
        }
    }

    let last_pushed = state.as_ref().map(|s| s.last_pushed_seq).unwrap_or(0);
    db.update_sync_state(SYNC_STATE_KEY, last_pushed, max_seq, None)?;

    Ok(count)
}

/// Apply a single remote mutation to the local database.
fn apply_remote_mutation(db: &Database, m: &PullMutation) -> Result<()> {
    match (m.entity.as_str(), m.op.as_str()) {
        ("observation", "insert" | "upsert") => {
            let obs: Observation = serde_json::from_value(m.payload.clone())
                .context("failed to deserialize observation from mutation payload")?;
            db.import_observation(&obs)?;
        }
        ("observation", "soft_delete") => {
            let id: i64 = m
                .entity_key
                .parse()
                .context("invalid entity_key for soft_delete")?;
            db.soft_delete(id)?;
        }
        ("observation", "hard_delete") => {
            let id: i64 = m
                .entity_key
                .parse()
                .context("invalid entity_key for hard_delete")?;
            db.hard_delete(id)?;
        }
        _ => {
            tracing::warn!(
                entity = %m.entity,
                op = %m.op,
                "unknown mutation type, skipping"
            );
        }
    }
    Ok(())
}

/// Run a single push-then-pull sync cycle.
/// Returns (pushed_count, pulled_count).
pub async fn sync_once(db: &Database, config: &SyncConfig) -> Result<(i64, i64)> {
    let pushed = push(db, config).await?;
    let pulled = pull(db, config).await?;
    Ok((pushed, pulled))
}
