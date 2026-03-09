use std::sync::Arc;

use anyhow::Result;
use axum::http::{HeaderMap, StatusCode};
use axum::{Json, Router, extract::State, routing};
use serde_json::{Value, json};
use sqlx_core::executor::Executor;
use sqlx_core::raw_sql::raw_sql;
use sqlx_postgres::PgPool;

use super::auth;

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
