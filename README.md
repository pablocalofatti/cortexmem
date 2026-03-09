# cortexmem

Persistent memory for AI coding agents. Hybrid search (FTS5 + semantic vectors), session lifecycle, and memory decay — all in a single Rust binary with zero external services.

cortexmem gives your AI agent **memory that survives across sessions and context compactions**. It stores decisions, patterns, bug fixes, and discoveries in a local SQLite database and retrieves them using a combination of keyword search (BM25) and semantic vector similarity (RRF fusion).

## Why cortexmem?

AI coding agents forget everything between sessions. Context windows get compacted. Decisions get lost. You end up re-explaining the same architecture choices, debugging the same issues, and losing institutional knowledge.

cortexmem fixes this:

- **Persistent memory** — observations survive across sessions and context compactions
- **Hybrid search** — FTS5 keyword matching + vector similarity with RRF fusion
- **Memory lifecycle** — buffer → working → core tiers with automatic decay
- **Deduplication** — content hash + topic key upsert prevents duplicate observations
- **Session tracking** — session start/end with summaries for context recovery
- **Zero infrastructure** — single binary, local SQLite, no servers or API keys needed
- **Works with any agent** — MCP protocol (stdio) compatible with Claude Code, Cursor, Windsurf, and more

## Quick Start

### Install

```bash
# From source
cargo install --path .

# The embedding model downloads automatically on first use
# Or download it manually:
cortexmem model download
```

### Set Up Your Agent

```bash
cortexmem setup
```

The interactive wizard configures cortexmem as an MCP server for your AI agent. Supports:

| Agent | Config Location |
|-------|----------------|
| Claude Code | `~/.claude/settings.json` |
| Cursor | `~/.cursor/mcp.json` |
| Windsurf | `~/.codeium/windsurf/mcp_config.json` |
| Cline / Roo Code | `.vscode/mcp.json` (workspace) |
| Continue | `~/.continue/config.json` |

For Claude Code, the wizard also installs the Memory Protocol skill (session hooks, compaction recovery, and the SKILL.md that teaches the agent when and how to save observations).

### Verify

```bash
cortexmem stats
cortexmem model status
```

## How It Works

### MCP Tools (14 tools)

cortexmem exposes 14 tools via the [Model Context Protocol](https://modelcontextprotocol.io/):

| Tool | Description |
|------|-------------|
| `mem_save` | Save an observation (decision, pattern, bug fix, etc.) with dedup |
| `mem_update` | Update fields of an existing observation by ID |
| `mem_search` | Hybrid FTS5 + vector search with RRF fusion |
| `mem_get` | Get full observation detail by ID |
| `mem_timeline` | Chronological context around a target observation |
| `mem_context` | Recent observations for context recovery at session start |
| `mem_suggest_topic` | Generate topic keys and find existing matches |
| `mem_session_start` | Start a new session, returns recent context |
| `mem_session_end` | End session with optional summary, triggers decay |
| `mem_session_summary` | Persist a compaction summary mid-session |
| `mem_delete` | Soft-delete an observation (recoverable) |
| `mem_stats` | Memory statistics by type and tier |
| `mem_compact` | Run decay cycle (promote/archive by access patterns) |
| `mem_model` | Check or download the embedding model |

### Observation Types

| Type | Use For |
|------|---------|
| `decision` | Architectural choices, trade-offs, rationale |
| `pattern` | Recurring code patterns, conventions, idioms |
| `bug_fix` | Root cause, fix, and prevention notes |
| `discovery` | Learned behavior, undocumented APIs, gotchas |
| `milestone` | Major completions, release notes |

### Search Architecture

```
Query
  ├─ FTS5 (BM25 keyword matching)
  ├─ Vector KNN (semantic similarity, 384-dim embeddings)
  └─ RRF Fusion (k=60) + recency boost + access frequency
       → Ranked results
```

If the embedding model isn't downloaded, search degrades gracefully to FTS5-only — still useful, just without semantic understanding.

### Memory Lifecycle

Observations flow through three tiers based on access patterns:

```
buffer → working → core
  │         │        │
  │         │        └─ Frequently accessed, high value (preserved)
  │         └─ Moderate access, proven useful (promoted on access)
  └─ New observations start here (archived if unused)
```

`mem_compact` evaluates each observation and promotes or archives based on access count, revision count, and age.

## CLI

```bash
# Save an observation
cortexmem save --title "Auth decision" --content "Chose JWT over sessions" --type decision

# Search memories
cortexmem search "authentication" --limit 10 --type decision

# Get full observation
cortexmem get 42

# View stats
cortexmem stats

# Run compaction
cortexmem compact

# Export/Import
cortexmem export --output backup.json --project myproject
cortexmem import backup.json          # merge mode (skips duplicates)
cortexmem import backup.json --replace # replace mode (wipes existing data)

# Embedding model
cortexmem model download
cortexmem model status
```

## Architecture

```
┌─────────────────────────────────────────┐
│              cortexmem binary           │
├──────────┬──────────┬───────────────────┤
│   CLI    │   MCP    │    Memory Mgr     │
│  (clap)  │ (rmcp)   │  (dedup, decay)   │
├──────────┴──────────┴───────────────────┤
│          Hybrid Search (RRF)            │
│       FTS5 BM25  +  Vector KNN         │
├──────────────────┬──────────────────────┤
│   SQLite (WAL)   │  fastembed (ONNX)    │
│ FTS5 + sqlite-vec│  all-MiniLM-L6-v2   │
└──────────────────┴──────────────────────┘
```

- **Single binary** — no daemon, no port management, no docker
- **SQLite WAL** — concurrent reads, single-writer, ~5ms queries
- **fastembed** — local ONNX inference, no API keys, Apple Silicon native
- **384-dim embeddings** — all-MiniLM-L6-v2 via fastembed

## MCP Server Setup

### Using `claude mcp add` (recommended)

The easiest way to register cortexmem as an MCP server:

```bash
# Install the binary
cargo install --path .

# Register with Claude Code
claude mcp add --transport stdio cortexmem -- cortexmem mcp

# Verify it's connected
claude mcp list
```

### Using the setup wizard

```bash
cortexmem setup
```

The wizard auto-detects your agent and writes the MCP config for you.

### Manual configuration

Add to your agent's MCP config:

```json
{
  "cortexmem": {
    "command": "cortexmem",
    "args": ["mcp"],
    "type": "stdio"
  }
}
```

If the binary isn't on your `PATH`, use the full path:

```json
{
  "cortexmem": {
    "command": "/Users/you/.cargo/bin/cortexmem",
    "args": ["mcp"],
    "type": "stdio"
  }
}
```

### Environment variables

| Variable | Description | Default |
|----------|-------------|---------|
| `CORTEXMEM_DB` | Override database file path | `<data_dir>/cortexmem/cortexmem.db` |
| `RUST_LOG` | Enable debug logging (e.g. `cortexmem=debug`) | off |

### Troubleshooting

| Problem | Solution |
|---------|----------|
| MCP server not showing up | Use `claude mcp list` to check. Try `claude mcp add` instead of editing settings.json manually |
| "unable to open database file" | Set `CORTEXMEM_DB` to a writable path, or ensure the data directory exists |
| Binary not found | Use the full path to the binary (e.g. `~/.cargo/bin/cortexmem`) |
| Embedding model missing | The model auto-downloads on first `mem_save` or `mem_search`. Or run `cortexmem model download` manually |

## Data Storage

All data lives in a single SQLite database:

| Platform | Location |
|----------|----------|
| macOS | `~/Library/Application Support/cortexmem/cortexmem.db` |
| Linux | `~/.local/share/cortexmem/cortexmem.db` |

## Plugin System (Claude Code)

When you run `cortexmem setup` with Claude Code, it installs:

- **Session hooks** — automatically call `mem_session_start` and `mem_session_end`
- **Compaction recovery** — saves context before context window compaction
- **Memory Protocol skill** — teaches the agent when and how to save observations

```
~/.claude/
├── hooks.json              ← session start/end hooks
├── scripts/
│   ├── session-start.sh
│   ├── session-end.sh
│   └── compaction-recovery.sh
└── skills/cortexmem/
    └── SKILL.md            ← Memory Protocol instructions
```

## License

MIT
