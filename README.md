# Igará

> **Igará** (Tupi): canoe or vessel used to travel between places.  
> Conceptually: a lightweight vehicle that carries you across distance.  
> **A vessel between your workstation and your remote machines.**

Igará is a local-first CLI/TUI for registering remote Linux hosts, generating one SSH key per host, managing `~/.ssh/config` blocks, creating Docker contexts over SSH, and bootstrapping remote machines.

## Highlights

- one SSH keypair per host
- local SQLite metadata store
- native Docker contexts over SSH
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
