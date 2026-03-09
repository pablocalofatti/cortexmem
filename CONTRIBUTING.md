# Contributing to cortexmem

Thanks for your interest in contributing! cortexmem is a Rust project — a persistent memory engine for AI coding agents.

## Getting Started

```bash
# Clone the repo
git clone https://github.com/pablocalofatti/cortexmem.git
cd cortexmem

# Build
cargo build

# Run tests (71 integration tests)
cargo test

# Download the embedding model (needed for vector search tests)
cargo run -- model download
```

## Development Workflow

### Before You Code

1. Check existing issues or open a new one to discuss your idea
2. Fork the repo and create a feature branch: `feat/description` or `fix/description`

### While You Code

1. **Run formatting before every commit:**
   ```bash
   cargo fmt
   ```

2. **Run clippy with warnings as errors:**
   ```bash
   cargo clippy -- -D warnings
   ```

3. **Run the test suite:**
   ```bash
   cargo test
   ```

4. **All three must pass** — CI enforces `cargo fmt --check`, `cargo clippy -D warnings`, and `cargo test`.

### Commit Messages

We use [Conventional Commits](https://www.conventionalcommits.org/):

```
feat: add new MCP tool for batch operations
fix: handle null topic_key in search results
chore: update rusqlite to 0.39
refactor: extract search ranking into separate module
test: add integration tests for export/import
docs: update README with setup instructions
```

One logical change per commit.

## Code Standards

These are enforced by review — read `CLAUDE.md` for the full list. Highlights:

### Error Handling
- `anyhow::Result` for application errors, `thiserror` for library errors
- No `.unwrap()` in production code (only in tests)
- Propagate with `?`, log at boundaries

### Module Structure
- All SQL lives in `src/db/` — no SQL strings elsewhere
- Integration tests in `tests/integration/`, unit tests in same file
- `pub(crate)` by default, `pub` only at crate boundary

### Testing
- Test behavior, not implementation
- Descriptive names: `should_return_empty_when_no_observations`
- Arrange-Act-Assert pattern
- `Database::open_in_memory()` for all DB tests

### Performance
- Prefer `&str` over `String` in function parameters
- Batch SQLite operations in transactions
- `Vec::with_capacity` when size is known

## Project Structure

```
src/
├── main.rs          # CLI entrypoint
├── lib.rs           # Crate root (re-exports)
├── cli/             # CLI subcommand handlers
│   ├── mod.rs       # save, search, get, stats, compact
│   ├── export.rs    # export/import commands
│   └── setup.rs     # interactive setup wizard
├── db/              # SQLite layer
│   ├── mod.rs       # Database struct, connection, migrations
│   ├── schema.rs    # CREATE TABLE statements
│   ├── observations.rs  # Observation CRUD
│   ├── sessions.rs  # Session CRUD
│   ├── fts.rs       # FTS5 search
│   └── vector.rs    # sqlite-vec operations
├── embed/           # Embedding model
│   ├── mod.rs
│   ├── model.rs     # fastembed model management
│   └── pipeline.rs  # Text → embedding pipeline
├── mcp/             # MCP server
│   ├── mod.rs       # Server startup
│   ├── tools.rs     # 14 tool handlers + public API
│   └── protocol.rs  # Output formatting
├── memory/          # Memory management
│   ├── mod.rs       # MemoryManager
│   ├── dedup.rs     # Content hash + topic key dedup
│   ├── decay.rs     # Tier evaluation rules
│   └── compact.rs   # Compaction engine
└── search/          # Search engine
    ├── mod.rs       # HybridSearcher
    └── rrf.rs       # Reciprocal Rank Fusion
```

## Running Specific Tests

```bash
# All tests
cargo test

# A specific test file
cargo test --test export_test

# A specific test
cargo test --test mcp_write_test mem_save_should_return_id

# Tests with output
cargo test -- --nocapture
```

## Pull Requests

1. Branch from `main`
2. Make sure `cargo fmt --check && cargo clippy -- -D warnings && cargo test` all pass
3. Open a PR against `main`
4. Claude Code Review will automatically review your diff

## License

By contributing, you agree that your contributions will be licensed under the MIT License.
