use igara::build::{BuildOptions, build_project_with_options};
use std::path::Path;
use std::time::Duration;

#[tokio::test]
async fn invalid_requests_fail_before_ssh() {
    let source = tempfile::tempdir().unwrap();
    for artifact in ["/escape", "../escape", "a/../escape"] {
        let options = BuildOptions {
            artifacts: vec![artifact.into()],
            ..Default::default()
        };
        let error = build_project_with_options(
            "never-connect.invalid",
            source.path(),
            &["true".into()],
            &[],
            false,
            &options,
        )
        .await
        .unwrap_err();
        assert!(error.to_string().contains("relative"));
    }
    let options = BuildOptions {
        connect_timeout: Duration::ZERO,
        ..Default::default()
    };
    assert!(
        build_project_with_options(
            "unused",
            source.path(),
            &["true".into()],
            &[],
            false,
            &options
        )
        .await
        .unwrap_err()
        .to_string()
        .contains("timeouts")
    );
    assert!(
        build_project_with_options(
            "-oProxyCommand=bad",
            source.path(),
            &["true".into()],
            &[],
            false,
            &BuildOptions::default()
        )
        .await
        .unwrap_err()
        .to_string()
        .contains("alias")
    );
    assert!(
        build_project_with_options(
            "unused",
            Path::new("/missing/igara-fixture-source"),
            &["true".into()],
            &[],
            false,
            &BuildOptions::default()
        )
        .await
        .is_err()
    );
}

#[tokio::test]
async fn cancelled_request_never_connects() {
    let source = tempfile::tempdir().unwrap();
    let options = BuildOptions::default();
    options.cancellation.cancel();
    let error = build_project_with_options(
        "never-connect.invalid",
        source.path(),
        &["true".into()],
        &[],
        false,
        &options,
    )
    .await
    .unwrap_err();
    assert!(error.to_string().contains("cancelled"));
}

#[tokio::test]
async fn ssh_handshake_is_bounded_even_when_peer_never_sends_a_banner() {
    let Ok(ssh) = which::which("ssh") else {
        // The cross-platform default suite may run without OpenSSH. The explicit
        // Linux integration gate always requires it instead of silently skipping.
        return;
    };
    assert!(ssh.is_file());
    let source = tempfile::tempdir().unwrap();
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let config = source.path().join("ssh_config");
    std::fs::write(
        &config,
        format!(
            "Host fixture\n HostName 127.0.0.1\n Port {}\n User fixture\n",
            listener.local_addr().unwrap().port()
        ),
    )
    .unwrap();
    let options = BuildOptions {
        ssh_config: Some(config),
        connect_timeout: Duration::from_secs(1),
        ..Default::default()
    };
    let start = std::time::Instant::now();
    assert!(
        build_project_with_options(
            "fixture",
            source.path(),
            &["true".into()],
            &[],
            false,
            &options
        )
        .await
        .is_err()
    );
    assert!(start.elapsed() < Duration::from_secs(5));
    assert!(!source.path().join("dist").exists());
}

#[test]
fn doctor_local_cli_does_not_initialize_config_or_database() {
    let root = tempfile::tempdir().unwrap();
    let config = root.path().join("not-created");
    let result = std::process::Command::new(env!("CARGO_BIN_EXE_igara"))
        .args(["doctor", "--json"])
        .env("IGARA_CONFIG_DIR", &config)
        .output()
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&result.stdout).unwrap();
    assert!(
        json["checks"]
            .as_array()
            .unwrap()
            .iter()
            .all(|check| !check["name"].as_str().unwrap().starts_with("remote."))
    );
    assert!(!config.exists());
}

#[test]
fn doctor_missing_tools_reports_failures_without_mutation() {
    let root = tempfile::tempdir().unwrap();
    let config = root.path().join("not-created");
    let result = std::process::Command::new(env!("CARGO_BIN_EXE_igara"))
        .args(["doctor", "--json"])
        .env("IGARA_CONFIG_DIR", &config)
        .env("PATH", root.path())
        .output()
        .unwrap();
    assert!(!result.status.success());
    let json: serde_json::Value = serde_json::from_slice(&result.stdout).unwrap();
    assert!(
        json["checks"]
            .as_array()
            .unwrap()
            .iter()
            .any(|check| check["name"] == "ssh" && check["status"] == "fail")
    );
    assert!(
        json["checks"]
            .as_array()
            .unwrap()
            .iter()
            .any(|check| check["name"] == "docker" && check["status"] == "warning")
    );
    assert!(!config.exists());
}
