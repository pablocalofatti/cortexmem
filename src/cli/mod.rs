#[cfg(feature = "cloud")]
pub mod cloud;
pub mod export;
pub mod setup;
pub mod sync;

use std::path::PathBuf;

use anyhow::Result;

use crate::db::Database;
use crate::embed::EmbeddingManager;
use crate::mcp::CortexMemServer;

/// Resolve the database path. Respects `CORTEXMEM_DB` env var, falling back to
/// platform default (~/.local/share/cortexmem/cortexmem.db on Linux,
/// ~/Library/Application Support/cortexmem/cortexmem.db on macOS).
pub fn db_path() -> PathBuf {
    std::env::var("CORTEXMEM_DB")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            dirs::data_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join("cortexmem")
                .join("cortexmem.db")
        })
}

/// Infer project name from the current working directory basename.
pub(crate) fn detect_project() -> String {
    std::env::current_dir()
        .ok()
        .and_then(|p| p.file_name().map(|n| n.to_string_lossy().into_owned()))
        .unwrap_or_else(|| "default".into())
}

/// Open DB and create a server instance for CLI operations.
fn open_server() -> Result<CortexMemServer> {
    let path = db_path();
    std::fs::create_dir_all(path.parent().unwrap())?;

    let db = Database::open(&path)?;
    let cache_dir = path.parent().unwrap_or(&path);
    let embed_mgr = EmbeddingManager::new(cache_dir);
    Ok(CortexMemServer::new(db, Some(embed_mgr)))
}

pub fn run_save(
    title: String,
    content: String,
    obs_type: String,
    topic_key: Option<String>,
    concepts: Option<Vec<String>>,
    facts: Option<Vec<String>>,
    files: Option<Vec<String>>,
) -> Result<()> {
    let server = open_server()?;
    let project = detect_project();

    let result = server.call_save(
        &project, &title, &content, &obs_type, concepts, facts, files, topic_key, None,
    )?;

    println!(
        "Saved observation id={} (embedded={})",
        result.id, result.was_embedded
    );
    Ok(())
}

pub fn run_search(
    query: String,
    limit: usize,
    obs_type: Option<String>,
    project: Option<String>,
) -> Result<()> {
    let server = open_server()?;
    let project = project.unwrap_or_else(detect_project);

    let results = server.call_search(
        &query,
        Some(&project),
        obs_type.as_deref(),
        None,
        Some(limit),
    );

    if results.is_empty() {
        println!("No results found.");
        return Ok(());
    }

    for r in &results {
        println!(
            "[{}] {} ({}) — score: {:.4}",
            r.id, r.title, r.obs_type, r.score,
        );
    }
    Ok(())
}

pub fn run_get(id: i64) -> Result<()> {
    let server = open_server()?;
    match server.call_get(id)? {
        Some(obs) => {
            println!("# {} (id: {})", obs.title, obs.id);
            println!(
                "Type: {} | Tier: {} | Scope: {}",
                obs.obs_type, obs.tier, obs.scope
            );
            if let Some(ref concepts) = obs.concepts
                && !concepts.is_empty()
            {
                println!("Concepts: {}", concepts.join(", "));
            }
            if let Some(ref facts) = obs.facts
                && !facts.is_empty()
            {
                println!("Facts: {}", facts.join("; "));
            }
            println!(
                "Accesses: {} | Revisions: {}",
                obs.access_count, obs.revision_count
            );
            println!("Created: {} | Updated: {}", obs.created_at, obs.updated_at);
            println!("\n{}", obs.content);
        }
        None => println!("Observation {id} not found."),
    }
    Ok(())
}

pub fn run_stats() -> Result<()> {
    let server = open_server()?;
    let stats = server.call_stats(None)?;

    println!("Total observations: {}", stats.total);
    println!("\nBy tier:");
    for (tier, count) in &stats.by_tier {
        println!("  {tier}: {count}");
    }
    println!("\nBy type:");
    for (t, count) in &stats.by_type {
        println!("  {t}: {count}");
    }
    Ok(())
}

pub fn run_model_status() -> Result<()> {
    let server = open_server()?;
    let mgr = server.memory_lock();
    match mgr.embed_mgr() {
        Some(e) => {
            let status = e.model_status();
            println!("Embedding model: {status:?}");
        }
        None => println!("Embedding model: disabled"),
    }
    Ok(())
}

pub fn run_model_download() -> Result<()> {
    let server = open_server()?;
    let mgr = server.memory_lock();
    match mgr.embed_mgr() {
        Some(e) => {
            println!("Downloading embedding model...");
            e.download_model()?;
            println!("Model ready.");
        }
        None => println!("Embedding model: disabled (no cache directory)"),
    }
    Ok(())
}

pub fn run_compact() -> Result<()> {
    let server = open_server()?;
    let stats = server.call_compact(None)?;
    println!(
        "Compaction complete: {} promoted, {} archived, {} unchanged",
        stats.promoted, stats.archived, stats.unchanged,
    );
    Ok(())
}

pub fn run_delete(id: i64, hard: bool) -> Result<()> {
    let server = open_server()?;
    if hard {
        server.call_hard_delete(id)?;
        println!("Observation {id} permanently deleted.");
    } else {
        server.call_delete(id)?;
        println!("Observation {id} soft-deleted.");
    }
    Ok(())
}

pub fn run_save_prompt(content: String, project: Option<String>) -> Result<()> {
    let server = open_server()?;
    let project = project.unwrap_or_else(detect_project);
    let id = server.call_save_prompt(None, &content, Some(&project))?;
    println!("Prompt saved: id={id}");
    Ok(())
}

pub fn run_recent_prompts(project: Option<String>, limit: i64) -> Result<()> {
    let server = open_server()?;
    let project = project.unwrap_or_else(detect_project);
    let prompts = server.call_recent_prompts(Some(&project), limit)?;
    if prompts.is_empty() {
        println!("No prompts found.");
        return Ok(());
    }
    for p in &prompts {
        println!("[{}] {} — {}", p.id, p.created_at, p.content);
    }
    Ok(())
}
