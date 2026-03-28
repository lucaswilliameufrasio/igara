# Development

This document provides guidelines for developing and maintaining Igará.

## Dependency Management

To maintain a clean and efficient dependency graph, we use `cargo tree`.

### Identifying Duplicates

Multiple versions of the same crate in the dependency graph can increase binary size and lead to compilation errors (especially with crates that link to native libraries).

To find duplicate crates, run:

```bash
cargo tree -d
```

This will list all crates that appear with more than one version in the graph.

### Optimizing the Tree

If you find duplicates, you can try to unify them:

1.  **Check the dependency graph:** Use `cargo tree -p <package>` to see which top-level dependencies are pulling in different versions.
2.  **Update dependencies:** Sometimes a simple `cargo update` will resolve issues if the version requirements overlap.
3.  **Precise updates:** If a sub-dependency needs a specific version, you can try:
    ```bash
    cargo update -p <crate_name> --precise <version>
    ```
4.  **Align versions in `Cargo.toml`:** Ensure that your direct dependencies are using compatible versions of shared sub-dependencies (e.g., `serde`, `rand`, `tokio`).

### Analyzing Size

For a more visual analysis of dependency size, you can use `cargo bloat`:

```bash
cargo bloat --release --crates
```

## Testing

Always run tests before submitting changes:

```bash
cargo test
```

We aim for high coverage of both unit tests (in the same file) and integration tests (for database and external interactions).
