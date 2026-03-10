use std::sync::Mutex;

use anyhow::Result;
use rmcp::{
    ServerHandler,
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{ServerCapabilities, ServerInfo, ToolsCapability},
    tool, tool_handler, tool_router,
};
use schemars::JsonSchema;
use serde::Deserialize;

use crate::db::{Database, NewObservation, Observation, Prompt, Session};
use crate::embed::{EmbeddingManager, ModelStatus};
use crate::memory::{CompactionStats, DedupResult, MemoryManager, SaveResult};

use super::protocol;

// ── Parameter Types ──────────────────────────────────────────────

#[derive(Debug, Deserialize, JsonSchema)]
pub struct MemSaveParams {
    pub project: String,
    pub title: String,
    pub content: String,
    #[serde(rename = "type")]
    pub obs_type: String,
    #[serde(default)]
    pub concepts: Option<Vec<String>>,
    #[serde(default)]
    pub facts: Option<Vec<String>>,
    #[serde(default)]
    pub files: Option<Vec<String>>,
    #[serde(default)]
    pub topic_key: Option<String>,
    #[serde(default)]
    pub scope: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct MemUpdateParams {
    pub id: i64,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub content: Option<String>,
    #[serde(default)]
    pub concepts: Option<Vec<String>>,
    #[serde(default)]
    pub facts: Option<Vec<String>>,
    #[serde(default)]
    pub files: Option<Vec<String>>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct MemSessionSummaryParams {
    pub summary: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct MemSearchParams {
    pub query: String,
    #[serde(default)]
    pub project: Option<String>,
    #[serde(rename = "type", default)]
    pub obs_type: Option<String>,
    #[serde(default)]
    pub scope: Option<String>,
    #[serde(default)]
    pub limit: Option<i64>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct MemGetParams {
    #[serde(default)]
    pub id: Option<i64>,
    #[serde(default)]
    pub ids: Option<Vec<i64>>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct MemTimelineParams {
    pub id: i64,
    #[serde(default)]
    pub window: Option<i64>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct MemContextParams {
    #[serde(default)]
    pub project: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct MemSuggestTopicParams {
    pub title: String,
    #[serde(default)]
    #[allow(dead_code)]
    pub content: Option<String>,
    #[serde(rename = "type", default)]
    pub obs_type: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct MemSessionStartParams {
    pub project: String,
    pub directory: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct MemSessionEndParams {
    #[serde(default)]
    pub summary: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct MemDeleteParams {
    pub id: i64,
    #[serde(default)]
    pub hard: Option<bool>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct MemStatsParams {
    #[serde(default)]
    pub project: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct MemCompactParams {
    #[serde(default)]
    pub project: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct MemModelParams {}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct MemSavePromptParams {
    pub content: String,
    #[serde(default)]
    pub project: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct MemRecentPromptsParams {
    #[serde(default)]
    pub project: Option<String>,
    #[serde(default)]
    pub limit: Option<i64>,
}

// ── Helpers ──────────────────────────────────────────────────────

fn generate_topic_key(obs_type: &str, title: &str) -> String {
    let family = match obs_type {
        "architecture" => "architecture",
        "decision" => "decision",
        "bug_fix" => "bug",
        "pattern" => "pattern",
        "config" => "config",
        "discovery" => "discovery",
        "learning" => "learning",
        "milestone" => "milestone",
        _ => "general",
    };
    let slug: String = title
        .to_lowercase()
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == ' ' {
                c
            } else {
                ' '
            }
        })
        .collect::<String>()
        .split_whitespace()
        .take(4)
        .collect::<Vec<_>>()
        .join("-");
    format!("{family}/{slug}")
}

// ── Server ───────────────────────────────────────────────────────

pub struct CortexMemServer {
    tool_router: ToolRouter<Self>,
    memory: Mutex<MemoryManager>,
    current_session: Mutex<Option<i64>>,
    last_search_query: Mutex<Option<String>>,
}

#[tool_handler(router = self.tool_router)]
impl ServerHandler for CortexMemServer {
    fn get_info(&self) -> ServerInfo {
        let mut capabilities = ServerCapabilities::default();
        capabilities.tools = Some(ToolsCapability::default());
        ServerInfo::new(capabilities).with_instructions(
            "Persistent vector memory for AI coding agents. \
             Use mem_save to store observations and mem_search to retrieve them.",
        )
    }
}

#[tool_router]
impl CortexMemServer {
    // ── Write Tools ──────────────────────────────────────────

    #[tool(
        name = "mem_save",
        description = "Save an observation (decision, pattern, bug fix, etc.) to persistent memory. Supports dedup via content hash and topic_key upsert."
    )]
    async fn mem_save(&self, Parameters(params): Parameters<MemSaveParams>) -> String {
        match self.call_save(
            &params.project,
            &params.title,
            &params.content,
            &params.obs_type,
            params.concepts,
            params.facts,
            params.files,
            params.topic_key,
            params.scope,
        ) {
            Ok(result) => {
                let status = match &result.dedup_status {
                    DedupResult::NewContent => "saved",
                    DedupResult::HashMatch(_) => "duplicate detected (skipped)",
                    DedupResult::TopicKeyUpsert(_) => "updated (topic_key upsert)",
                };
                format!(
                    "Observation {}: id={}, embedded={}",
                    status, result.id, result.was_embedded
                )
            }
            Err(e) => format!("Error saving observation: {e}"),
        }
    }

    #[tool(
        name = "mem_update",
        description = "Update fields of an existing observation by ID. Recomputes content hash and re-embeds."
    )]
    async fn mem_update(&self, Parameters(params): Parameters<MemUpdateParams>) -> String {
        match self.call_update(
            params.id,
            params.title.as_deref(),
            params.content.as_deref(),
            params.concepts.as_ref(),
            params.facts.as_ref(),
            params.files.as_ref(),
        ) {
            Ok(()) => format!("Observation {} updated.", params.id),
            Err(e) => format!("Error updating observation: {e}"),
        }
    }

    #[tool(
        name = "mem_session_summary",
        description = "Persist a compaction summary for the current session. Call this when context is about to be compacted."
    )]
    async fn mem_session_summary(
        &self,
        Parameters(params): Parameters<MemSessionSummaryParams>,
    ) -> String {
        let session_id = self.current_session.lock().unwrap();
        match *session_id {
            Some(id) => match self.call_session_summary(id, &params.summary) {
                Ok(()) => format!("Session {id} summary saved."),
                Err(e) => format!("Error saving summary: {e}"),
            },
            None => "No active session. Call mem_session_start first.".to_string(),
        }
    }

    // ── Read Tools ───────────────────────────────────────────

    #[tool(
        name = "mem_search",
        description = "Search memory using hybrid FTS5 + vector similarity with RRF fusion. Returns compact results (id, title, type, concepts)."
    )]
    async fn mem_search(&self, Parameters(params): Parameters<MemSearchParams>) -> String {
        let results = self.call_search(
            &params.query,
            params.project.as_deref(),
            params.obs_type.as_deref(),
            params.scope.as_deref(),
            params.limit.map(|l| l as usize),
        );

        // Store the query for feedback recording on subsequent mem_get
        *self.last_search_query.lock().unwrap() = Some(params.query.clone());

        if results.is_empty() {
            return "No results found.".to_string();
        }

        let mut out = String::new();
        for r in &results {
            out.push_str(&format!(
                "[{}] {} ({}){} — score: {:.4}\n",
                r.id,
                r.title,
                r.obs_type,
                r.concepts
                    .as_ref()
                    .map(|c| format!(" — {}", c.join(", ")))
                    .unwrap_or_default(),
                r.score,
            ));
        }
        out
    }

    #[tool(
        name = "mem_get",
        description = "Get full observation detail by ID or multiple IDs. Returns all fields including content, facts, files."
    )]
    async fn mem_get(&self, Parameters(params): Parameters<MemGetParams>) -> String {
        let ids = if let Some(id) = params.id {
            vec![id]
        } else if let Some(ids) = params.ids {
            ids
        } else {
            return "Provide either 'id' or 'ids' parameter.".to_string();
        };

        match self.call_get_multiple(&ids) {
            Ok(observations) => {
                // Track access and record search feedback for each
                let last_query = self.last_search_query.lock().unwrap().take();
                let session_id = *self.current_session.lock().unwrap();
                for obs in &observations {
                    let _ = self.track_access(obs.id);
                    if let Some(ref query) = last_query {
                        let mgr = self.memory.lock().unwrap();
                        mgr.db()
                            .record_search_feedback(query, obs.id, session_id)
                            .ok();
                    }
                }
                if observations.is_empty() {
                    return "No observations found.".to_string();
                }
                observations
                    .iter()
                    .map(protocol::format_full)
                    .collect::<Vec<_>>()
                    .join("\n---\n\n")
            }
            Err(e) => format!("Error getting observations: {e}"),
        }
    }

    #[tool(
        name = "mem_timeline",
        description = "Get chronological context around a target observation. Shows what was saved before and after."
    )]
    async fn mem_timeline(&self, Parameters(params): Parameters<MemTimelineParams>) -> String {
        let window = params.window.unwrap_or(5);

        // Need project context — get it from the observation
        let mgr = self.memory.lock().unwrap();
        let obs = match mgr.db().get_observation(params.id) {
            Ok(Some(obs)) => obs,
            Ok(None) => return format!("Observation {} not found.", params.id),
            Err(e) => return format!("Error: {e}"),
        };

        match mgr.db().get_timeline(&obs.project, params.id, window) {
            Ok(timeline) => {
                if timeline.is_empty() {
                    return "No timeline context found.".to_string();
                }
                protocol::format_compact(&timeline)
            }
            Err(e) => format!("Error getting timeline: {e}"),
        }
    }

    #[tool(
        name = "mem_context",
        description = "Get recent observations and prompts from previous sessions for the current project. Use at session start for context recovery."
    )]
    async fn mem_context(&self, Parameters(params): Parameters<MemContextParams>) -> String {
        const OBS_LIMIT: i64 = 20;
        const PROMPT_LIMIT: i64 = 10;
        let project = params.project.as_deref();

        let obs_section = match self.call_context(project, OBS_LIMIT) {
            Ok(observations) if !observations.is_empty() => {
                format!(
                    "## Recent Observations\n{}",
                    protocol::format_compact(&observations)
                )
            }
            Ok(_) => String::new(),
            Err(e) => format!("Error getting observations: {e}"),
        };

        let prompt_section = match self.call_recent_prompts(project, PROMPT_LIMIT) {
            Ok(prompts) => protocol::format_prompts(&prompts),
            Err(e) => format!("Error getting prompts: {e}"),
        };

        match (obs_section.is_empty(), prompt_section.is_empty()) {
            (true, true) => "No previous context found.".to_string(),
            (false, true) => obs_section,
            (true, false) => prompt_section,
            (false, false) => format!("{obs_section}\n{prompt_section}"),
        }
    }

    #[tool(
        name = "mem_suggest_topic",
        description = "Generate a topic_key for an observation and find matching existing keys. Use before mem_save to enable upsert behavior."
    )]
    async fn mem_suggest_topic(
        &self,
        Parameters(params): Parameters<MemSuggestTopicParams>,
    ) -> String {
        let obs_type = params.obs_type.as_deref().unwrap_or("general");
        self.call_suggest_topic(obs_type, &params.title)
    }

    // ── Lifecycle Tools ──────────────────────────────────────

    #[tool(
        name = "mem_session_start",
        description = "Start a new memory session for a project. Creates session record and returns recent context."
    )]
    async fn mem_session_start(
        &self,
        Parameters(params): Parameters<MemSessionStartParams>,
    ) -> String {
        match self.call_session_start(&params.project, &params.directory) {
            Ok(session_id) => {
                // Return recent context alongside session ID
                let context = self
                    .call_context(Some(&params.project), 10)
                    .map(|obs| protocol::format_compact(&obs))
                    .unwrap_or_default();

                format!(
                    "Session {session_id} started for project '{}'.\n\n{context}",
                    params.project
                )
            }
            Err(e) => format!("Error starting session: {e}"),
        }
    }

    #[tool(
        name = "mem_session_end",
        description = "End the current session. Optionally stores a session summary and triggers decay cycle."
    )]
    async fn mem_session_end(&self, Parameters(params): Parameters<MemSessionEndParams>) -> String {
        let session_id = {
            let guard = self.current_session.lock().unwrap();
            *guard
        };

        match session_id {
            Some(id) => match self.call_session_end(id, params.summary.as_deref()) {
                Ok(()) => {
                    *self.current_session.lock().unwrap() = None;
                    format!("Session {id} ended.")
                }
                Err(e) => format!("Error ending session: {e}"),
            },
            None => "No active session to end.".to_string(),
        }
    }

    // ── Admin Tools ──────────────────────────────────────────

    #[tool(
        name = "mem_delete",
        description = "Delete an observation by ID. Soft-delete by default (sets deleted_at, recoverable). Pass hard=true to permanently remove from all tables."
    )]
    async fn mem_delete(&self, Parameters(params): Parameters<MemDeleteParams>) -> String {
        if params.hard.unwrap_or(false) {
            match self.call_hard_delete(params.id) {
                Ok(()) => format!("Observation {} permanently deleted.", params.id),
                Err(e) => format!("Error hard-deleting observation: {e}"),
            }
        } else {
            match self.call_delete(params.id) {
                Ok(()) => format!("Observation {} soft-deleted.", params.id),
                Err(e) => format!("Error deleting observation: {e}"),
            }
        }
    }

    #[tool(
        name = "mem_stats",
        description = "Show memory statistics: counts by type/tier, database size, embedding model status."
    )]
    async fn mem_stats(&self, Parameters(params): Parameters<MemStatsParams>) -> String {
        match self.call_stats(params.project.as_deref()) {
            Ok(stats) => {
                let model_status = {
                    let mgr = self.memory.lock().unwrap();
                    match mgr.embed_mgr() {
                        Some(e) => match e.model_status() {
                            ModelStatus::Ready => "ready",
                            ModelStatus::NotDownloaded => "not downloaded",
                        },
                        None => "disabled",
                    }
                };

                protocol::format_stats(
                    params.project.as_deref().unwrap_or("all"),
                    stats.total,
                    &stats.by_tier,
                    &stats.by_type,
                    model_status,
                )
            }
            Err(e) => format!("Error getting stats: {e}"),
        }
    }

    #[tool(
        name = "mem_compact",
        description = "Run decay cycle: promote frequently accessed observations, archive stale ones. Returns stats."
    )]
    async fn mem_compact(&self, Parameters(params): Parameters<MemCompactParams>) -> String {
        match self.call_compact(params.project.as_deref()) {
            Ok(stats) => protocol::format_compaction(&stats),
            Err(e) => format!("Error running compaction: {e}"),
        }
    }

    #[tool(
        name = "mem_model",
        description = "Check or download the embedding model. Shows model status and triggers download if needed."
    )]
    async fn mem_model(&self, Parameters(_params): Parameters<MemModelParams>) -> String {
        let mgr = self.memory.lock().unwrap();
        match mgr.embed_mgr() {
            Some(e) => match e.model_status() {
                ModelStatus::Ready => "Embedding model: ready (all-MiniLM-L6-v2)".to_string(),
                ModelStatus::NotDownloaded => {
                    drop(mgr);
                    match self.call_download_model() {
                        Ok(()) => "Embedding model downloaded and ready.".to_string(),
                        Err(e) => format!("Error downloading model: {e}"),
                    }
                }
            },
            None => "Embedding model: disabled (no cache directory configured)".to_string(),
        }
    }

    #[tool(
        name = "mem_save_prompt",
        description = "Save a user prompt to the prompt log for the current project. Used to track what tasks were requested."
    )]
    async fn mem_save_prompt(&self, Parameters(params): Parameters<MemSavePromptParams>) -> String {
        let session_id = *self.current_session.lock().unwrap();
        let project = params.project.clone();
        match self.call_save_prompt(session_id, &params.content, project.as_deref()) {
            Ok(id) => format!("Prompt saved: id={id}"),
            Err(e) => format!("Error saving prompt: {e}"),
        }
    }

    #[tool(
        name = "mem_recent_prompts",
        description = "Retrieve recent user prompts for the current project. Use to recall what was asked in previous sessions."
    )]
    async fn mem_recent_prompts(
        &self,
        Parameters(params): Parameters<MemRecentPromptsParams>,
    ) -> String {
        let limit = params.limit.unwrap_or(10);
        match self.call_recent_prompts(params.project.as_deref(), limit) {
            Ok(prompts) => {
                if prompts.is_empty() {
                    return "No prompts found.".to_string();
                }
                prompts
                    .iter()
                    .map(|p| format!("[{}] {} — {}", p.id, p.created_at, p.content))
                    .collect::<Vec<_>>()
                    .join("\n")
            }
            Err(e) => format!("Error retrieving prompts: {e}"),
        }
    }
}

// ── Public API (for tests and CLI) ───────────────────────────────

impl CortexMemServer {
    pub fn new(db: Database, embed_mgr: Option<EmbeddingManager>) -> Self {
        Self {
            tool_router: Self::tool_router(),
            memory: Mutex::new(MemoryManager::new(db, embed_mgr)),
            current_session: Mutex::new(None),
            last_search_query: Mutex::new(None),
        }
    }

    pub fn list_tools(&self) -> Vec<rmcp::model::Tool> {
        self.tool_router.list_all()
    }

    // ── Write operations ─────────────────────────────────────

    #[allow(clippy::too_many_arguments)]
    pub fn call_save(
        &self,
        project: &str,
        title: &str,
        content: &str,
        obs_type: &str,
        concepts: Option<Vec<String>>,
        facts: Option<Vec<String>>,
        files: Option<Vec<String>>,
        topic_key: Option<String>,
        scope: Option<String>,
    ) -> Result<SaveResult> {
        let session_id = *self.current_session.lock().unwrap();
        let obs = NewObservation {
            project: project.into(),
            title: title.into(),
            content: content.into(),
            obs_type: obs_type.into(),
            concepts,
            facts,
            files,
            topic_key,
            scope: scope.unwrap_or_else(|| "project".into()),
            session_id,
        };
        let mgr = self.memory.lock().unwrap();
        let result = mgr.save_observation(&obs)?;

        // Capture mutation for sync (fire-and-forget)
        match &result.dedup_status {
            DedupResult::HashMatch(_) => {} // No write — skip mutation
            _ => {
                let op = match &result.dedup_status {
                    DedupResult::TopicKeyUpsert(_) => "upsert",
                    _ => "insert",
                };
                // Serialize the full observation so the remote end can reconstruct it
                let payload = match mgr.db().get_observation(result.id) {
                    Ok(Some(full_obs)) => {
                        serde_json::to_string(&full_obs).unwrap_or_else(|_| "{}".to_string())
                    }
                    _ => serde_json::json!({
                        "title": title,
                        "content": content,
                        "type": obs_type,
                    })
                    .to_string(),
                };
                if let Err(e) = crate::sync::mutations::capture_mutation(
                    mgr.db(),
                    "observation",
                    &result.id.to_string(),
                    op,
                    &payload,
                    project,
                ) {
                    tracing::warn!("Failed to capture sync mutation: {e}");
                }
            }
        }

        Ok(result)
    }

    pub fn call_update(
        &self,
        id: i64,
        title: Option<&str>,
        content: Option<&str>,
        concepts: Option<&Vec<String>>,
        facts: Option<&Vec<String>>,
        files: Option<&Vec<String>>,
    ) -> Result<()> {
        let mgr = self.memory.lock().unwrap();
        mgr.db()
            .update_observation_fields(id, title, content, concepts, facts, files)?;

        // Re-sync FTS
        mgr.db().remove_from_fts(id).ok();
        mgr.db().sync_observation_to_fts(id)?;

        // Capture mutation for sync — serialize full updated observation
        match mgr.db().get_observation(id) {
            Ok(Some(obs)) => {
                let payload = serde_json::to_string(&obs).unwrap_or_else(|_| "{}".to_string());
                if let Err(e) = crate::sync::mutations::capture_mutation(
                    mgr.db(),
                    "observation",
                    &id.to_string(),
                    "update",
                    &payload,
                    &obs.project,
                ) {
                    tracing::warn!("Failed to capture update mutation: {e}");
                }
            }
            Ok(None) => {
                tracing::warn!("Cannot capture update mutation: observation {id} not found");
            }
            Err(e) => {
                tracing::warn!("Cannot capture update mutation for {id}: {e}");
            }
        }

        Ok(())
    }

    pub fn call_session_start(&self, project: &str, directory: &str) -> Result<i64> {
        let mgr = self.memory.lock().unwrap();
        let session_id = mgr.db().create_session(project, directory)?;
        drop(mgr);
        *self.current_session.lock().unwrap() = Some(session_id);
        Ok(session_id)
    }

    pub fn call_session_summary(&self, session_id: i64, summary: &str) -> Result<()> {
        let mgr = self.memory.lock().unwrap();
        mgr.db().set_session_summary(session_id, summary)
    }

    // ── Read operations ──────────────────────────────────────

    pub fn call_get(&self, id: i64) -> Result<Option<Observation>> {
        let mgr = self.memory.lock().unwrap();
        mgr.db().get_observation(id)
    }

    pub fn call_get_session(&self, id: i64) -> Result<Option<Session>> {
        let mgr = self.memory.lock().unwrap();
        mgr.db().get_session(id)
    }

    pub fn call_get_multiple(&self, ids: &[i64]) -> Result<Vec<Observation>> {
        let mgr = self.memory.lock().unwrap();
        let mut results = Vec::with_capacity(ids.len());
        for &id in ids {
            if let Some(obs) = mgr.db().get_observation(id)? {
                results.push(obs);
            }
        }
        Ok(results)
    }

    pub fn call_get_and_track(&self, id: i64) -> Result<Option<Observation>> {
        let mgr = self.memory.lock().unwrap();
        mgr.db().increment_access_count(id)?;
        mgr.db().get_observation(id)
    }

    fn track_access(&self, id: i64) -> Result<()> {
        let mgr = self.memory.lock().unwrap();
        mgr.db().increment_access_count(id)
    }

    pub fn call_search(
        &self,
        query: &str,
        project: Option<&str>,
        obs_type: Option<&str>,
        scope: Option<&str>,
        limit: Option<usize>,
    ) -> Vec<crate::search::SearchResult> {
        let mgr = self.memory.lock().unwrap();
        let searcher = crate::search::HybridSearcher::new(mgr.db(), mgr.embed_mgr());
        let params = crate::search::SearchParams {
            query: query.into(),
            project: project.map(String::from),
            obs_type: obs_type.map(String::from),
            scope: scope.map(String::from),
            limit: limit.unwrap_or(20),
        };
        match searcher.search(&params) {
            Ok(results) => results,
            Err(e) => {
                tracing::error!("Search failed: {e:#}");
                vec![]
            }
        }
    }

    pub fn call_timeline(
        &self,
        id: i64,
        window: Option<i64>,
        project: &str,
    ) -> Result<Vec<Observation>> {
        let mgr = self.memory.lock().unwrap();
        mgr.db().get_timeline(project, id, window.unwrap_or(5))
    }

    pub fn call_context(&self, project: Option<&str>, limit: i64) -> Result<Vec<Observation>> {
        let mgr = self.memory.lock().unwrap();
        match project {
            Some(p) => mgr.db().list_observations(p, limit),
            None => {
                let all = mgr.db().list_all_active_observations()?;
                Ok(all.into_iter().take(limit as usize).collect())
            }
        }
    }

    pub fn call_suggest_topic(&self, obs_type: &str, title: &str) -> String {
        let suggested = generate_topic_key(obs_type, title);
        let family = suggested.split('/').next().unwrap_or("general");
        let mgr = self.memory.lock().unwrap();

        let existing: Vec<(String, i64)> = mgr
            .db()
            .list_all_active_observations()
            .unwrap_or_default()
            .iter()
            .filter_map(|obs| {
                obs.topic_key.as_ref().and_then(|key| {
                    if key.starts_with(&format!("{family}/")) {
                        Some((key.clone(), obs.revision_count))
                    } else {
                        None
                    }
                })
            })
            .collect::<std::collections::BTreeSet<_>>()
            .into_iter()
            .take(5)
            .collect();

        let mut out = format!("Suggested: {suggested}\n");
        if !existing.is_empty() {
            out.push_str("Existing matches:\n");
            for (key, revisions) in &existing {
                out.push_str(&format!(
                    "  - {key} ({} revision{})\n",
                    revisions,
                    if *revisions == 1 { "" } else { "s" }
                ));
            }
        }
        out
    }

    // ── Lifecycle operations ─────────────────────────────────

    pub fn call_session_end(&self, session_id: i64, summary: Option<&str>) -> Result<()> {
        let mgr = self.memory.lock().unwrap();
        mgr.db().end_session(session_id, summary)
    }

    pub fn call_delete(&self, id: i64) -> Result<()> {
        let mgr = self.memory.lock().unwrap();

        // Get observation before delete to capture project for mutation
        let obs_before = mgr.db().get_observation(id)?;

        mgr.db().soft_delete(id)?;
        mgr.db().remove_from_fts(id).ok();

        // Capture mutation for sync (fire-and-forget).
        // obs_before is None when deleting a non-existent ID — nothing to sync.
        if let Some(obs) = obs_before
            && let Err(e) = crate::sync::mutations::capture_mutation(
                mgr.db(),
                "observation",
                &id.to_string(),
                "soft_delete",
                "{}",
                &obs.project,
            )
        {
            tracing::warn!("Failed to capture delete mutation: {e}");
        }

        Ok(())
    }

    pub fn call_hard_delete(&self, id: i64) -> Result<()> {
        let mgr = self.memory.lock().unwrap();

        // Get observation before delete to capture project for mutation
        let obs_before = mgr.db().get_observation(id)?;

        mgr.db().remove_from_fts(id).ok();
        mgr.db().delete_vector(id).ok();
        mgr.db().hard_delete(id)?;

        // Capture mutation for sync (fire-and-forget).
        // obs_before is None when deleting a non-existent ID — nothing to sync.
        if let Some(obs) = obs_before
            && let Err(e) = crate::sync::mutations::capture_mutation(
                mgr.db(),
                "observation",
                &id.to_string(),
                "hard_delete",
                "{}",
                &obs.project,
            )
        {
            tracing::warn!("Failed to capture hard_delete mutation: {e}");
        }

        Ok(())
    }

    pub fn call_stats(&self, project: Option<&str>) -> Result<StatsResult> {
        let mgr = self.memory.lock().unwrap();
        let total = mgr.db().count_active(project)?;
        let by_tier = mgr.db().count_by_tier(project)?;
        let by_type = mgr.db().count_by_type(project)?;
        Ok(StatsResult {
            total,
            by_tier,
            by_type,
        })
    }

    pub fn call_compact(&self, project: Option<&str>) -> Result<CompactionStats> {
        let mgr = self.memory.lock().unwrap();
        crate::memory::run_compaction(mgr.db(), project)
    }

    fn call_download_model(&self) -> Result<()> {
        let mgr = self.memory.lock().unwrap();
        match mgr.embed_mgr() {
            Some(e) => e.download_model(),
            None => Err(anyhow::anyhow!("No embedding manager configured")),
        }
    }

    pub fn call_save_prompt(
        &self,
        session_id: Option<i64>,
        content: &str,
        project: Option<&str>,
    ) -> Result<i64> {
        let mgr = self.memory.lock().unwrap();
        mgr.db().insert_prompt(session_id, content, project)
    }

    pub fn call_recent_prompts(&self, project: Option<&str>, limit: i64) -> Result<Vec<Prompt>> {
        let mgr = self.memory.lock().unwrap();
        mgr.db().get_recent_prompts(project, limit)
    }

    pub fn call_list_sessions(&self, project: Option<&str>) -> Vec<Session> {
        let mgr = self.memory.lock().unwrap();
        mgr.db()
            .list_all_sessions_for_export(project)
            .unwrap_or_default()
    }

    /// Expose the memory manager lock for testing (e.g., backdating observations).
    pub fn memory_lock(&self) -> std::sync::MutexGuard<'_, MemoryManager> {
        self.memory.lock().unwrap()
    }
}

// ── Stats Result ─────────────────────────────────────────────────

#[derive(Debug)]
pub struct StatsResult {
    pub total: usize,
    pub by_tier: Vec<(String, i64)>,
    pub by_type: Vec<(String, i64)>,
}
