# cortexmem Roadmap Design — v1.0.0 through v1.2.0

**Date:** 2026-03-09
**Status:** Approved

## Overview

This document specifies features across three releases that evolve cortexmem from a Claude Code-only memory tool into an agent-agnostic platform with HTTP API, cloud sync, TUI, and git-based team sharing.

**Design decisions captured:**
- Interactive wizard for setup (dialoguer)
- JSON for export/import format
- Same binary for HTTP server (`cortexmem serve`)
- Self-hosted Postgres backend for cloud sync
- ratatui for TUI

---

## v1.0.0 — High Impact, Low Effort

### Feature 1: `cortexmem setup` — Interactive Setup Wizard

**New dependency:** `dialoguer = "0.11"`

**Subcommand:** `cortexmem setup`

**Flow:**
1. "Which agent do you use?" → select from list
2. Auto-detect config path for selected agent
3. Generate MCP config JSON (stdio transport)
4. Copy plugin hooks + skills (Claude Code only)
5. Print success message

**Agent config paths:**

| Agent | Config Location | Format |
|-------|----------------|--------|
| Claude Code | `~/.claude/settings.json` → `mcpServers` | JSON merge |
| OpenCode | `~/.config/opencode/config.json` → `mcpServers` | JSON merge |
| Cursor | `~/.cursor/mcp.json` | JSON merge |
| Windsurf | `~/.codeium/windsurf/mcp_config.json` | JSON merge |
| VS Code | `.vscode/mcp.json` (workspace-level) | JSON create/merge |
| Gemini CLI | `~/.gemini/settings.json` | JSON merge |

**MCP config written for all agents:**
```json
{
  "cortexmem": {
    "command": "cortexmem",
    "args": ["mcp"],
    "type": "stdio"
  }
}
```

**Claude Code extras:**
- Copy `plugin/hooks/hooks.json` → `~/.claude/hooks/`
- Copy `plugin/scripts/` → `~/.claude/scripts/cortexmem/`
- Copy `plugin/skills/memory-protocol/SKILL.md` → `~/.claude/skills/cortexmem/`

**Non-Claude agents:** SKILL.md content is already embedded via `ServerInfo.with_instructions()`, so all agents get the Memory Protocol guidance through MCP.

**Implementation notes:**
- New file: `src/cli/setup.rs`
- Add `Setup` variant to `Commands` enum in `main.rs`
- Use `dialoguer::Select` for agent choice, `dialoguer::Confirm` for overwrite prompts
- All file operations use `std::fs` with create_dir_all for parents

### Feature 2: Improve `mem_suggest_topic`

**Current behavior:** Lists all existing topic_keys (useless for new projects).

**New behavior:** Generate family-prefixed keys from type + title using heuristic rules, plus return matching existing keys.

**Key generation logic:**
```rust
fn suggest_topic_key(obs_type: &str, title: &str) -> String {
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
    let slug = title.to_lowercase()
        .replace(|c: char| !c.is_alphanumeric() && c != ' ', "")
        .split_whitespace()
        .take(4)
        .collect::<Vec<_>>()
        .join("-");
    format!("{family}/{slug}")
}
```

**MCP tool changes:**
- `MemSuggestTopicParams` adds `obs_type: Option<String>` field
- Response format: suggested key + top 3 existing matching keys (by FTS similarity on the title)
- Example response:
  ```
  Suggested: architecture/jwt-middleware
  Existing matches:
    - architecture/auth-model (3 revisions)
    - architecture/api-gateway (1 revision)
  ```

**Implementation:** Modify `mem_suggest_topic` handler in `src/mcp/tools.rs`. Add `suggest_topic_key()` helper function. Add FTS query for matching existing keys.

### Feature 3: Export/Import

**CLI:**
```
cortexmem export [--output FILE] [--project PROJECT]
cortexmem import <FILE> [--merge|--replace]
```

**Default output:** `cortexmem-export.json` in current directory.

**JSON format:**
```json
{
  "version": "1.0",
  "exported_at": "2026-03-09T00:00:00Z",
  "project_filter": null,
  "sessions": [
    {
      "id": 1,
      "project": "cortexmem",
      "directory": "/path/to/project",
      "summary": "...",
      "started_at": "2026-03-08T20:00:00",
      "ended_at": "2026-03-08T22:00:00"
    }
  ],
  "observations": [
    {
      "id": 1,
      "session_id": 1,
      "project": "cortexmem",
      "topic_key": "architecture/mcp-server",
      "type": "architecture",
      "title": "MCP server design",
      "content": "...",
      "concepts": ["mcp", "rmcp"],
      "facts": ["uses rmcp 1.1"],
      "files": ["src/mcp/tools.rs"],
      "scope": "project",
      "tier": "working",
      "access_count": 5,
      "revision_count": 2,
      "content_hash": "abc123...",
      "created_at": "2026-03-08T20:30:00",
      "updated_at": "2026-03-08T21:00:00",
      "deleted_at": null
    }
  ]
}
```

**Import modes:**
- `--merge` (default): Skip observations with matching `content_hash`, insert new ones. Session IDs are remapped to avoid conflicts.
- `--replace`: Drop all data, import fresh. Confirmation prompt before proceeding.

**Implementation:**
- New file: `src/cli/export.rs`
- Add `Export` and `Import` variants to `Commands` enum
- Add `export_all()` and `import_from_file()` methods to `Database`
- Derive `Serialize` on `Observation` and `Session` structs (already have `serde` dep)
- No new dependencies

### Feature 4: Multi-Agent Setup Support

Covered by Feature 1. The interactive wizard handles all 6 agents. Key difference per agent:

- **Claude Code:** Full setup (MCP config + hooks + scripts + skills)
- **All others:** MCP config only (Memory Protocol delivered via `ServerInfo.with_instructions()`)

---

## v1.1.0 — Next Release

### Feature 5: HTTP API — `cortexmem serve`

**New dependencies:** `axum = "0.8"`, `tower-http = "0.6"` (CORS)

**Subcommand:** `cortexmem serve [--port 7437] [--host 0.0.0.0]`

**Architecture:** `Arc<CortexMemServer>` shared across axum handlers. Same `call_*` methods used by CLI and MCP — zero logic duplication.

```
HTTP Request → axum handler → Arc<CortexMemServer>.call_*() → SQLite
MCP Request  → rmcp handler → &CortexMemServer.call_*()     → SQLite
CLI Command  → cli handler  → CortexMemServer.call_*()      → SQLite
```

**Endpoints:**

| Method | Path | Maps to | Description |
|--------|------|---------|-------------|
| GET | `/health` | — | `{"status":"ok","version":"..."}` |
| POST | `/sessions` | `call_session_start` | Create session `{project, directory}` |
| POST | `/sessions/:id/end` | `call_session_end` | End session `{summary?}` |
| GET | `/sessions/recent` | new query | `?project=X&limit=N` |
| POST | `/observations` | `call_save` | Create observation |
| GET | `/observations/recent` | `call_context` | `?project=X&scope=&limit=N` |
| GET | `/observations/:id` | `call_get` | Full observation |
| PATCH | `/observations/:id` | `call_update` | Partial update |
| DELETE | `/observations/:id` | `call_delete` | `?hard=true` for permanent |
| GET | `/search` | `call_search` | `?q=&type=&project=&scope=&limit=` |
| GET | `/timeline` | timeline query | `?observation_id=N&before=5&after=5` |
| GET | `/context` | `call_context` | `?project=X&scope=` |
| GET | `/export` | export logic | JSON dump |
| POST | `/import` | import logic | JSON import |
| GET | `/stats` | `call_stats` | Memory statistics |

**Concurrency:** `Mutex<MemoryManager>` serializes writes, WAL mode allows concurrent reads. Sufficient for local dev tool.

**Auth:** None for v0.2.0 (localhost only).

**Implementation:**
- New file: `src/http/mod.rs` with axum router + handlers
- New file: `src/http/routes.rs` with endpoint implementations
- Add `Serve` variant to `Commands` enum
- Add `pub mod http` to `lib.rs`

### Feature 6: `mem_save_prompt` — User Prompt Storage

**Schema change (migration v2):**
```sql
CREATE TABLE IF NOT EXISTS user_prompts (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id  INTEGER REFERENCES sessions(id),
    content     TEXT NOT NULL,
    project     TEXT,
    created_at  TEXT DEFAULT (datetime('now'))
);

CREATE VIRTUAL TABLE IF NOT EXISTS prompts_fts USING fts5(
    content, project,
    content=user_prompts,
    content_rowid=id,
    tokenize='porter unicode61'
);
```

**New MCP tool:**
```
mem_save_prompt(content: String, session_id?: i64, project?: String)
```

**New DB methods:**
- `insert_prompt(session_id, content, project) -> Result<i64>`
- `get_recent_prompts(project, limit) -> Result<Vec<Prompt>>`
- `search_prompts(query, project, limit) -> Result<Vec<Prompt>>`

**Integration:** `mem_context` response updated to include recent prompts alongside observations.

**HTTP endpoints:**
- `POST /prompts` → save prompt
- `GET /prompts/recent` → `?project=X&limit=N`
- `GET /prompts/search` → `?q=QUERY&project=X&limit=N`

**Implementation:**
- New file: `src/db/prompts.rs`
- Update `src/db/schema.rs` migration to v2
- Add `mem_save_prompt` tool handler to `src/mcp/tools.rs`
- Update `mem_context` to include prompts
- New struct: `Prompt { id, session_id, content, project, created_at }`

### Feature 7: Hard Delete Option

**Change to `MemDeleteParams`:**
```rust
pub struct MemDeleteParams {
    pub id: i64,
    #[serde(default)]
    pub hard: Option<bool>,
}
```

**Behavior:**
- `hard: false/None` (default) → `UPDATE observations SET deleted_at = datetime('now') WHERE id = ?`
- `hard: true` → `DELETE FROM observations WHERE id = ?` + remove from FTS + remove from vec_observations

**New DB method:** `hard_delete_observation(id) -> Result<()>`

**CLI:** Add `Delete` variant: `cortexmem delete <id> [--hard]`

**HTTP:** `DELETE /observations/:id?hard=true`

### Feature 8: Homebrew Formula

**Create tap repo:** `pablocalofatti/homebrew-tap`

**Formula:** `Formula/cortexmem.rb` that downloads pre-built binaries from GitHub Releases.

```ruby
class Cortexmem < Formula
  desc "Persistent vector memory for AI coding agents"
  homepage "https://github.com/pablocalofatti/cortexmem"
  version "1.0.0"

  on_macos do
    on_arm do
      url "https://github.com/pablocalofatti/cortexmem/releases/download/v#{version}/cortexmem-darwin-arm64.tar.gz"
      sha256 "PLACEHOLDER"
    end
    on_intel do
      url "https://github.com/pablocalofatti/cortexmem/releases/download/v#{version}/cortexmem-darwin-x64.tar.gz"
      sha256 "PLACEHOLDER"
    end
  end

  on_linux do
    on_arm do
      url "https://github.com/pablocalofatti/cortexmem/releases/download/v#{version}/cortexmem-linux-arm64.tar.gz"
      sha256 "PLACEHOLDER"
    end
    on_intel do
      url "https://github.com/pablocalofatti/cortexmem/releases/download/v#{version}/cortexmem-linux-x64.tar.gz"
      sha256 "PLACEHOLDER"
    end
  end

  def install
    bin.install "cortexmem"
  end

  test do
    assert_match "cortexmem", shell_output("#{bin}/cortexmem --version")
  end
end
```

**Release workflow update:** Tar.gz each platform binary, upload as release assets, update tap formula SHA256 hashes.

**Installation:** `brew install pablocalofatti/tap/cortexmem`

---

## v1.2.0 — Major Release

### Feature 9: Cloud Sync — Self-hosted Postgres Backend

**New dependencies:** `sqlx = "0.8"`, `jsonwebtoken = "9"`, `argon2 = "0.5"`

**Two components:**
1. **Sync engine** in cortexmem binary (local SQLite ↔ cloud server)
2. **Cloud server** as `cortexmem cloud serve`

**Postgres schema:**
```sql
CREATE TABLE accounts (
    id            UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    email         TEXT UNIQUE NOT NULL,
    password_hash TEXT NOT NULL,
    created_at    TIMESTAMPTZ DEFAULT now()
);

CREATE TABLE api_keys (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    account_id  UUID REFERENCES accounts(id),
    key_hash    TEXT NOT NULL,
    prefix      TEXT NOT NULL,
    created_at  TIMESTAMPTZ DEFAULT now(),
    revoked_at  TIMESTAMPTZ
);

CREATE TABLE sync_mutations (
    seq         BIGSERIAL PRIMARY KEY,
    account_id  UUID REFERENCES accounts(id),
    entity      TEXT NOT NULL,
    entity_key  TEXT NOT NULL,
    op          TEXT NOT NULL,
    payload     JSONB NOT NULL,
    project     TEXT NOT NULL,
    occurred_at TIMESTAMPTZ NOT NULL,
    acked_at    TIMESTAMPTZ
);

CREATE TABLE enrolled_projects (
    account_id  UUID REFERENCES accounts(id),
    project     TEXT NOT NULL,
    enrolled_at TIMESTAMPTZ DEFAULT now(),
    PRIMARY KEY (account_id, project)
);
```

**Local sync tables (SQLite migration v3):**
```sql
CREATE TABLE IF NOT EXISTS sync_mutations (
    seq         INTEGER PRIMARY KEY AUTOINCREMENT,
    entity      TEXT NOT NULL,
    entity_key  TEXT NOT NULL,
    op          TEXT NOT NULL,
    payload     TEXT NOT NULL,
    project     TEXT NOT NULL,
    occurred_at TEXT NOT NULL,
    acked_at    TEXT
);

CREATE TABLE IF NOT EXISTS sync_state (
    target_key      TEXT PRIMARY KEY,
    last_pushed_seq INTEGER DEFAULT 0,
    last_pulled_seq INTEGER DEFAULT 0,
    last_error      TEXT,
    updated_at      TEXT DEFAULT (datetime('now'))
);
```

**Sync protocol (mutation-based):**
1. **Capture:** Every write op appends to local `sync_mutations`
2. **Push:** POST unacked mutations in batches to cloud
3. **Pull:** GET mutations from server with `seq > last_pulled_seq`, apply locally
4. **Ack:** Server acks, client updates `last_pushed_seq`
5. **Conflict resolution:** Last-writer-wins by `occurred_at`

**Auto-sync:** Background thread, push+pull every 60s, exponential backoff on failure.

**CLI subcommands:**
```
cortexmem cloud serve --port 8080 --database-url URL
cortexmem cloud register --server URL
cortexmem cloud login --server URL
cortexmem cloud sync [--auto]
cortexmem cloud sync-status
cortexmem cloud api-key
cortexmem cloud enroll <project>
cortexmem cloud unenroll <project>
cortexmem cloud projects
```

**Cloud API endpoints:**

| Method | Path | Auth | Description |
|--------|------|------|-------------|
| POST | `/auth/register` | None | Create account |
| POST | `/auth/login` | None | Get JWT |
| POST | `/auth/api-key` | JWT | Generate API key (`ctx_...`) |
| POST | `/sync/push` | API key | Upload mutations batch |
| GET | `/sync/pull` | API key | `?since_seq=N&project=X` |
| POST | `/sync/ack` | API key | Ack pushed mutations |
| GET | `/sync/status` | API key | Sync state |
| POST | `/projects/enroll` | API key | Enroll project |
| DELETE | `/projects/:name` | API key | Unenroll |
| GET | `/projects` | API key | List enrolled |
| GET | `/health` | None | Server health |

**Docker compose:**
```yaml
services:
  postgres:
    image: postgres:16-alpine
    environment:
      POSTGRES_DB: cortexmem_cloud
      POSTGRES_USER: cortexmem
      POSTGRES_PASSWORD: cortexmem_dev
    ports: ["5433:5432"]
    volumes: ["pgdata:/var/lib/postgresql/data"]

  cortexmem-cloud:
    build: .
    command: ["cortexmem", "cloud", "serve"]
    environment:
      CORTEXMEM_DATABASE_URL: postgres://cortexmem:cortexmem_dev@postgres:5432/cortexmem_cloud
      CORTEXMEM_JWT_SECRET: ${JWT_SECRET:-change-me-in-production}
      CORTEXMEM_PORT: "8080"
    ports: ["8080:8080"]
    depends_on: [postgres]

volumes:
  pgdata:
```

**Environment variables:**

| Variable | Description | Default |
|----------|-------------|---------|
| `CORTEXMEM_CLOUD_URL` | Cloud server URL | None |
| `CORTEXMEM_CLOUD_TOKEN` | API key for auth | None |
| `CORTEXMEM_DATABASE_URL` | Postgres DSN (server) | None |
| `CORTEXMEM_JWT_SECRET` | JWT signing secret (≥32 chars) | None |
| `CORTEXMEM_PORT` | Cloud server port | 8080 |

**Implementation:**
- New module: `src/cloud/` with `mod.rs`, `server.rs`, `auth.rs`, `sync.rs`, `schema.rs`
- New module: `src/sync/` with `engine.rs`, `mutations.rs`
- Add `Cloud` subcommand group to `Commands` enum
- Docker files in project root

### Feature 10: TUI — Interactive Terminal Browser

**New dependencies:** `ratatui = "0.29"`, `crossterm = "0.28"`

**Subcommand:** `cortexmem tui`

**Screens:**

| Screen | Content | Navigation |
|--------|---------|------------|
| Dashboard | Stats overview + menu | Entry point |
| Search | Text input | `s` or `/` from any screen |
| Search Results | Scrollable list | `Enter` → detail |
| Observation Detail | Full content, scrollable | `t` → timeline, `Esc` → back |
| Timeline | Chronological context | `Enter` → detail |
| Sessions | Session list | `Enter` → session detail |
| Session Detail | Observations within session | `Enter` → observation detail |

**Keybindings:**

| Key | Action |
|-----|--------|
| `j/k` or `↑/↓` | Navigate lists |
| `Enter` | Select / drill into |
| `t` | Timeline for selected |
| `s` or `/` | Quick search |
| `Esc` | Go back |
| `q` | Quit |
| `d` | Delete (with confirm) |
| `e` | Export current view |

**Architecture:**
```rust
enum Screen {
    Dashboard,
    Search { query: String },
    SearchResults { results: Vec<SearchResult> },
    ObservationDetail { obs: Observation },
    Timeline { center: i64, items: Vec<Observation> },
    Sessions { sessions: Vec<Session> },
    SessionDetail { session: Session, observations: Vec<Observation> },
}

struct App {
    screen: Screen,
    server: Arc<CortexMemServer>,
    selected_index: usize,
    scroll_offset: usize,
}
```

Reuses `CortexMemServer.call_*()` — no new DB queries.

**Color theme:** Catppuccin Mocha palette.

**Implementation:**
- New module: `src/tui/` with `mod.rs`, `app.rs`, `screens/` (one file per screen), `theme.rs`
- Add `Tui` variant to `Commands` enum

### Feature 11: Git Sync — Repository-based Team Sharing

**Subcommand:**
```
cortexmem sync [--repo PATH] [--auto]
cortexmem sync --init
cortexmem sync --status
```

**How it works:**
1. **Init:** Creates/clones sync git repo at `~/.cortexmem/sync/` (or custom path)
2. **Export:** New observations since last sync → JSON chunk file: `chunks/{project}/{timestamp}.json`
3. **Commit + Push:** Commits chunk, pushes to remote
4. **Pull + Import:** Pulls remote, finds new chunks, imports with dedup

**Chunk format:**
```json
{
  "chunk_id": "uuid-v4",
  "source": "machine-hostname",
  "project": "cortexmem",
  "exported_at": "2026-03-09T00:00:00Z",
  "observations": [...],
  "sessions": [...]
}
```

**Dedup on import:** `sync_chunks` table tracks imported chunk IDs.

```sql
CREATE TABLE IF NOT EXISTS sync_chunks (
    chunk_id    TEXT PRIMARY KEY,
    imported_at TEXT DEFAULT (datetime('now'))
);
```

**Auto-sync:** Background daemon, syncs every 5 minutes.

**Conflict resolution:** Append-only chunks, content_hash dedup prevents exact duplicates.

**New dependency:** `uuid = "1"` for chunk IDs (or use content hash).

**Implementation:**
- New module: `src/sync/git.rs`
- Uses `std::process::Command` to invoke `git` CLI (no git library dependency)
- Add `Sync` subcommand group to `Commands` enum

---

## Dependency Summary

| Version | New Dependencies |
|---------|-----------------|
| v1.0.0 | `dialoguer = "0.11"` |
| v1.1.0 | `axum = "0.8"`, `tower-http = "0.6"` |
| v1.2.0 | `sqlx = "0.8"`, `jsonwebtoken = "9"`, `argon2 = "0.5"`, `ratatui = "0.29"`, `crossterm = "0.28"`, `uuid = "1"` |

## Migration Plan

| Version | Schema Version | Changes |
|---------|---------------|---------|
| v1.0.0 | 1 (current) | No schema changes |
| v1.1.0 | 2 | Add `user_prompts` + `prompts_fts` tables |
| v1.2.0 | 3 | Add `sync_mutations`, `sync_state`, `sync_chunks` tables |

Migrations are additive — existing data is never lost.
