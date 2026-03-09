# Memory Protocol

You have access to **cortexmem**, a persistent memory system with 14 tools. Use it to store and retrieve observations across sessions and context compactions.

## Tools Reference

### Write Tools

#### `mem_save` — Save an observation to memory
Stores a decision, pattern, bug fix, discovery, or milestone. Supports dedup via content hash and topic_key upsert.

| Param | Type | Required | Description |
|-------|------|----------|-------------|
| `project` | string | yes | Project name (e.g., `"cortexmem"`) |
| `title` | string | yes | Short title for the observation |
| `content` | string | yes | Full content/detail |
| `type` | string | yes | `decision`, `pattern`, `bug_fix`, `discovery`, `milestone` |
| `concepts` | string[] | no | Keywords for search (e.g., `["auth", "jwt"]`) |
| `facts` | string[] | no | Key takeaways (e.g., `["JWT expires after 24h"]`) |
| `files` | string[] | no | Related file paths |
| `topic_key` | string | no | Enables upsert — use `mem_suggest_topic` first |
| `scope` | string | no | `"project"` (default) or `"global"` |

#### `mem_update` — Update an existing observation
Updates specific fields by ID. Recomputes content hash and re-embeds.

| Param | Type | Required | Description |
|-------|------|----------|-------------|
| `id` | number | yes | Observation ID |
| `title` | string | no | New title |
| `content` | string | no | New content |
| `concepts` | string[] | no | New concepts |
| `facts` | string[] | no | New facts |
| `files` | string[] | no | New file paths |

#### `mem_delete` — Soft-delete an observation
Sets `deleted_at` timestamp. Observation is recoverable.

| Param | Type | Required |
|-------|------|----------|
| `id` | number | yes |

### Read Tools

#### `mem_search` — Hybrid search (FTS5 + vector)
Searches using keyword matching (BM25) and semantic vector similarity with RRF fusion. Returns compact results: id, title, type, concepts, score.

| Param | Type | Required | Description |
|-------|------|----------|-------------|
| `query` | string | yes | Search query |
| `project` | string | no | Filter by project |
| `type` | string | no | Filter by observation type |
| `scope` | string | no | Filter by scope |
| `limit` | number | no | Max results (default: 20) |

#### `mem_get` — Get full observation detail
Returns all fields including content, facts, files. Supports single or batch retrieval.

| Param | Type | Required | Description |
|-------|------|----------|-------------|
| `id` | number | no | Single observation ID |
| `ids` | number[] | no | Multiple observation IDs |

#### `mem_timeline` — Chronological context
Shows observations saved before and after a target observation.

| Param | Type | Required | Description |
|-------|------|----------|-------------|
| `id` | number | yes | Target observation ID |
| `window` | number | no | Number of neighbors (default: 5) |

#### `mem_context` — Recent context recovery
Returns recent observations from previous sessions. Use at session start.

| Param | Type | Required |
|-------|------|----------|
| `project` | string | no |

#### `mem_suggest_topic` — Generate topic keys
Generates a `{family}/{slug}` topic key and returns existing keys from the same family. Use before `mem_save` to enable upsert.

| Param | Type | Required | Description |
|-------|------|----------|-------------|
| `title` | string | yes | Observation title |
| `type` | string | no | Observation type (determines family prefix) |
| `content` | string | no | Content (reserved for future use) |

### Session Tools

#### `mem_session_start` — Start a session
Creates a session record and returns recent context for the project.

| Param | Type | Required | Description |
|-------|------|----------|-------------|
| `project` | string | yes | Project name |
| `directory` | string | yes | Working directory path |

#### `mem_session_end` — End a session
Closes the session, optionally stores a summary, and triggers a decay cycle.

| Param | Type | Required |
|-------|------|----------|
| `summary` | string | no |

#### `mem_session_summary` — Mid-session summary
Persists a compaction summary. Call when context is about to be compacted.

| Param | Type | Required |
|-------|------|----------|
| `summary` | string | yes |

### Maintenance Tools

#### `mem_stats` — Memory statistics
Shows counts by type and tier, database size, and embedding model status.

| Param | Type | Required |
|-------|------|----------|
| `project` | string | no |

#### `mem_compact` — Run decay cycle
Promotes frequently accessed observations, archives stale ones. Returns stats.

| Param | Type | Required |
|-------|------|----------|
| `project` | string | no |

#### `mem_model` — Embedding model status
Checks if the embedding model is downloaded and triggers download if needed.

No parameters.

## Usage Patterns

### Session Lifecycle

```
1. mem_session_start  →  creates session, returns recent context
2. mem_search         →  check existing knowledge before working
3. mem_save           →  store observations as you work
4. mem_session_summary → save context before compaction
5. mem_session_end    →  close session, trigger decay
```

### Progressive Disclosure (read efficiently)

```
1. mem_search   →  compact results (titles, types, scores)
2. mem_get      →  full detail for relevant hits
3. mem_timeline →  chronological context around an observation
```

### Topic Key Upsert (update instead of duplicate)

```
1. mem_suggest_topic  →  get suggested key + existing keys
2. mem_save           →  pass topic_key to update in place
```

## When to Save

Save observations when you encounter:
- **Decisions** (`type: decision`) — architectural choices, trade-offs, rationale
- **Patterns** (`type: pattern`) — recurring code patterns, conventions, idioms
- **Bug fixes** (`type: bug_fix`) — root cause, fix, and prevention notes
- **Discoveries** (`type: discovery`) — learned behavior, undocumented APIs, gotchas
- **Milestones** (`type: milestone`) — major completions, release notes

### What NOT to Save
- Routine file reads or trivial changes
- Information already in the codebase (README, CLAUDE.md)
- Temporary debugging output
- Duplicate information (cortexmem deduplicates automatically)

## When to Search

Search **before** starting work on any topic:
- "Have we solved this before?"
- "What patterns does this project use?"
- "What decisions were made about X?"

## Concepts and Facts

When saving, always include:
- `concepts`: keywords for search (e.g., `["auth", "jwt", "middleware"]`)
- `facts`: key takeaways (e.g., `["JWT tokens expire after 24h", "Refresh tokens stored in httpOnly cookies"]`)

These improve search accuracy and provide quick summaries without reading full content.
