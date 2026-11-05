use crate::owned_process::{OwnedChild, spawn_owned};
use color_eyre::eyre::{Context, Result, bail, eyre};
use std::future::Future;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::Duration;
use tokio::io::{self, AsyncReadExt, AsyncWriteExt};
use tokio::process::Command;

pub use crate::build_options::BuildOptions;

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

#[derive(Debug)]
pub struct BuildResult {
    pub retained_workspace: Option<String>,
    pub artifacts: Vec<PathBuf>,
}

/// Compatibility entry point; defaults are bounded and noninteractive.
pub async fn build_project(
    alias: &str,
    source: &Path,
    command: &[String],
    excludes: &[String],
    keep: bool,
) -> Result<Option<String>> {
    let result = build_project_with_options(
        alias,
        source,
        command,
        excludes,
        keep,
        &BuildOptions::default(),
    )
    .await?;
    Ok(result.retained_workspace)
}

pub async fn build_project_with_options(
    alias: &str,
    source: &Path,
    command: &[String],
    excludes: &[String],
    keep: bool,
    options: &BuildOptions,
) -> Result<BuildResult> {
    build_with_output(
        alias,
        source,
        command,
        excludes,
        keep,
        options,
        BuildOutput::Inherit,
    )
    .await
}

pub(crate) async fn build_with_output(
    alias: &str,
    source: &Path,
    command: &[String],
    excludes: &[String],
    keep: bool,
    options: &BuildOptions,
    output: BuildOutput,
) -> Result<BuildResult> {
    options.validate()?;
    if alias.is_empty() || alias.starts_with('-') || alias.contains(['\0', '\n', '\r']) {
        bail!("invalid SSH alias");
    }
    if command.first().is_none_or(String::is_empty)
        || command
            .iter()
            .chain(excludes)
            .any(|value| value.contains('\0'))
    {
        bail!("a non-empty executable and NUL-free arguments/excludes are required");
    }
    if !source.is_dir() {
        bail!("build source is not a directory: {}", source.display());
    }
    let source = source
        .canonicalize()
        .context("failed to resolve build source")?;
    // Validate local destinations before executing any remote code.
    for artifact in &options.artifacts {
        let destination = options
            .output
            .join(artifact.rsplit('/').next().unwrap_or_default());
        match destination.symlink_metadata() {
            Ok(_) => bail!(
                "refusing to overwrite artifact destination {}",
                destination.display()
            ),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => {
                return Err(error).with_context(|| {
                    format!(
                        "cannot inspect artifact destination {}",
                        destination.display()
                    )
                });
            }
        }
    }
    bounded(options, options.connect_timeout + Duration::from_secs(2), "SSH preflight",
        status_command(ssh_command(alias, options,
            "python3 -c 'import os, signal, select; assert hasattr(os, \"WNOWAIT\")' && command -v tar >/dev/null"), output)).await?;
    let id = uuid7::uuid7().to_string();
    let remote_path = format!("~/.cache/igara/builds/{id}");
    let transfer = bounded(
        options,
        options.transfer_timeout,
        "source transfer",
        transfer_source(alias, &source, &id, excludes, options),
    )
    .await;
    if let Err(error) = transfer {
        return Err(failure_with_cleanup(error, alias, &id, options, keep).await);
    }
    let script = helper_command(
        "run",
        &id,
        &[
            options.execution_timeout.as_secs_f64().to_string(),
            serde_json::to_string(command)?,
        ],
    );
    let execution = bounded(
        options,
        options.execution_timeout + Duration::from_secs(3),
        "remote execution",
        status_command(ssh_command(alias, options, &script), output),
    )
    .await;
    if let Err(error) = execution {
        return Err(failure_with_cleanup(error, alias, &id, options, keep).await);
    }
    let artifacts = match bounded(
        options,
        options.transfer_timeout,
        "artifact transfer",
        download_artifacts(alias, &id, options),
    )
    .await
    {
        Ok(paths) => paths,
        Err(error) => {
            // Never discard the only remaining copy after a failed download/publication.
            return Err(eyre!(
                "{error:#}; workspace retained (if reachable) at {remote_path}"
            ));
        }
    };
    if !keep {
        cleanup(alias, &id, options).await.with_context(|| format!(
            "build succeeded, but cleanup is unconfirmed; workspace may remain at {remote_path}; local artifacts: {artifacts:?}"))?;
    }
    Ok(BuildResult {
        retained_workspace: keep.then_some(remote_path),
        artifacts,
    })
}

async fn failure_with_cleanup(
    error: color_eyre::Report,
    alias: &str,
    id: &str,
    options: &BuildOptions,
    keep: bool,
) -> color_eyre::Report {
    let location = format!("~/.cache/igara/builds/{id}");
    if keep {
        return eyre!("{error:#}; workspace retained (if created) at {location}");
    }
    // Cancellation drops stdin/SSH; give the remote supervisor time to stop its group.
    tokio::time::sleep(Duration::from_millis(500)).await;
    match cleanup(alias, id, options).await {
        Ok(()) => eyre!(
            "{error:#}; workspace removed; remote termination is best-effort on connection loss"
        ),
        Err(cleanup_error) => eyre!(
            "{error:#}; cleanup unconfirmed ({cleanup_error:#}); workspace may remain at {location}"
        ),
    }
}

pub(crate) fn ssh_command(alias: &str, options: &BuildOptions, script: &str) -> Command {
    let mut command = Command::new("ssh");
    if let Some(config) = &options.ssh_config {
        command.arg("-F").arg(config);
    }
    command.args([
        "-T",
        "-o",
        "BatchMode=yes",
        "-o",
        "StrictHostKeyChecking=yes",
        "-o",
        "ConnectionAttempts=1",
        "-o",
        "ServerAliveInterval=5",
        "-o",
        "ServerAliveCountMax=2",
        "-o",
        &format!(
            "ConnectTimeout={}",
            options.connect_timeout.as_secs().max(1)
        ),
        "--",
        alias,
        script,
    ]);
    command.kill_on_drop(true);
    command
}

async fn bounded<T>(
    options: &BuildOptions,
    budget: Duration,
    phase: &str,
    future: impl Future<Output = Result<T>>,
) -> Result<T> {
    tokio::select! {
        biased;
        _ = options.cancellation.cancelled() => bail!("{phase} cancelled"),
        result = tokio::time::timeout(budget, future) => {
            result.map_err(|_| eyre!("{phase} timed out after {} seconds", budget.as_secs_f64()))?
        }
    }
}

async fn status_command(mut command: Command, output: BuildOutput) -> Result<()> {
    // Hold stdin open as an execution lease. Dropping the future closes it.
    command.stdin(Stdio::piped());
    if matches!(output, BuildOutput::Stderr) {
        command.stdout(Stdio::piped());
    }
    let mut child = spawn_owned(&mut command).context("failed to start SSH")?;
    let lease = child.take_stdin();
    let forwarding = async {
        if let Some(mut stdout) = child.take_stdout() {
            io::copy(&mut stdout, &mut io::stderr()).await?;
        }
        Ok::<(), std::io::Error>(())
    };
    forwarding.await.context("failed to forward build output")?;
    let status = child.wait().await?;
    drop(lease);
    if !status.success() {
        match status.code() {
            Some(124) => bail!("remote execution timed out (remote deadline reached)"),
            Some(125) => bail!("remote execution cancelled (execution lease closed)"),
            _ => bail!("SSH command failed with {status}"),
        }
    }
    Ok(())
}

async fn transfer_source(
    alias: &str,
    source: &Path,
    id: &str,
    excludes: &[String],
    options: &BuildOptions,
) -> Result<()> {
    let script = format!(
        "set -eu\nworkdir=\"$HOME/.cache/igara/builds/{id}\"\nmkdir -p \"$(dirname \"$workdir\")\"\nmkdir -m 700 \"$workdir\"\ntar -xf - -C \"$workdir\""
    );
    let mut ssh = spawn_owned(
        ssh_command(alias, options, &script)
            .stdin(Stdio::piped())
            .stdout(Stdio::null()),
    )?;
    let mut tar = spawn_tar(source, excludes)?;
    let mut input = ssh.take_stdin().ok_or_else(|| eyre!("missing SSH stdin"))?;
    let mut archive = tar
        .take_stdout()
        .ok_or_else(|| eyre!("missing tar stdout"))?;
    io::copy(&mut archive, &mut input).await?;
    drop(input);
    let (tar_status, ssh_status) = tokio::try_join!(tar.wait(), ssh.wait())?;
    if !tar_status.success() || !ssh_status.success() {
        bail!("source transfer failed: tar {tar_status}, SSH {ssh_status}");
    }
    Ok(())
}

fn spawn_tar(source: &Path, excludes: &[String]) -> Result<OwnedChild> {
    let mut command = Command::new("tar");
    command.current_dir(source).kill_on_drop(true);
    for pattern in DEFAULT_EXCLUDES
        .iter()
        .copied()
        .chain(excludes.iter().map(String::as_str))
    {
        command.arg(format!("--exclude={pattern}"));
    }
    spawn_owned(command.args(["-cf", "-", "."]).stdout(Stdio::piped()))
        .context("failed to start local tar")
}

async fn download_artifacts(alias: &str, id: &str, options: &BuildOptions) -> Result<Vec<PathBuf>> {
    if options.artifacts.is_empty() {
        return Ok(Vec::new());
    }
    // Stage privately on the destination filesystem for no-replace promotion.
    std::fs::create_dir_all(&options.output)
        .context("failed to create artifact output directory")?;
    let staging = tempfile::tempdir_in(&options.output)?;
    let mut remaining = options.max_artifact_bytes;
    let mut staged = Vec::new();
    for artifact in &options.artifacts {
        let script = helper_command("artifact", id, std::slice::from_ref(artifact));
        let mut child = spawn_owned(
            ssh_command(alias, options, &script)
                .stdin(Stdio::null())
                .stdout(Stdio::piped()),
        )?;
        let mut stdout = child
            .take_stdout()
            .ok_or_else(|| eyre!("missing artifact stdout"))?;
        let mut header = [0_u8; 9];
        stdout
            .read_exact(&mut header)
            .await
            .context("invalid artifact header")?;
        let size = u64::from_be_bytes(header[..8].try_into()?);
        if size > remaining || header[8] > 1 {
            bail!(
                "artifact byte limit exceeded or invalid metadata ({})",
                options.max_artifact_bytes
            );
        }
        let temporary = tempfile::NamedTempFile::new_in(staging.path())?;
        let mut file = tokio::fs::File::from_std(temporary.reopen()?);
        let bytes = io::copy(&mut stdout.take(size + 1), &mut file).await?;
        file.flush().await?;
        file.sync_all().await?;
        drop(file);
        if bytes != size {
            bail!("artifact length changed during transfer");
        }
        if !child.wait().await?.success() {
            bail!("failed to retrieve artifact {artifact:?}");
        }
        remaining -= bytes;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            temporary
                .as_file()
                .set_permissions(std::fs::Permissions::from_mode(if header[8] == 1 {
                    0o700
                } else {
                    0o600
                }))?;
        }
        staged.push((
            temporary,
            options
                .output
                .join(artifact.rsplit('/').next().unwrap_or_default()),
        ));
    }
    let mut published = Vec::new();
    for (temporary, destination) in staged {
        temporary.persist_noclobber(&destination).map_err(|error| eyre!(
            "cannot publish {} without overwriting: {error}; partial local artifacts: {published:?}", destination.display()))?;
        published.push(destination);
    }
    Ok(published)
}

async fn cleanup(alias: &str, id: &str, options: &BuildOptions) -> Result<()> {
    let script = format!("rm -rf -- \"$HOME/.cache/igara/builds/{id}\"");
    // Independent budget/token: cancellation must not suppress cleanup itself.
    tokio::time::timeout(
        Duration::from_secs(10),
        status_command(ssh_command(alias, options, &script), BuildOutput::Stderr),
    )
    .await
    .context("cleanup timed out")?
}

fn helper_command(operation: &str, id: &str, arguments: &[String]) -> String {
    let mut command = format!(
        "python3 -c {} {} {}",
        shell_quote(include_str!("remote_build.py")),
        shell_quote(operation),
        shell_quote(id)
    );
    for argument in arguments {
        command.push(' ');
        command.push_str(&shell_quote(argument));
    }
    command
}

fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\"'\"'"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shell_quote_preserves_shell_metacharacters_as_data() {
        assert_eq!(shell_quote("a'b; $(echo no)"), "'a'\"'\"'b; $(echo no)'");
    }

    #[test]
    fn artifact_paths_and_basename_collisions_are_rejected() {
        for path in [
            "../escape",
            "/absolute",
            "dir/../file",
            "dir//file",
            "./file",
            "a\\b",
            "file\n",
        ] {
            let options = BuildOptions {
                artifacts: vec![path.into()],
                ..Default::default()
            };
            assert!(options.validate().is_err(), "{path:?}");
        }
        let options = BuildOptions {
            artifacts: vec!["a/app".into(), "b/APP".into()],
            ..Default::default()
        };
        assert!(options.validate().is_err());
    }

    #[test]
    fn default_excludes_cover_common_build_outputs_and_credentials() {
        for value in ["target", "node_modules", ".env.*", ".ssh", "id_ed25519"] {
            assert!(DEFAULT_EXCLUDES.contains(&value));
        }
    }
}
