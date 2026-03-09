use std::fmt;

use crate::db::schema::CURRENT_VERSION;
use crate::embed::ModelStatus;
use crate::mcp::CortexMemServer;

/// Outcome severity for a single diagnostic check.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CheckStatus {
    Ok,
    Warn,
    Fail,
}

impl fmt::Display for CheckStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Ok => write!(f, "OK"),
            Self::Warn => write!(f, "WARN"),
            Self::Fail => write!(f, "FAIL"),
        }
    }
}

/// Result of a single diagnostic check.
#[derive(Debug)]
pub struct CheckResult {
    pub name: &'static str,
    pub status: CheckStatus,
    pub detail: String,
}

impl CheckResult {
    pub fn passed(&self) -> bool {
        self.status == CheckStatus::Ok
    }
}

/// Run all diagnostic checks against the given server instance.
pub fn run_checks(server: &CortexMemServer) -> Vec<CheckResult> {
    vec![
        check_binary_version(),
        check_database(server),
        check_schema_version(server),
        check_embedding_model(server),
        check_fts_index(server),
        check_vector_index(server),
        check_mcp_config(),
        check_cloud_sync(),
        check_git_sync(),
    ]
}

/// Print formatted diagnostic output to stdout.
pub fn print_results(results: &[CheckResult]) {
    let version = env!("CARGO_PKG_VERSION");
    println!("cortexmem doctor v{version}");
    println!();

    for r in results {
        let tag = match r.status {
            CheckStatus::Ok => "OK",
            CheckStatus::Warn => "WARN",
            CheckStatus::Fail => "FAIL",
        };
        println!("  [{tag:^4}] {}: {}", r.name, r.detail);
    }

    let passed = results
        .iter()
        .filter(|r| r.status == CheckStatus::Ok)
        .count();
    let failed = results
        .iter()
        .filter(|r| r.status == CheckStatus::Fail)
        .count();
    let warnings = results
        .iter()
        .filter(|r| r.status == CheckStatus::Warn)
        .count();

    println!();
    println!("{passed} checks passed, {failed} failed, {warnings} warnings");
}

fn check_binary_version() -> CheckResult {
    let version = env!("CARGO_PKG_VERSION");
    CheckResult {
        name: "Binary version",
        status: CheckStatus::Ok,
        detail: format!("v{version}"),
    }
}

fn check_database(server: &CortexMemServer) -> CheckResult {
    match server.call_stats(None) {
        Ok(stats) => CheckResult {
            name: "Database",
            status: CheckStatus::Ok,
            detail: format!("{} observations", stats.total),
        },
        Err(e) => CheckResult {
            name: "Database",
            status: CheckStatus::Fail,
            detail: format!("error: {e}"),
        },
    }
}

fn check_schema_version(server: &CortexMemServer) -> CheckResult {
    let mgr = server.memory_lock();
    let db = mgr.db();
    match db.schema_version() {
        Ok(v) if v == CURRENT_VERSION => CheckResult {
            name: "Schema version",
            status: CheckStatus::Ok,
            detail: format!("v{v} (current)"),
        },
        Ok(v) => CheckResult {
            name: "Schema version",
            status: CheckStatus::Warn,
            detail: format!("v{v} (expected v{CURRENT_VERSION})"),
        },
        Err(e) => CheckResult {
            name: "Schema version",
            status: CheckStatus::Fail,
            detail: format!("error: {e}"),
        },
    }
}

fn check_embedding_model(server: &CortexMemServer) -> CheckResult {
    let mgr = server.memory_lock();
    match mgr.embed_mgr() {
        Some(e) => match e.model_status() {
            ModelStatus::Ready => CheckResult {
                name: "Embedding model",
                status: CheckStatus::Ok,
                detail: "ready".into(),
            },
            ModelStatus::NotDownloaded => CheckResult {
                name: "Embedding model",
                status: CheckStatus::Warn,
                detail: "not downloaded (run `cortexmem model download`)".into(),
            },
        },
        None => CheckResult {
            name: "Embedding model",
            status: CheckStatus::Warn,
            detail: "disabled (no cache directory)".into(),
        },
    }
}

fn check_fts_index(server: &CortexMemServer) -> CheckResult {
    let mgr = server.memory_lock();
    let db = mgr.db();
    match db.count_fts_entries() {
        Ok(count) => CheckResult {
            name: "FTS5 index",
            status: CheckStatus::Ok,
            detail: format!("{count} entries"),
        },
        Err(e) => CheckResult {
            name: "FTS5 index",
            status: CheckStatus::Fail,
            detail: format!("error: {e}"),
        },
    }
}

fn check_vector_index(server: &CortexMemServer) -> CheckResult {
    let mgr = server.memory_lock();
    let db = mgr.db();
    match db.count_vector_entries() {
        Ok(count) => CheckResult {
            name: "Vector index",
            status: CheckStatus::Ok,
            detail: format!("{count} entries"),
        },
        Err(e) => CheckResult {
            name: "Vector index",
            status: CheckStatus::Fail,
            detail: format!("error: {e}"),
        },
    }
}

fn check_mcp_config() -> CheckResult {
    let config_paths = mcp_config_paths();
    for path in &config_paths {
        if let Ok(contents) = std::fs::read_to_string(path)
            && contents.contains("cortexmem")
        {
            return CheckResult {
                name: "MCP config",
                status: CheckStatus::Ok,
                detail: format!("found in {}", path.display()),
            };
        }
    }
    CheckResult {
        name: "MCP config",
        status: CheckStatus::Warn,
        detail: "not found in any known agent config".into(),
    }
}

fn mcp_config_paths() -> Vec<std::path::PathBuf> {
    let mut paths = Vec::new();
    if let Some(home) = dirs::home_dir() {
        paths.push(home.join(".claude.json"));
        paths.push(home.join(".claude/settings.json"));
        paths.push(home.join(".cursor/mcp.json"));
        paths.push(home.join(".config/Code/User/settings.json"));
    }
    paths
}

fn check_cloud_sync() -> CheckResult {
    match std::env::var("CORTEXMEM_CLOUD_URL") {
        Ok(url) if !url.is_empty() => CheckResult {
            name: "Cloud sync",
            status: CheckStatus::Ok,
            detail: format!("configured ({url})"),
        },
        _ => CheckResult {
            name: "Cloud sync",
            status: CheckStatus::Warn,
            detail: "not configured".into(),
        },
    }
}

fn check_git_sync() -> CheckResult {
    let sync_path = dirs::home_dir()
        .map(|h| h.join(".cortexmem/sync/.git"))
        .unwrap_or_default();

    if sync_path.exists() {
        CheckResult {
            name: "Git sync",
            status: CheckStatus::Ok,
            detail: "initialized".into(),
        }
    } else {
        CheckResult {
            name: "Git sync",
            status: CheckStatus::Warn,
            detail: "not initialized".into(),
        }
    }
}
