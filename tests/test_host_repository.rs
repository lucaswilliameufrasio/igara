mod common;

use igara::models::ConnectionMode;

#[test]
fn test_insert_and_retrieve_host() {
    let repo = common::setup_repo();
    let host = common::make_host("test-insert");

    repo.insert_host(&host).unwrap();

    let retrieved = repo.get_host_by_name("test-insert").unwrap();
    assert_eq!(retrieved.name, "test-insert");
    assert_eq!(retrieved.hostname, "test-insert.example.com");
    assert_eq!(retrieved.user_name, "root");
    assert_eq!(retrieved.port, 22);
    assert_eq!(retrieved.connection_mode, ConnectionMode::Lan);
    assert!(!retrieved.is_default);
    assert!(retrieved.health.is_none());
}

#[test]
fn test_insert_host_with_tags() {
    let repo = common::setup_repo();
    let host = common::make_host_with_tags("tagged-host", vec!["prod".into(), "ubuntu".into()]);

    repo.insert_host(&host).unwrap();

    let retrieved = repo.get_host_by_name("tagged-host").unwrap();
    assert_eq!(
        retrieved.tags,
        vec!["prod".to_string(), "ubuntu".to_string()]
    );
}

#[test]
fn test_insert_host_with_proxy_jump() {
    let repo = common::setup_repo();
    let host = igara::models::HostProfile::new(igara::models::CreateHostOptions {
        name: "jumped".to_string(),
        hostname: "10.0.0.1".to_string(),
        user_name: "user".to_string(),
        port: 2222,
        connection_mode: ConnectionMode::Public,
        proxy_jump: Some("bastion.example.com".to_string()),
        ssh_key_path: "/tmp/key".to_string(),
        tags: vec![],
    });

    repo.insert_host(&host).unwrap();

    let retrieved = repo.get_host_by_name("jumped").unwrap();
    assert_eq!(
        retrieved.proxy_jump,
        Some("bastion.example.com".to_string())
    );
    assert_eq!(retrieved.port, 2222);
}

#[test]
fn test_list_hosts_sorted() {
    let repo = common::setup_repo();

    let host_c = common::make_host("charlie");
    let host_a = common::make_host("alpha");
    let host_b = common::make_host("bravo");

    // Insert out of order
    repo.insert_host(&host_c).unwrap();
    repo.insert_host(&host_a).unwrap();
    repo.insert_host(&host_b).unwrap();

    let hosts = repo.list_hosts().unwrap();
    assert_eq!(hosts.len(), 3);
    assert_eq!(hosts[0].name, "alpha");
    assert_eq!(hosts[1].name, "bravo");
    assert_eq!(hosts[2].name, "charlie");
}

#[test]
fn test_get_host_by_ssh_alias() {
    let repo = common::setup_repo();
    let host = common::make_host("alias-test");
    repo.insert_host(&host).unwrap();

    // get_host_by_name also matches ssh_alias
    let retrieved = repo.get_host_by_name("alias-test").unwrap();
    assert_eq!(retrieved.ssh_alias, "alias-test");
}

#[test]
fn test_get_nonexistent_host_returns_error() {
    let repo = common::setup_repo();
    let err = repo.get_host_by_name("does-not-exist").unwrap_err();
    assert!(err.to_string().contains("not found"));
}

#[test]
fn test_delete_host() {
    let repo = common::setup_repo();
    let host = common::make_host("to-delete");
    repo.insert_host(&host).unwrap();

    assert!(repo.delete_host("to-delete").unwrap());
    assert!(repo.get_host_by_name("to-delete").is_err());
}

#[test]
fn test_delete_nonexistent_host_returns_false() {
    let repo = common::setup_repo();
    assert!(!repo.delete_host("never-existed").unwrap());
}

#[test]
fn test_set_default_host() {
    let repo = common::setup_repo();
    let host_a = common::make_host("alpha");
    let host_b = common::make_host("bravo");

    repo.insert_host(&host_a).unwrap();
    repo.insert_host(&host_b).unwrap();

    repo.set_default_host("alpha").unwrap();
    let retrieved = repo.get_host_by_name("alpha").unwrap();
    assert!(retrieved.is_default);

    // Switching default should unset previous
    repo.set_default_host("bravo").unwrap();
    let alpha = repo.get_host_by_name("alpha").unwrap();
    let bravo = repo.get_host_by_name("bravo").unwrap();
    assert!(!alpha.is_default);
    assert!(bravo.is_default);
}

#[test]
fn test_duplicate_name_rejected() {
    let repo = common::setup_repo();
    let host = common::make_host("unique");
    repo.insert_host(&host).unwrap();

    let dup = common::make_host("unique");
    let err = repo.insert_host(&dup).unwrap_err();
    // SQLite UNIQUE constraint violation
    assert!(err.to_string().contains("UNIQUE") || err.to_string().contains("unique"));
}

#[test]
fn test_multiple_hosts_with_tags_listed() {
    let repo = common::setup_repo();

    let host_a = common::make_host_with_tags("server-a", vec!["web".into()]);
    let host_b = common::make_host_with_tags("server-b", vec!["db".into(), "prod".into()]);

    repo.insert_host(&host_a).unwrap();
    repo.insert_host(&host_b).unwrap();

    let hosts = repo.list_hosts().unwrap();
    let a = hosts.iter().find(|h| h.name == "server-a").unwrap();
    let b = hosts.iter().find(|h| h.name == "server-b").unwrap();

    assert_eq!(a.tags, vec!["web"]);
    assert_eq!(b.tags, vec!["db", "prod"]);
}
