use color_eyre::eyre::Result;
use rusqlite::Connection;
use rusqlite_migration::{M, Migrations};
use std::path::Path;

pub fn open(path: &Path) -> Result<Connection> {
    let mut conn = Connection::open(path)?;
    conn.execute("PRAGMA foreign_keys = ON", [])?;
    run_migrations(&mut conn)?;
    Ok(conn)
}

pub fn run_migrations(conn: &mut Connection) -> Result<()> {
    let migrations = Migrations::new(vec![
        M::up(include_str!("../migrations/0001_init.sql")),
        M::up(include_str!("../migrations/0002_capabilities.sql")),
        M::up(include_str!("../migrations/0003_bootstrap_runs.sql")),
        M::up(include_str!("../migrations/0004_settings.sql")),
        M::up(include_str!("../migrations/0005_indices.sql")),
        M::up(include_str!("../migrations/0006_proxy_jump.sql")),
        M::up(include_str!("../migrations/0007_health.sql")),
    ]);
    migrations.to_latest(conn)?;
    Ok(())
}
