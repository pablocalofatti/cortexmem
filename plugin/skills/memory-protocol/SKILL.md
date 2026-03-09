# Memory Protocol

You have access to **cortexmem**, a persistent memory system. Use it to store and retrieve observations across sessions.

## When to Save (mem_save)

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
- Duplicate information (cortexmem handles dedup automatically)

## When to Search (mem_search)

Search **before** starting work on any topic:
- "Have we solved this before?"
- "What patterns does this project use?"
- "What decisions were made about X?"

## Progressive Disclosure

Follow this retrieval pattern:
1. `mem_search` — get compact results (titles, types, scores)
2. `mem_get` — retrieve full detail for relevant observations
3. `mem_timeline` — see chronological context around an observation

## Topic Keys

Use `topic_key` for evolving knowledge that should be updated in place:
- `architecture/auth` — authentication design decisions
- `patterns/error-handling` — error handling conventions
- `setup/database` — database configuration

When saving with a `topic_key`, cortexmem automatically updates the existing observation instead of creating a duplicate.

Use `mem_suggest_topic` to find existing topic keys before creating new ones.

## Session Lifecycle

1. **Start**: Call `mem_session_start` at the beginning of each session
2. **Work**: Save observations as you go
3. **Before compaction**: Call `mem_session_summary` with a brief recap
4. **End**: Call `mem_session_end` when the session closes

## Concepts and Facts

When saving, include:
- `concepts`: keywords for search (e.g., `["auth", "jwt", "middleware"]`)
- `facts`: key takeaways (e.g., `["JWT tokens expire after 24h", "Refresh tokens stored in httpOnly cookies"]`)

These improve search accuracy and provide quick summaries without reading full content.
