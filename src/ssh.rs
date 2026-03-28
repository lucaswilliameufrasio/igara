use crate::config::AppPaths;
use color_eyre::eyre::{bail, Context, Result};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

pub fn key_path(paths: &AppPaths, alias: &str) -> PathBuf {
    paths.keys_dir.join(format!("{alias}_ed25519"))
}

pub fn generate_host_key(key_path: &Path, comment: &str) -> Result<()> {
    if key_path.exists() {
        return Ok(());
    }

    let status = Command::new("ssh-keygen")
        .args(["-t", "ed25519", "-C", comment, "-f"])
        .arg(key_path)
        .args(["-N", ""])
        .status()
        .context("failed to spawn ssh-keygen")?;

    if !status.success() {
        bail!("ssh-keygen failed with status {status}");
    }

    Ok(())
}

pub fn render_ssh_block(alias: &str, hostname: &str, user: &str, port: u16, key_path: &Path) -> String {
    format!(
        "# >>> igara managed: {alias} >>>\nHost {alias}\n  HostName {hostname}\n  User {user}\n  Port {port}\n  IdentityFile {}\n  IdentitiesOnly yes\n  ServerAliveInterval 30\n  ServerAliveCountMax 120\n  TCPKeepAlive yes\n# <<< igara managed: {alias} <<<\n",
        key_path.display()
    )
}

pub fn upsert_managed_block(ssh_config_path: &Path, alias: &str, block: &str) -> Result<()> {
    let existing = if ssh_config_path.exists() {
        fs::read_to_string(ssh_config_path)?
    } else {
        String::new()
    };

    let start = format!("# >>> igara managed: {alias} >>>");
    let end = format!("# <<< igara managed: {alias} <<<");

    let updated = if let (Some(s), Some(e)) = (existing.find(&start), existing.find(&end)) {
        let end_idx = e + end.len();
        let mut new_content = String::new();
        new_content.push_str(&existing[..s]);
        if !new_content.ends_with('\n') && !new_content.is_empty() {
            new_content.push('\n');
        }
        new_content.push_str(block);
        if end_idx < existing.len() {
            if !existing[end_idx..].starts_with('\n') {
                new_content.push('\n');
            }
            new_content.push_str(&existing[end_idx..]);
        }
        new_content
    } else {
        let mut new_content = existing;
        if !new_content.is_empty() && !new_content.ends_with('\n') {
            new_content.push('\n');
        }
        new_content.push('\n');
        new_content.push_str(block);
        new_content
    };

    fs::write(ssh_config_path, updated)?;
    Ok(())
}

pub fn remove_managed_block(ssh_config_path: &Path, alias: &str) -> Result<()> {
    if !ssh_config_path.exists() {
        return Ok(());
    }

    let existing = fs::read_to_string(ssh_config_path)?;
    let start = format!("# >>> igara managed: {alias} >>>");
    let end = format!("# <<< igara managed: {alias} <<<");
    if let (Some(s), Some(e)) = (existing.find(&start), existing.find(&end)) {
        let end_idx = e + end.len();
        let mut new_content = String::new();
        new_content.push_str(&existing[..s]);
        if end_idx < existing.len() {
            let tail = existing[end_idx..].trim_start_matches('\n');
            if !tail.is_empty() && !new_content.ends_with('\n') && !new_content.is_empty() {
                new_content.push('\n');
            }
            new_content.push_str(tail);
        }
        fs::write(ssh_config_path, new_content)?;
    }
    Ok(())
}

pub fn test_ssh(alias: &str) -> Result<()> {
    let status = Command::new("ssh")
        .args(["-o", "BatchMode=yes", alias, "true"])
        .status()
        .context("failed to spawn ssh")?;

    if !status.success() {
        bail!("ssh connectivity test failed for host alias '{alias}'");
    }

    Ok(())
}

pub fn install_public_key(alias: &str, pub_key_path: &Path) -> Result<()> {
    if which::which("ssh-copy-id").is_ok() {
        let status = Command::new("ssh-copy-id")
            .arg("-i")
            .arg(pub_key_path)
            .arg(alias)
            .status()
            .context("failed to spawn ssh-copy-id")?;
        if !status.success() {
            bail!("ssh-copy-id failed for host alias '{alias}'");
        }
        return Ok(());
    }

    let public_key = fs::read_to_string(pub_key_path)
        .with_context(|| format!("failed to read {}", pub_key_path.display()))?;

    let remote_cmd = format!(
        "mkdir -p ~/.ssh && chmod 700 ~/.ssh && grep -qxF '{}' ~/.ssh/authorized_keys || echo '{}' >> ~/.ssh/authorized_keys && chmod 600 ~/.ssh/authorized_keys",
        public_key.trim(),
        public_key.trim()
    );

    let status = Command::new("ssh")
        .arg(alias)
        .arg(remote_cmd)
        .status()
        .context("failed to append public key over ssh")?;

    if !status.success() {
        bail!("failed to install public key on host alias '{alias}'");
    }

    Ok(())
}
