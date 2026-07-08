mod common;

#[test]
fn test_all_migrations_apply_cleanly() {
    // If setup_db doesn't panic, all migrations applied
    let _conn = common::setup_db();
}

#[test]
fn test_migration_creates_all_tables() {
    let conn = common::setup_db();

    let tables: Vec<String> = conn
        .prepare("SELECT name FROM sqlite_master WHERE type='table' ORDER BY name")
        .unwrap()
        .query_map([], |row| row.get(0))
        .unwrap()
        .collect::<Result<Vec<String>, _>>()
        .unwrap();

    assert!(tables.contains(&"hosts".to_string()));
    assert!(tables.contains(&"host_tags".to_string()));
    assert!(tables.contains(&"host_capabilities".to_string()));
    assert!(tables.contains(&"bootstrap_runs".to_string()));
    assert!(tables.contains(&"app_settings".to_string()));
    assert!(tables.contains(&"host_health".to_string()));
}

#[test]
fn test_migration_creates_index() {
    let conn = common::setup_db();

    let indexes: Vec<String> = conn
        .prepare(
            "SELECT name FROM sqlite_master WHERE type='index' AND name IS NOT NULL ORDER BY name",
        )
        .unwrap()
        .query_map([], |row| row.get(0))
        .unwrap()
        .collect::<Result<Vec<String>, _>>()
        .unwrap();

    assert!(indexes.contains(&"host_tags_tag_idx".to_string()));
}

#[test]
fn test_migration_is_idempotent() {
    let mut conn = common::setup_db();
    igara::db::run_migrations(&mut conn).unwrap();
    igara::db::run_migrations(&mut conn).unwrap();
}

#[test]
fn test_app_settings_singleton_exists() {
    let conn = common::setup_db();
    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM app_settings", [], |row| row.get(0))
        .unwrap();
    assert_eq!(count, 1, "app_settings should have exactly one row");
}
