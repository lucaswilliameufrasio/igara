use color_eyre::eyre::{Result, bail};
use std::collections::HashSet;
use std::path::PathBuf;
use std::time::Duration;
use tokio_util::sync::CancellationToken;

/// Budgets apply per phase; artifact bytes are bounded across all selected files.
#[derive(Clone, Debug)]
pub struct BuildOptions {
    pub artifacts: Vec<String>,
    pub output: PathBuf,
    pub connect_timeout: Duration,
    pub transfer_timeout: Duration,
    pub execution_timeout: Duration,
    pub max_artifact_bytes: u64,
    pub cancellation: CancellationToken,
    /// Optional dedicated OpenSSH config (useful for isolated clients/tests).
    pub ssh_config: Option<PathBuf>,
}

impl Default for BuildOptions {
    fn default() -> Self {
        Self {
            artifacts: Vec::new(),
            output: PathBuf::from("dist"),
            connect_timeout: Duration::from_secs(10),
            transfer_timeout: Duration::from_secs(300),
            execution_timeout: Duration::from_secs(3600),
            max_artifact_bytes: 512 * 1024 * 1024,
            cancellation: CancellationToken::new(),
            ssh_config: None,
        }
    }
}

impl BuildOptions {
    pub fn validate(&self) -> Result<()> {
        for budget in [
            self.connect_timeout,
            self.transfer_timeout,
            self.execution_timeout,
        ] {
            if budget.is_zero() || budget > Duration::from_secs(7 * 24 * 3600) {
                bail!("timeouts must be positive and no greater than seven days");
            }
        }
        if self.max_artifact_bytes == 0 || self.max_artifact_bytes == u64::MAX {
            bail!("artifact byte limit must be positive and smaller than u64::MAX");
        }
        if self.artifacts.len() > 128 {
            bail!("at most 128 artifact files may be selected");
        }
        if !self.artifacts.is_empty() && self.output.as_os_str().is_empty() {
            bail!("artifact output directory must not be empty");
        }
        let mut names = HashSet::new();
        for artifact in &self.artifacts {
            if artifact.contains('\\')
                || artifact.chars().any(char::is_control)
                || artifact
                    .split('/')
                    .any(|part| part.is_empty() || part == "." || part == "..")
            {
                bail!("artifact must be a workspace-relative file without traversal: {artifact:?}");
            }
            let name = artifact.rsplit('/').next().unwrap_or_default();
            let stem = name
                .split('.')
                .next()
                .unwrap_or_default()
                .to_ascii_uppercase();
            let reserved = [
                "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7",
                "COM8", "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8",
                "LPT9",
            ];
            // Basenames are portable across all client platforms.
            if reserved.contains(&stem.as_str())
                || name.contains([':', '*', '?', '"', '<', '>', '|'])
                || name.ends_with(['.', ' '])
                || !names.insert(name.to_lowercase())
            {
                bail!("artifact basenames must be unique and portable: {name:?}");
            }
        }
        Ok(())
    }
}
