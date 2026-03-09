# cortexmem — Engineering Standards

## Architecture

Single Rust binary: CLI + MCP server (stdio). SQLite (WAL) + FTS5 + sqlite-vec + fastembed ONNX.

## Rust Code Rules

### Error Handling
- Use `anyhow::Result` for application errors, `thiserror` for library errors
- No `.unwrap()` in production code — only in tests
- Propagate errors with `?` operator
- Log errors at the boundary, propagate within

### Types & Safety
- No `unsafe` without a `// SAFETY:` comment explaining the invariant
- Prefer strong types over primitive obsession (`ProjectName` over `String`)
- Use `#[must_use]` on functions returning `Result`
- Exhaustive match arms — no wildcard `_` on enums we own

### Module Structure
- `mod.rs` re-exports public API only
- One struct per file when the struct has multiple methods
- Integration tests in `tests/integration/`, unit tests in same file as code
- `pub(crate)` by default, `pub` only at crate boundary

### Database
- All SQL in `src/db/` — no SQL strings outside this module
- Parameterized queries only — never interpolate user input
- Transactions for multi-statement writes
- WAL mode always — set on connection open

### Testing
- Test behavior, not implementation
- Descriptive names: `should_return_404_when_user_not_found`
- Arrange-Act-Assert pattern
- `Database::open_in_memory()` for all DB tests — no temp files

### Performance
- No allocations in hot paths without benchmarking
- Prefer `&str` over `String` in function parameters
- Use `Vec::with_capacity` when size is known
- Batch SQLite operations in transactions

### Linting & Formatting
- **Always run before committing:** `cargo fmt && cargo clippy -- -D warnings`
- `cargo fmt --check` is enforced in CI — code that doesn't pass will fail the pipeline
- Fix all clippy warnings — they are treated as errors (`-D warnings`)

### Git
- Conventional commits: `feat:`, `fix:`, `chore:`, `refactor:`, `test:`, `docs:`
- One logical change per commit

## Key Paths

| Path | Purpose |
|------|---------|
| `src/main.rs` | CLI entrypoint + lib re-export |
| `src/db/` | SQLite schema, CRUD, FTS5 |
| `src/embed/` | fastembed model management |
| `src/search/` | Vector KNN, RRF fusion |
| `src/memory/` | Dedup, decay, compaction |
| `src/mcp/` | MCP server + 14 tool handlers |
| `src/cli/` | CLI subcommand handlers |
| `tests/integration/` | Integration tests |
