use std::path::Path;

use anyhow::Result;

use crate::db::Database;
use crate::sync::git;

use super::{db_path, detect_project};

/// Default sync directory: sits next to the database file.
fn default_sync_dir() -> std::path::PathBuf {
    db_path()
        .parent()
        .map(|p| p.join("git-sync"))
        .unwrap_or_else(|| std::path::PathBuf::from("git-sync"))
}

fn open_db() -> Result<Database> {
    let path = db_path();
    std::fs::create_dir_all(path.parent().unwrap())?;
    Database::open(&path)
}

/// Initialize a git sync repository (clone remote or init local).
pub fn run_sync_init(repo: Option<&str>, path: Option<&Path>) -> Result<()> {
    let sync_dir = path
        .map(std::path::PathBuf::from)
        .unwrap_or_else(default_sync_dir);

    git::init_sync_repo(&sync_dir, repo)?;

    println!("Git sync repo initialized at {}", sync_dir.display());
    if let Some(url) = repo {
        println!("Remote: {url}");
    }
    Ok(())
}

/// Run a single sync cycle: export chunk, commit/push, pull, import.
pub fn run_sync(project: Option<&str>) -> Result<()> {
    let db = open_db()?;
    let project = project.map(String::from).unwrap_or_else(detect_project);
    let sync_dir = default_sync_dir();

    if !sync_dir.join(".git").exists() {
        anyhow::bail!("Git sync repo not initialized. Run `cortexmem git-sync init` first.");
    }

    let (exported, imported) = git::sync_via_git(&db, &sync_dir, &project)?;
    println!("Sync complete: {exported} observations exported, {imported} imported");
    Ok(())
}

/// Show current git sync status from the sync_state table.
pub fn run_sync_status() -> Result<()> {
    let db = open_db()?;
    let sync_dir = default_sync_dir();

    println!("Sync directory: {}", sync_dir.display());
    println!(
        "Initialized: {}",
        if sync_dir.join(".git").exists() {
            "yes"
        } else {
            "no"
        }
    );

    match db.get_sync_state("git:default")? {
        Some(state) => {
            println!("Last pushed seq: {}", state.last_pushed_seq);
            println!("Last pulled seq: {}", state.last_pulled_seq);
            println!("Last updated: {}", state.updated_at);
            if let Some(ref err) = state.last_error {
                println!("Last error: {err}");
            }
        }
        None => println!("No sync state recorded yet."),
    }
    Ok(())
}

/// Run auto-sync on a fixed interval (seconds). Loops forever.
pub async fn run_sync_auto(interval: u64, project: Option<&str>) -> Result<()> {
    let project = project.map(String::from).unwrap_or_else(detect_project);
    let sync_dir = default_sync_dir();

    if !sync_dir.join(".git").exists() {
        anyhow::bail!("Git sync repo not initialized. Run `cortexmem git-sync init` first.");
    }

    println!(
        "Auto-sync started for project '{}' (every {interval}s). Press Ctrl+C to stop.",
        project
    );

    loop {
        let db = open_db()?;
        match git::sync_via_git(&db, &sync_dir, &project) {
            Ok((exported, imported)) => {
                println!("Sync: {exported} exported, {imported} imported");
            }
            Err(e) => {
                eprintln!("Sync error: {e}");
            }
        }
        tokio::time::sleep(std::time::Duration::from_secs(interval)).await;
    }
}
