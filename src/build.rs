use color_eyre::eyre::{Context, Result, bail};
use std::path::Path;
use std::process::Stdio;
use tokio::io;
use tokio::process::{Child, Command};

const DEFAULT_EXCLUDES: &[&str] = &[
    ".git",
    ".hg",
    ".svn",
    ".aws",
    ".ssh",
    ".kube",
    ".terraform",
    "target",
    "node_modules",
    "dist",
    ".next",
    ".turbo",
    ".venv",
    "venv",
    "__pycache__",
    ".cache",
    ".env",
    ".env.*",
    ".npmrc",
    ".pypirc",
    ".netrc",
    ".git-credentials",
    ".docker/config.json",
    "id_rsa",
    "id_ed25519",
    "id_ecdsa",
    "id_dsa",
    "*.pem",
    "*.key",
    "*.p12",
    "*.pfx",
    "*.jks",
];

#[derive(Clone, Copy)]
pub(crate) enum BuildOutput {
    Inherit,
    Stderr,
}

impl BuildOutput {
    fn transfer_stdout(self) -> Stdio {
        match self {
            Self::Inherit => Stdio::inherit(),
            Self::Stderr => Stdio::null(),
        }
    }
}

pub async fn build_project(
    ssh_alias: &str,
    source: &Path,
    command: &[String],
    excludes: &[String],
    keep: bool,
) -> Result<Option<String>> {
    build_project_with_output(
        ssh_alias,
        source,
        command,
        excludes,
        keep,
        BuildOutput::Inherit,
    )
    .await
}

pub(crate) async fn build_project_with_output(
    ssh_alias: &str,
    source: &Path,
    command: &[String],
    excludes: &[String],
    keep: bool,
    output: BuildOutput,
) -> Result<Option<String>> {
    if command.is_empty() {
        bail!("a build command is required after '--'");
    }

    if !source.is_dir() {
        bail!("build source is not a directory: {}", source.display());
    }

    let source = source
        .canonicalize()
        .with_context(|| format!("failed to resolve build source {}", source.display()))?;
    let build_id = uuid7::uuid7().to_string();
    let remote_path = format!("~/.cache/igara/builds/{build_id}");

    if let Err(error) = transfer_source(ssh_alias, &source, &build_id, excludes, output).await {
        let _ = cleanup_remote_workspace(ssh_alias, &build_id, output).await;
        return Err(error);
    }

    let remote_command = render_remote_build_command(&build_id, command, keep);
    let mut remote_build = Command::new("ssh");
    remote_build.arg(ssh_alias).arg(remote_command);
    let status = run_command(remote_build, output)
        .await
        .context("failed to start remote build over SSH");

    let status = match status {
        Ok(status) => status,
        Err(error) => {
            if !keep {
                let _ = cleanup_remote_workspace(ssh_alias, &build_id, output).await;
            }
            return Err(error);
        }
    };

    if !status.success() {
        if keep {
            bail!("remote build failed on host '{ssh_alias}'; workspace retained at {remote_path}");
        }
        bail!("remote build failed on host '{ssh_alias}'");
    }

    if keep {
        Ok(Some(remote_path))
    } else {
        Ok(None)
    }
}

async fn transfer_source(
    ssh_alias: &str,
    source: &Path,
    build_id: &str,
    excludes: &[String],
    output: BuildOutput,
) -> Result<()> {
    let remote_script = format!(
        "set -eu\n\
         build_root=\"$HOME/.cache/igara/builds\"\n\
         workdir=\"$build_root/{build_id}\"\n\
         mkdir -p \"$build_root\"\n\
         mkdir \"$workdir\"\n\
         tar -xf - -C \"$workdir\""
    );

    let mut ssh = Command::new("ssh")
        .arg(ssh_alias)
        .arg(remote_sh_command(&remote_script))
        .stdout(output.transfer_stdout())
        .stdin(Stdio::piped())
        .spawn()
        .context("failed to start SSH source transfer")?;

    let Some(mut ssh_stdin) = ssh.stdin.take() else {
        let _ = ssh.kill().await;
        let _ = ssh.wait().await;
        bail!("failed to open SSH stdin for source transfer");
    };

    let mut tar = match spawn_tar(source, excludes) {
        Ok(child) => child,
        Err(error) => {
            drop(ssh_stdin);
            let _ = ssh.kill().await;
            let _ = ssh.wait().await;
            return Err(error);
        }
    };

    let Some(mut tar_stdout) = tar.stdout.take() else {
        drop(ssh_stdin);
        let _ = tar.kill().await;
        let _ = ssh.kill().await;
        let _ = tar.wait().await;
        let _ = ssh.wait().await;
        bail!("failed to open local tar stdout");
    };

    let transfer_result = io::copy(&mut tar_stdout, &mut ssh_stdin)
        .await
        .context("failed to stream the source archive to the remote host");
    drop(ssh_stdin);

    if let Err(error) = transfer_result {
        let _ = tar.kill().await;
        let _ = ssh.kill().await;
        let _ = tar.wait().await;
        let _ = ssh.wait().await;
        return Err(error);
    }

    let (tar_status, ssh_status) = tokio::join!(tar.wait(), ssh.wait());
    let tar_status = tar_status.context("failed while creating the local source archive")?;
    let ssh_status = ssh_status.context("failed while transferring the source archive")?;

    if !tar_status.success() {
        bail!("creating the local source archive failed");
    }

    if !ssh_status.success() {
        bail!("transferring the source archive to '{ssh_alias}' failed");
    }

    Ok(())
}

fn spawn_tar(source: &Path, excludes: &[String]) -> Result<Child> {
    let mut command = Command::new("tar");
    command.current_dir(source);

    for pattern in DEFAULT_EXCLUDES {
        command.arg(format!("--exclude={pattern}"));
    }

    for pattern in excludes {
        if pattern.contains('\0') {
            bail!("exclude patterns cannot contain a NUL byte");
        }
        command.arg(format!("--exclude={pattern}"));
    }

    command
        .args(["-cf", "-", "."])
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .context("failed to start tar; install a tar implementation to build remotely")
}

async fn cleanup_remote_workspace(
    ssh_alias: &str,
    build_id: &str,
    output: BuildOutput,
) -> Result<()> {
    let remote_script = format!(
        "workdir=\"$HOME/.cache/igara/builds/{build_id}\"\n\
         rm -rf -- \"$workdir\""
    );
    let status = Command::new("ssh")
        .arg(ssh_alias)
        .arg(remote_sh_command(&remote_script))
        .stdout(output.transfer_stdout())
        .status()
        .await
        .context("failed to clean up the incomplete remote build workspace")?;

    if !status.success() {
        bail!("failed to clean up the incomplete remote build workspace");
    }

    Ok(())
}

async fn run_command(
    mut command: Command,
    output: BuildOutput,
) -> Result<std::process::ExitStatus> {
    match output {
        BuildOutput::Inherit => command.status().await.context("failed to run command"),
        BuildOutput::Stderr => {
            let mut child = command
                .stdout(Stdio::piped())
                .spawn()
                .context("failed to start command")?;
            let Some(mut stdout) = child.stdout.take() else {
                let _ = child.kill().await;
                let _ = child.wait().await;
                bail!("failed to capture command stdout");
            };

            let mut stderr = io::stderr();
            if let Err(error) = io::copy(&mut stdout, &mut stderr).await {
                let _ = child.kill().await;
                let _ = child.wait().await;
                return Err(error).context("failed to forward remote build output to stderr");
            }

            child
                .wait()
                .await
                .context("failed while waiting for command")
        }
    }
}

fn render_remote_build_command(build_id: &str, command: &[String], keep: bool) -> String {
    remote_sh_command(&remote_build_script(build_id, command, keep))
}

fn remote_build_script(build_id: &str, command: &[String], keep: bool) -> String {
    let mut script = format!(
        "set +e\n\
         workdir=\"$HOME/.cache/igara/builds/{build_id}\"\n\
         cd \"$workdir\" || exit 125\n"
    );

    if !keep {
        script.push_str("trap 'rm -rf -- \"$workdir\"' 0\n");
    }

    let mut quoted_command = Vec::with_capacity(command.len());
    for argument in command {
        quoted_command.push(shell_quote(argument));
    }
    script.push_str("(\n");
    script.push_str(&quoted_command.join(" "));
    script.push_str("\n)\nstatus=$?\nexit \"$status\"\n");

    script
}

fn remote_sh_command(script: &str) -> String {
    format!("sh -c {}", shell_quote(script))
}

fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\"'\"'"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shell_quote_preserves_shell_metacharacters_as_data() {
        assert_eq!(shell_quote("cargo"), "'cargo'");
        assert_eq!(
            shell_quote("say 'hello'; touch /tmp/nope"),
            "'say '\"'\"'hello'\"'\"'; touch /tmp/nope'"
        );
    }

    #[test]
    fn remote_build_command_quotes_each_argument_and_removes_workspace_by_default() {
        let command = vec![
            "sh".to_string(),
            "-c".to_string(),
            "echo safe; touch /tmp/not-run".to_string(),
        ];

        let rendered = remote_build_script("build-id", &command, false);

        assert!(rendered.contains("trap 'rm -rf -- \"$workdir\"' 0"));
        assert!(rendered.contains("'echo safe; touch /tmp/not-run'"));
        assert!(rendered.contains("cd \"$workdir\" || exit 125"));
        assert!(rendered.contains("(\n"));
        assert!(rendered.contains("status=$?\nexit \"$status\""));
    }

    #[test]
    fn remote_build_command_keeps_workspace_when_requested() {
        let rendered = remote_build_script("build-id", &["true".to_string()], true);

        assert!(!rendered.contains("trap 'rm -rf"));
        assert!(rendered.contains("'true'"));
    }

    #[test]
    fn default_excludes_cover_common_build_outputs_and_credentials() {
        assert!(DEFAULT_EXCLUDES.contains(&"target"));
        assert!(DEFAULT_EXCLUDES.contains(&"node_modules"));
        assert!(DEFAULT_EXCLUDES.contains(&".env.*"));
        assert!(DEFAULT_EXCLUDES.contains(&".ssh"));
        assert!(DEFAULT_EXCLUDES.contains(&"id_ed25519"));
    }
}
