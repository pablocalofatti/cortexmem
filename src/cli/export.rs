use std::path::PathBuf;

use anyhow::Result;
use serde::Serialize;

use crate::db::Observation;
use crate::db::Session;

use super::open_server;

#[derive(Debug, Serialize)]
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
