use std::path::PathBuf;

use anyhow::{Context, Result, bail};
use clap::Subcommand;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;

use super::{db_path, open_server};

#[derive(Subcommand)]
pub enum CloudAction {
    /// Start the cloud sync server
    Serve {
        #[arg(short, long, default_value = "8080")]
        port: u16,
        #[arg(long, default_value = "127.0.0.1")]
        host: String,
    },
    /// Register a new account on a cloud server
    Register {
        #[arg(long)]
        server: String,
    },
    /// Login to a cloud server
    Login {
        #[arg(long)]
        server: String,
    },
    /// Sync local data with cloud (push then pull)
    Sync,
    /// Show sync status
    SyncStatus,
    /// Generate a new API key
    ApiKey,
    /// Enroll a project for sync
    Enroll { project: String },
    /// Unenroll a project from sync
    Unenroll { project: String },
    /// List enrolled projects
    Projects,
}

// ---------------------------------------------------------------------------
// Config persistence
// ---------------------------------------------------------------------------

const CONFIG_FILE: &str = "cloud.json";

#[derive(Debug, Serialize, Deserialize, Default)]
struct CloudConfig {
    server_url: Option<String>,
    token: Option<String>,
    api_key: Option<String>,
}

fn config_dir() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("cortexmem")
}

fn load_config() -> Result<CloudConfig> {
    let path = config_dir().join(CONFIG_FILE);
    if !path.exists() {
        return Ok(CloudConfig::default());
    }
    let data = std::fs::read_to_string(&path)
        .with_context(|| format!("Failed to read config at {}", path.display()))?;
    serde_json::from_str(&data).with_context(|| "Invalid cloud config JSON")
}

fn save_config(config: &CloudConfig) -> Result<()> {
    let dir = config_dir();
    std::fs::create_dir_all(&dir)
        .with_context(|| format!("Failed to create config dir {}", dir.display()))?;
    let path = dir.join(CONFIG_FILE);
    let data = serde_json::to_string_pretty(config)?;
    std::fs::write(&path, data)
        .with_context(|| format!("Failed to write config at {}", path.display()))?;
    Ok(())
}

fn require_server_url(config: &CloudConfig) -> Result<String> {
    config
        .server_url
        .clone()
        .ok_or_else(|| anyhow::anyhow!("No server configured. Run `cortexmem cloud login` first."))
}

fn require_token(config: &CloudConfig) -> Result<String> {
    config
        .token
        .clone()
        .ok_or_else(|| anyhow::anyhow!("Not logged in. Run `cortexmem cloud login` first."))
}

fn require_api_key(config: &CloudConfig) -> Result<String> {
    config.api_key.clone().ok_or_else(|| {
        anyhow::anyhow!("No API key configured. Run `cortexmem cloud api-key` first.")
    })
}

// ---------------------------------------------------------------------------
// Dispatch
// ---------------------------------------------------------------------------

pub async fn run_cloud(action: CloudAction) -> Result<()> {
    match action {
        CloudAction::Serve { port, host } => run_serve(&host, port).await,
        CloudAction::Register { server } => run_register(&server).await,
        CloudAction::Login { server } => run_login(&server).await,
        CloudAction::Sync => run_sync().await,
        CloudAction::SyncStatus => run_sync_status(),
        CloudAction::ApiKey => run_api_key().await,
        CloudAction::Enroll { project } => run_enroll(&project).await,
        CloudAction::Unenroll { project } => run_unenroll(&project).await,
        CloudAction::Projects => run_projects().await,
    }
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

async fn run_serve(host: &str, port: u16) -> Result<()> {
    let database_url = std::env::var("DATABASE_URL")
        .context("DATABASE_URL env var is required to start the cloud server")?;
    let jwt_secret = std::env::var("JWT_SECRET")
        .context("JWT_SECRET env var is required to start the cloud server")?;

    println!("Starting cloud server on {host}:{port}...");
    crate::cloud::server::start_cloud_server(&database_url, &jwt_secret, host, port).await
}

async fn run_register(server: &str) -> Result<()> {
    let email: String = dialoguer::Input::new()
        .with_prompt("Email")
        .interact_text()?;
    let password: String = dialoguer::Password::new()
        .with_prompt("Password")
        .with_confirmation("Confirm password", "Passwords do not match")
        .interact()?;

    let client = Client::new();
    let resp = client
        .post(format!("{server}/auth/register"))
        .json(&json!({ "email": email, "password": password }))
        .send()
        .await?;

    if !resp.status().is_success() {
        let body = resp.text().await.unwrap_or_default();
        bail!("Registration failed: {body}");
    }

    let body: serde_json::Value = resp.json().await?;
    let account_id = body
        .get("account_id")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");
    println!("Registered account: {account_id}");

    // Auto-login after registration
    println!("Logging in...");
    do_login(server, &email, &password).await
}

async fn run_login(server: &str) -> Result<()> {
    let email: String = dialoguer::Input::new()
        .with_prompt("Email")
        .interact_text()?;
    let password: String = dialoguer::Password::new()
        .with_prompt("Password")
        .interact()?;

    do_login(server, &email, &password).await
}

async fn do_login(server: &str, email: &str, password: &str) -> Result<()> {
    let client = Client::new();
    let resp = client
        .post(format!("{server}/auth/login"))
        .json(&json!({ "email": email, "password": password }))
        .send()
        .await?;

    if !resp.status().is_success() {
        let body = resp.text().await.unwrap_or_default();
        bail!("Login failed: {body}");
    }

    #[derive(Deserialize)]
    struct LoginResp {
        token: String,
        #[allow(dead_code)]
        account_id: String,
    }

    let body: LoginResp = resp.json().await?;

    let mut config = load_config()?;
    config.server_url = Some(server.to_string());
    config.token = Some(body.token);
    save_config(&config)?;

    println!(
        "Logged in. Config saved to {}",
        config_dir().join(CONFIG_FILE).display()
    );
    Ok(())
}

async fn run_sync() -> Result<()> {
    let config = load_config()?;
    let server_url = require_server_url(&config)?;
    let api_key = require_api_key(&config)?;

    let sync_config = crate::sync::engine::SyncConfig {
        server_url,
        api_key,
    };

    let path = db_path();
    std::fs::create_dir_all(path.parent().unwrap_or(&path))?;
    let db = crate::db::Database::open(&path)?;

    let (pushed, pulled) = crate::sync::engine::sync_once(&db, &sync_config).await?;
    println!("Sync complete: {pushed} pushed, {pulled} pulled");
    Ok(())
}

fn run_sync_status() -> Result<()> {
    let server = open_server()?;
    let mgr = server.memory_lock();
    let state = mgr.db().get_sync_state("cloud")?;

    match state {
        Some(s) => {
            println!("Sync target: {}", s.target_key);
            println!("Last pushed seq: {}", s.last_pushed_seq);
            println!("Last pulled seq: {}", s.last_pulled_seq);
            if let Some(ref err) = s.last_error {
                println!("Last error: {err}");
            }
            println!("Updated at: {}", s.updated_at);
        }
        None => println!("No sync state found. Run `cortexmem cloud sync` first."),
    }
    Ok(())
}

async fn run_api_key() -> Result<()> {
    let config = load_config()?;
    let server_url = require_server_url(&config)?;
    let token = require_token(&config)?;

    let client = Client::new();
    let resp = client
        .post(format!("{server_url}/auth/api-key"))
        .bearer_auth(&token)
        .send()
        .await?;

    if !resp.status().is_success() {
        let body = resp.text().await.unwrap_or_default();
        bail!("Failed to generate API key: {body}");
    }

    #[derive(Deserialize)]
    struct ApiKeyResp {
        key: String,
        prefix: String,
    }

    let body: ApiKeyResp = resp.json().await?;

    let mut config = load_config()?;
    config.api_key = Some(body.key);
    save_config(&config)?;

    println!(
        "API key generated (prefix: {}). Saved to config.",
        body.prefix
    );
    Ok(())
}

async fn run_enroll(project: &str) -> Result<()> {
    let config = load_config()?;
    let server_url = require_server_url(&config)?;
    let api_key = require_api_key(&config)?;

    let client = Client::new();
    let resp = client
        .post(format!("{server_url}/projects/enroll"))
        .bearer_auth(&api_key)
        .json(&json!({ "project": project }))
        .send()
        .await?;

    if !resp.status().is_success() {
        let body = resp.text().await.unwrap_or_default();
        bail!("Failed to enroll project: {body}");
    }

    println!("Project '{project}' enrolled for sync.");
    Ok(())
}

async fn run_unenroll(project: &str) -> Result<()> {
    let config = load_config()?;
    let server_url = require_server_url(&config)?;
    let api_key = require_api_key(&config)?;

    let client = Client::new();
    let resp = client
        .delete(format!("{server_url}/projects/{project}"))
        .bearer_auth(&api_key)
        .send()
        .await?;

    if !resp.status().is_success() {
        let body = resp.text().await.unwrap_or_default();
        bail!("Failed to unenroll project: {body}");
    }

    println!("Project '{project}' unenrolled from sync.");
    Ok(())
}

async fn run_projects() -> Result<()> {
    let config = load_config()?;
    let server_url = require_server_url(&config)?;
    let api_key = require_api_key(&config)?;

    let client = Client::new();
    let resp = client
        .get(format!("{server_url}/projects"))
        .bearer_auth(&api_key)
        .send()
        .await?;

    if !resp.status().is_success() {
        let body = resp.text().await.unwrap_or_default();
        bail!("Failed to list projects: {body}");
    }

    #[derive(Deserialize)]
    struct ProjectsResp {
        projects: Vec<String>,
    }

    let body: ProjectsResp = resp.json().await?;

    if body.projects.is_empty() {
        println!("No projects enrolled.");
    } else {
        println!("Enrolled projects:");
        for p in &body.projects {
            println!("  - {p}");
        }
    }
    Ok(())
}
