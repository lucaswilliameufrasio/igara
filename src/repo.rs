use crate::errors::IgaraError;
use crate::models::{HostHealth, HostId, HostProfile, PlatformFamily};
use chrono::{DateTime, Utc};
use color_eyre::eyre::Result;
use parking_lot::Mutex;
use rusqlite::{params, Connection, OptionalExtension};

pub struct HostRepository {
    conn: Mutex<Connection>,
}

impl HostRepository {
    pub fn new(conn: Connection) -> Self {
        Self {
            conn: Mutex::new(conn),
        }
    }

    pub fn insert_host(&self, host: &HostProfile) -> Result<()> {
        let conn = self.conn.lock();
        let tx = conn.unchecked_transaction()?;

        tx.execute(
            r#"
            INSERT INTO hosts (
              id, name, hostname, user_name, port, connection_mode,
              platform_family, proxy_jump, ssh_alias, ssh_key_path, docker_context_name,
              is_default, created_at, updated_at
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)
            "#,
            params![
                host.id.to_string(),
                host.name,
                host.hostname,
                host.user_name,
                host.port,
                host.connection_mode.to_string(),
                host.platform_family.map(|p| p.to_string()),
                host.proxy_jump,
                host.ssh_alias,
                host.ssh_key_path,
                host.docker_context_name,
                if host.is_default { 1 } else { 0 },
                host.created_at.to_rfc3339(),
                host.updated_at.to_rfc3339(),
            ],
        )?;

        for tag in &host.tags {
            tx.execute(
                "INSERT INTO host_tags (host_id, tag) VALUES (?1, ?2)",
                params![host.id.to_string(), tag],
            )?;
        }

        tx.commit()?;
        Ok(())
    }

    pub fn list_hosts(&self) -> Result<Vec<HostProfile>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            r#"
            SELECT id, name, hostname, user_name, port, connection_mode,
                   platform_family, proxy_jump, ssh_alias, ssh_key_path, docker_context_name,
                   is_default, created_at, updated_at
            FROM hosts
            ORDER BY name ASC
            "#,
        )?;

        let rows = stmt.query_map([], |row| {
            let id: String = row.get(0)?;
            let connection_mode: String = row.get(5)?;
            let platform_family: Option<String> = row.get(6)?;
            let proxy_jump: Option<String> = row.get(7)?;
            let created_at: String = row.get(12)?;
            let updated_at: String = row.get(13)?;

            Ok(HostProfile {
                id: HostId(id),
                name: row.get(1)?,
                hostname: row.get(2)?,
                user_name: row.get(3)?,
                port: row.get(4)?,
                connection_mode: connection_mode.parse().expect("valid connection mode"),
                platform_family: platform_family.map(|value| {
                    value
                        .parse::<PlatformFamily>()
                        .expect("valid platform family")
                }),
                proxy_jump,
                ssh_alias: row.get(8)?,
                ssh_key_path: row.get(9)?,
                docker_context_name: row.get(10)?,
                is_default: row.get::<_, i64>(11)? == 1,
                tags: Vec::new(),
                health: None,
                created_at: DateTime::parse_from_rfc3339(&created_at)
                    .expect("valid created_at")
                    .with_timezone(&Utc),
                updated_at: DateTime::parse_from_rfc3339(&updated_at)
                    .expect("valid updated_at")
                    .with_timezone(&Utc),
            })
        })?;

        let mut hosts = Vec::new();
        for row in rows {
            let mut host = row?;
            host.tags = self.tags_for_host_with_conn(&conn, &host.id)?;
            host.health = self.get_health_for_host(&conn, &host.id)?;
            hosts.push(host);
        }

        Ok(hosts)
    }

    pub fn get_host_by_name(&self, name: &str) -> Result<HostProfile> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            r#"
            SELECT id, name, hostname, user_name, port, connection_mode,
                   platform_family, proxy_jump, ssh_alias, ssh_key_path, docker_context_name,
                   is_default, created_at, updated_at
            FROM hosts
            WHERE name = ?1 OR ssh_alias = ?1
            LIMIT 1
            "#,
        )?;

        let row = stmt
            .query_row([name], |row| {
                let id: String = row.get(0)?;
                let connection_mode: String = row.get(5)?;
                let platform_family: Option<String> = row.get(6)?;
                let proxy_jump: Option<String> = row.get(7)?;
                let created_at: String = row.get(12)?;
                let updated_at: String = row.get(13)?;

                Ok(HostProfile {
                    id: HostId(id),
                    name: row.get(1)?,
                    hostname: row.get(2)?,
                    user_name: row.get(3)?,
                    port: row.get(4)?,
                    connection_mode: connection_mode.parse().expect("valid connection mode"),
                    platform_family: platform_family.map(|v| v.parse().expect("valid family")),
                    proxy_jump,
                    ssh_alias: row.get(8)?,
                    ssh_key_path: row.get(9)?,
                    docker_context_name: row.get(10)?,
                    is_default: row.get::<_, i64>(11)? == 1,
                    tags: Vec::new(),
                    health: None,
                    created_at: DateTime::parse_from_rfc3339(&created_at)
                        .expect("valid created_at")
                        .with_timezone(&Utc),
                    updated_at: DateTime::parse_from_rfc3339(&updated_at)
                        .expect("valid updated_at")
                        .with_timezone(&Utc),
                })
            })
            .optional()?;

        let mut host = row.ok_or_else(|| IgaraError::HostNotFound(name.to_string()))?;
        host.tags = self.tags_for_host_with_conn(&conn, &host.id)?;
        host.health = self.get_health_for_host(&conn, &host.id)?;
        Ok(host)
    }

    pub fn delete_host(&self, name: &str) -> Result<bool> {
        let conn = self.conn.lock();
        let changed = conn.execute(
            "DELETE FROM hosts WHERE name = ?1 OR ssh_alias = ?1",
            [name],
        )?;
        Ok(changed > 0)
    }

    pub fn set_default_host(&self, name: &str) -> Result<()> {
        let host = self.get_host_by_name(name)?;
        let conn = self.conn.lock();
        conn.execute("UPDATE hosts SET is_default = 0", [])?;
        conn.execute(
            "UPDATE hosts SET is_default = 1 WHERE id = ?1",
            [host.id.to_string()],
        )?;
        conn.execute(
            r#"
            INSERT INTO app_settings (id, default_host_id, last_used_host_id, first_run_completed)
            VALUES (1, ?1, NULL, 1)
            ON CONFLICT(id) DO UPDATE SET default_host_id = excluded.default_host_id
            "#,
            [host.id.to_string()],
        )?;
        Ok(())
    }

    pub fn update_health(&self, host_id: &HostId, is_up: bool) -> Result<()> {
        let conn = self.conn.lock();
        conn.execute(
            r#"
            INSERT INTO host_health (host_id, is_up, last_checked_at)
            VALUES (?1, ?2, ?3)
            ON CONFLICT(host_id) DO UPDATE SET
                is_up = excluded.is_up,
                last_checked_at = excluded.last_checked_at
            "#,
            params![host_id.to_string(), if is_up { 1 } else { 0 }, Utc::now().to_rfc3339()],
        )?;
        Ok(())
    }

    fn get_health_for_host(&self, conn: &Connection, host_id: &HostId) -> Result<Option<HostHealth>> {
        let mut stmt = conn.prepare("SELECT is_up, last_checked_at FROM host_health WHERE host_id = ?1")?;
        let row = stmt.query_row([host_id.to_string()], |row| {
            let last_checked_at: String = row.get(1)?;
            Ok(HostHealth {
                host_id: host_id.clone(),
                is_up: row.get::<_, i64>(0)? == 1,
                last_checked_at: DateTime::parse_from_rfc3339(&last_checked_at)
                    .expect("valid last_checked_at")
                    .with_timezone(&Utc),
            })
        }).optional()?;
        Ok(row)
    }

    fn tags_for_host_with_conn(&self, conn: &Connection, host_id: &HostId) -> Result<Vec<String>> {
        let mut stmt = conn.prepare("SELECT tag FROM host_tags WHERE host_id = ?1 ORDER BY tag ASC")?;
        let rows = stmt.query_map([host_id.to_string()], |row| row.get::<_, String>(0))?;
        let mut tags = Vec::new();
        for row in rows {
            tags.push(row?);
        }
        Ok(tags)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::run_migrations;
    use crate::models::{ConnectionMode, HostProfile};

    fn setup_db() -> Connection {
        let mut conn = Connection::open_in_memory().unwrap();
        conn.execute("PRAGMA foreign_keys = ON", []).unwrap();
        run_migrations(&mut conn).unwrap();
        conn
    }

    #[test]
    fn test_insert_and_get_host() -> Result<()> {
        let conn = setup_db();
        let repo = HostRepository::new(conn);

        let host = HostProfile::new(
            "test-host".to_string(),
            "1.2.3.4".to_string(),
            "user".to_string(),
            22,
            ConnectionMode::Lan,
            None,
            "/tmp/key".to_string(),
            vec!["tag1".to_string(), "tag2".to_string()],
        );

        repo.insert_host(&host)?;

        let retrieved = repo.get_host_by_name("test-host")?;
        assert_eq!(retrieved.name, "test-host");
        assert_eq!(retrieved.hostname, "1.2.3.4");
        assert_eq!(retrieved.tags, vec!["tag1".to_string(), "tag2".to_string()]);

        Ok(())
    }

    #[test]
    fn test_list_hosts() -> Result<()> {
        let conn = setup_db();
        let repo = HostRepository::new(conn);

        let host1 = HostProfile::new(
            "host1".to_string(),
            "1.1.1.1".to_string(),
            "user".to_string(),
            22,
            ConnectionMode::Lan,
            None,
            "/tmp/key1".to_string(),
            vec![],
        );
        let host2 = HostProfile::new(
            "host2".to_string(),
            "2.2.2.2".to_string(),
            "user".to_string(),
            22,
            ConnectionMode::Lan,
            None,
            "/tmp/key2".to_string(),
            vec![],
        );

        repo.insert_host(&host1)?;
        repo.insert_host(&host2)?;

        let hosts = repo.list_hosts()?;
        assert_eq!(hosts.len(), 2);
        assert_eq!(hosts[0].name, "host1");
        assert_eq!(hosts[1].name, "host2");

        Ok(())
    }

    #[test]
    fn test_delete_host() -> Result<()> {
        let conn = setup_db();
        let repo = HostRepository::new(conn);

        let host = HostProfile::new(
            "to-delete".to_string(),
            "1.1.1.1".to_string(),
            "user".to_string(),
            22,
            ConnectionMode::Lan,
            None,
            "/tmp/key".to_string(),
            vec![],
        );

        repo.insert_host(&host)?;
        assert!(repo.delete_host("to-delete")?);
        assert!(repo.get_host_by_name("to-delete").is_err());

        Ok(())
    }
}
