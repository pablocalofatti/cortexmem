use std::sync::Mutex;

use rmcp::{
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::ServerInfo,
    tool, tool_handler, tool_router, ServerHandler,
};
use schemars::JsonSchema;
use serde::Deserialize;

use crate::db::Database;
use crate::embed::EmbeddingManager;
use crate::memory::MemoryManager;

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
    pub content: String,
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

// ── Server ───────────────────────────────────────────────────────

pub struct CortexMemServer {
    tool_router: ToolRouter<Self>,
    memory: Mutex<MemoryManager>,
}

#[tool_handler(router = self.tool_router)]
impl ServerHandler for CortexMemServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(Default::default()).with_instructions(
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
    async fn mem_save(&self, Parameters(_params): Parameters<MemSaveParams>) -> String {
        // TODO: Task 10 — implement write handler
        "Not yet implemented".to_string()
    }

    #[tool(
        name = "mem_update",
        description = "Update fields of an existing observation by ID. Recomputes content hash and re-embeds."
    )]
    async fn mem_update(&self, Parameters(_params): Parameters<MemUpdateParams>) -> String {
        "Not yet implemented".to_string()
    }

    #[tool(
        name = "mem_session_summary",
        description = "Persist a compaction summary for the current session. Call this when context is about to be compacted."
    )]
    async fn mem_session_summary(
        &self,
        Parameters(_params): Parameters<MemSessionSummaryParams>,
    ) -> String {
        "Not yet implemented".to_string()
    }

    // ── Read Tools ───────────────────────────────────────────

    #[tool(
        name = "mem_search",
        description = "Search memory using hybrid FTS5 + vector similarity with RRF fusion. Returns compact results (id, title, type, concepts)."
    )]
    async fn mem_search(&self, Parameters(_params): Parameters<MemSearchParams>) -> String {
        "Not yet implemented".to_string()
    }

    #[tool(
        name = "mem_get",
        description = "Get full observation detail by ID or multiple IDs. Returns all fields including content, facts, files."
    )]
    async fn mem_get(&self, Parameters(_params): Parameters<MemGetParams>) -> String {
        "Not yet implemented".to_string()
    }

    #[tool(
        name = "mem_timeline",
        description = "Get chronological context around a target observation. Shows what was saved before and after."
    )]
    async fn mem_timeline(&self, Parameters(_params): Parameters<MemTimelineParams>) -> String {
        "Not yet implemented".to_string()
    }

    #[tool(
        name = "mem_context",
        description = "Get recent observations from previous sessions for the current project. Use at session start for context recovery."
    )]
    async fn mem_context(&self, Parameters(_params): Parameters<MemContextParams>) -> String {
        "Not yet implemented".to_string()
    }

    #[tool(
        name = "mem_suggest_topic",
        description = "Suggest matching existing topic_keys for a given title and content. Helps maintain consistent topic organization."
    )]
    async fn mem_suggest_topic(
        &self,
        Parameters(_params): Parameters<MemSuggestTopicParams>,
    ) -> String {
        "Not yet implemented".to_string()
    }

    // ── Lifecycle Tools ──────────────────────────────────────

    #[tool(
        name = "mem_session_start",
        description = "Start a new memory session for a project. Creates session record and returns recent context."
    )]
    async fn mem_session_start(
        &self,
        Parameters(_params): Parameters<MemSessionStartParams>,
    ) -> String {
        "Not yet implemented".to_string()
    }

    #[tool(
        name = "mem_session_end",
        description = "End the current session. Optionally stores a session summary and triggers decay cycle."
    )]
    async fn mem_session_end(
        &self,
        Parameters(_params): Parameters<MemSessionEndParams>,
    ) -> String {
        "Not yet implemented".to_string()
    }

    // ── Admin Tools ──────────────────────────────────────────

    #[tool(
        name = "mem_delete",
        description = "Soft-delete an observation by ID (sets deleted_at, recoverable)."
    )]
    async fn mem_delete(&self, Parameters(_params): Parameters<MemDeleteParams>) -> String {
        "Not yet implemented".to_string()
    }

    #[tool(
        name = "mem_stats",
        description = "Show memory statistics: counts by type/tier, database size, embedding model status."
    )]
    async fn mem_stats(&self, Parameters(_params): Parameters<MemStatsParams>) -> String {
        "Not yet implemented".to_string()
    }

    #[tool(
        name = "mem_compact",
        description = "Run decay cycle: promote frequently accessed observations, archive stale ones. Returns stats."
    )]
    async fn mem_compact(&self, Parameters(_params): Parameters<MemCompactParams>) -> String {
        "Not yet implemented".to_string()
    }

    #[tool(
        name = "mem_model",
        description = "Check or download the embedding model. Shows model status and triggers download if needed."
    )]
    async fn mem_model(&self, Parameters(_params): Parameters<MemModelParams>) -> String {
        "Not yet implemented".to_string()
    }
}

impl CortexMemServer {
    pub fn new(db: Database, embed_mgr: Option<EmbeddingManager>) -> Self {
        Self {
            tool_router: Self::tool_router(),
            memory: Mutex::new(MemoryManager::new(db, embed_mgr)),
        }
    }

    pub fn list_tools(&self) -> Vec<rmcp::model::Tool> {
        self.tool_router.list_all()
    }
}
