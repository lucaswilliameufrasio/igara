#![allow(dead_code)]

use igara::db::run_migrations;
use igara::models::{ConnectionMode, CreateHostOptions, HostProfile};
use igara::repo::HostRepository;
use rusqlite::Connection;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

pub fn setup_db() -> Connection {
    let mut conn = Connection::open_in_memory().unwrap();
    conn.execute("PRAGMA foreign_keys = ON", []).unwrap();
    run_migrations(&mut conn).unwrap();
    conn
}

pub fn setup_repo() -> Arc<HostRepository> {
    Arc::new(HostRepository::new(setup_db()))
}

pub fn make_host(name: &str) -> HostProfile {
    HostProfile::new(CreateHostOptions {
        name: name.to_string(),
        hostname: format!("{}.example.com", name),
        user_name: "root".to_string(),
        port: 22,
        connection_mode: ConnectionMode::Lan,
        proxy_jump: None,
        ssh_key_path: format!("/tmp/igara-test-keys/{}", name),
        tags: vec![],
    })
}

#[allow(dead_code)]
pub fn make_host_with_tags(name: &str, tags: Vec<String>) -> HostProfile {
    HostProfile::new(CreateHostOptions {
        name: name.to_string(),
        hostname: format!("{}.example.com", name),
        user_name: "root".to_string(),
        port: 22,
        connection_mode: ConnectionMode::Lan,
        proxy_jump: None,
        ssh_key_path: format!("/tmp/{}", name),
        tags,
    })
}

#[allow(dead_code)]
pub fn unique_id() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos() as u64
}
