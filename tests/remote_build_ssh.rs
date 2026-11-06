//! Explicit opt-in real OpenSSH fixture; never reads registered hosts or user SSH config.
#![cfg(target_os = "linux")]

use color_eyre::eyre::{Result, bail};
use igara::build::{BuildOptions, build_project_with_options};
use std::fs;
use std::net::{TcpListener, TcpStream};
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};
use tempfile::TempDir;

struct SshFixture {
    server: Child,
    root: TempDir,
    home: PathBuf,
    source: PathBuf,
    options: BuildOptions,
}

impl SshFixture {
    fn new() -> Result<Self> {
        let root = tempfile::tempdir()?;
        let home = root.path().join("home");
        let source = root.path().join("source");
        fs::create_dir(&home)?;
        fs::create_dir(&source)?;
        fs::write(source.join("input.txt"), "payload ' ; $()\n")?;
        fs::write(source.join(".env.secret"), "synthetic-not-a-credential")?;
        let host_key = root.path().join("host_key");
        let client_key = root.path().join("client_key");
        for key in [&host_key, &client_key] {
            let status = Command::new("ssh-keygen")
                .env_clear()
                .env("PATH", "/usr/bin:/bin")
                .args(["-q", "-t", "ed25519", "-N", "", "-f"])
                .arg(key)
                .status()?;
            if !status.success() {
                bail!("fixture ssh-keygen failed");
            }
        }
        let user = String::from_utf8(Command::new("id").arg("-un").output()?.stdout)?
            .trim()
            .to_string();
        let listener = TcpListener::bind("127.0.0.1:0")?;
        let port = listener.local_addr()?.port();
        let wrapper = root.path().join("remote.sh");
        fs::write(
            &wrapper,
            format!(
                "#!/bin/sh\nexport HOME='{}'\nexport PATH='/usr/bin:/bin'\nunset SSH_AUTH_SOCK\ncd \"$HOME\" || exit 1\nexec /bin/sh -c \"$SSH_ORIGINAL_COMMAND\"\n",
                home.display()
            ),
        )?;
        let server_config = root.path().join("sshd_config");
        fs::write(
            &server_config,
            format!(
                "ListenAddress 127.0.0.1\nPort {port}\nHostKey {}\nPidFile {}\nAuthorizedKeysFile {}\nStrictModes no\nUsePAM no\nPasswordAuthentication no\nKbdInteractiveAuthentication no\nPubkeyAuthentication yes\nPermitRootLogin yes\nAllowUsers {user}\nAllowAgentForwarding no\nAllowTcpForwarding no\nX11Forwarding no\nForceCommand /bin/sh {}\nLogLevel ERROR\n",
                host_key.display(),
                root.path().join("sshd.pid").display(),
                client_key.with_extension("pub").display(),
                wrapper.display()
            ),
        )?;
        let known_hosts = root.path().join("known_hosts");
        fs::write(
            &known_hosts,
            format!(
                "[127.0.0.1]:{port} {}",
                fs::read_to_string(host_key.with_extension("pub"))?
            ),
        )?;
        let config = root.path().join("ssh_config");
        fs::write(
            &config,
            format!(
                "Host fixture\n HostName 127.0.0.1\n Port {port}\n User {user}\n IdentityFile {}\n IdentitiesOnly yes\n IdentityAgent none\n UserKnownHostsFile {}\n GlobalKnownHostsFile /dev/null\n ProxyCommand none\n",
                client_key.display(),
                known_hosts.display()
            ),
        )?;
        drop(listener);
        let log = fs::File::create(root.path().join("sshd.log"))?;
        let mut server = Command::new(which::which("sshd")?)
            .env_clear()
            .env("PATH", "/usr/bin:/bin")
            .arg("-D")
            .arg("-e")
            .arg("-f")
            .arg(server_config)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(log)
            .spawn()?;
        let start = Instant::now();
        loop {
            if let Some(status) = server.try_wait()? {
                bail!(
                    "fixture sshd exited {status}: {}",
                    fs::read_to_string(root.path().join("sshd.log"))?
                );
            }
            if TcpStream::connect((std::net::Ipv4Addr::LOCALHOST, port)).is_ok() {
                break;
            }
            if start.elapsed() > Duration::from_secs(5) {
                server.kill()?;
                server.wait()?;
                bail!("fixture sshd startup timed out");
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        let options = BuildOptions {
            ssh_config: Some(config),
            output: root.path().join("output"),
            connect_timeout: Duration::from_secs(2),
            transfer_timeout: Duration::from_secs(5),
            execution_timeout: Duration::from_secs(5),
            ..Default::default()
        };
        Ok(Self {
            server,
            root,
            home,
            source,
            options,
        })
    }

    fn workspaces(&self) -> Result<Vec<PathBuf>> {
        let root = self.home.join(".cache/igara/builds");
        if !root.exists() {
            return Ok(Vec::new());
        }
        Ok(fs::read_dir(root)?
            .map(|entry| entry.map(|entry| entry.path()))
            .collect::<std::io::Result<_>>()?)
    }

    async fn run(&self, script: &str, keep: bool) -> Result<igara::build::BuildResult> {
        build_project_with_options(
            "fixture",
            &self.source,
            &["sh".into(), "-c".into(), script.into()],
            &[],
            keep,
            &self.options,
        )
        .await
    }

    fn register_host(&self) -> Result<PathBuf> {
        use igara::models::{ConnectionMode, CreateHostOptions, HostProfile};
        let config = self.root.path().join("registered-config");
        fs::create_dir_all(config.join("db"))?;
        let repo =
            igara::repo::HostRepository::new(igara::db::open(&config.join("db/igara.sqlite3"))?);
        repo.insert_host(&HostProfile::new(CreateHostOptions {
            name: "fixture".into(),
            hostname: "127.0.0.1".into(),
            user_name: "synthetic".into(),
            port: 22,
            connection_mode: ConnectionMode::Lan,
            proxy_jump: None,
            ssh_key_path: self.root.path().join("client_key").display().to_string(),
            tags: Vec::new(),
        }))?;
        Ok(config)
    }
}

impl Drop for SshFixture {
    fn drop(&mut self) {
        let _ = self.server.kill();
        let _ = self.server.wait();
    }
}

#[tokio::test]
#[ignore = "requires local Linux OpenSSH server; run make test-ssh"]
async fn real_transfer_artifacts_excludes_quoting_and_cleanup() -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    let mut fixture = SshFixture::new()?;
    fixture.options.artifacts = vec!["out/a ' ; file.txt".into(), "out/binary".into()];
    fs::write(fixture.source.join("binary-input"), [0, 255, 1, 128])?;
    let result = fixture
        .run(
            "test ! -e .env.secret && mkdir out && cp input.txt \"out/a ' ; file.txt\" && cp binary-input out/binary && chmod +x out/binary",
            false,
        )
        .await?;
    assert_eq!(
        fs::read_to_string(&result.artifacts[0])?,
        "payload ' ; $()\n"
    );
    assert!(fixture.workspaces()?.is_empty());
    assert!(result.retained_workspace.is_none());
    assert_eq!(fs::read(&result.artifacts[1])?, [0, 255, 1, 128]);
    assert_eq!(
        fs::metadata(&result.artifacts[1])?.permissions().mode() & 0o777,
        0o700
    );
    fixture.options.artifacts.clear();
    let retained = fixture.run("true", true).await?;
    assert!(retained.retained_workspace.is_some());
    assert_eq!(fixture.workspaces()?.len(), 1);
    Ok(())
}

#[tokio::test]
#[ignore = "requires local Linux OpenSSH server; run make test-ssh"]
async fn failed_download_retains_workspace_and_never_overwrites() -> Result<()> {
    let mut fixture = SshFixture::new()?;
    fixture.options.artifacts = vec!["missing".into()];
    let error = fixture.run("true", false).await.unwrap_err();
    assert!(error.to_string().contains("workspace retained"));
    assert_eq!(fixture.workspaces()?.len(), 1);
    fs::create_dir_all(&fixture.options.output)?;
    fs::write(fixture.options.output.join("missing"), "original")?;
    assert!(
        fixture
            .run("touch executed", false)
            .await
            .unwrap_err()
            .to_string()
            .contains("overwrite")
    );
    assert_eq!(
        fs::read_to_string(fixture.options.output.join("missing"))?,
        "original"
    );
    Ok(())
}

#[tokio::test]
#[ignore = "requires local Linux OpenSSH server; run make test-ssh"]
async fn symlinks_hardlinks_special_files_and_oversize_are_rejected() -> Result<()> {
    for script in [
        "ln -s /etc/passwd artifact",
        "ln input.txt artifact",
        "mkfifo artifact",
        "mkdir real; cp input.txt real/file; ln -s real artifact",
    ] {
        let mut fixture = SshFixture::new()?;
        fixture.options.artifacts = vec!["artifact".into()];
        assert!(fixture.run(script, false).await.is_err());
        assert!(!fixture.options.output.join("artifact").exists());
    }
    let mut fixture = SshFixture::new()?;
    fixture.options.artifacts = vec!["input.txt".into()];
    fixture.options.max_artifact_bytes = 2;
    assert!(
        fixture
            .run("true", false)
            .await
            .unwrap_err()
            .to_string()
            .contains("byte limit")
    );
    Ok(())
}

#[tokio::test]
#[ignore = "requires local Linux OpenSSH server; run make test-ssh"]
async fn failure_keep_timeout_and_cancellation_stop_descendants() -> Result<()> {
    let fixture = SshFixture::new()?;
    assert!(fixture.run("exit 42", false).await.is_err());
    assert!(fixture.workspaces()?.is_empty());
    assert!(
        fixture
            .run("exit 42", true)
            .await
            .unwrap_err()
            .to_string()
            .contains("retained")
    );
    assert_eq!(fixture.workspaces()?.len(), 1);
    for cancel in [false, true] {
        let mut fixture = SshFixture::new()?;
        fixture.options.execution_timeout = Duration::from_millis(300);
        if cancel {
            fixture.options.execution_timeout = Duration::from_secs(5);
            let token = fixture.options.cancellation.clone();
            let marker = fixture.home.join("started");
            tokio::spawn(async move {
                for _ in 0..100 {
                    if marker.exists() {
                        token.cancel();
                        return;
                    }
                    tokio::time::sleep(Duration::from_millis(20)).await;
                }
                token.cancel();
            });
        }
        let start = Instant::now();
        let error = fixture
            .run(
                "touch \"$HOME/started\"; (sleep 2; touch \"$HOME/escaped\") & wait",
                false,
            )
            .await
            .unwrap_err();
        assert!(start.elapsed() < Duration::from_secs(8), "{error:#}");
        assert!(
            fixture.home.join("started").exists(),
            "execution never started: {error:#}"
        );
        tokio::time::sleep(Duration::from_secs(3)).await;
        assert!(
            !fixture.home.join("escaped").exists(),
            "descendant survived: {error:#}"
        );
        assert!(fixture.workspaces()?.is_empty());
    }
    Ok(())
}

#[tokio::test]
#[ignore = "requires local Linux OpenSSH server; run make test-ssh"]
async fn doctor_inspects_only_explicit_host_without_mutation() -> Result<()> {
    use igara::models::{ConnectionMode, CreateHostOptions, HostProfile};
    let fixture = SshFixture::new()?;
    let host = HostProfile::new(CreateHostOptions {
        name: "fixture".into(),
        hostname: "127.0.0.1".into(),
        user_name: "synthetic".into(),
        port: 22,
        connection_mode: ConnectionMode::Lan,
        proxy_jump: None,
        ssh_key_path: fixture.root.path().join("client_key").display().to_string(),
        tags: Vec::new(),
    });
    let report = igara::doctor::diagnose(Some(&host), &fixture.options).await?;
    assert!(!report.has_failures(), "{report:?}");
    assert!(
        report
            .checks
            .iter()
            .any(|check| check.name == "remote.disk")
    );
    assert!(!fixture.home.join(".cache").exists());
    Ok(())
}

#[tokio::test]
#[ignore = "requires local Linux OpenSSH server; run make test-ssh"]
async fn cli_build_and_doctor_use_isolated_registered_host() -> Result<()> {
    use igara::models::{ConnectionMode, CreateHostOptions, HostProfile};
    let fixture = SshFixture::new()?;
    let config = fixture.root.path().join("config");
    fs::create_dir(&config)?;
    let db_file = config.join("db/igara.sqlite3");
    fs::create_dir(config.join("db"))?;
    let repo = igara::repo::HostRepository::new(igara::db::open(&db_file)?);
    let host = HostProfile::new(CreateHostOptions {
        name: "fixture".into(),
        hostname: "127.0.0.1".into(),
        user_name: "synthetic".into(),
        port: 22,
        connection_mode: ConnectionMode::Lan,
        proxy_jump: None,
        ssh_key_path: fixture.root.path().join("client_key").display().to_string(),
        tags: Vec::new(),
    });
    repo.insert_host(&host)?;
    let output = tokio::process::Command::new(env!("CARGO_BIN_EXE_igara"))
        .env("IGARA_CONFIG_DIR", &config)
        .args(["build", "fixture", "--source"])
        .arg(&fixture.source)
        .arg("--ssh-config")
        .arg(fixture.options.ssh_config.as_ref().unwrap())
        .args(["--artifact", "input.txt", "--output"])
        .arg(&fixture.options.output)
        .args(["--", "true"])
        .output()
        .await?;
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        fs::read_to_string(fixture.options.output.join("input.txt"))?,
        "payload ' ; $()\n"
    );
    assert!(fixture.workspaces()?.is_empty());
    let before = fs::read(&db_file)?;
    let output = tokio::process::Command::new(env!("CARGO_BIN_EXE_igara"))
        .env("IGARA_CONFIG_DIR", &config)
        .args(["doctor", "--host", "fixture", "--json", "--ssh-config"])
        .arg(fixture.options.ssh_config.as_ref().unwrap())
        .output()
        .await?;
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let report: serde_json::Value = serde_json::from_slice(&output.stdout)?;
    assert!(
        report["checks"]
            .as_array()
            .unwrap()
            .iter()
            .any(|check| check["name"] == "remote.ssh")
    );
    assert_eq!(before, fs::read(db_file)?);
    Ok(())
}

#[tokio::test]
#[ignore = "requires local Linux OpenSSH server; run make test-ssh"]
async fn transfer_timeout_and_connection_loss_are_reported_honestly() -> Result<()> {
    let mut fixture = SshFixture::new()?;
    fixture.options.transfer_timeout = Duration::from_millis(1);
    let error = fixture
        .run("touch should-not-run", false)
        .await
        .unwrap_err();
    assert!(
        error.to_string().contains("source transfer timed out"),
        "{error:#}"
    );
    assert!(fixture.workspaces()?.is_empty());
    fixture.server.kill()?;
    fixture.server.wait()?;
    let error = fixture.run("true", false).await.unwrap_err();
    assert!(
        error.to_string().contains("SSH command failed"),
        "{error:#}"
    );
    assert!(fixture.workspaces()?.is_empty());
    Ok(())
}

#[tokio::test]
#[ignore = "requires local Linux OpenSSH server; run make test-ssh"]
async fn cleanup_failure_preserves_recovery_location() -> Result<()> {
    let mut fixture = SshFixture::new()?;
    let source = fixture.source.clone();
    let options = fixture.options.clone();
    let task = tokio::spawn(async move {
        build_project_with_options(
            "fixture",
            &source,
            &[
                "sh".into(),
                "-c".into(),
                "touch \"$HOME/ready\"; sleep 1; exit 9".into(),
            ],
            &[],
            false,
            &options,
        )
        .await
    });
    let deadline = Instant::now() + Duration::from_secs(5);
    while !fixture.home.join("ready").exists() && Instant::now() < deadline {
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    assert!(fixture.home.join("ready").exists());
    // Existing SSH session may finish; no NEW cleanup session can connect.
    fixture.server.kill()?;
    fixture.server.wait()?;
    let error = task.await?.unwrap_err();
    assert!(
        error.to_string().contains("cleanup unconfirmed"),
        "{error:#}"
    );
    assert!(
        error.to_string().contains("workspace may remain"),
        "{error:#}"
    );
    assert_eq!(fixture.workspaces()?.len(), 1);
    Ok(())
}

#[tokio::test]
#[ignore = "requires local Linux OpenSSH server; run make test-ssh"]
async fn cli_ctrl_c_cancels_execution_and_cleans_workspace() -> Result<()> {
    let fixture = SshFixture::new()?;
    let config = fixture.register_host()?;
    let mut child = tokio::process::Command::new(env!("CARGO_BIN_EXE_igara"))
        .env("IGARA_CONFIG_DIR", config)
        .args(["build", "fixture", "--source"])
        .arg(&fixture.source)
        .arg("--ssh-config")
        .arg(fixture.options.ssh_config.as_ref().unwrap())
        .args([
            "--execution-timeout",
            "5",
            "--",
            "sh",
            "-c",
            "touch \"$HOME/cli-started\"; (sleep 2; touch \"$HOME/cli-survived\") & wait",
        ])
        .kill_on_drop(true)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    let deadline = Instant::now() + Duration::from_secs(15);
    while !fixture.home.join("cli-started").exists() && Instant::now() < deadline {
        if let Some(status) = child.try_wait()? {
            let result = child.wait_with_output().await?;
            bail!(
                "CLI exited before remote execution ({status}): stdout={} stderr={}",
                String::from_utf8_lossy(&result.stdout),
                String::from_utf8_lossy(&result.stderr)
            );
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    if !fixture.home.join("cli-started").exists() {
        let _ = child.start_kill();
        let result = child.wait_with_output().await?;
        bail!(
            "timed out waiting for remote CLI marker: stdout={} stderr={}",
            String::from_utf8_lossy(&result.stdout),
            String::from_utf8_lossy(&result.stderr)
        );
    }
    let pid = rustix::process::Pid::from_raw(child.id().unwrap() as i32).unwrap();
    rustix::process::kill_process(pid, rustix::process::Signal::INT)?;
    let result = tokio::time::timeout(Duration::from_secs(10), child.wait_with_output()).await??;
    assert!(!result.status.success());
    assert!(String::from_utf8_lossy(&result.stderr).contains("cancelled"));
    tokio::time::sleep(Duration::from_secs(3)).await;
    assert!(!fixture.home.join("cli-survived").exists());
    assert!(fixture.workspaces()?.is_empty());
    Ok(())
}
