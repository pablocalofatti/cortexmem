mod routes;

use std::sync::Arc;

use axum::Router;
use tower_http::cors::{Any, CorsLayer};

use crate::mcp::CortexMemServer;

pub use routes::AppError;

pub type SharedState = Arc<CortexMemServer>;

pub fn build_router(state: SharedState) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    Router::new()
        .merge(routes::health_routes())
        .merge(routes::observation_routes())
        .merge(routes::session_routes())
        .merge(routes::search_routes())
        .merge(routes::admin_routes())
        .merge(routes::prompt_routes())
        .layer(cors)
        .with_state(state)
}

pub async fn start_http_server(state: SharedState, host: &str, port: u16) -> anyhow::Result<()> {
    let app = build_router(state);
    let addr = format!("{host}:{port}");
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    tracing::info!("HTTP server listening on {addr}");
    axum::serve(listener, app).await?;
    Ok(())
}
