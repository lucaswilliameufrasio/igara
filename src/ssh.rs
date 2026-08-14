use crate::config::AppPaths;
use crate::models::HostProfile;
use color_eyre::eyre::{Context, Result, bail};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use tokio::io::AsyncWriteExt;
use tokio::process::Command;

pub fn key_path(paths: &AppPaths, alias: &str) -> PathBuf {
    paths.keys_dir.join(format!("{alias}_ed25519"))
}

pub fn generate_host_key(key_path: &Path, comment: &str) -> Result<()> {
    if key_path.exists() {
        return Ok(());
    }

    let status = std::process::Command::new("ssh-keygen")
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

pub fn render_ssh_block(
    alias: &str,
    hostname: &str,
    user: &str,
    port: u16,
    key_path: &Path,
    proxy_jump: Option<&str>,
) -> String {
    let mut block = format!(
        "# >>> igara managed: {alias} >>>\nHost {alias}\n  HostName {hostname}\n  User {user}\n  Port {port}\n  IdentityFile \"{}\"\n  IdentitiesOnly yes\n  ServerAliveInterval 30\n  ServerAliveCountMax 120\n  TCPKeepAlive yes\n",
        key_path.display()
    );

    if let Some(jump) = proxy_jump {
        block.push_str(&format!("  ProxyJump {jump}\n"));
    }

    block.push_str(&format!("# <<< igara managed: {alias} <<<\n"));
    block
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

pub async fn test_ssh(alias: &str) -> Result<()> {
    let status = Command::new("ssh")
        .args([
            "-o",
            "BatchMode=yes",
            "-o",
            "ConnectTimeout=5",
            alias,
            "true",
        ])
        .status()
        .await
        .context("failed to spawn ssh")?;

    if !status.success() {
        bail!("ssh connectivity test failed for host alias '{alias}'");
    }

    Ok(())
}

pub fn ssh_connect_args(host: &HostProfile) -> Vec<String> {
    let mut args = Vec::new();
    if host.port != 22 {
        args.push("-p".to_string());
        args.push(host.port.to_string());
    }
    if let Some(jump) = &host.proxy_jump {
        args.push("-o".to_string());
        args.push(format!("ProxyJump={jump}"));
    }
    args.push(format!("{}@{}", host.user_name, host.hostname));
    args
}

pub async fn install_public_key(host: &HostProfile, pub_key_path: &Path) -> Result<()> {
    let connect_args = ssh_connect_args(host);

    if which::which("ssh-copy-id").is_ok() {
        let mut cmd = Command::new("ssh-copy-id");
        cmd.arg("-i").arg(pub_key_path);
        for arg in &connect_args {
            cmd.arg(arg);
        }
        let status = cmd.status().await.context("failed to spawn ssh-copy-id")?;
        if !status.success() {
            bail!("ssh-copy-id failed for host '{}'", host.name);
        }
        return Ok(());
    }

    let public_key = fs::read_to_string(pub_key_path)
        .with_context(|| format!("failed to read {}", pub_key_path.display()))?;

    // Securely pipe public key to remote host using $(cat) for the grep check
    let remote_cmd = "mkdir -p ~/.ssh && chmod 700 ~/.ssh && PK=\"$(cat)\"; grep -qxF \"$PK\" ~/.ssh/authorized_keys || echo \"$PK\" >> ~/.ssh/authorized_keys && chmod 600 ~/.ssh/authorized_keys";

    let mut child = Command::new("ssh")
        .args(&connect_args)
        .arg("bash")
        .arg("-c")
        .arg(remote_cmd)
        .stdin(Stdio::piped())
        .spawn()
        .context("failed to spawn ssh for public key installation")?;

    let mut stdin = child
        .stdin
        .take()
        .ok_or_else(|| color_eyre::eyre::eyre!("failed to open stdin for ssh process"))?;
    stdin.write_all(public_key.trim().as_bytes()).await?;
    drop(stdin);

    let status = child
        .wait()
        .await
        .context("failed to wait for ssh child process")?;

    if !status.success() {
        bail!("failed to install public key on host '{}'", host.name);
    }

    Ok(())
}

#[allow(dead_code)]
pub async fn push_binary(alias: &str, local_path: &Path, remote_dest: &str) -> Result<()> {
    if !local_path.exists() {
        bail!("local binary not found at {}", local_path.display());
    }

    let binary_name = local_path
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| color_eyre::eyre::eyre!("invalid binary path"))?;

    // Create a tar archive of the binary and pipe it to the remote host
    let remote_cmd =
        format!("mkdir -p $(dirname {remote_dest}) && tar -x -C $(dirname {remote_dest})");

    let mut child = Command::new("ssh")
        .arg(alias)
        .arg(remote_cmd)
        .stdin(Stdio::piped())
        .spawn()
        .context("failed to spawn ssh for binary transfer")?;

    let mut stdin = child
        .stdin
        .take()
        .ok_or_else(|| color_eyre::eyre::eyre!("failed to open stdin for ssh process"))?;

    // Run tar locally to archive the binary (async)
    let mut local_tar = Command::new("tar")
        .arg("-c")
        .arg("-C")
        .arg(local_path.parent().unwrap_or(Path::new(".")))
        .arg(binary_name)
        .stdout(Stdio::piped())
        .spawn()
        .context("failed to run local tar")?;

    if let Some(mut local_stdout) = local_tar.stdout.take() {
        tokio::io::copy(&mut local_stdout, &mut stdin).await?;
    }

    drop(stdin);
    let status = child.wait().await?;
    let _ = local_tar.wait().await?;

    if !status.success() {
        bail!("binary transfer failed for host alias '{alias}'");
    }

    // Ensure it's executable on the other side
    let chmod_status = Command::new("ssh")
        .arg(alias)
        .arg(format!("chmod +x {remote_dest}"))
        .status()
        .await?;

    if !chmod_status.success() {
        bail!("failed to set executable permissions on remote host");
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{ConnectionMode, CreateHostOptions};
    use tempfile::NamedTempFile;

    #[test]
    fn test_render_ssh_block() {
        let block = render_ssh_block("alias", "host", "user", 22, Path::new("/path/to/key"), None);
        assert!(block.contains("Host alias"));
        assert!(block.contains("HostName host"));
        assert!(block.contains("User user"));
        assert!(block.contains("Port 22"));
        assert!(block.contains("IdentityFile \"/path/to/key\""));
        assert!(!block.contains("ProxyJump"));

        let block_jump = render_ssh_block(
            "alias",
            "host",
            "user",
            22,
            Path::new("/path/to/key"),
            Some("bastion"),
        );
        assert!(block_jump.contains("ProxyJump bastion"));
    }

    fn test_host(port: u16, proxy_jump: Option<&str>) -> HostProfile {
        HostProfile::new(CreateHostOptions {
            name: "test-host".to_string(),
            hostname: "host.example.com".to_string(),
            user_name: "user".to_string(),
            port,
            connection_mode: ConnectionMode::Public,
            proxy_jump: proxy_jump.map(str::to_string),
            ssh_key_path: "/key".to_string(),
            tags: vec![],
        })
    }

    #[test]
    fn test_ssh_connect_args() {
        let host_default = test_host(22, None);
        assert_eq!(
            ssh_connect_args(&host_default),
            vec!["user@host.example.com"]
        );

        let host_custom_port = test_host(2222, None);
        assert_eq!(
            ssh_connect_args(&host_custom_port),
            vec!["-p", "2222", "user@host.example.com"]
        );

        let host_jump = test_host(22, Some("bastion"));
        assert_eq!(
            ssh_connect_args(&host_jump),
            vec!["-o", "ProxyJump=bastion", "user@host.example.com"]
        );

        let host_full = test_host(2222, Some("bastion"));
        assert_eq!(
            ssh_connect_args(&host_full),
            vec![
                "-p",
                "2222",
                "-o",
                "ProxyJump=bastion",
                "user@host.example.com"
            ]
        );
    }

    #[tokio::test]
    async fn test_upsert_and_remove_ssh_block() -> Result<()> {
        let temp_file = NamedTempFile::new()?;
        let path = temp_file.path();

        // 1. Initial Insert
        let block1 = render_ssh_block("host1", "1.1.1.1", "root", 22, Path::new("/key1"), None);
        upsert_managed_block(path, "host1", &block1)?;
        let content = fs::read_to_string(path)?;
        assert!(content.contains("Host host1"));

        // 2. Insert another host
        let block2 = render_ssh_block("host2", "2.2.2.2", "root", 22, Path::new("/key2"), None);
        upsert_managed_block(path, "host2", &block2)?;
        let content = fs::read_to_string(path)?;
        assert!(content.contains("Host host1"));
        assert!(content.contains("Host host2"));

        // 3. Update host1 (upsert)
        let block1_updated =
            render_ssh_block("host1", "1.1.1.2", "root", 22, Path::new("/key1"), None);
        upsert_managed_block(path, "host1", &block1_updated)?;
        let content = fs::read_to_string(path)?;
        assert!(content.contains("HostName 1.1.1.2"));
        assert!(content.contains("Host host2"));

        // 4. Remove host1
        remove_managed_block(path, "host1")?;
        let content = fs::read_to_string(path)?;
        assert!(!content.contains("Host host1"));
        assert!(content.contains("Host host2"));

        Ok(())
    }
}
