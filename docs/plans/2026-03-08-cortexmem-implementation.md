# cortexmem v0.1.0 Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Build a persistent vector memory engine for AI coding agents — single Rust binary, 14 MCP tools, hybrid search (FTS5 + semantic vectors), 3-tier memory lifecycle.

**Architecture:** Monolithic Rust binary serving as both CLI and MCP server (stdio). SQLite with WAL mode for storage, FTS5 for keyword search, sqlite-vec for vector KNN, fastembed for local ONNX embeddings. RRF fusion combines both search signals.

**Tech Stack:** Rust, rmcp 1.1, rusqlite 0.38 (bundled + vtab), sqlite-vec 0.1, fastembed 5.12, tokio, clap 4, serde, sha2, chrono, tracing.

**Design doc:** `docs/plans/2026-03-08-cortexmem-design.md`

---

## Task 1: Project Scaffold & CI Pipeline

**Files:**
- Create: `Cargo.toml`
- Create: `CLAUDE.md`
- Create: `src/main.rs`
- Create: `.github/workflows/ci.yml`
- Create: `.github/workflows/pr-gate.yml`
- Create: `.github/workflows/release.yml`
- Modify: `.gitignore`
- Modify: `README.md`

**Step 1: Create Cargo.toml with all dependencies**

```toml
[package]
name = "cortexmem"
version = "0.1.0"
edition = "2024"
license = "MIT"
description = "Persistent vector memory for AI coding agents"
repository = "https://github.com/pablocalofatti/cortexmem"
readme = "README.md"
keywords = ["memory", "vector-search", "mcp", "ai-agents", "embeddings"]
categories = ["command-line-utilities", "database"]

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

**Step 2: Create minimal main.rs that compiles**

```rust
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "cortexmem", version, about = "Persistent vector memory for AI coding agents")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Launch MCP server (stdio transport)
    Mcp,
    /// Show version and status
    Status,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .with_ansi(false)
        .init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Mcp => {
            tracing::info!("MCP server starting...");
            // TODO: Task 9
            Ok(())
        }
        Commands::Status => {
            println!("cortexmem v{}", env!("CARGO_PKG_VERSION"));
            Ok(())
        }
    }
}
```

**Step 3: Create CLAUDE.md with Rust code rules**

Copy the Rust engineering standards from design section 6. Full content already approved.

**Step 4: Create CI workflow (.github/workflows/ci.yml)**

```yaml
name: CI

on:
  push:
    branches: [main]
  pull_request:
    branches: [main]

jobs:
  ci:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - uses: dtolnay/rust-toolchain@stable
        with:
          components: clippy, rustfmt

      - uses: Swatinem/rust-cache@v2

      - name: Format check
        run: cargo fmt --check

      - name: Lint
        run: cargo clippy -- -D warnings

      - name: Test
        run: cargo test --verbose
```

**Step 5: Create PR Gate workflow (.github/workflows/pr-gate.yml)**

Adapt from minion-toolkit: auto-pass for owner/claude[bot]/github-actions[bot], block external.

**Step 6: Create Release workflow (.github/workflows/release.yml)**

Adapt from minion-toolkit: conventional commit parsing, semver bump, tag, GitHub Release, changelog PR via `RELEASE_TOKEN`. Update version in `Cargo.toml` instead of `package.json`.

**Step 7: Update .gitignore**

Add: `target/`, `*.db`, `*.db-wal`, `*.db-shm`, `.cortexmem/`

**Step 8: Run `cargo build` to verify compilation**

Run: `cargo build`
Expected: Successful compilation with warnings about unused imports (OK at this stage)

**Step 9: Commit**

```bash
git add -A
git commit -m "feat: project scaffold with Cargo.toml, CLI entrypoint, and CI pipeline"
```

---

## Task 2: Database Layer — Schema & Connection

**Files:**
- Create: `src/db/mod.rs`
- Create: `src/db/schema.rs`
- Create: `tests/integration/db_test.rs`
- Modify: `src/main.rs` (add mod db)

**Step 1: Write the failing test**

```rust
// tests/integration/db_test.rs
use cortexmem::db::Database;

#[test]
fn should_initialize_database_with_schema() {
    let db = Database::open_in_memory().unwrap();
    let version = db.schema_version().unwrap();
    assert_eq!(version, 1);
}

#[test]
fn should_use_wal_mode() {
    let db = Database::open_in_memory().unwrap();
    let mode = db.journal_mode().unwrap();
    assert_eq!(mode, "wal");
}

#[test]
fn should_register_sqlite_vec_extension() {
    let db = Database::open_in_memory().unwrap();
    let has_vec = db.has_vec_extension().unwrap();
    assert!(has_vec);
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test --test db_test`
Expected: FAIL — module `db` not found

**Step 3: Implement Database struct**

Create `src/db/mod.rs`:
- `Database` struct wrapping `rusqlite::Connection`
- `open(path)` and `open_in_memory()` constructors
- WAL mode pragma on connection
- Register sqlite-vec extension via `sqlite3_auto_extension`
- `schema_version()`, `journal_mode()`, `has_vec_extension()` methods

Create `src/db/schema.rs`:
- `migrate(conn)` function that creates all tables (observations, observations_fts, vec_observations, sessions, meta)
- Schema version tracking in `meta` table
- Idempotent — runs on every open, skips if already at current version

Add `pub mod db;` to `src/main.rs` (convert to lib.rs + main.rs pattern for testability).

**Step 4: Run test to verify it passes**

Run: `cargo test --test db_test`
Expected: PASS

**Step 5: Commit**

```bash
git commit -m "feat(db): add database layer with schema, WAL mode, and sqlite-vec registration"
```

---

## Task 3: Database Layer — Observations CRUD

**Files:**
- Create: `src/db/observations.rs`
- Create: `tests/integration/observations_test.rs`

**Step 1: Write failing tests**

```rust
// tests/integration/observations_test.rs

#[test]
fn should_insert_observation() {
    let db = Database::open_in_memory().unwrap();
    let obs = NewObservation {
        project: "myproject".into(),
        title: "Auth decision".into(),
        content: "Chose JWT over sessions".into(),
        obs_type: "decision".into(),
        concepts: Some(vec!["auth".into(), "jwt".into()]),
        facts: Some(vec!["JWT chosen for stateless auth".into()]),
        files: Some(vec!["src/auth.ts".into()]),
        topic_key: Some("architecture/auth".into()),
        scope: "project".into(),
        session_id: None,
    };
    let id = db.insert_observation(&obs).unwrap();
    assert!(id > 0);
}

#[test]
fn should_get_observation_by_id() {
    // insert then get, verify all fields match
}

#[test]
fn should_find_by_topic_key() {
    // insert with topic_key, then find_by_topic_key
}

#[test]
fn should_upsert_on_topic_key_match() {
    // insert twice with same topic_key, verify revision_count = 2
}

#[test]
fn should_soft_delete() {
    // insert, soft_delete, verify excluded from find_all
}

#[test]
fn should_increment_access_count() {
    // insert, access, verify access_count = 1
}

#[test]
fn should_find_by_content_hash_within_window() {
    // insert, check hash within 15 min window
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test --test observations_test`
Expected: FAIL

**Step 3: Implement observations CRUD**

Create `src/db/observations.rs`:
- `NewObservation` struct (input DTO)
- `Observation` struct (full row)
- `insert_observation(&self, obs: &NewObservation) -> Result<i64>`
- `get_observation(&self, id: i64) -> Result<Option<Observation>>`
- `find_by_topic_key(&self, project: &str, topic_key: &str) -> Result<Option<Observation>>`
- `find_by_content_hash(&self, hash: &str, within_minutes: i64) -> Result<Option<Observation>>`
- `upsert_observation(&self, obs: &NewObservation) -> Result<i64>` (topic_key match = update)
- `soft_delete(&self, id: i64) -> Result<()>`
- `increment_access_count(&self, id: i64) -> Result<()>`
- `list_observations(&self, project: &str, limit: usize) -> Result<Vec<Observation>>`
- SHA-256 hash computation for content_hash using `sha2` crate
- JSON serialization for concepts/facts/files arrays

**Step 4: Run tests**

Run: `cargo test --test observations_test`
Expected: PASS

**Step 5: Commit**

```bash
git commit -m "feat(db): add observations CRUD with topic_key upsert and content hash dedup"
```

---

## Task 4: Database Layer — Sessions & FTS5

**Files:**
- Create: `src/db/sessions.rs`
- Create: `src/db/fts.rs`
- Create: `tests/integration/sessions_test.rs`
- Create: `tests/integration/fts_test.rs`

**Step 1: Write failing tests**

```rust
// tests/integration/sessions_test.rs
#[test]
fn should_create_session() { /* create, verify id */ }

#[test]
fn should_end_session_with_summary() { /* create, end, verify ended_at + summary */ }

#[test]
fn should_get_latest_session_for_project() { /* create 2, get latest */ }

// tests/integration/fts_test.rs
#[test]
fn should_index_observation_in_fts5() { /* insert obs, sync fts, search by keyword */ }

#[test]
fn should_find_by_partial_match() { /* "auth" should match "authentication" via porter stemmer */ }

#[test]
fn should_rank_by_bm25() { /* insert 3 obs, verify most relevant ranks first */ }

#[test]
fn should_exclude_soft_deleted_from_fts() { /* insert, delete, verify not in search results */ }
```

**Step 2: Run tests — expect fail**

**Step 3: Implement sessions CRUD and FTS5 sync**

`src/db/sessions.rs`:
- `create_session(project, directory) -> Result<i64>`
- `end_session(id, summary) -> Result<()>`
- `get_latest_session(project) -> Result<Option<Session>>`
- `set_session_summary(id, summary) -> Result<()>`

`src/db/fts.rs`:
- `sync_observation_to_fts(id) -> Result<()>` — INSERT into observations_fts content table
- `remove_from_fts(id) -> Result<()>` — DELETE from fts on soft-delete
- `search_fts(query, project?, limit) -> Result<Vec<FtsResult>>` — MATCH query with BM25 ranking
- `FtsResult { rowid, rank }` struct

**Step 4: Run tests — expect pass**

**Step 5: Commit**

```bash
git commit -m "feat(db): add sessions CRUD and FTS5 full-text search with BM25 ranking"
```

---

## Task 5: Embedding Layer

**Files:**
- Create: `src/embed/mod.rs`
- Create: `src/embed/model.rs`
- Create: `src/embed/pipeline.rs`
- Create: `tests/integration/embed_test.rs`

**Step 1: Write failing tests**

```rust
// tests/integration/embed_test.rs
#[test]
fn should_report_model_not_downloaded() {
    let manager = EmbeddingManager::new("/tmp/cortexmem-test-models");
    assert!(!manager.is_model_available());
}

#[test]
fn should_generate_embedding_with_correct_dimensions() {
    // This test requires model download — mark with #[ignore] for CI
    // Run manually: cargo test --test embed_test -- --ignored
    let manager = EmbeddingManager::new_with_download("/tmp/cortexmem-test-models").unwrap();
    let embedding = manager.embed("authentication middleware").unwrap();
    assert_eq!(embedding.len(), 384);
}

#[test]
fn should_build_search_text_from_observation() {
    let text = build_search_text("Auth decision", "Chose JWT", &["auth", "jwt"], &["stateless"]);
    assert!(text.contains("Auth decision"));
    assert!(text.contains("jwt"));
}
```

**Step 2: Run tests — expect fail**

**Step 3: Implement embedding layer**

`src/embed/mod.rs`: Re-export types.

`src/embed/model.rs`:
- `EmbeddingManager` struct holding `Option<TextEmbedding>` from fastembed
- `new(model_dir) -> Self` — checks if model exists, does NOT download
- `new_with_download(model_dir) -> Result<Self>` — downloads model if missing
- `is_model_available() -> bool`
- `download_model() -> Result<()>` — explicit download trigger
- `model_status() -> ModelStatus` enum (NotDownloaded, Downloading, Ready)
- Model: `EmbeddingModel::NomicEmbedTextV15` with `InitOptions { model_name, cache_dir }`
- Default cache dir: `~/.cortexmem/models/`

`src/embed/pipeline.rs`:
- `build_search_text(title, content, concepts, facts) -> String`
- `embed_text(manager, text) -> Result<Option<Vec<f32>>>` — returns None if model not available
- Wraps fastembed call in `tokio::task::spawn_blocking` for async safety

**Step 4: Run tests — expect pass** (non-ignored ones)

**Step 5: Commit**

```bash
git commit -m "feat(embed): add embedding layer with fastembed ONNX and model management"
```

---

## Task 6: Vector Search Layer

**Files:**
- Create: `src/search/mod.rs`
- Create: `src/search/vector.rs`
- Create: `tests/integration/vector_test.rs`

**Step 1: Write failing tests**

```rust
// tests/integration/vector_test.rs
#[test]
fn should_insert_and_query_vector() {
    let db = Database::open_in_memory().unwrap();
    // Insert observation with embedding
    let embedding: Vec<f32> = vec![0.1; 384];
    db.insert_vector(1, &embedding).unwrap();

    // Query KNN
    let query: Vec<f32> = vec![0.1; 384]; // identical = distance 0
    let results = db.search_vector(&query, 10).unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].rowid, 1);
}

#[test]
fn should_return_results_ordered_by_distance() {
    // Insert 3 vectors at different distances from query, verify ordering
}

#[test]
fn should_handle_empty_vector_table() {
    // Query with no vectors inserted, verify empty results
}

#[test]
fn should_delete_and_reinsert_vector() {
    // For observation updates: delete old vector, insert new one
}
```

**Step 2: Run tests — expect fail**

**Step 3: Implement vector search**

`src/search/vector.rs`:
- `insert_vector(rowid, embedding) -> Result<()>`
- `delete_vector(rowid) -> Result<()>`
- `search_vector(query_embedding, limit) -> Result<Vec<VecResult>>`
- `VecResult { rowid, distance }` struct
- Embedding passed as `&[f32]` converted to bytes for sqlite-vec
- Query uses `MATCH` syntax with `ORDER BY distance LIMIT ?`

**Step 4: Run tests — expect pass**

**Step 5: Commit**

```bash
git commit -m "feat(search): add vector KNN search via sqlite-vec"
```

---

## Task 7: Hybrid Search with RRF Fusion

**Files:**
- Create: `src/search/fts.rs` (search-layer wrapper around db/fts)
- Create: `src/search/rrf.rs`
- Create: `tests/integration/search_test.rs`

**Step 1: Write failing tests**

```rust
// tests/integration/search_test.rs
#[test]
fn should_search_fts_only_when_no_model() {
    // Insert observations, search without embeddings
    // Should return FTS5 results only
}

#[test]
fn should_search_hybrid_when_model_available() {
    // Insert observations with embeddings
    // Search should combine FTS5 + vector results via RRF
}

#[test]
fn should_fuse_results_with_rrf() {
    // Unit test: given two ranked lists, verify RRF scores
    let fts_ranks = vec![(1, 0), (2, 1), (3, 2)];  // (rowid, rank)
    let vec_ranks = vec![(3, 0), (1, 1), (4, 2)];
    let fused = rrf_fuse(&fts_ranks, &vec_ranks, 60);
    // id=1: 1/(60+0) + 1/(60+1) = 0.01667 + 0.01639 = 0.03306
    // id=3: 1/(60+2) + 1/(60+0) = 0.01613 + 0.01667 = 0.03279
    // id=1 should rank first (appears high in both)
    assert_eq!(fused[0].0, 1);
}

#[test]
fn should_boost_by_recency() {
    // Newer observations should get a boost
}

#[test]
fn should_filter_by_project() {
    // Results should only include matching project
}

#[test]
fn should_respect_limit() {
    // Insert 30 obs, search with limit=10, verify 10 results
}
```

**Step 2: Run tests — expect fail**

**Step 3: Implement hybrid search**

`src/search/fts.rs`:
- Thin wrapper calling `db.search_fts()` and returning `Vec<(i64, usize)>` (rowid, rank)

`src/search/rrf.rs`:
- `rrf_fuse(fts_results, vec_results, k) -> Vec<(i64, f64)>` — pure function
- RRF formula: `score(d) = 1/(k + rank_fts) + 1/(k + rank_vec)`
- For items in only one list: single term only
- Sort by score descending

`src/search/mod.rs`:
- `HybridSearcher` struct holding reference to `Database` and optional `EmbeddingManager`
- `search(query, project?, type?, scope?, limit) -> Result<Vec<SearchResult>>`
- Flow: FTS5 top 50 → vector KNN top 50 (if model available) → RRF → boost → limit
- Boost formula: `final_score = rrf_score * recency_factor * (1 + 0.1 * access_count)`
- `recency_factor = 1.0 / (1.0 + days_since_update * 0.01)`
- `SearchResult { id, title, obs_type, concepts, created_at, score }` (compact format)

**Step 4: Run tests — expect pass**

**Step 5: Commit**

```bash
git commit -m "feat(search): add hybrid search with RRF fusion and recency boosting"
```

---

## Task 8: Memory Lifecycle — Dedup, Decay, Compaction

**Files:**
- Create: `src/memory/mod.rs`
- Create: `src/memory/dedup.rs`
- Create: `src/memory/decay.rs`
- Create: `src/memory/compact.rs`
- Create: `tests/integration/dedup_test.rs`
- Create: `tests/integration/lifecycle_test.rs`

**Step 1: Write failing tests**

```rust
// tests/integration/dedup_test.rs
#[test]
fn should_detect_hash_duplicate_within_window() {
    // Save same content twice within 15min, second should return DedupResult::HashMatch
}

#[test]
fn should_allow_hash_duplicate_after_window() {
    // Save same content, manually set created_at to 20min ago, second should insert
}

#[test]
fn should_upsert_on_topic_key_match() {
    // Save with topic_key "arch/auth", save again with same key
    // Should update content, increment revision_count
}

#[test]
fn should_detect_similar_content() {
    // Save two semantically similar observations
    // Should return DedupResult::SimilarContent with similarity score
    // (only when embeddings available)
}

// tests/integration/lifecycle_test.rs
#[test]
fn should_promote_buffer_to_working_on_access() {
    // Insert (buffer), access 2 times, verify tier = working
}

#[test]
fn should_promote_working_to_core_on_5_accesses() {
    // Insert, access 5 times, run compact, verify tier = core
}

#[test]
fn should_archive_stale_buffer() {
    // Insert, set created_at to 31 days ago, run compact, verify deleted_at set
}

#[test]
fn should_never_archive_core() {
    // Insert, promote to core, set old dates, run compact, verify NOT archived
}

#[test]
fn should_return_compaction_stats() {
    // Insert various observations, run compact, verify stats report
}
```

**Step 2: Run tests — expect fail**

**Step 3: Implement memory lifecycle**

`src/memory/dedup.rs`:
- `DedupResult` enum: `NewContent`, `HashMatch(i64)`, `TopicKeyUpsert(i64)`, `SimilarContent(i64, f64)`
- `check_dedup(db, embed_mgr, obs) -> Result<DedupResult>`
- Pipeline: hash check → topic_key check → similarity check (if model available)

`src/memory/decay.rs`:
- `evaluate_tier(obs) -> Tier` — applies promotion/demotion rules
- Buffer: access_count >= 2 OR accessed within 7 days → Working
- Working: access_count >= 5 OR revision_count >= 3 → Core
- Buffer: no access 30 days → archive
- Working: no access 90 days → archive
- Core: never archive

`src/memory/compact.rs`:
- `CompactionStats { promoted, archived, unchanged }`
- `run_compaction(db, project?) -> Result<CompactionStats>`
- Iterates all non-deleted observations, applies decay rules, updates tiers

`src/memory/mod.rs`:
- `MemoryManager` struct combining `Database`, `EmbeddingManager`, dedup, and compact
- `save_observation(obs) -> Result<SaveResult>` — dedup pipeline → insert/upsert → embed → sync FTS + vec
- `SaveResult { id, dedup_status, was_embedded }` struct

**Step 4: Run tests — expect pass**

**Step 5: Commit**

```bash
git commit -m "feat(memory): add dedup pipeline, 3-tier decay, and compaction sweep"
```

---

## Task 9: MCP Server — Tool Registration

**Files:**
- Create: `src/mcp/mod.rs`
- Create: `src/mcp/tools.rs`
- Create: `src/mcp/protocol.rs`
- Create: `tests/integration/mcp_test.rs`

**Step 1: Write failing tests**

```rust
// tests/integration/mcp_test.rs
#[test]
fn should_list_14_tools() {
    // Instantiate CortexMemServer, verify tool count
}

#[test]
fn should_save_via_mcp_tool() {
    // Call mem_save tool handler directly, verify observation created
}

#[test]
fn should_search_via_mcp_tool() {
    // Save observation, call mem_search, verify compact result format
}

#[test]
fn should_get_full_observation_via_mcp() {
    // Save, search, get by id — verify full content returned
}
```

**Step 2: Run tests — expect fail**

**Step 3: Implement MCP server**

`src/mcp/protocol.rs`:
- `CompactResult` struct (id, title, type, concepts, created_at) — for mem_search
- `FullResult` struct (all fields) — for mem_get
- `TimelineResult` struct — for mem_timeline
- `format_compact(observations) -> String` — token-efficient formatting
- `format_full(observation) -> String`
- `format_stats(stats) -> String`

`src/mcp/tools.rs`:
- `CortexMemServer` struct holding `MemoryManager`
- `#[tool_router]` impl block with `#[tool]` macro for each of the 14 tools:
  - `mem_save`, `mem_update`, `mem_session_summary`
  - `mem_search`, `mem_get`, `mem_timeline`, `mem_context`, `mem_suggest_topic`
  - `mem_session_start`, `mem_session_end`
  - `mem_delete`, `mem_stats`, `mem_compact`
- Each tool: parse parameters → call MemoryManager → format response → return `CallToolResult`

`src/mcp/mod.rs`:
- `start_mcp_server(db_path) -> Result<()>` — creates server, serves stdio transport
- Called from `Commands::Mcp` in main.rs

**Step 4: Run tests — expect pass**

**Step 5: Commit**

```bash
git commit -m "feat(mcp): add MCP server with 14 tools via rmcp stdio transport"
```

---

## Task 10: MCP Tools — Write Operations (mem_save, mem_update, mem_session_summary)

**Files:**
- Modify: `src/mcp/tools.rs` (implement write tool handlers)
- Create: `tests/integration/mcp_write_test.rs`

**Step 1: Write failing tests**

```rust
#[test]
fn mem_save_should_return_id_and_dedup_status() {
    // Call mem_save, verify response contains id
}

#[test]
fn mem_save_should_dedup_hash_match() {
    // Save same content twice, verify second returns "duplicate detected"
}

#[test]
fn mem_save_should_upsert_topic_key() {
    // Save with topic_key, save again, verify revision_count in response
}

#[test]
fn mem_update_should_modify_fields() {
    // Save, update title, get, verify new title
}

#[test]
fn mem_update_should_recompute_hash_and_embedding() {
    // Save, update content, verify new hash
}

#[test]
fn mem_session_summary_should_persist() {
    // Start session, save summary, end session, verify summary stored
}
```

**Step 2–5: Implement, test, commit**

```bash
git commit -m "feat(mcp): implement mem_save, mem_update, mem_session_summary write tools"
```

---

## Task 11: MCP Tools — Read Operations (mem_search, mem_get, mem_timeline, mem_context, mem_suggest_topic)

**Files:**
- Modify: `src/mcp/tools.rs` (implement read tool handlers)
- Create: `tests/integration/mcp_read_test.rs`

**Step 1: Write failing tests**

```rust
#[test]
fn mem_search_should_return_compact_results() {
    // Save 3 observations, search, verify compact format (no full content)
}

#[test]
fn mem_search_should_filter_by_type() {
    // Save decision + discovery, search with type=decision, verify only decision returned
}

#[test]
fn mem_get_should_return_full_observation() {
    // Save, get by id, verify all fields present including content
}

#[test]
fn mem_get_should_accept_multiple_ids() {
    // Save 3, get ids=[1,2], verify 2 returned
}

#[test]
fn mem_get_should_increment_access_count() {
    // Save, get, get again, verify access_count = 2
}

#[test]
fn mem_timeline_should_show_surrounding_observations() {
    // Save 5 observations, timeline for id=3 with window=2
    // Should return ids 1,2,3,4,5 in chronological order
}

#[test]
fn mem_context_should_return_recent_from_previous_sessions() {
    // Create session 1 with observations, end it
    // Create session 2, call mem_context
    // Should include observations from session 1
}

#[test]
fn mem_suggest_topic_should_find_similar_existing_keys() {
    // Save with topic_key "architecture/auth"
    // Call suggest_topic with title "Auth middleware design"
    // Should suggest "architecture/auth"
}
```

**Step 2–5: Implement, test, commit**

```bash
git commit -m "feat(mcp): implement mem_search, mem_get, mem_timeline, mem_context, mem_suggest_topic"
```

---

## Task 12: MCP Tools — Lifecycle & Admin (session_start, session_end, delete, stats, compact)

**Files:**
- Modify: `src/mcp/tools.rs` (implement remaining tool handlers)
- Create: `tests/integration/mcp_lifecycle_test.rs`

**Step 1: Write failing tests**

```rust
#[test]
fn mem_session_start_should_create_session_and_return_context() {
    // Call session_start, verify session created + context returned
}

#[test]
fn mem_session_end_should_close_session() {
    // Start, end with summary, verify ended_at set
}

#[test]
fn mem_delete_should_soft_delete() {
    // Save, delete, search — should not appear
}

#[test]
fn mem_stats_should_return_counts() {
    // Save 3 observations (2 decisions, 1 pattern), call stats
    // Verify counts by type and tier
}

#[test]
fn mem_compact_should_return_stats() {
    // Save observations with old dates, compact, verify promotion/archive counts
}
```

**Step 2–5: Implement, test, commit**

```bash
git commit -m "feat(mcp): implement session lifecycle, delete, stats, and compact tools"
```

---

## Task 13: CLI Commands

**Files:**
- Create: `src/cli/mod.rs`
- Create: `src/cli/save.rs`
- Create: `src/cli/search.rs`
- Create: `src/cli/stats.rs`
- Create: `src/cli/model.rs`
- Create: `src/cli/mcp.rs`
- Modify: `src/main.rs` (wire up all subcommands)

**Step 1: Define CLI subcommands**

```rust
#[derive(Subcommand)]
enum Commands {
    /// Launch MCP server (stdio transport)
    Mcp,
    /// Save an observation
    Save {
        #[arg(short, long)]
        title: String,
        #[arg(short, long)]
        content: String,
        #[arg(long, default_value = "discovery")]
        r#type: String,
        #[arg(long)]
        topic_key: Option<String>,
        #[arg(long, value_delimiter = ',')]
        concepts: Option<Vec<String>>,
        #[arg(long, value_delimiter = ',')]
        facts: Option<Vec<String>>,
        #[arg(long, value_delimiter = ',')]
        files: Option<Vec<String>>,
    },
    /// Search memories
    Search {
        query: String,
        #[arg(short, long, default_value = "20")]
        limit: usize,
        #[arg(long)]
        r#type: Option<String>,
        #[arg(long)]
        project: Option<String>,
    },
    /// Get full observation by ID
    Get { id: i64 },
    /// Show database statistics
    Stats,
    /// Manage embedding model
    Model {
        #[command(subcommand)]
        action: ModelAction,
    },
    /// Run compaction
    Compact,
}

#[derive(Subcommand)]
enum ModelAction {
    /// Download the embedding model
    Download,
    /// Show model status
    Status,
}
```

**Step 2: Implement each CLI handler**

Each handler: detect project from `pwd` → open DB → call MemoryManager method → format output to stdout.

**Step 3: Test manually**

```bash
cargo run -- save --title "Test" --content "Hello world" --type decision
cargo run -- search "test"
cargo run -- get 1
cargo run -- stats
cargo run -- model status
```

**Step 4: Commit**

```bash
git commit -m "feat(cli): add save, search, get, stats, model, and compact subcommands"
```

---

## Task 14: Plugin Package — Hooks & Memory Protocol Skill

**Files:**
- Create: `plugin/hooks/hooks.json`
- Create: `plugin/scripts/session-start.sh`
- Create: `plugin/scripts/session-end.sh`
- Create: `plugin/scripts/compaction-recovery.sh`
- Create: `plugin/skills/memory-protocol/SKILL.md`

**Step 1: Create hooks.json**

```json
{
  "hooks": [
    {
      "event": "SessionStart",
      "command": "plugin/scripts/session-start.sh",
      "timeout": 5000
    },
    {
      "event": "Stop",
      "command": "plugin/scripts/session-end.sh",
      "timeout": 5000
    }
  ]
}
```

**Step 2: Create session-start.sh**

Detects project from `$PWD`, calls `cortexmem mcp` to verify binary exists, outputs Memory Protocol instructions + recent context.

**Step 3: Create session-end.sh**

Calls `cortexmem` CLI to end the session.

**Step 4: Create compaction-recovery.sh**

Re-injects Memory Protocol skill text + instructions to call `mem_session_summary` and `mem_context`.

**Step 5: Create Memory Protocol SKILL.md**

The instructions injected into agent context that guide when/what to save and search. Content:
- When to call `mem_save` (decisions, discoveries, patterns, bugs, milestones)
- When NOT to save (routine reads, trivial changes)
- When to call `mem_search` (before starting work on a topic)
- How to use topic_keys for evolving knowledge
- Progressive disclosure workflow: search → get → full detail
- Compaction recovery instructions

**Step 6: Commit**

```bash
git commit -m "feat(plugin): add Claude Code hooks, session scripts, and Memory Protocol skill"
```

---

## Post-Implementation

After all 14 tasks are complete:

1. **Push to main** — triggers CI + release workflow
2. **Verify GitHub Release** created with v0.1.0 tag
3. **Cross-compile build** — verify binaries for all platforms
4. **Test npm package** — `npx cortexmem status` on clean machine
5. **Test Claude Code integration** — configure MCP server, verify tools appear
6. **Update README.md** with installation instructions and usage examples
