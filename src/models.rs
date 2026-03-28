use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};
use std::str::FromStr;

use crate::errors::IgaraError;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HostId(pub String);

impl HostId {
    pub fn new() -> Self {
        Self(uuid7::uuid7().to_string())
    }
}

impl Display for HostId {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConnectionMode {
    Tailscale,
    Lan,
    Public,
}

impl Display for ConnectionMode {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Tailscale => write!(f, "tailscale"),
            Self::Lan => write!(f, "lan"),
            Self::Public => write!(f, "public"),
        }
    }
}

impl FromStr for ConnectionMode {
    type Err = IgaraError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "tailscale" => Ok(Self::Tailscale),
            "lan" => Ok(Self::Lan),
            "public" => Ok(Self::Public),
            other => Err(IgaraError::InvalidConnectionMode(other.to_string())),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PlatformFamily {
    Arch,
    Debian,
    Fedora,
    Unknown,
}

impl Display for PlatformFamily {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Arch => write!(f, "arch"),
            Self::Debian => write!(f, "debian"),
            Self::Fedora => write!(f, "fedora"),
            Self::Unknown => write!(f, "unknown"),
        }
    }
}

impl FromStr for PlatformFamily {
    type Err = IgaraError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "arch" => Ok(Self::Arch),
            "debian" => Ok(Self::Debian),
            "fedora" => Ok(Self::Fedora),
            "unknown" => Ok(Self::Unknown),
            other => Err(IgaraError::InvalidPlatformFamily(other.to_string())),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HostProfile {
    pub id: HostId,
    pub name: String,
    pub hostname: String,
    pub user_name: String,
    pub port: u16,
    pub connection_mode: ConnectionMode,
    pub platform_family: Option<PlatformFamily>,
    pub proxy_jump: Option<String>,
    pub ssh_alias: String,
    pub ssh_key_path: String,
    pub docker_context_name: String,
    pub is_default: bool,
    pub tags: Vec<String>,
    pub health: Option<HostHealth>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl HostProfile {
    pub fn new(
        name: String,
        hostname: String,
        user_name: String,
        port: u16,
        connection_mode: ConnectionMode,
        proxy_jump: Option<String>,
        ssh_key_path: String,
        tags: Vec<String>,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: HostId::new(),
            ssh_alias: name.clone(),
            docker_context_name: name.clone(),
            name,
            hostname,
            user_name,
            port,
            connection_mode,
            platform_family: None,
            proxy_jump,
            ssh_key_path,
            is_default: false,
            tags,
            health: None,
            created_at: now,
            updated_at: now,
        }
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HostCapabilities {
    pub host_id: HostId,
    pub os_name: Option<String>,
    pub kernel_version: Option<String>,
    pub arch: Option<String>,
    pub has_docker: bool,
    pub has_buildx: bool,
    pub has_buildkit_enabled: bool,
    pub has_tailscale: bool,
    pub has_k3s: bool,
    pub has_nvidia: bool,
    pub docker_version: Option<String>,
    pub tailscale_ip: Option<String>,
    pub last_checked_at: Option<DateTime<Utc>>,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BootstrapStatus {
    Pending,
    Running,
    Success,
    Failed,
}

impl Display for BootstrapStatus {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pending => write!(f, "pending"),
            Self::Running => write!(f, "running"),
            Self::Success => write!(f, "success"),
            Self::Failed => write!(f, "failed"),
        }
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BootstrapRun {
    pub id: String,
    pub host_id: HostId,
    pub action_name: String,
    pub status: BootstrapStatus,
    pub output: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HostHealth {
    pub host_id: HostId,
    pub is_up: bool,
    pub last_checked_at: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_connection_mode_from_str() {
        assert_eq!(
            "tailscale".parse::<ConnectionMode>().unwrap(),
            ConnectionMode::Tailscale
        );
        assert_eq!(
            "lan".parse::<ConnectionMode>().unwrap(),
            ConnectionMode::Lan
        );
        assert_eq!(
            "public".parse::<ConnectionMode>().unwrap(),
            ConnectionMode::Public
        );
        assert!("invalid".parse::<ConnectionMode>().is_err());
    }

    #[test]
    fn test_platform_family_from_str() {
        assert_eq!(
            "arch".parse::<PlatformFamily>().unwrap(),
            PlatformFamily::Arch
        );
        assert_eq!(
            "debian".parse::<PlatformFamily>().unwrap(),
            PlatformFamily::Debian
        );
        assert_eq!(
            "fedora".parse::<PlatformFamily>().unwrap(),
            PlatformFamily::Fedora
        );
        assert_eq!(
            "unknown".parse::<PlatformFamily>().unwrap(),
            PlatformFamily::Unknown
        );
        assert!("invalid".parse::<PlatformFamily>().is_err());
    }

    #[test]
    fn test_host_id_new() {
        let id1 = HostId::new();
        let id2 = HostId::new();
        assert_ne!(id1.0, id2.0);
        assert_eq!(id1.0.len(), 36); // UUID v7 string length
    }

    #[test]
    fn test_host_profile_with_extras() {
        let host = HostProfile::new(
            "bastion-host".to_string(),
            "1.1.1.1".to_string(),
            "root".to_string(),
            22,
            ConnectionMode::Public,
            Some("proxy.example.com".to_string()),
            "/key".to_string(),
            vec!["prod".to_string()],
        );

        assert_eq!(host.proxy_jump, Some("proxy.example.com".to_string()));
        assert!(host.health.is_none());
        assert_eq!(host.tags, vec!["prod".to_string()]);
    }
}
