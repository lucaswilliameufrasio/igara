use crate::models::PlatformFamily;
use color_eyre::eyre::{bail, Context, Result};
use std::process::Command;

#[derive(Debug, Clone)]
pub struct InspectResult {
    pub platform_family: PlatformFamily,
    pub os_release: String,
    pub has_docker: bool,
    pub has_tailscale: bool,
    pub has_buildx: bool,
    pub uname_m: Option<String>,
}

pub fn inspect(alias: &str) -> Result<InspectResult> {
    let os_release = run_ssh(alias, "cat /etc/os-release")?;
    let platform_family = detect_platform_family(&os_release);
    let has_docker = run_ssh_status(alias, "command -v docker >/dev/null 2>&1")?;
    let has_tailscale = run_ssh_status(alias, "command -v tailscale >/dev/null 2>&1")?;
    let has_buildx = run_ssh_status(alias, "docker buildx version >/dev/null 2>&1")?;
    let uname_m = Some(run_ssh(alias, "uname -m")?.trim().to_string());

    Ok(InspectResult {
        platform_family,
        os_release,
        has_docker,
        has_tailscale,
        has_buildx,
        uname_m,
    })
}

pub fn apply(alias: &str, family: PlatformFamily) -> Result<()> {
    let commands = match family {
        PlatformFamily::Arch => vec![
            "sudo pacman -Sy --noconfirm docker tailscale",
            "sudo systemctl enable --now docker",
            "sudo systemctl enable --now tailscaled",
            r#"sudo mkdir -p /etc/docker && printf '{\n  \"features\": {\n    \"buildkit\": true\n  }\n}\n' | sudo tee /etc/docker/daemon.json >/dev/null"#,
            "sudo systemctl restart docker",
        ],
        PlatformFamily::Debian => vec![
            "sudo apt-get update",
            "sudo apt-get install -y docker.io tailscale",
            "sudo systemctl enable --now docker",
            "sudo systemctl enable --now tailscaled",
            r#"sudo mkdir -p /etc/docker && printf '{\n  \"features\": {\n    \"buildkit\": true\n  }\n}\n' | sudo tee /etc/docker/daemon.json >/dev/null"#,
            "sudo systemctl restart docker",
        ],
        PlatformFamily::Fedora => vec![
            "sudo dnf install -y docker tailscale",
            "sudo systemctl enable --now docker",
            "sudo systemctl enable --now tailscaled",
            r#"sudo mkdir -p /etc/docker && printf '{\n  \"features\": {\n    \"buildkit\": true\n  }\n}\n' | sudo tee /etc/docker/daemon.json >/dev/null"#,
            "sudo systemctl restart docker",
        ],
        PlatformFamily::Unknown => bail!("unsupported remote platform family"),
    };

    for command in commands {
        let status = Command::new("ssh")
            .arg(alias)
            .arg(command)
            .status()
            .with_context(|| format!("failed to run remote command: {command}"))?;
        if !status.success() {
            bail!("remote bootstrap command failed: {command}");
        }
    }

    Ok(())
}

fn run_ssh(alias: &str, command: &str) -> Result<String> {
    let output = Command::new("ssh")
        .arg(alias)
        .arg(command)
        .output()
        .with_context(|| format!("failed to run remote command: {command}"))?;

    if !output.status.success() {
        bail!("remote command failed: {command}");
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

fn run_ssh_status(alias: &str, command: &str) -> Result<bool> {
    let status = Command::new("ssh")
        .arg(alias)
        .arg(command)
        .status()
        .with_context(|| format!("failed to run remote status command: {command}"))?;
    Ok(status.success())
}

fn detect_platform_family(os_release: &str) -> PlatformFamily {
    let normalized = os_release.to_ascii_lowercase();
    if normalized.contains("cachyos") || normalized.contains("arch") {
        PlatformFamily::Arch
    } else if normalized.contains("ubuntu") || normalized.contains("debian") {
        PlatformFamily::Debian
    } else if normalized.contains("fedora") {
        PlatformFamily::Fedora
    } else {
        PlatformFamily::Unknown
    }
}
