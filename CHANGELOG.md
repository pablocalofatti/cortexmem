# Changelog — cortexmem

Persistent vector memory for AI coding agents. All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [1.3.0] - 2026-03-09

### Added

**Distribution & Installation**
- Homebrew formula support via `pablocalofatti/tap/cortexmem` tap
- npm/npx wrapper (`npm install -g cortexmem`) with platform-specific binary download
- Shell installer (`curl -fsSL .../install.sh | sh`) with platform detection, curl/wget fallback, PATH setup
- Docker image (`ghcr.io/pablocalofatti/cortexmem`) for cloud server deployment
- Multi-stage Dockerfile: `rust:1.83-slim` builder → `debian:bookworm-slim` runtime

**Diagnostics (`cortexmem doctor`)**
- 9 diagnostic checks: database, schema, embedding model, FTS5 index, vector index, MCP config, cloud sync, git sync
- `--fix` flag for auto-repair (model download, FTS reindex, setup wizard)
- Formatted output with OK/WARN/FAIL status per check

**Setup wizard improvements**
- Auto-detection of installed agents before prompting
- Post-setup verification (runs `cortexmem --version` to confirm binary)
- Added Zed and Cline agent support (now 8 agents total)

**Release workflow**
- Added darwin-x64 target to build matrix (4 targets total)
- npm publish step syncing version from git tag
- Docker build and push to GHCR with version + latest tags
- Homebrew tap update trigger via repository dispatch

### Changed
- `docker-compose.yml` updated to use published GHCR image instead of local build
- Setup wizard `config_path()` and `ALL_AGENTS` are now public for doctor command reuse
- Test suite at 112 integration tests

## [1.2.0] - 2026-03-09

### Added

**Cloud Sync (`--features cloud`)**
- PostgreSQL-backed cloud server with Axum for multi-machine memory sharing
- User authentication with Argon2 password hashing and JWT tokens
- API key authentication for programmatic access
- Project enrollment and per-project sync boundaries
- Push/pull sync endpoints with server-side conflict resolution
- Acknowledgment-based sync protocol ensuring at-least-once delivery
- Client-side sync engine (`cortexmem cloud push` / `cortexmem cloud pull`)
- 10 CLI subcommands under `cortexmem cloud`: `serve`, `register`, `login`, `create-api-key`, `enroll`, `push`, `pull`, `auto-sync`, `status`, `set-server`
- All cloud dependencies behind `cloud` feature flag — zero overhead when unused
- Mutation capture table for tracking local changes between syncs

**Interactive TUI (`cortexmem tui`)**
- Full terminal UI dashboard built with ratatui 0.29 and crossterm 0.28
- 7 screens: Dashboard, Search Input, Search Results, Observation Detail, Timeline, Sessions List, Session Detail
- Catppuccin Mocha color theme with 11 palette constants
- Screen stack navigation with push/pop (Esc to go back)
- Dashboard shows memory statistics: total count, by-tier breakdown, by-type breakdown
- Search with live text input, cursor movement (Left/Right/Home/End), and navigable results
- Observation detail view with scrollable content showing all fields
- Timeline view for chronological exploration around a target observation
- Sessions list and detail views for browsing session history
- Vim-style keybindings (j/k) alongside arrow keys throughout

**Git Sync (`cortexmem git-sync`)**
- Git-based team sync for sharing memories without a cloud server
- Chunk-based JSON export/import with content-hash dedup on import
- 4 CLI subcommands: `init`, `run`, `status`, `auto`
- `git-sync init` — initialize a sync repo (clone remote or init local)
- `git-sync run` — single sync cycle: export, commit/push, pull, import
- `git-sync status` — show sync state (last pushed/pulled seq, errors)
- `git-sync auto` — run sync on a fixed interval (default 300s)
- Sync state tracking in `sync_state` SQLite table

**Cloud integration tests**
- 4 integration tests for cloud auth: password hash/verify, JWT create/verify, invalid JWT rejection, wrong-secret rejection
- CI workflow updated with cloud feature lint and test steps

### Changed
- Test suite expanded with cloud auth integration tests
- CI pipeline now validates cloud feature compilation separately

## [1.1.0] - 2026-03-09

### Added

**HTTP API (`cortexmem serve`)**
- Full REST API via axum 0.8 on port 7437 (localhost-only)
- 16 endpoints: health, CRUD observations, sessions, search, context, timeline, stats, compact, prompts
- CORS enabled via tower-http for local dev tool integrations
- All handlers share `CortexMemServer.call_*()` — zero logic duplication with MCP and CLI

**User prompt storage (`mem_save_prompt`)**
- New `user_prompts` table with FTS5 search (schema v2, additive migration)
- `mem_save_prompt` MCP tool — stores user prompts separately from observations
- `mem_recent_prompts` MCP tool — retrieve recent prompts by project
- `mem_context` enriched — now includes recent prompts alongside observations
- CLI: `cortexmem save-prompt` and `cortexmem recent-prompts` commands

**Hard delete**
- `mem_delete` now accepts `hard=true` for permanent removal
- Hard delete removes from observations, FTS5 index, and vector store
- CLI: `cortexmem delete <id> --hard`

**Cross-platform release binaries**
- Matrix build for 4 targets: darwin-arm64, darwin-x64, linux-arm64, linux-x64
- Archives uploaded as GitHub release assets

**SKILL.md improvements**
- Progressive disclosure enforcement — mandatory `mem_get` after `mem_search`
- Topic key naming conventions with 9 family prefixes
- Workflow state persistence pattern for multi-agent orchestrators

### Changed
- Schema version bumped from 1 to 2 (additive — existing data untouched)
- MCP tool count increased from 14 to 16
- Test suite expanded from 71 to 107 integration tests

## [1.0.0] - 2026-03-09

### Fixed
- **Embedding model mismatch** — switched from NomicEmbedTextV15 (768-dim) to AllMiniLML6V2 (384-dim) matching the sqlite-vec schema
- **Silent search failures** — `call_search` now logs errors instead of swallowing them via `unwrap_or_default()`
- **MCP server startup** — added `CORTEXMEM_DB` env var to override database path for environments where default path is inaccessible

### Added

**Auto-download embedding model**
- Model downloads automatically on first `mem_save` or `mem_search` call — no manual step needed
- MCP Server Setup guide in README with `claude mcp add` command, env vars, and troubleshooting

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

[Unreleased]: https://github.com/pablocalofatti/cortexmem/compare/v1.3.0...HEAD
[1.3.0]: https://github.com/pablocalofatti/cortexmem/compare/v1.2.0...v1.3.0
[1.2.0]: https://github.com/pablocalofatti/cortexmem/compare/v1.1.0...v1.2.0
[1.1.0]: https://github.com/pablocalofatti/cortexmem/compare/v1.0.0...v1.1.0
[1.0.0]: https://github.com/pablocalofatti/cortexmem/releases/tag/v1.0.0
