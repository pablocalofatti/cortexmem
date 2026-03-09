use std::sync::Arc;

use anyhow::Result;
use axum::extract::{Path, Query};
use axum::http::{HeaderMap, StatusCode};
use axum::{Json, Router, extract::State, routing};
use serde_json::{Value, json};
use sqlx_core::executor::Executor;
use sqlx_core::raw_sql::raw_sql;
use sqlx_postgres::PgPool;

use super::auth;
use super::sync;

pub struct CloudState {
    pub pool: PgPool,
    pub jwt_secret: String,
}

pub type SharedCloudState = Arc<CloudState>;

pub fn build_cloud_router(state: SharedCloudState) -> Router {
    Router::new()
        .route("/health", routing::get(health))
        .route("/auth/register", routing::post(register_handler))
        .route("/auth/login", routing::post(login_handler))
        .route("/auth/api-key", routing::post(create_api_key_handler))
        .route("/sync/push", routing::post(push_handler))
        .route("/sync/pull", routing::get(pull_handler))
        .route("/sync/ack", routing::post(ack_handler))
        .route("/projects/enroll", routing::post(enroll_handler))
        .route("/projects/:name", routing::delete(unenroll_handler))
        .route("/projects", routing::get(list_projects_handler))
        .with_state(state)
}

async fn health(State(state): State<SharedCloudState>) -> Json<Value> {
    let db_ok = state.pool.execute(raw_sql("SELECT 1")).await.is_ok();
    Json(json!({
        "status": if db_ok { "ok" } else { "degraded" },
        "version": env!("CARGO_PKG_VERSION"),
    }))
}

async fn register_handler(
    State(state): State<SharedCloudState>,
    Json(body): Json<auth::RegisterRequest>,
) -> Result<(StatusCode, Json<Value>), (StatusCode, String)> {
    let account_id = auth::register(&state.pool, &body.email, &body.password)
        .await
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;

    Ok((
        StatusCode::CREATED,
        Json(json!({
            "account_id": account_id.to_string(),
        })),
    ))
}

async fn login_handler(
    State(state): State<SharedCloudState>,
    Json(body): Json<auth::LoginRequest>,
) -> Result<(StatusCode, Json<Value>), (StatusCode, String)> {
    let response = auth::login(&state.pool, &body.email, &body.password, &state.jwt_secret)
        .await
        .map_err(|e| (StatusCode::UNAUTHORIZED, e.to_string()))?;

    Ok((
        StatusCode::OK,
        Json(json!({
            "token": response.token,
            "account_id": response.account_id,
        })),
    ))
}

fn extract_bearer(headers: &HeaderMap) -> Result<&str, (StatusCode, String)> {
    headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .ok_or((
            StatusCode::UNAUTHORIZED,
            "Missing authorization header".to_string(),
        ))
}

async fn create_api_key_handler(
    State(state): State<SharedCloudState>,
    headers: HeaderMap,
) -> Result<(StatusCode, Json<Value>), (StatusCode, String)> {
    let token = extract_bearer(&headers)?;
    let claims = auth::verify_jwt(token, &state.jwt_secret)
        .map_err(|e| (StatusCode::UNAUTHORIZED, e.to_string()))?;

    let account_id: uuid::Uuid = claims
        .sub
        .parse()
        .map_err(|e: uuid::Error| (StatusCode::UNAUTHORIZED, e.to_string()))?;

    let api_key = auth::generate_api_key(&state.pool, &account_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok((
        StatusCode::CREATED,
        Json(json!({
            "key": api_key.key,
            "prefix": api_key.prefix,
        })),
    ))
}

async fn authenticate_api_key(
    pool: &PgPool,
    headers: &HeaderMap,
) -> Result<uuid::Uuid, (StatusCode, String)> {
    let key = extract_bearer(headers)?;
    auth::verify_api_key(pool, key)
        .await
        .map_err(|e| (StatusCode::UNAUTHORIZED, e.to_string()))
}

async fn push_handler(
    State(state): State<SharedCloudState>,
    headers: HeaderMap,
    Json(body): Json<sync::PushRequest>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let account_id = authenticate_api_key(&state.pool, &headers).await?;

    let response = sync::push_mutations(&state.pool, &account_id, &body)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(json!({
        "accepted": response.accepted,
        "last_seq": response.last_seq,
    })))
}

async fn pull_handler(
    State(state): State<SharedCloudState>,
    headers: HeaderMap,
    Query(params): Query<sync::PullParams>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let account_id = authenticate_api_key(&state.pool, &headers).await?;

    let mutations = sync::pull_mutations(&state.pool, &account_id, &params)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(json!({ "mutations": mutations })))
}

async fn ack_handler(
    State(state): State<SharedCloudState>,
    headers: HeaderMap,
    Json(body): Json<sync::AckRequest>,
) -> Result<StatusCode, (StatusCode, String)> {
    let account_id = authenticate_api_key(&state.pool, &headers).await?;

    sync::ack_mutations(&state.pool, &account_id, body.up_to_seq)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(StatusCode::NO_CONTENT)
}

async fn enroll_handler(
    State(state): State<SharedCloudState>,
    headers: HeaderMap,
    Json(body): Json<sync::EnrollRequest>,
) -> Result<StatusCode, (StatusCode, String)> {
    let account_id = authenticate_api_key(&state.pool, &headers).await?;

    sync::enroll_project(&state.pool, &account_id, &body.project)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(StatusCode::CREATED)
}

async fn unenroll_handler(
    State(state): State<SharedCloudState>,
    headers: HeaderMap,
    Path(name): Path<String>,
) -> Result<StatusCode, (StatusCode, String)> {
    let account_id = authenticate_api_key(&state.pool, &headers).await?;

    sync::unenroll_project(&state.pool, &account_id, &name)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(StatusCode::NO_CONTENT)
}

async fn list_projects_handler(
    State(state): State<SharedCloudState>,
    headers: HeaderMap,
) -> Result<Json<Value>, (StatusCode, String)> {
    let account_id = authenticate_api_key(&state.pool, &headers).await?;

    let projects = sync::list_projects(&state.pool, &account_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(json!({ "projects": projects })))
}

pub async fn start_cloud_server(
    database_url: &str,
    jwt_secret: &str,
    host: &str,
    port: u16,
) -> Result<()> {
    let pool = PgPool::connect(database_url).await?;
    super::schema::run_migrations(&pool).await?;

    let state = Arc::new(CloudState {
        pool,
        jwt_secret: jwt_secret.to_string(),
    });

    let app = build_cloud_router(state);
    let listener = tokio::net::TcpListener::bind(format!("{host}:{port}")).await?;
    tracing::info!("Cloud server listening on {host}:{port}");
    axum::serve(listener, app).await?;
    Ok(())
}
