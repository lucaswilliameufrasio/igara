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
- GitHub Actions workflows for CI and cargo-dist binary releases
- shell and PowerShell release installers

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
igara build beefy --artifact target/release/app --output ./dist -- cargo build --release
```

The command after `--` runs in a temporary remote workspace as the configured SSH
user. By default, Igará removes that workspace after successful execution and
artifact retrieval; pass
`--keep` to retain source files and build artifacts under
`~/.cache/igara/builds/` on the host. Repeat `--exclude PATTERN` to omit additional
paths from the transferred source. Igará also excludes common build outputs and
credential files (including `.git`, `target`, `node_modules`, `.env*`, and private
key files). Exclusions are tar patterns, not a full `.gitignore` parser; review
the source and add `--exclude` patterns for project-specific credentials.

Remote Linux hosts need `tar` and Python 3. SSH is noninteractive and requires a
previously verified host key in `known_hosts` (Igará never disables host-key
checking). Use `--ssh-config PATH` for an explicit dedicated OpenSSH config.

Repeat `--artifact` to retrieve workspace-relative **regular files**, without
globs, directory downloads, symlinks or hard links. Files are saved by basename
under `--output` (default `dist`); duplicate basenames and existing destinations
are rejected. Downloads have a combined 512 MiB limit (`--max-artifact-bytes`).
A failed download retains the remote workspace; multiple-file publication can
partially succeed, and the error reports already published files.

Budgets are in seconds: `--connect-timeout` (10), `--transfer-timeout` (300 per
source/artifact phase), and `--execution-timeout` (3600). Ctrl+C requests bounded
cancellation and cleanup. The remote supervisor stops the command's process
group, including ordinary descendants. Network loss, forced client termination,
or commands that deliberately daemonize can make termination/cleanup uncertain;
errors report retained or potentially remaining workspaces. No automatic retries
repeat a build. See [the design and limitations](docs/remote-build-design.md).

Igará uses the existing SSH configuration and does not copy the local SSH key or
environment variables to the host. The build command itself runs with the SSH
user's permissions; only run trusted project code on trusted hosts. Private package
registries or other services may still require their own credentials.

## Diagnose prerequisites

```bash
igara doctor --json                  # local tools only; no DB initialization
igara doctor --host beefy            # explicitly inspect this registered host
```

Doctor checks local SSH/tar/key-generation tools, optional Docker, the selected
host's key permissions (Unix; Windows ACLs need manual inspection), bounded SSH
authentication, Python/process supervision, remote tar/Docker availability and
home filesystem free space. It does not install tools, fix permissions, create
configuration or scan other hosts. Docker presence does not prove daemon health;
free space is a snapshot, not a reservation. Failed checks return a nonzero exit
status; optional missing tools/low disk space are warnings.

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

The install script delegates to the shell installer attached to the latest
GitHub Release. Set `IGARA_VERSION` to install a specific release.

```bash
curl -fsSL https://raw.githubusercontent.com/lucaswilliameufrasio/igara/main/scripts/install.sh | sh
```

```bash
curl -fsSL https://raw.githubusercontent.com/lucaswilliameufrasio/igara/main/scripts/install.sh | IGARA_VERSION=0.1.0 sh
```

or

```bash
wget -qO- https://raw.githubusercontent.com/lucaswilliameufrasio/igara/main/scripts/install.sh | sh
```

Windows users can run `igara-installer.ps1` from the GitHub Release assets.

## Contributing

For instructions on setting up a development environment and managing dependencies, see [DEVELOPMENT.md](DEVELOPMENT.md).
