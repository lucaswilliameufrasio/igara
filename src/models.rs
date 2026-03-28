use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};
use std::str::FromStr;

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
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "tailscale" => Ok(Self::Tailscale),
            "lan" => Ok(Self::Lan),
            "public" => Ok(Self::Public),
            other => Err(format!("invalid connection mode: {other}")),
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
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "arch" => Ok(Self::Arch),
            "debian" => Ok(Self::Debian),
            "fedora" => Ok(Self::Fedora),
            "unknown" => Ok(Self::Unknown),
            other => Err(format!("invalid platform family: {other}")),
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
    pub ssh_alias: String,
    pub ssh_key_path: String,
    pub docker_context_name: String,
    pub is_default: bool,
    pub tags: Vec<String>,
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
            ssh_key_path,
            is_default: false,
            tags,
            created_at: now,
            updated_at: now,
        }
    }
}

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BootstrapRun {
    pub id: String,
    pub host_id: HostId,
    pub action_name: String,
    pub status: BootstrapStatus,
    pub output: String,
    pub created_at: DateTime<Utc>,
}
