# Changelog — cortexmem

Persistent vector memory for AI coding agents. All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Fixed
- **Embedding model mismatch** — switched from NomicEmbedTextV15 (768-dim) to AllMiniLML6V2 (384-dim) matching the sqlite-vec schema
- **Silent search failures** — `call_search` now logs errors instead of swallowing them via `unwrap_or_default()`
- **MCP server startup** — added `CORTEXMEM_DB` env var to override database path for environments where default path is inaccessible

### Added
- **Auto-download embedding model** on first `mem_save` or `mem_search` call — no manual download step needed
- **MCP Server Setup guide** in README with `claude mcp add` command, env vars, and troubleshooting

## [1.0.0] - 2026-03-09

### Added

**Setup wizard**
- Interactive `cortexmem setup` command using `dialoguer` for multi-agent configuration
- Supports 6 AI agents: Claude Code, Cursor, Windsurf, Cline, Roo Code, Continue
- Writes agent-specific MCP JSON config automatically
- For Claude Code: installs Memory Protocol skill, session hooks, and compaction recovery scripts

**Export / Import**
- `cortexmem export` — dump all sessions and observations to JSON, with optional `--project` filter
- `cortexmem import <file>` — merge mode (default) skips duplicates by content hash
- `cortexmem import <file> --replace` — replace mode wipes existing data with confirmation prompt

**Improved topic suggestions**
- `mem_suggest_topic` generates deterministic `{family}/{slug}` topic keys from observation type and title
- Returns existing keys from the same family for consistency

**Database layer**
- SQLite-backed persistent storage using `rusqlite` with bundled SQLite
- Schema for observations table with tier, topic key, scope, access/revision counts, and timestamps
- WAL mode for concurrent read access
- Parameterized queries only — no SQL interpolation

**Full-text search (FTS5)**
- FTS5 virtual table for keyword-based memory search with BM25 ranking
- Porter stemming + unicode61 tokenizer
- Project-scoped search filtering

**Embedding layer**
- Local embedding generation via `fastembed` (all-MiniLM-L6-v2, 384 dimensions)
- No API keys or network access required for inference
- Model download on first use, graceful degradation to FTS5-only if unavailable

**Vector search**
- `sqlite-vec` extension for native vector similarity search inside SQLite
- Cosine similarity KNN ranking for semantic queries
- Embedding vectors stored alongside observations for zero-latency retrieval

**Hybrid search with RRF**
- Reciprocal Rank Fusion (k=60) combining FTS5 BM25 scores and vector similarity
- Recency boost and access frequency weighting
- Unified ranked result list via `mem_search`

**Memory lifecycle**
- 3-tier memory system: `buffer` → `working` → `core`
- SHA-256 content hashing for exact-duplicate detection on `mem_save`
- Topic key upsert — saving with the same `topic_key` updates the existing observation
- Automatic decay scoring based on access count, revision count, and age
- `mem_compact` to promote, archive, or soft-delete observations based on tier rules

**MCP server with 14 tools**
- `mem_save` — save an observation with dedup (content hash + topic key upsert)
- `mem_update` — update fields of an existing observation by ID
- `mem_search` — hybrid FTS5 + vector search with RRF fusion
- `mem_get` — get full observation detail by ID or batch IDs
- `mem_timeline` — chronological context around a target observation
- `mem_context` — recent observations for context recovery at session start
- `mem_suggest_topic` — generate topic keys and find existing matches
- `mem_session_start` — start a session, returns recent context
- `mem_session_end` — end session with optional summary, triggers decay
- `mem_session_summary` — persist a compaction summary mid-session
- `mem_delete` — soft-delete an observation (recoverable)
- `mem_stats` — memory statistics by type and tier
- `mem_compact` — run decay cycle (promote/archive)
- `mem_model` — check or download the embedding model

**CLI**
- `cortexmem mcp` — launch the MCP stdio server
- `cortexmem save` — save an observation from the command line
- `cortexmem search` — hybrid search with type and project filters
- `cortexmem get <id>` — retrieve full observation by ID
- `cortexmem stats` — print memory statistics
- `cortexmem compact` — run memory compaction manually
- `cortexmem model download` — download the embedding model
- `cortexmem model status` — check embedding model status
- `cortexmem export` — export memories to JSON
- `cortexmem import` — import memories from JSON
- `cortexmem setup` — interactive setup wizard

**Claude Code plugin**
- Session hooks (`hooks.json`) for automatic `mem_session_start` and `mem_session_end`
- Compaction recovery script to save context before context window compaction
- Memory Protocol skill (SKILL.md) with full 14-tool reference and usage patterns

**CI / CD**
- `cargo fmt --check` + `cargo clippy -D warnings` + `cargo test` on every PR
- PR gate to block external contributors
- Claude Code Review workflow using `anthropics/claude-code-action` with OAuth token
- Automated release workflow for cross-platform binary publishing

**Testing**
- 71 integration tests covering all MCP tools, search, lifecycle, dedup, export/import
- 2 unit tests for embedding pipeline
- `Database::open_in_memory()` for all DB tests — no temp files

[Unreleased]: https://github.com/pablocalofatti/cortexmem/compare/v1.0.0...HEAD
[1.0.0]: https://github.com/pablocalofatti/cortexmem/releases/tag/v1.0.0
