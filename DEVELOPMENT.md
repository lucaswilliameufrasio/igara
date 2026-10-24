# Development

This document provides guidelines for developing and maintaining Igará.

## Prerequisites

- Rust 1.99.1 (pinned in `rust-toolchain.toml`, auto-installed by `rustup`)
- `ssh-keygen` (OpenSSH client tools)
- `docker` CLI (optional, needed for Docker context features)
- `ssh` client (optional, needed for remote host operations)
- `tar` on the workstation and remote Linux host (needed for remote builds)

## Setup

Install the development tooling (nextest, cargo-watch, coverage/size tools):

```bash
make setup
```

## Quick start

```bash
# Build
cargo build

# Run (no args launches TUI)
cargo run

# Run a specific command
cargo run -- host add --name my-host --hostname 10.0.0.1 --user root
```

## Testing

### Run all tests (nextest)

```bash
make test
# or
cargo nextest run
```

Watch mode (requires `cargo-watch`):
```bash
make test-watch
```

Show output:
```bash
make test-nocapture
```

### Test structure

- **Unit tests** live alongside the code in `#[cfg(test)]` modules within each source file.
- **Integration tests** live in `tests/` and test the crate's public API end-to-end.

### Testing without real SSH hosts

Set `IGARA_CONFIG_DIR` to a temp directory to isolate from your real config:

```bash
IGARA_CONFIG_DIR=/tmp/igara-dev cargo run
```

The `config` and `repo` modules are tested with in-memory SQLite databases, so they
don't touch the filesystem outside the test sandbox.

The MCP stdio integration test starts an isolated Igará process and checks its
JSON-RPC tool catalog. It does not connect to SSH hosts or start remote builds.

### Docker-dependent tests

Integration tests that require Docker will auto-skip if `docker` is not found in PATH.

### Coverage

To measure test coverage (requires `cargo-tarpaulin` or `cargo-llvm-cov`):

```bash
cargo tarpaulin --out html
# or
cargo llvm-cov --html
```

## Debugging

Enable debug logging:

```bash
RUST_LOG=debug cargo run
```

For even more detail:
```bash
RUST_LOG=trace cargo run
```

Backtrace on errors:
```bash
RUST_BACKTRACE=1 cargo run
```

## Dependency Management

To maintain a clean and efficient dependency graph, we use `cargo tree`.

### Identifying Duplicates

Multiple versions of the same crate in the dependency graph can increase binary size
and lead to compilation errors (especially with crates that link to native libraries).

To find duplicate crates:
```bash
cargo tree -d
```

### Optimizing the Tree

If you find duplicates, you can try to unify them:

1. **Check the dependency graph:** Use `cargo tree -p <package>` to see which top-level
   dependencies are pulling in different versions.
2. **Update dependencies:** Sometimes a simple `cargo update` will resolve issues if the
   version requirements overlap.
3. **Precise updates:** If a sub-dependency needs a specific version:
   ```bash
   cargo update -p <crate_name> --precise <version>
   ```
4. **Align versions in `Cargo.toml`:** Ensure that your direct dependencies are using
   compatible versions of shared sub-dependencies (e.g., `serde`, `rand`, `tokio`).

### Analyzing Size

```bash
cargo bloat --release --crates
```

## Linting

```bash
cargo clippy --all-targets --all-features -- -D warnings
cargo fmt --all -- --check
```

## CI

The project uses GitHub Actions with two workflows:

- **CI** (`ci.yml`): runs on push to `main` and on PRs – lint, test, and build release
  on ubuntu, macOS, and Windows.
- **Release** (`release.yml`): triggered on `v*` tags – cross-compiles and publishes
  binaries to GitHub Releases.

## Project structure

```
src/
  main.rs       – entry point, CLI dispatcher
  lib.rs        – crate root, re-exports all modules
  cli.rs        – clap argument definitions
  config.rs     – filesystem paths (config dir, keys, db)
  db.rs         – SQLite connection + migrations
  repo.rs       – data access layer (host CRUD)
  models.rs     – domain types (HostProfile, enums)
  ssh.rs        – SSH keygen, config management, remote commands
  docker.rs     – Docker context management
  bootstrap.rs  – remote host inspection + package install
  build.rs      – remote project source transfer and build execution
  mcp.rs        – local stdio MCP tools for host summaries and explicit builds
  services.rs   – shared infra deploy (Postgres, Valkey)
  tui.rs        – Ratatui terminal dashboard
  errors.rs     – error types
migrations/    – SQL migration files (applied in order)
tests/         – integration tests
docs/          – agent integration and configuration guides
.claude/skills – shared Claude Code/OpenCode skills
examples/hooks – optional agent hooks
```
