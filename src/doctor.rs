use crate::build::{BuildOptions, ssh_command};
use crate::models::HostProfile;
use color_eyre::eyre::{Result, eyre};
use serde::{Deserialize, Serialize};
use std::process::Stdio;
use std::time::Duration;
use tokio::io::AsyncReadExt;

#[derive(Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CheckStatus {
    Pass,
    Warning,
    Fail,
}

#[derive(Debug, Serialize)]
pub struct Check {
    pub name: String,
    pub status: CheckStatus,
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct DoctorReport {
    pub checks: Vec<Check>,
}

impl DoctorReport {
    pub fn has_failures(&self) -> bool {
        self.checks
            .iter()
            .any(|check| check.status == CheckStatus::Fail)
    }

    fn add(&mut self, name: &str, status: CheckStatus, message: &str) {
        self.checks.push(Check {
            name: name.into(),
            status,
            message: message.into(),
        });
    }
}

#[derive(Deserialize)]
struct RemoteFacts {
    tar: bool,
    docker: bool,
    supervision: bool,
    free_bytes: u64,
}

/// Read-only checks. No database initialization, permission repair, or implicit fleet scan.
pub async fn diagnose(host: Option<&HostProfile>, options: &BuildOptions) -> Result<DoctorReport> {
    options.validate()?;
    let mut report = DoctorReport { checks: Vec::new() };
    for (tool, required) in [
        ("ssh", true),
        ("tar", true),
        ("ssh-keygen", true),
        ("docker", false),
    ] {
        let present = which::which(tool).is_ok();
        report.add(
            tool,
            if present {
                CheckStatus::Pass
            } else if required {
                CheckStatus::Fail
            } else {
                CheckStatus::Warning
            },
            if present {
                "available"
            } else {
                "not found on PATH"
            },
        );
    }
    if let Some(host) = host {
        check_key(&mut report, std::path::Path::new(&host.ssh_key_path));
        let script = "python3 -c 'import json,shutil,os; print(json.dumps(dict(tar=shutil.which(\"tar\") is not None,docker=shutil.which(\"docker\") is not None,supervision=hasattr(os,\"WNOWAIT\") and hasattr(os,\"killpg\"),free_bytes=shutil.disk_usage(os.path.expanduser(\"~\")).free)))'";
        let mut command = ssh_command(&host.ssh_alias, options, script);
        command
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null());
        let facts = async {
            let mut child = crate::owned_process::spawn_owned(&mut command)?;
            let stdout = child
                .take_stdout()
                .ok_or_else(|| eyre!("missing diagnostic output"))?;
            let mut bytes = Vec::new();
            stdout.take(16385).read_to_end(&mut bytes).await?;
            if bytes.len() > 16384 || !child.wait().await?.success() {
                return Err(eyre!("SSH failed or diagnostic response exceeded limit"));
            }
            Ok::<RemoteFacts, color_eyre::Report>(serde_json::from_slice(&bytes)?)
        };
        match tokio::time::timeout(options.connect_timeout + Duration::from_secs(5), facts).await {
            Ok(Ok(facts)) => {
                report.add("remote.ssh", CheckStatus::Pass, "authenticated; Python 3 available");
                report.add("remote.supervision", if facts.supervision { CheckStatus::Pass } else { CheckStatus::Fail },
                    "Linux process-group supervision support");
                report.add("remote.tar", if facts.tar { CheckStatus::Pass } else { CheckStatus::Fail },
                    if facts.tar { "available" } else { "install tar on the selected host" });
                report.add("remote.docker", if facts.docker { CheckStatus::Pass } else { CheckStatus::Warning },
                    if facts.docker { "CLI available (daemon not probed)" } else { "optional Docker CLI absent" });
                report.add("remote.disk", if facts.free_bytes >= 1024 * 1024 * 1024 { CheckStatus::Pass } else {
                    CheckStatus::Warning
                }, &format!("{} bytes available in home filesystem (not a reservation)", facts.free_bytes));
            }
            _ => report.add("remote.ssh", CheckStatus::Fail,
                "bounded SSH/Python probe failed; check known_hosts, authentication, connectivity and Python 3"),
        }
    }
    Ok(report)
}

fn check_key(report: &mut DoctorReport, path: &std::path::Path) {
    let Ok(metadata) = path.symlink_metadata() else {
        report.add(
            "ssh.key",
            CheckStatus::Fail,
            "configured private key is missing or unreadable",
        );
        return;
    };
    if !metadata.is_file() || metadata.file_type().is_symlink() {
        report.add(
            "ssh.key",
            CheckStatus::Fail,
            "private key must be a regular file, not a symlink",
        );
        return;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if metadata.permissions().mode() & 0o077 != 0 {
            report.add(
                "ssh.key",
                CheckStatus::Fail,
                "private key is accessible to group/others; restrict permissions",
            );
            return;
        }
        report.add(
            "ssh.key",
            CheckStatus::Pass,
            "regular private key; restrictive Unix permissions",
        );
    }
    #[cfg(not(unix))]
    report.add(
        "ssh.key",
        CheckStatus::Warning,
        "key exists; inspect Windows ACLs manually",
    );
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use std::os::unix::fs::{PermissionsExt, symlink};

    #[test]
    fn key_checks_reject_unsafe_permissions_and_links_without_repair() {
        let root = tempfile::tempdir().unwrap();
        let key = root.path().join("synthetic-key");
        std::fs::write(&key, "synthetic fixture, not a credential").unwrap();
        std::fs::set_permissions(&key, std::fs::Permissions::from_mode(0o644)).unwrap();
        let mut report = DoctorReport { checks: Vec::new() };
        check_key(&mut report, &key);
        assert!(report.has_failures());
        assert_eq!(
            std::fs::metadata(&key).unwrap().permissions().mode() & 0o777,
            0o644
        );
        std::fs::set_permissions(&key, std::fs::Permissions::from_mode(0o600)).unwrap();
        let mut report = DoctorReport { checks: Vec::new() };
        check_key(&mut report, &key);
        assert!(!report.has_failures());
        let link = root.path().join("link");
        symlink(&key, &link).unwrap();
        check_key(&mut report, &link);
        assert!(report.has_failures());
    }
}
