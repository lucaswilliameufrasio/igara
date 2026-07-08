use color_eyre::eyre::{Context, Result, bail};
use std::process::Command;

#[cfg(test)]
fn docker_in_path() -> bool {
    which::which("docker").is_ok()
}

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

    // Also setup a buildx builder for this context to offload builds
    let _ = Command::new("docker")
        .args(["buildx", "create", "--name", name, "--use", name])
        .status();

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

pub fn run_docker_command(ssh_alias: &str, args: &[&str]) -> Result<()> {
    let host = format!("ssh://{ssh_alias}");
    let mut cmd_args = vec!["-H", &host];
    cmd_args.extend(args);

    let status = Command::new("docker")
        .args(cmd_args)
        .status()
        .context(format!("failed to run docker command with args {:?}", args))?;

    if !status.success() {
        bail!("docker command failed");
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_context_exists_returns_false_for_nonexistent() {
        if !docker_in_path() {
            eprintln!("skipping: docker not found in PATH");
            return;
        }
        let result = context_exists("igara-test-nonexistent-ctx-do-not-use").unwrap();
        assert!(!result);
    }

    #[test]
    fn test_create_and_remove_context_idempotent() {
        if !docker_in_path() {
            eprintln!("skipping: docker not found in PATH");
            return;
        }
        let name = format!("igara-test-{}", uuid7::uuid7());

        assert!(
            !context_exists(&name).unwrap(),
            "context should not exist yet"
        );

        create_context(&name, "igara-test-nonexistent-host").unwrap();
        assert!(
            context_exists(&name).unwrap(),
            "context should exist after create"
        );

        // Idempotent: creating again is a no-op
        create_context(&name, "igara-test-nonexistent-host").unwrap();
        assert!(context_exists(&name).unwrap());

        remove_context(&name).unwrap();
        assert!(
            !context_exists(&name).unwrap(),
            "context should be gone after remove"
        );

        // Idempotent: removing again is a no-op
        remove_context(&name).unwrap();
        assert!(!context_exists(&name).unwrap());
    }
}
