use igara::docker;

fn docker_available() -> bool {
    which::which("docker").is_ok()
}

#[test]
fn test_context_exists_for_nonexistent() {
    if !docker_available() {
        eprintln!("skipping: docker not found in PATH");
        return;
    }
    assert!(!docker::context_exists("igara-test-nonexistent-ctx").unwrap());
}

#[test]
fn test_create_and_remove_context_full_cycle() {
    if !docker_available() {
        eprintln!("skipping: docker not found in PATH");
        return;
    }

    let name = format!("igara-test-{}", uuid7::uuid7());

    // Create
    docker::create_context(&name, "igara-test-nonexistent-host").unwrap();
    assert!(docker::context_exists(&name).unwrap());

    // Idempotent create
    docker::create_context(&name, "igara-test-nonexistent-host").unwrap();
    assert!(docker::context_exists(&name).unwrap());

    // Remove
    docker::remove_context(&name).unwrap();
    assert!(!docker::context_exists(&name).unwrap());

    // Idempotent remove
    docker::remove_context(&name).unwrap();
    assert!(!docker::context_exists(&name).unwrap());
}

#[test]
fn test_remove_nonexistent_context_is_noop() {
    if !docker_available() {
        eprintln!("skipping: docker not found in PATH");
        return;
    }
    docker::remove_context("igara-test-ctx-that-never-existed").unwrap();
}
