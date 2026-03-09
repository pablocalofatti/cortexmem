mod protocol;
mod tools;

pub use tools::CortexMemServer;

use anyhow::Result;

use crate::db::Database;
use crate::embed::EmbeddingManager;

pub async fn start_mcp_server(db_path: &str) -> Result<()> {
    let db_path = std::path::Path::new(db_path);
    let db = Database::open(db_path)?;
    let cache_dir = db_path.parent().unwrap_or(db_path);
    let embed_mgr = EmbeddingManager::new(cache_dir);
    let server = CortexMemServer::new(db, Some(embed_mgr));

    let transport = rmcp::transport::io::stdio();
    let service = rmcp::serve_server(server, transport).await?;

    service.waiting().await?;
    Ok(())
}
