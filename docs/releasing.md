# Releasing

Igará uses `git-cliff` to prepare changelogs and `cargo-dist` to build and publish
cross-platform GitHub releases. The release-preparation workflow opens a pull
request; publishing only starts later, when a version tag is pushed.

## Version policy

- Use stable Semantic Versioning (`MAJOR.MINOR.PATCH`) and tags named `vX.Y.Z`.
- Keep the version in `Cargo.toml` and the tag identical.
- The first release may use the package's existing version when no release tag
  exists. Igará is currently `0.1.0`, so the initial release can be `v0.1.0`
  without changing the version ahead of time.
- Later releases must use a version newer than the current package version.

## Prepare a release

1. Ensure `main` is green and contains the changes to release.
2. In GitHub, open **Actions → Prepare Release → Run workflow** from `main`.
3. Enter the release version. For the first release, enter `0.1.0`.
4. Review and merge the generated pull request. It contains the changelog,
   `Cargo.toml`, and synchronized `Cargo.lock` changes.
5. After merging, pull the latest `main` and push the matching tag:

   ```bash
   git checkout main
   git pull --ff-only origin main
   git tag v0.1.0
   git push origin v0.1.0
   ```

The tag triggers the generated `Release` workflow. `cargo-dist` builds archives
and checksums for Linux x86_64/aarch64 (musl), macOS x86_64/aarch64, and Windows
x86_64, then creates the GitHub Release and shell/PowerShell installers.

## One-time GitHub setting

The `Prepare Release` workflow uses the repository's `GITHUB_TOKEN` to push its
branch and open a pull request. In **Settings → Actions → General → Workflow
permissions**, allow read/write access and enable **Allow GitHub Actions to
create and approve pull requests**. Release preparation is restricted to runs
dispatched from `main`; normal pull-request review is the approval gate.

## Local preflight

Install the configured `cargo-dist` version and validate the plan before a
release if desired:

```bash
cargo install cargo-dist --locked --version 0.33.1
cargo dist plan --tag v0.1.0
cargo dist build --tag v0.1.0 --artifacts=local --target x86_64-unknown-linux-musl
```

The build command above requires the Linux musl target and linker tools to be
available locally. These commands plan and build locally; do not run
`cargo dist host` as part of a preflight because it uploads artifacts and
creates/publishes a GitHub Release.

## Install a release

The generated shell installer is available at:

```bash
curl --proto '=https' --tlsv1.2 -LsSf \
  https://github.com/lucaswilliameufrasio/igara/releases/latest/download/igara-installer.sh | sh
```

For PowerShell, use the `igara-installer.ps1` asset attached to the release.
