use std::path::PathBuf;

use anyhow::{Context, Result};
use dialoguer::Confirm;
use serde::{Deserialize, Serialize};

use crate::db::Observation;
use crate::db::Session;

use super::open_server;

#[derive(Debug, Serialize, Deserialize)]
pub struct ExportData {
    pub version: String,
    pub exported_at: String,
    pub project_filter: Option<String>,
    pub sessions: Vec<Session>,
    pub observations: Vec<Observation>,
}

pub fn run_export(output: Option<PathBuf>, project: Option<String>) -> Result<()> {
    let server = open_server()?;
    let mgr = server.memory_lock();
    let db = mgr.db();

    let sessions = db.list_all_sessions_for_export(project.as_deref())?;
    let observations = db.list_all_observations_for_export(project.as_deref())?;

    let exported_at: String =
        db.conn()
            .query_row("SELECT datetime('now')", [], |row| row.get(0))?;

    let export = ExportData {
        version: "1.0".into(),
        exported_at,
        project_filter: project,
        sessions,
        observations,
    };

    let json = serde_json::to_string_pretty(&export)?;
    let path = output.unwrap_or_else(|| PathBuf::from("cortexmem-export.json"));
    std::fs::write(&path, &json)?;

    println!(
        "Exported {} sessions and {} observations to {}",
        export.sessions.len(),
        export.observations.len(),
        path.display()
    );
    Ok(())
}

pub fn run_import(file: PathBuf, replace: bool) -> Result<()> {
    let contents = std::fs::read_to_string(&file)
        .with_context(|| format!("Could not read {}", file.display()))?;
    let data: ExportData =
        serde_json::from_str(&contents).context("Invalid export file format")?;

    let server = open_server()?;
    let mgr = server.memory_lock();
    let db = mgr.db();

    if replace {
        let confirm = Confirm::new()
            .with_prompt("Replace mode will DELETE all existing data. Continue?")
            .default(false)
            .interact()?;
        if !confirm {
            println!("Aborted.");
            return Ok(());
        }
        db.conn().execute_batch(
            "DELETE FROM observations_fts;
             DELETE FROM observations;
             DELETE FROM sessions;",
        )?;
        println!("Existing data cleared.");
    }

    let mut imported = 0;
    let mut skipped = 0;

    for obs in &data.observations {
        match db.import_observation(obs)? {
            true => imported += 1,
            false => skipped += 1,
        }
    }

    println!(
        "Import complete: {} imported, {} skipped (duplicates) from {}",
        imported,
        skipped,
        file.display()
    );
    Ok(())
}
