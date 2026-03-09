use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "cortexmem", version, about = "Persistent vector memory for AI coding agents")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Launch MCP server (stdio transport)
    Mcp,
    /// Show version and status
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
            let db_path = dirs::data_dir()
                .unwrap_or_else(|| std::path::PathBuf::from("."))
                .join("cortexmem")
                .join("cortexmem.db");
            std::fs::create_dir_all(db_path.parent().unwrap())?;
            cortexmem::mcp::start_mcp_server(db_path.to_str().unwrap()).await
        }
        Commands::Status => {
            println!("cortexmem v{}", env!("CARGO_PKG_VERSION"));
            Ok(())
        }
    }
}
