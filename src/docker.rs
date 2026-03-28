use color_eyre::eyre::{bail, Context, Result};
use std::process::Command;

pub fn context_exists(name: &str) -> Result<bool> {
    let output = Command::new("docker")
        .args(["context", "inspect", name])
        .output()
        .context("failed to spawn docker context inspect")?;
    Ok(output.status.success())
}

pub fn create_context(name: &str, ssh_alias: &str) -> Result<()> {
    if context_exists(name)? {
        return Ok(());
    }

    let host = format!("host=ssh://{ssh_alias}");
    let status = Command::new("docker")
        .args(["context", "create", name, "--docker", &host])
        .status()
        .context("failed to spawn docker context create")?;

    if !status.success() {
        bail!("docker context create failed for '{name}'");
    }

    Ok(())
}

pub fn remove_context(name: &str) -> Result<()> {
    if !context_exists(name)? {
        return Ok(());
    }

    let status = Command::new("docker")
        .args(["context", "rm", "-f", name])
        .status()
        .context("failed to spawn docker context rm")?;

    if !status.success() {
        bail!("docker context rm failed for '{name}'");
    }

    Ok(())
}

pub fn use_context(name: &str) -> Result<()> {
    let status = Command::new("docker")
        .args(["context", "use", name])
        .status()
        .context("failed to spawn docker context use")?;

    if !status.success() {
        bail!("docker context use failed for '{name}'");
    }

    Ok(())
}

pub fn docker_info(context_name: &str) -> Result<String> {
    let output = Command::new("docker")
        .args(["--context", context_name, "info", "--format", "{{json .}}"])
        .output()
        .context("failed to spawn docker info")?;

    if !output.status.success() {
        bail!("docker info failed for context '{context_name}'");
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

pub fn docker_ps(context_name: &str) -> Result<String> {
    let output = Command::new("docker")
        .args(["--context", context_name, "ps", "--format", "json"])
        .output()
        .context("failed to spawn docker ps")?;

    if !output.status.success() {
        bail!("docker ps failed for context '{context_name}'");
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}
