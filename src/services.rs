use color_eyre::eyre::Result;
use tokio::process::Command;
use crate::models::HostProfile;

pub enum SharedService {
    Postgres,
    Valkey,
}

impl SharedService {
    pub fn name(&self) -> &str {
        match self {
            Self::Postgres => "postgres",
            Self::Valkey => "valkey",
        }
    }

    pub fn docker_command(&self) -> Vec<&str> {
        match self {
            Self::Postgres => vec![
                "run", "-d",
                "--name", "igara-postgres",
                "-e", "POSTGRES_PASSWORD=postgres",
                "-p", "5432:5432",
                "postgres:latest"
            ],
            Self::Valkey => vec![
                "run", "-d",
                "--name", "igara-valkey",
                "-p", "6379:6379",
                "valkey/valkey:latest"
            ],
        }
    }
}

pub async fn deploy_service(host: &HostProfile, service: SharedService) -> Result<()> {
    let mut args = vec![host.ssh_alias.as_str(), "docker"];
    args.extend(service.docker_command());

    let status = Command::new("ssh")
        .args(args)
        .status()
        .await?;

    if !status.success() {
        color_eyre::eyre::bail!("Failed to deploy {} on {}", service.name(), host.name);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_service_commands() {
        let pg = SharedService::Postgres;
        let pg_cmd = pg.docker_command();
        assert!(pg_cmd.contains(&"postgres:latest"));
        assert!(pg_cmd.contains(&"5432:5432"));

        let vk = SharedService::Valkey;
        let vk_cmd = vk.docker_command();
        assert!(vk_cmd.contains(&"valkey/valkey:latest"));
        assert!(vk_cmd.contains(&"6379:6379"));
    }
}
