use std::sync::Arc;

use anyhow::Result;
use axum::{Json, Router, extract::State, routing};
use serde_json::json;
use sqlx_core::executor::Executor;
use sqlx_core::raw_sql::raw_sql;
use sqlx_postgres::PgPool;

pub struct CloudState {
    pub pool: PgPool,
    pub jwt_secret: String,
}

pub type SharedCloudState = Arc<CloudState>;

pub fn build_cloud_router(state: SharedCloudState) -> Router {
    Router::new()
        .route("/health", routing::get(health))
        .with_state(state)
}

async fn health(State(state): State<SharedCloudState>) -> Json<serde_json::Value> {
    let db_ok = state.pool.execute(raw_sql("SELECT 1")).await.is_ok();
    Json(json!({
        "status": if db_ok { "ok" } else { "degraded" },
        "version": env!("CARGO_PKG_VERSION"),
    }))
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
