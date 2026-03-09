use anyhow::{Result, bail};
use serde::{Deserialize, Serialize};
use sqlx_core::query::query;
use sqlx_core::row::Row;
use sqlx_postgres::{PgPool, PgRow, Postgres};
use uuid::Uuid;

const DEFAULT_PULL_LIMIT: i64 = 100;
const MAX_PULL_LIMIT: i64 = 1000;

#[derive(Debug, Deserialize)]
pub struct PushRequest {
    pub mutations: Vec<PushMutation>,
}

#[derive(Debug, Deserialize)]
pub struct PushMutation {
    pub entity: String,
    pub entity_key: String,
    pub op: String,
    pub payload: serde_json::Value,
    pub project: String,
    pub occurred_at: String,
}

#[derive(Debug, Serialize)]
pub struct PushResponse {
    pub accepted: i64,
    pub last_seq: i64,
}

#[derive(Debug, Serialize)]
pub struct PullMutation {
    pub seq: i64,
    pub entity: String,
    pub entity_key: String,
    pub op: String,
    pub payload: serde_json::Value,
    pub project: String,
    pub occurred_at: String,
}

#[derive(Debug, Deserialize)]
pub struct AckRequest {
    pub up_to_seq: i64,
}

#[derive(Debug, Deserialize)]
pub struct PullParams {
    pub since_seq: i64,
    pub project: Option<String>,
    pub limit: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct EnrollRequest {
    pub project: String,
}

pub async fn push_mutations(
    pool: &PgPool,
    account_id: &Uuid,
    req: &PushRequest,
) -> Result<PushResponse> {
    // Pre-load enrolled projects to avoid N+1 queries
    let enrolled_rows: Vec<PgRow> =
        query::<Postgres>("SELECT project FROM enrolled_projects WHERE account_id = $1")
            .bind(account_id)
            .fetch_all(pool)
            .await?;

    let enrolled_projects: std::collections::HashSet<String> = enrolled_rows
        .iter()
        .map(|r| r.get::<String, _>("project"))
        .collect();

    let mut accepted: i64 = 0;
    let mut last_seq: i64 = 0;

    for mutation in &req.mutations {
        if !enrolled_projects.contains(&mutation.project) {
            continue;
        }

        let row: PgRow = query::<Postgres>(
            "INSERT INTO sync_mutations (account_id, entity, entity_key, op, payload, project, occurred_at) \
             VALUES ($1, $2, $3, $4, $5, $6, $7::timestamptz) RETURNING seq",
        )
        .bind(account_id)
        .bind(&mutation.entity)
        .bind(&mutation.entity_key)
        .bind(&mutation.op)
        .bind(&mutation.payload)
        .bind(&mutation.project)
        .bind(&mutation.occurred_at)
        .fetch_one(pool)
        .await?;

        last_seq = row.get("seq");
        accepted += 1;
    }

    Ok(PushResponse { accepted, last_seq })
}

pub async fn pull_mutations(
    pool: &PgPool,
    account_id: &Uuid,
    params: &PullParams,
) -> Result<Vec<PullMutation>> {
    let limit = params
        .limit
        .unwrap_or(DEFAULT_PULL_LIMIT)
        .min(MAX_PULL_LIMIT);

    let rows: Vec<PgRow> = if let Some(ref project) = params.project {
        query::<Postgres>(
            "SELECT seq, entity, entity_key, op, payload, project, occurred_at::text AS occurred_at_text \
             FROM sync_mutations \
             WHERE account_id = $1 AND seq > $2 AND project = $3 \
             ORDER BY seq ASC LIMIT $4",
        )
        .bind(account_id)
        .bind(params.since_seq)
        .bind(project)
        .bind(limit)
        .fetch_all(pool)
        .await?
    } else {
        query::<Postgres>(
            "SELECT seq, entity, entity_key, op, payload, project, occurred_at::text AS occurred_at_text \
             FROM sync_mutations \
             WHERE account_id = $1 AND seq > $2 \
             ORDER BY seq ASC LIMIT $3",
        )
        .bind(account_id)
        .bind(params.since_seq)
        .bind(limit)
        .fetch_all(pool)
        .await?
    };

    let mutations = rows
        .iter()
        .map(|row| PullMutation {
            seq: row.get("seq"),
            entity: row.get("entity"),
            entity_key: row.get("entity_key"),
            op: row.get("op"),
            payload: row.get("payload"),
            project: row.get("project"),
            occurred_at: row.get("occurred_at_text"),
        })
        .collect();

    Ok(mutations)
}

pub async fn ack_mutations(pool: &PgPool, account_id: &Uuid, up_to_seq: i64) -> Result<()> {
    if up_to_seq < 0 {
        bail!("up_to_seq must be non-negative");
    }

    query::<Postgres>(
        "UPDATE sync_mutations SET acked_at = now() \
         WHERE account_id = $1 AND seq <= $2 AND acked_at IS NULL",
    )
    .bind(account_id)
    .bind(up_to_seq)
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn enroll_project(pool: &PgPool, account_id: &Uuid, project: &str) -> Result<()> {
    query::<Postgres>(
        "INSERT INTO enrolled_projects (account_id, project) VALUES ($1, $2) ON CONFLICT DO NOTHING",
    )
    .bind(account_id)
    .bind(project)
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn unenroll_project(pool: &PgPool, account_id: &Uuid, project: &str) -> Result<()> {
    query::<Postgres>("DELETE FROM enrolled_projects WHERE account_id = $1 AND project = $2")
        .bind(account_id)
        .bind(project)
        .execute(pool)
        .await?;

    Ok(())
}

pub async fn list_projects(pool: &PgPool, account_id: &Uuid) -> Result<Vec<String>> {
    let rows: Vec<PgRow> = query::<Postgres>(
        "SELECT project FROM enrolled_projects WHERE account_id = $1 ORDER BY enrolled_at",
    )
    .bind(account_id)
    .fetch_all(pool)
    .await?;

    let projects = rows.iter().map(|row| row.get("project")).collect();
    Ok(projects)
}
