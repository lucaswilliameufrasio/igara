use crate::errors::IgaraError;
use crate::models::{ConnectionMode, HostId, HostProfile, PlatformFamily};
use chrono::{DateTime, Utc};
use color_eyre::eyre::Result;
use rusqlite::{params, Connection, OptionalExtension};

pub struct HostRepository<'a> {
    conn: &'a Connection,
}

impl<'a> HostRepository<'a> {
    pub fn new(conn: &'a Connection) -> Self {
        Self { conn }
    }

    pub fn insert_host(&self, host: &HostProfile) -> Result<()> {
        self.conn.execute(
            r#"
            INSERT INTO hosts (
              id, name, hostname, user_name, port, connection_mode,
              platform_family, ssh_alias, ssh_key_path, docker_context_name,
              is_default, created_at, updated_at
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)
            "#,
            params![
                host.id.to_string(),
                host.name,
                host.hostname,
                host.user_name,
                host.port,
                host.connection_mode.to_string(),
                host.platform_family.map(|p| p.to_string()),
                host.ssh_alias,
                host.ssh_key_path,
                host.docker_context_name,
                if host.is_default { 1 } else { 0 },
                host.created_at.to_rfc3339(),
                host.updated_at.to_rfc3339(),
            ],
        )?;

        for tag in &host.tags {
            self.conn.execute(
                "INSERT INTO host_tags (host_id, tag) VALUES (?1, ?2)",
                params![host.id.to_string(), tag],
            )?;
        }

        Ok(())
    }

    pub fn list_hosts(&self) -> Result<Vec<HostProfile>> {
        let mut stmt = self.conn.prepare(
            r#"
            SELECT id, name, hostname, user_name, port, connection_mode,
                   platform_family, ssh_alias, ssh_key_path, docker_context_name,
                   is_default, created_at, updated_at
            FROM hosts
            ORDER BY name ASC
            "#,
        )?;

        let rows = stmt.query_map([], |row| {
            let id: String = row.get(0)?;
            let connection_mode: String = row.get(5)?;
            let platform_family: Option<String> = row.get(6)?;
            let created_at: String = row.get(11)?;
            let updated_at: String = row.get(12)?;

            Ok(HostProfile {
                id: HostId(id),
                name: row.get(1)?,
                hostname: row.get(2)?,
                user_name: row.get(3)?,
                port: row.get(4)?,
                connection_mode: connection_mode.parse().expect("valid connection mode"),
                platform_family: platform_family
                    .map(|value| value.parse::<PlatformFamily>().expect("valid platform family")),
                ssh_alias: row.get(7)?,
                ssh_key_path: row.get(8)?,
                docker_context_name: row.get(9)?,
                is_default: row.get::<_, i64>(10)? == 1,
                tags: Vec::new(),
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
            host.tags = self.tags_for_host(&host.id)?;
            hosts.push(host);
        }

        Ok(hosts)
    }

    pub fn get_host_by_name(&self, name: &str) -> Result<HostProfile> {
        let mut stmt = self.conn.prepare(
            r#"
            SELECT id, name, hostname, user_name, port, connection_mode,
                   platform_family, ssh_alias, ssh_key_path, docker_context_name,
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
                let created_at: String = row.get(11)?;
                let updated_at: String = row.get(12)?;

                Ok(HostProfile {
                    id: HostId(id),
                    name: row.get(1)?,
                    hostname: row.get(2)?,
                    user_name: row.get(3)?,
                    port: row.get(4)?,
                    connection_mode: connection_mode.parse().expect("valid connection mode"),
                    platform_family: platform_family.map(|v| v.parse().expect("valid family")),
                    ssh_alias: row.get(7)?,
                    ssh_key_path: row.get(8)?,
                    docker_context_name: row.get(9)?,
                    is_default: row.get::<_, i64>(10)? == 1,
                    tags: Vec::new(),
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
        host.tags = self.tags_for_host(&host.id)?;
        Ok(host)
    }

    pub fn delete_host(&self, name: &str) -> Result<bool> {
        let changed = self.conn.execute("DELETE FROM hosts WHERE name = ?1 OR ssh_alias = ?1", [name])?;
        Ok(changed > 0)
    }

    pub fn set_default_host(&self, name: &str) -> Result<()> {
        let host = self.get_host_by_name(name)?;
        self.conn.execute("UPDATE hosts SET is_default = 0", [])?;
        self.conn.execute(
            "UPDATE hosts SET is_default = 1 WHERE id = ?1",
            [host.id.to_string()],
        )?;
        self.conn.execute(
            r#"
            INSERT INTO app_settings (id, default_host_id, last_used_host_id, first_run_completed)
            VALUES (1, ?1, NULL, 1)
            ON CONFLICT(id) DO UPDATE SET default_host_id = excluded.default_host_id
            "#,
            [host.id.to_string()],
        )?;
        Ok(())
    }

    fn tags_for_host(&self, host_id: &HostId) -> Result<Vec<String>> {
        let mut stmt = self.conn.prepare("SELECT tag FROM host_tags WHERE host_id = ?1 ORDER BY tag ASC")?;
        let rows = stmt.query_map([host_id.to_string()], |row| row.get::<_, String>(0))?;
        let mut tags = Vec::new();
        for row in rows {
            tags.push(row?);
        }
        Ok(tags)
    }
}
