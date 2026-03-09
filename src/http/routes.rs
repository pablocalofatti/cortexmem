use std::sync::Arc;

use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{delete, get, patch, post},
};
use serde::{Deserialize, Serialize};

use crate::mcp::CortexMemServer;

type SharedState = Arc<CortexMemServer>;

// ── Error Type ──────────────────────────────────────────────────

pub enum AppError {
    NotFound(String),
    BadRequest(String),
    Internal(anyhow::Error),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            AppError::NotFound(msg) => (StatusCode::NOT_FOUND, msg),
            AppError::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg),
            AppError::Internal(err) => (StatusCode::INTERNAL_SERVER_ERROR, format!("{err:#}")),
        };
        (status, Json(serde_json::json!({"error": message}))).into_response()
    }
}

impl From<anyhow::Error> for AppError {
    fn from(err: anyhow::Error) -> Self {
        AppError::Internal(err)
    }
}

// ── Health ──────────────────────────────────────────────────────

pub fn health_routes() -> Router<SharedState> {
    Router::new().route("/health", get(health))
}

async fn health() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "status": "ok",
        "version": env!("CARGO_PKG_VERSION"),
    }))
}

// ── Observations ────────────────────────────────────────────────

pub fn observation_routes() -> Router<SharedState> {
    Router::new()
        .route("/observations", post(create_observation))
        .route("/observations/{id}", get(get_observation))
        .route("/observations/{id}", patch(update_observation))
        .route("/observations/{id}", delete(delete_observation))
}

#[derive(Deserialize)]
struct CreateObservationBody {
    project: String,
    title: String,
    content: String,
    #[serde(rename = "type")]
    obs_type: String,
    #[serde(default)]
    concepts: Option<Vec<String>>,
    #[serde(default)]
    facts: Option<Vec<String>>,
    #[serde(default)]
    files: Option<Vec<String>>,
    #[serde(default)]
    topic_key: Option<String>,
    #[serde(default)]
    scope: Option<String>,
}

#[derive(Serialize)]
struct SaveResponse {
    id: i64,
    status: String,
    embedded: bool,
}

async fn create_observation(
    State(state): State<SharedState>,
    Json(body): Json<CreateObservationBody>,
) -> Result<(StatusCode, Json<SaveResponse>), AppError> {
    let result = state.call_save(
        &body.project,
        &body.title,
        &body.content,
        &body.obs_type,
        body.concepts,
        body.facts,
        body.files,
        body.topic_key,
        body.scope,
    )?;
    let status = match &result.dedup_status {
        crate::memory::DedupResult::NewContent => "saved",
        crate::memory::DedupResult::HashMatch(_) => "duplicate",
        crate::memory::DedupResult::TopicKeyUpsert(_) => "upserted",
    };
    Ok((
        StatusCode::CREATED,
        Json(SaveResponse {
            id: result.id,
            status: status.to_string(),
            embedded: result.was_embedded,
        }),
    ))
}

async fn get_observation(
    State(state): State<SharedState>,
    Path(id): Path<i64>,
) -> Result<Json<serde_json::Value>, AppError> {
    match state.call_get(id)? {
        Some(obs) => Ok(Json(serde_json::to_value(obs).unwrap_or_default())),
        None => Err(AppError::NotFound(format!("Observation {id} not found"))),
    }
}

#[derive(Deserialize)]
struct UpdateBody {
    #[serde(default)]
    title: Option<String>,
    #[serde(default)]
    content: Option<String>,
    #[serde(default)]
    concepts: Option<Vec<String>>,
    #[serde(default)]
    facts: Option<Vec<String>>,
    #[serde(default)]
    files: Option<Vec<String>>,
}

async fn update_observation(
    State(state): State<SharedState>,
    Path(id): Path<i64>,
    Json(body): Json<UpdateBody>,
) -> Result<Json<serde_json::Value>, AppError> {
    state.call_update(
        id,
        body.title.as_deref(),
        body.content.as_deref(),
        body.concepts.as_ref(),
        body.facts.as_ref(),
        body.files.as_ref(),
    )?;
    Ok(Json(serde_json::json!({"updated": id})))
}

#[derive(Deserialize)]
struct DeleteQuery {
    #[serde(default)]
    hard: Option<bool>,
}

async fn delete_observation(
    State(state): State<SharedState>,
    Path(id): Path<i64>,
    Query(query): Query<DeleteQuery>,
) -> Result<Json<serde_json::Value>, AppError> {
    if query.hard.unwrap_or(false) {
        state.call_hard_delete(id)?;
        Ok(Json(serde_json::json!({"deleted": id, "mode": "hard"})))
    } else {
        state.call_delete(id)?;
        Ok(Json(serde_json::json!({"deleted": id, "mode": "soft"})))
    }
}

// ── Sessions ────────────────────────────────────────────────────

pub fn session_routes() -> Router<SharedState> {
    Router::new()
        .route("/sessions", post(create_session))
        .route("/sessions/{id}/end", post(end_session))
}

#[derive(Deserialize)]
struct CreateSessionBody {
    project: String,
    directory: String,
}

async fn create_session(
    State(state): State<SharedState>,
    Json(body): Json<CreateSessionBody>,
) -> Result<(StatusCode, Json<serde_json::Value>), AppError> {
    let id = state.call_session_start(&body.project, &body.directory)?;
    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({"session_id": id})),
    ))
}

#[derive(Deserialize)]
struct EndSessionBody {
    #[serde(default)]
    summary: Option<String>,
}

async fn end_session(
    State(state): State<SharedState>,
    Path(id): Path<i64>,
    Json(body): Json<EndSessionBody>,
) -> Result<Json<serde_json::Value>, AppError> {
    state.call_session_end(id, body.summary.as_deref())?;
    Ok(Json(serde_json::json!({"ended": id})))
}

// ── Search & Context ────────────────────────────────────────────

pub fn search_routes() -> Router<SharedState> {
    Router::new()
        .route("/search", get(search))
        .route("/context", get(context))
        .route("/timeline", get(timeline))
}

#[derive(Deserialize)]
struct SearchQuery {
    q: String,
    #[serde(default)]
    project: Option<String>,
    #[serde(rename = "type", default)]
    obs_type: Option<String>,
    #[serde(default)]
    scope: Option<String>,
    #[serde(default)]
    limit: Option<usize>,
}

async fn search(
    State(state): State<SharedState>,
    Query(query): Query<SearchQuery>,
) -> Json<serde_json::Value> {
    let results = state.call_search(
        &query.q,
        query.project.as_deref(),
        query.obs_type.as_deref(),
        query.scope.as_deref(),
        query.limit,
    );
    Json(serde_json::to_value(results).unwrap_or_default())
}

#[derive(Deserialize)]
struct ContextQuery {
    #[serde(default)]
    project: Option<String>,
    #[serde(default)]
    limit: Option<i64>,
}

async fn context(
    State(state): State<SharedState>,
    Query(query): Query<ContextQuery>,
) -> Result<Json<serde_json::Value>, AppError> {
    let limit = query.limit.unwrap_or(20);
    let observations = state.call_context(query.project.as_deref(), limit)?;
    Ok(Json(serde_json::to_value(observations).unwrap_or_default()))
}

#[derive(Deserialize)]
struct TimelineQuery {
    observation_id: i64,
    #[serde(default)]
    window: Option<i64>,
}

async fn timeline(
    State(state): State<SharedState>,
    Query(query): Query<TimelineQuery>,
) -> Result<Json<serde_json::Value>, AppError> {
    let obs = state.call_get(query.observation_id)?.ok_or_else(|| {
        AppError::NotFound(format!("Observation {} not found", query.observation_id))
    })?;
    let results = state.call_timeline(query.observation_id, query.window, &obs.project)?;
    Ok(Json(serde_json::to_value(results).unwrap_or_default()))
}

// ── Admin ───────────────────────────────────────────────────────

pub fn admin_routes() -> Router<SharedState> {
    Router::new()
        .route("/stats", get(stats))
        .route("/compact", post(compact))
}

#[derive(Deserialize)]
struct ProjectQuery {
    #[serde(default)]
    project: Option<String>,
}

async fn stats(
    State(state): State<SharedState>,
    Query(query): Query<ProjectQuery>,
) -> Result<Json<serde_json::Value>, AppError> {
    let result = state.call_stats(query.project.as_deref())?;
    Ok(Json(serde_json::json!({
        "total": result.total,
        "by_tier": result.by_tier,
        "by_type": result.by_type,
    })))
}

async fn compact(
    State(state): State<SharedState>,
    Query(query): Query<ProjectQuery>,
) -> Result<Json<serde_json::Value>, AppError> {
    let result = state.call_compact(query.project.as_deref())?;
    Ok(Json(serde_json::json!({
        "promoted": result.promoted,
        "archived": result.archived,
        "unchanged": result.unchanged,
    })))
}

// ── Prompts ─────────────────────────────────────────────────────

pub fn prompt_routes() -> Router<SharedState> {
    Router::new()
        .route("/prompts", post(save_prompt))
        .route("/prompts/recent", get(recent_prompts))
}

#[derive(Deserialize)]
struct SavePromptBody {
    content: String,
    #[serde(default)]
    project: Option<String>,
}

async fn save_prompt(
    State(state): State<SharedState>,
    Json(body): Json<SavePromptBody>,
) -> Result<(StatusCode, Json<serde_json::Value>), AppError> {
    let id = state.call_save_prompt(None, &body.content, body.project.as_deref())?;
    Ok((StatusCode::CREATED, Json(serde_json::json!({"id": id}))))
}

#[derive(Deserialize)]
struct RecentPromptsQuery {
    #[serde(default)]
    project: Option<String>,
    #[serde(default)]
    limit: Option<i64>,
}

async fn recent_prompts(
    State(state): State<SharedState>,
    Query(query): Query<RecentPromptsQuery>,
) -> Result<Json<serde_json::Value>, AppError> {
    let limit = query.limit.unwrap_or(20);
    let prompts = state.call_recent_prompts(query.project.as_deref(), limit)?;
    Ok(Json(serde_json::to_value(prompts).unwrap_or_default()))
}
