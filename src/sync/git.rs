use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::db::Database;
use crate::db::Observation;
use crate::db::Session;

#[derive(Debug, Serialize, Deserialize)]
pub struct SyncChunk {
    pub chunk_id: String,
    pub source: String,
    pub project: String,
    pub exported_at: String,
    pub observations: Vec<Observation>,
    pub sessions: Vec<Session>,
}

pub fn create_chunk(db: &Database, project: Option<&str>) -> Result<SyncChunk> {
    let observations = db.list_all_observations_for_export(project)?;
    let sessions = db.list_all_sessions_for_export(project)?;

    let hostname = std::env::var("HOSTNAME")
        .or_else(|_| std::env::var("COMPUTERNAME"))
        .unwrap_or_else(|_| "unknown".to_string());

    Ok(SyncChunk {
        chunk_id: uuid::Uuid::new_v4().to_string(),
        source: hostname,
        project: project.unwrap_or("all").to_string(),
        exported_at: chrono::Utc::now().to_rfc3339(),
        observations,
        sessions,
    })
}

pub fn import_chunk(db: &Database, json: &str) -> Result<usize> {
    let chunk: SyncChunk = serde_json::from_str(json)?;

    // Check if chunk was already imported
    if !db.record_sync_chunk(&chunk.chunk_id)? {
        return Ok(0); // Already imported
    }

    let mut count = 0;
    for obs in &chunk.observations {
        if db.import_observation(obs)? {
            count += 1;
        }
    }
    Ok(count)
}

/// Full git sync cycle: export, commit+push, pull, import
pub fn sync_via_git(
    db: &Database,
    sync_dir: &std::path::Path,
    project: &str,
) -> Result<(usize, usize)> {
    use std::fs;
    use std::process::Command;

    let chunks_dir = sync_dir.join("chunks").join(project);
    fs::create_dir_all(&chunks_dir)?;

    // 1. Export
    let chunk = create_chunk(db, Some(project))?;
    let chunk_file = chunks_dir.join(format!("{}.json", chunk.chunk_id));
    fs::write(&chunk_file, serde_json::to_string_pretty(&chunk)?)?;
    let exported = chunk.observations.len();

    // 2. Git add + commit + push
    Command::new("git")
        .args(["add", "."])
        .current_dir(sync_dir)
        .output()?;
    let commit_result = Command::new("git")
        .args([
            "commit",
            "-m",
            &format!("sync: {project} chunk {}", chunk.chunk_id),
        ])
        .current_dir(sync_dir)
        .output()?;
    if commit_result.status.success() {
        Command::new("git")
            .args(["push"])
            .current_dir(sync_dir)
            .output()?;
    }

    // 3. Git pull
    Command::new("git")
        .args(["pull", "--rebase"])
        .current_dir(sync_dir)
        .output()?;

    // 4. Import new chunks
    let mut imported = 0;
    for entry in fs::read_dir(&chunks_dir)? {
        let entry = entry?;
        if entry.path().extension().is_some_and(|e| e == "json") {
            let content = fs::read_to_string(entry.path())?;
            imported += import_chunk(db, &content)?;
        }
    }

    Ok((exported, imported))
}

pub fn init_sync_repo(sync_dir: &std::path::Path, remote_url: Option<&str>) -> Result<()> {
    use std::fs;
    use std::process::Command;

    if sync_dir.join(".git").exists() {
        return Ok(());
    }

    fs::create_dir_all(sync_dir)?;
    if let Some(url) = remote_url {
        Command::new("git")
            .args(["clone", url, "."])
            .current_dir(sync_dir)
            .output()?;
    } else {
        Command::new("git")
            .args(["init"])
            .current_dir(sync_dir)
            .output()?;
    }
    Ok(())
}
