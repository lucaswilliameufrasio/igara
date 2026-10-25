# Changelog

All notable changes to this project will be documented in this file.

## [0.1.0] - 2026-10-24

### Bug Fixes

- Resolve clippy warnings and formatting issues
- *(ssh)* Install key via raw user@hostname and drop redundant buildx create

### CI / Build

- Upgrade Rust toolchain and dependencies
- *(release)* Add cargo-dist release workflow

### Chores

- Add make check target to validate format, lint, and tests
- Use rust 1.99.0

### Features

- Add initial code
- Modernize tooling and implement parallel fleet management
- *(ssh)* Implement push_binary function for remote binary transfer
- Add Makefile and refactor HostProfile::new to use CreateHostOptions struct
- *(build)* Add remote project builds
- *(mcp)* Add safe agent integration

### Refactor

- Replace let _/expect with propagated errors and add nextest tooling

### Styling

- Apply cargo fmt across the project

### Testing

- Add integration test suite, lib.rs crate root, and improve dev setup
