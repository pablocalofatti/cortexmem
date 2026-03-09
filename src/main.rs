use clap::{Parser, Subcommand};

#[cfg(feature = "cloud")]
use cortexmem::cli::cloud::CloudAction;

#[derive(Parser)]
#[command(
    name = "cortexmem",
    version,
    about = "Persistent vector memory for AI coding agents"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Launch MCP server (stdio transport)
    Mcp,
    /// Save an observation to memory
    Save {
        #[arg(short, long)]
        title: String,
        #[arg(short, long)]
        content: String,
        #[arg(long, default_value = "discovery")]
        r#type: String,
        #[arg(long)]
        topic_key: Option<String>,
        #[arg(long, value_delimiter = ',')]
        concepts: Option<Vec<String>>,
        #[arg(long, value_delimiter = ',')]
        facts: Option<Vec<String>>,
        #[arg(long, value_delimiter = ',')]
        files: Option<Vec<String>>,
    },
    /// Search memories
    Search {
        query: String,
        #[arg(short, long, default_value = "20")]
        limit: usize,
        #[arg(long)]
        r#type: Option<String>,
        #[arg(long)]
        project: Option<String>,
    },
    /// Get full observation by ID
    Get { id: i64 },
    /// Show database statistics
    Stats,
    /// Manage embedding model
    Model {
        #[command(subcommand)]
        action: ModelAction,
    },
    /// Run compaction (promote/archive observations by decay rules)
    Compact,
    /// Export all memories to a JSON file
    Export {
        #[arg(short, long)]
        output: Option<std::path::PathBuf>,
        #[arg(long)]
        project: Option<String>,
    },
    /// Import memories from a JSON export file
    Import {
        file: std::path::PathBuf,
        /// Replace all existing data instead of merging
        #[arg(long)]
        replace: bool,
    },
    /// Set up cortexmem for your AI agent (interactive wizard)
    Setup,
    /// Delete an observation by ID
    Delete {
        id: i64,
        /// Permanently remove from all tables (default: soft-delete only)
        #[arg(long)]
        hard: bool,
    },
    /// Save a user prompt to the prompt log
    SavePrompt {
        content: String,
        #[arg(long)]
        project: Option<String>,
    },
    /// List recent prompts for a project
    RecentPrompts {
        #[arg(long)]
        project: Option<String>,
        #[arg(short, long, default_value = "10")]
        limit: i64,
    },
    /// Start HTTP API server on the given host and port
    Serve {
        #[arg(short, long, default_value = "7437")]
        port: u16,
        #[arg(long, default_value = "127.0.0.1")]
        host: String,
    },
    #[cfg(feature = "cloud")]
    /// Cloud sync server and management
    Cloud {
        #[command(subcommand)]
        action: CloudAction,
    },
}

#[derive(Subcommand)]
enum ModelAction {
    /// Download the embedding model
    Download,
    /// Show model status
    Status,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .with_ansi(false)
        .init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Mcp => {
            tracing::info!("MCP server starting...");
            let db_path = std::env::var("CORTEXMEM_DB")
                .map(std::path::PathBuf::from)
                .unwrap_or_else(|_| {
                    dirs::data_dir()
                        .unwrap_or_else(|| std::path::PathBuf::from("."))
                        .join("cortexmem")
                        .join("cortexmem.db")
                });
            std::fs::create_dir_all(db_path.parent().unwrap())?;
            cortexmem::mcp::start_mcp_server(db_path.to_str().unwrap()).await
        }
        Commands::Save {
            title,
            content,
            r#type,
            topic_key,
            concepts,
            facts,
            files,
        } => cortexmem::cli::run_save(title, content, r#type, topic_key, concepts, facts, files),
        Commands::Search {
            query,
            limit,
            r#type,
            project,
        } => cortexmem::cli::run_search(query, limit, r#type, project),
        Commands::Get { id } => cortexmem::cli::run_get(id),
        Commands::Stats => cortexmem::cli::run_stats(),
        Commands::Model { action } => match action {
            ModelAction::Download => cortexmem::cli::run_model_download(),
            ModelAction::Status => cortexmem::cli::run_model_status(),
        },
        Commands::Compact => cortexmem::cli::run_compact(),
        Commands::Export { output, project } => cortexmem::cli::export::run_export(output, project),
        Commands::Import { file, replace } => cortexmem::cli::export::run_import(file, replace),
        Commands::Setup => cortexmem::cli::setup::run_setup(),
        Commands::Delete { id, hard } => cortexmem::cli::run_delete(id, hard),
        Commands::SavePrompt { content, project } => {
            cortexmem::cli::run_save_prompt(content, project)
        }
        Commands::RecentPrompts { project, limit } => {
            cortexmem::cli::run_recent_prompts(project, limit)
        }
        Commands::Serve { port, host } => {
            let db_path = std::env::var("CORTEXMEM_DB")
                .map(std::path::PathBuf::from)
                .unwrap_or_else(|_| {
                    dirs::data_dir()
                        .unwrap_or_else(|| std::path::PathBuf::from("."))
                        .join("cortexmem")
                        .join("cortexmem.db")
                });
            std::fs::create_dir_all(db_path.parent().unwrap())?;
            let db = cortexmem::db::Database::open(&db_path)?;
            let cache_dir = db_path.parent().unwrap().to_path_buf();
            let embed_mgr = cortexmem::embed::EmbeddingManager::new(&cache_dir);
            let server =
                std::sync::Arc::new(cortexmem::mcp::CortexMemServer::new(db, Some(embed_mgr)));
            cortexmem::http::start_http_server(server, &host, port).await
        }
        #[cfg(feature = "cloud")]
        Commands::Cloud { action } => cortexmem::cli::cloud::run_cloud(action).await,
    }
}
