mod common;

use igara::models::HostId;

#[test]
fn test_insert_and_retrieve_health() {
    let repo = common::setup_repo();
    let host = common::make_host("health-check");
    repo.insert_host(&host).unwrap();

    repo.update_health(&host.id, true).unwrap();

    let retrieved = repo.get_host_by_name("health-check").unwrap();
    let health = retrieved.health.expect("health should be present");
    assert!(health.is_up);
    assert_eq!(health.host_id.to_string(), host.id.to_string());
}

#[test]
fn test_health_upsert_updates_existing() {
    let repo = common::setup_repo();
    let host = common::make_host("health-upsert");
    repo.insert_host(&host).unwrap();

    repo.update_health(&host.id, false).unwrap();
    let retrieved = repo.get_host_by_name("health-upsert").unwrap();
    assert!(!retrieved.health.unwrap().is_up);

    // Upsert: update to UP
    repo.update_health(&host.id, true).unwrap();
    let retrieved = repo.get_host_by_name("health-upsert").unwrap();
    assert!(retrieved.health.unwrap().is_up);
}

#[test]
fn test_health_reflects_in_list() {
    let repo = common::setup_repo();
    let up_host = common::make_host("up-host");
    let down_host = common::make_host("down-host");

    repo.insert_host(&up_host).unwrap();
    repo.insert_host(&down_host).unwrap();

    repo.update_health(&up_host.id, true).unwrap();
    repo.update_health(&down_host.id, false).unwrap();

    let hosts = repo.list_hosts().unwrap();
    let up = hosts.iter().find(|h| h.name == "up-host").unwrap();
    let down = hosts.iter().find(|h| h.name == "down-host").unwrap();

    assert!(up.health.as_ref().unwrap().is_up);
    assert!(!down.health.as_ref().unwrap().is_up);
}

#[test]
fn test_health_for_nonexistent_host_id() {
    let repo = common::setup_repo();
    // update_health doesn't validate that host_id exists in hosts table
    // This should not panic or error (ON CONFLICT handles it, but FK might fail)
    let fake_id = HostId::new();
    let result = repo.update_health(&fake_id, true);
    // The FK constraint should reject this since the host doesn't exist
    assert!(
        result.is_err(),
        "should reject health for non-existent host due to FK"
    );
}
