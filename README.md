# Igará

> **Igará** (Tupi): canoe or vessel used to travel between places.  
> Conceptually: a lightweight vehicle that carries you across distance.  
> **A vessel between your workstation and your remote machines.**

Igará is a local-first CLI/TUI for registering remote Linux hosts, generating one SSH key per host, managing `~/.ssh/config` blocks, creating Docker contexts over SSH, running project builds remotely, and bootstrapping remote machines.

## Highlights

- one SSH keypair per host
- local SQLite metadata store
- native Docker contexts over SSH
- remote builds for non-Docker projects over SSH
- remote inspection and bootstrap for Arch/CachyOS, Debian/Ubuntu, and Fedora
- simple Ratatui dashboard for host overview
- GitHub Actions workflows for CI and binary releases
- install script for `curl | sh` or `wget | sh`

## Quick start

```bash
cargo run -- host add --name beefy --hostname 100.x.y.z --user lucas --mode tailscale --install-key
cargo run -- docker create-context beefy
cargo run -- docker use beefy
cargo run -- bootstrap inspect beefy
```

## Remote builds

Run any build command on a registered host, syncing the current project directory
over the existing SSH connection:

```bash
igara build beefy --source ./my-project -- cargo build --release
igara build beefy --source ./my-project --keep -- npm run build
```

The command after `--` runs in a temporary remote workspace as the configured SSH
user. By default, Igará removes that workspace when the command exits; pass
`--keep` to retain source files and build artifacts under
`~/.cache/igara/builds/` on the host. Repeat `--exclude PATTERN` to omit additional
paths from the transferred source. Igará also excludes common build outputs and
credential files (including `.git`, `target`, `node_modules`, `.env*`, and private
key files). Exclusions are tar patterns, not a full `.gitignore` parser; review
the source and add `--exclude` patterns for project-specific credentials.

Igará uses the existing SSH configuration and does not copy the local SSH key or
environment variables to the host. The build command itself runs with the SSH
user's permissions; only run trusted project code on trusted hosts. Private package
registries or other services may still require their own credentials.

## Agent integration

Igará includes an optional local MCP server for Claude Code and OpenCode. It exposes
only `list_hosts` (name, connection mode, and default status) and `remote_build`.
The latter transfers the selected source directory and runs the supplied command as
the configured SSH user. It is never started automatically by Igará or by the
example hooks.

Start the stdio server with `igara mcp serve` (or `cargo run -- mcp serve` during
development). Configuration examples, permission guidance, hook examples, and the
shared agent skill are documented in
[docs/agent-integration.md](docs/agent-integration.md). The server keeps stdout
reserved for MCP messages; remote build output is forwarded to stderr.

## Local paths

- config dir: `~/.config/igara`
- database: `~/.config/igara/db/igara.sqlite3`
- keys: `~/.config/igara/keys`
- ssh config: `~/.ssh/config`

## Install from GitHub release

```bash
curl -fsSL https://raw.githubusercontent.com/lucaswilliameufrasio/igara/main/scripts/install.sh | sh
```

or

```bash
wget -qO- https://raw.githubusercontent.com/lucaswilliameufrasio/igara/main/scripts/install.sh | sh
```

## Contributing

For instructions on setting up a development environment and managing dependencies, see [DEVELOPMENT.md](DEVELOPMENT.md).
