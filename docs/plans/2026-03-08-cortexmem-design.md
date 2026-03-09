# cortexmem — Design Document

**Date:** 2026-03-08
**Status:** Approved
**Version:** 0.1.0

## Overview

cortexmem is an embedded memory engine for AI coding agents. It provides persistent, searchable memory that survives across sessions and context compactions — powered by hybrid search combining SQLite FTS5 keyword matching with semantic vector similarity, all in a single Rust binary with zero external dependencies.

Built in Rust. Consumed via MCP protocol. Works with any AI coding tool.

## Key Decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Language | Rust | Single binary ~5MB, no GC, lower RAM than Go |
| MCP SDK | `rmcp` 1.1.0 | Official Anthropic SDK, derive macros |
| Storage | `rusqlite` 0.38 + SQLite WAL | 50M downloads, concurrent reads |
| Keyword search | FTS5 (built into SQLite) | BM25 ranking, exact match |
| Vector search | `sqlite-vec` 0.1.7 | Same DB file, no second process |
| Embeddings | `fastembed` 5.12 | Local ONNX, Apple Silicon native |
| Embedding model | nomic-embed-text-v1.5 @ 384d | 8K context, Matryoshka dims |
| Search fusion | RRF (k=60) | Rank-based, no score calibration |
| Architecture | Monolithic binary | No daemon, no port management |
| Data isolation | Single DB, project column | Simple backup, WAL concurrency |
| Model loading | Download on first run | Ships light, degrades to FTS5-only if offline |
| npm distribution | Platform-specific optionalDependencies | Industry standard (esbuild pattern) |
| Scope | Full-featured v0.1.0 | 14 MCP tools, hybrid search, lifecycle, hooks |

## Data Model

### SQLite Schema (`~/.cortexmem/cortexmem.db`)

```sql
CREATE TABLE observations (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id      INTEGER REFERENCES sessions(id),
    project         TEXT NOT NULL,
    topic_key       TEXT,
    type            TEXT NOT NULL,     -- decision, discovery, pattern, bug, milestone
    title           TEXT NOT NULL,
    content         TEXT NOT NULL,
    concepts        TEXT,              -- JSON array
    facts           TEXT,              -- JSON array
    files           TEXT,              -- JSON array
    scope           TEXT DEFAULT 'project',
    tier            TEXT DEFAULT 'buffer',  -- buffer | working | core
    access_count    INTEGER DEFAULT 0,
    revision_count  INTEGER DEFAULT 1,
    content_hash    TEXT NOT NULL,
    embedding       BLOB,             -- float32[384], nullable
    created_at      TEXT DEFAULT (datetime('now')),
    updated_at      TEXT DEFAULT (datetime('now')),
    deleted_at      TEXT
);

CREATE VIRTUAL TABLE observations_fts USING fts5(
    title, content, concepts, facts, type, project,
    content=observations,
    content_rowid=id,
    tokenize='porter unicode61'
);

CREATE VIRTUAL TABLE vec_observations USING vec0(
    embedding float[384]
);

CREATE TABLE sessions (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    project     TEXT NOT NULL,
    directory   TEXT NOT NULL,
    summary     TEXT,
    started_at  TEXT DEFAULT (datetime('now')),
    ended_at    TEXT
);

CREATE TABLE meta (
    key   TEXT PRIMARY KEY,
    value TEXT
);
```

### Key Design Points

- **`topic_key` upsert**: Same project + topic_key = update in place, increment `revision_count`
- **`content_hash`**: SHA-256 dedup within 15-minute windows
- **`tier`**: Buffer → Working → Core lifecycle (decay managed by `mem_compact`)
- **`embedding`**: Nullable — populated async or on first search
- **Soft delete**: `deleted_at` column, never hard delete

## MCP Tools (14 total)

### Write Tools

| Tool | Parameters | Behavior |
|------|------------|----------|
| `mem_save` | `title`, `content`, `type`, `concepts[]`, `facts[]`, `files[]`, `topic_key?`, `scope?` | Dedup check → topic_key upsert if match → embed → store |
| `mem_update` | `id`, `title?`, `content?`, `concepts?`, `facts?`, `files?` | Update existing, recompute hash + embedding |
| `mem_session_summary` | `summary` | Persists compaction summary for current session |

### Read Tools (Progressive Disclosure)

| Tool | Parameters | Returns | ~Tokens |
|------|------------|---------|---------|
| `mem_search` | `query`, `type?`, `project?`, `scope?`, `limit?` | Compact: `[{id, title, type, concepts, created_at}]` | ~50/result |
| `mem_get` | `id` OR `ids[]` | Full observation detail, all fields | ~500-1K/result |
| `mem_timeline` | `id`, `window?` | Chronological context around target | ~300/result |
| `mem_context` | `project?` | Recent observations from previous sessions | ~2K total |
| `mem_suggest_topic` | `title`, `content` | Suggests matching existing topic_keys | ~200 |

### Lifecycle Tools

| Tool | Parameters | Behavior |
|------|------------|----------|
| `mem_session_start` | `project`, `directory` | Creates session, returns recent context |
| `mem_session_end` | `summary?` | Marks session ended, stores summary |

### Admin Tools

| Tool | Parameters | Behavior |
|------|------------|----------|
| `mem_delete` | `id` | Soft-delete (sets `deleted_at`) |
| `mem_stats` | `project?` | DB stats: counts by type/tier, DB size, model status |
| `mem_compact` | `project?` | Decay cycle: promote/archive based on access rules |

### Search Flow

```
mem_search "authentication middleware"
    │
    ├─ FTS5: MATCH query → top 50 by BM25
    ├─ sqlite-vec: embedding KNN → top 50 by cosine distance
    │    (skipped if model not available, falls back to FTS5-only)
    ├─ RRF fusion: score(d) = 1/(60+rankA) + 1/(60+rankB)
    ├─ Boost: recency × access_count × project_match
    └─ Return top 20 compact results
```

## Session Lifecycle & Hooks

### Hook Flow

```
SessionStart
  → Start cortexmem MCP server (stdio)
  → Call mem_session_start(project, directory)
  → Inject Memory Protocol + mem_context results

During Session
  → Agent calls mem_save/mem_search as needed

Compaction Recovery
  → Re-inject Memory Protocol instructions
  → Agent calls mem_session_summary to persist compacted context
  → Agent calls mem_context to recover key memories

Stop / SessionEnd
  → Agent generates session summary
  → Call mem_session_end(summary)
  → Run decay cycle if due
```

### Memory Protocol Skill

Injected into agent context via SKILL.md. Guides the agent on:
- **When to save**: Architecture decisions, bug root causes, patterns, milestones
- **When not to save**: Routine reads, trivial changes, intermediate debug steps
- **When to search**: Before starting work on a topic with potential prior context
- **Topic key usage**: For evolving knowledge (`architecture/auth`, `decision/database`)

## Memory Lifecycle

### 3-Tier Decay Model

| Tier | Promotion Rule | Demotion Rule |
|------|---------------|---------------|
| **buffer** | Default for new observations | No access for 30 days → soft-delete |
| **working** | Accessed within 7 days OR accessed 2+ times | No access for 90 days → soft-delete |
| **core** | Accessed 5+ times OR topic_key with 3+ revisions | Never demoted |

### Dedup Pipeline (on every `mem_save`)

1. **Hash check**: SHA-256 match within 15min → increment duplicate_count, skip
2. **Topic key**: Same project + topic_key → upsert (update, re-embed)
3. **Similarity**: Cosine > 0.92 → warn in response, save with flag

### Compaction (`mem_compact`)

Rule-based decay sweep (no LLM calls in v0.1.0):
1. Check all buffer/working observations against access rules
2. Promote or archive accordingly
3. Return stats report

## Project Structure

```
cortexmem/
├── Cargo.toml
├── CLAUDE.md
├── CHANGELOG.md
├── LICENSE
├── README.md
├── src/
│   ├── main.rs
│   ├── cli/
│   │   ├── mod.rs
│   │   ├── save.rs
│   │   ├── search.rs
│   │   ├── stats.rs
│   │   ├── model.rs
│   │   └── mcp.rs
│   ├── mcp/
│   │   ├── mod.rs
│   │   ├── tools.rs
│   │   └── protocol.rs
│   ├── db/
│   │   ├── mod.rs
│   │   ├── schema.rs
│   │   ├── observations.rs
│   │   ├── sessions.rs
│   │   └── fts.rs
│   ├── search/
│   │   ├── mod.rs
│   │   ├── fts.rs
│   │   ├── vector.rs
│   │   └── rrf.rs
│   ├── embed/
│   │   ├── mod.rs
│   │   ├── model.rs
│   │   └── pipeline.rs
│   └── memory/
│       ├── mod.rs
│       ├── dedup.rs
│       ├── decay.rs
│       └── compact.rs
├── tests/
│   ├── integration/
│   │   ├── search_test.rs
│   │   ├── dedup_test.rs
│   │   ├── lifecycle_test.rs
│   │   └── mcp_test.rs
│   └── fixtures/
├── npm/
│   ├── cortexmem/
│   │   ├── package.json
│   │   └── bin/run.js
│   ├── cortexmem-darwin-arm64/
│   ├── cortexmem-darwin-x64/
│   ├── cortexmem-linux-x64/
│   └── cortexmem-win32-x64/
├── plugin/
│   ├── hooks/hooks.json
│   ├── scripts/
│   │   ├── session-start.sh
│   │   ├── session-end.sh
│   │   └── compaction-recovery.sh
│   └── skills/memory-protocol/SKILL.md
└── .github/workflows/
    ├── ci.yml
    ├── pr-gate.yml
    ├── code-review.yml
    ├── release.yml
    └── build.yml
```

## Dependencies

```toml
[dependencies]
rmcp = { version = "1.1", features = ["server", "transport-io"] }
rusqlite = { version = "0.38", features = ["bundled", "vtab"] }
sqlite-vec = "0.1"
fastembed = "5.12"
tokio = { version = "1", features = ["full"] }
clap = { version = "4", features = ["derive"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
schemars = "0.8"
sha2 = "0.10"
chrono = { version = "0.4", features = ["serde"] }
anyhow = "1"
thiserror = "2"
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
dirs = "6"

[profile.release]
lto = true
strip = true
codegen-units = 1
```

## CI/CD Pipeline

| Workflow | Trigger | Steps |
|----------|---------|-------|
| ci.yml | Push + PR to main | `cargo fmt --check` → `cargo clippy -D warnings` → `cargo test` |
| pr-gate.yml | PR events | Auto-pass owner/claude[bot]/github-actions[bot] |
| code-review.yml | PR opened/synced | Claude Opus review against CLAUDE.md |
| release.yml | Push to main | Conventional commit bump → tag → GitHub Release → changelog PR |
| build.yml | Release created | Cross-compile → attach binaries → publish crates.io + npm |

## Distribution

| Channel | Command |
|---------|---------|
| npm | `npx cortexmem` / `npm install -g cortexmem` |
| Cargo | `cargo install cortexmem` |
| Homebrew | `brew install pablocalofatti/tap/cortexmem` |
| GitHub Releases | Prebuilt binaries for macOS/Linux/Windows |
| Claude Code plugin | `claude plugin install cortexmem` |

## Research Sources

This design was informed by deep analysis of 5 existing projects:
- **claude-mem** (thedotmack) — ChromaDB + SQLite hybrid, 3-runtime complexity
- **engram** (Gentleman-Programming) — Go binary, SQLite FTS5, zero dependencies
- **agent-teams-lite** (Gentleman-Programming) — SDD orchestration + engram integration
- **engram-rs** (kael-bit) — Rust reimplementation with 3-layer decay model
- **Mastra Observational Memory** — 95% on LongMemEval, 3-tier compression

Key patterns adopted:
- Progressive disclosure (claude-mem + engram)
- Compaction recovery hooks (engram)
- Topic key upsert with revision tracking (engram)
- 3-tier decay lifecycle (engram-rs / Atkinson-Shiffrin)
- RRF hybrid search fusion (Alex Garcia's sqlite-vec tutorial)
- Platform-specific npm packages (esbuild pattern)
