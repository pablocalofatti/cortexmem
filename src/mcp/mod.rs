pub mod protocol;
mod tools;

pub use tools::CortexMemServer;
pub use tools::StatsResult;

use anyhow::Result;

use crate::db::Database;
use crate::embed::EmbeddingManager;

pub async fn start_mcp_server(db_path: &str) -> Result<()> {
    let db_path = std::path::Path::new(db_path);
    let db = Database::open(db_path)?;
    let config = crate::config::Config::load();
    db.set_meta("embedding_model", &config.embedding.model).ok();
    let cache_dir = db_path.parent().unwrap_or(db_path);
    let embed_mgr = EmbeddingManager::new_with_model(cache_dir, &config.embedding.model);
    let server = CortexMemServer::new(db, Some(embed_mgr));

    let transport = rmcp::transport::io::stdio();
    let service = rmcp::serve_server(server, transport).await?;

    service.waiting().await?;
    Ok(())
}
