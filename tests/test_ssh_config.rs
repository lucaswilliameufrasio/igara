use igara::ssh;
use std::fs;
use std::path::Path;
use tempfile::NamedTempFile;

fn render_block(alias: &str, hostname: &str) -> String {
    ssh::render_ssh_block(alias, hostname, "root", 22, Path::new("/tmp/key"), None)
}

#[test]
fn test_initial_insert_on_empty_file() {
    let tmp = NamedTempFile::new().unwrap();
    let block = render_block("host1", "1.1.1.1");

    ssh::upsert_managed_block(tmp.path(), "host1", &block).unwrap();

    let content = fs::read_to_string(tmp.path()).unwrap();
    assert!(content.contains("Host host1"));
    assert!(content.contains("HostName 1.1.1.1"));
    assert!(content.contains("# >>> igara managed: host1 >>>"));
    assert!(content.contains("# <<< igara managed: host1 <<<"));
}

#[test]
fn test_multiple_hosts_in_config() {
    let tmp = NamedTempFile::new().unwrap();

    ssh::upsert_managed_block(tmp.path(), "alpha", &render_block("alpha", "10.0.0.1")).unwrap();
    ssh::upsert_managed_block(tmp.path(), "bravo", &render_block("bravo", "10.0.0.2")).unwrap();
    ssh::upsert_managed_block(tmp.path(), "charlie", &render_block("charlie", "10.0.0.3")).unwrap();

    let content = fs::read_to_string(tmp.path()).unwrap();
    assert!(content.contains("Host alpha"));
    assert!(content.contains("Host bravo"));
    assert!(content.contains("Host charlie"));
}

#[test]
fn test_update_existing_block() {
    let tmp = NamedTempFile::new().unwrap();

    ssh::upsert_managed_block(tmp.path(), "host1", &render_block("host1", "1.1.1.1")).unwrap();
    // Update hostname
    ssh::upsert_managed_block(tmp.path(), "host1", &render_block("host1", "9.9.9.9")).unwrap();

    let content = fs::read_to_string(tmp.path()).unwrap();
    assert!(content.contains("HostName 9.9.9.9"));
    assert!(!content.contains("HostName 1.1.1.1"));
}

#[test]
fn test_remove_one_host_keeps_others() {
    let tmp = NamedTempFile::new().unwrap();

    ssh::upsert_managed_block(tmp.path(), "alpha", &render_block("alpha", "10.0.0.1")).unwrap();
    ssh::upsert_managed_block(tmp.path(), "bravo", &render_block("bravo", "10.0.0.2")).unwrap();

    ssh::remove_managed_block(tmp.path(), "alpha").unwrap();

    let content = fs::read_to_string(tmp.path()).unwrap();
    assert!(!content.contains("Host alpha"), "alpha should be removed");
    assert!(content.contains("Host bravo"), "bravo should remain");
}

#[test]
fn test_remove_nonexistent_block_is_noop() {
    let tmp = NamedTempFile::new().unwrap();
    fs::write(tmp.path(), "Unmanaged content\n").unwrap();

    // Removing a block that doesn't exist should not change the file
    ssh::remove_managed_block(tmp.path(), "ghost").unwrap();
    let content = fs::read_to_string(tmp.path()).unwrap();
    assert_eq!(content, "Unmanaged content\n");
}

#[test]
fn test_remove_on_nonexistent_file_is_noop() {
    let path = Path::new("/tmp/igara-test-nonexistent-ssh-config");
    // Clean up if it exists
    let _ = fs::remove_file(path);
    // Should not panic or error
    ssh::remove_managed_block(path, "any").unwrap();
}

#[test]
fn test_unmanaged_content_preserved() {
    let tmp = NamedTempFile::new().unwrap();
    fs::write(
        tmp.path(),
        "# User-created config\nHost my-personal\n  HostName 1.2.3.4\n",
    )
    .unwrap();

    ssh::upsert_managed_block(tmp.path(), "managed1", &render_block("managed1", "5.5.5.5"))
        .unwrap();

    let content = fs::read_to_string(tmp.path()).unwrap();
    assert!(content.contains("my-personal"));
    assert!(content.contains("managed1"));
}

#[test]
fn test_interleaved_unmanaged_content_preserved() {
    let tmp = NamedTempFile::new().unwrap();

    // Insert first managed block
    ssh::upsert_managed_block(tmp.path(), "alpha", &render_block("alpha", "10.0.0.1")).unwrap();
    // Insert unmanaged content
    let content_after_alpha = fs::read_to_string(tmp.path()).unwrap();
    let with_unmanaged = content_after_alpha.trim().to_string()
        + "\n\n# Unmanaged in middle\nHost middle\n  HostName 9.9.9.9\n\n";
    fs::write(tmp.path(), &with_unmanaged).unwrap();
    // Insert second managed block
    ssh::upsert_managed_block(tmp.path(), "omega", &render_block("omega", "10.0.0.2")).unwrap();

    let content = fs::read_to_string(tmp.path()).unwrap();
    assert!(content.contains("Host alpha"));
    assert!(content.contains("Unmanaged in middle"));
    assert!(content.contains("Host omega"));

    // Remove first managed block, unmanaged + omega should remain
    ssh::remove_managed_block(tmp.path(), "alpha").unwrap();
    let content = fs::read_to_string(tmp.path()).unwrap();
    assert!(!content.contains("Host alpha"));
    assert!(content.contains("Unmanaged in middle"));
    assert!(content.contains("Host omega"));
}
