use crate::build::{self, BuildOutput};
use crate::models::HostProfile;
use crate::repo::HostRepository;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{CallToolResult, ContentBlock};
use rmcp::{RoleServer, service::RequestContext};
use rmcp::{ServiceExt, schemars, tool, tool_router, transport::stdio};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::Arc;

#[derive(Clone)]
struct IgaraMcpServer {
    repo: Arc<HostRepository>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct RemoteBuildArgs {
    /// Name of a host already registered with Igará.
    host: String,
    /// Local source directory; relative paths use the MCP process working directory.
    #[serde(default)]
    source: Option<String>,
    /// Command and arguments to execute remotely. This runs with the SSH user's permissions.
    command: Vec<String>,
    /// Additional tar exclude patterns for files that should not be transferred.
    #[serde(default)]
    excludes: Vec<String>,
    /// Keep the remote workspace and build artifacts instead of removing them.
    #[serde(default)]
    keep: bool,
    /// Explicit workspace-relative files to download by basename, without overwriting.
    #[serde(default)]
    artifacts: Vec<String>,
    #[serde(default)]
    output: Option<String>,
    #[serde(default)]
    connect_timeout: Option<u64>,
    #[serde(default)]
    transfer_timeout: Option<u64>,
    #[serde(default)]
    execution_timeout: Option<u64>,
    #[serde(default)]
    max_artifact_bytes: Option<u64>,
}

#[derive(Debug, Serialize)]
struct HostSummary {
    name: String,
    connection_mode: String,
    is_default: bool,
}

impl From<&HostProfile> for HostSummary {
    fn from(host: &HostProfile) -> Self {
        Self {
            name: host.name.clone(),
            connection_mode: host.connection_mode.to_string(),
            is_default: host.is_default,
        }
    }
}

#[tool_router(server_handler)]
impl IgaraMcpServer {
    #[tool(
        description = "List registered Igará hosts. Returns only each host's name, connection mode, and default status; it does not expose addresses, usernames, SSH aliases, key paths, or credentials."
    )]
    fn list_hosts(&self) -> CallToolResult {
        let hosts = match self.repo.list_hosts() {
            Ok(hosts) => hosts,
            Err(error) => {
                return tool_error(format!("failed to list registered hosts: {error}"));
            }
        };

        let summaries = hosts.iter().map(HostSummary::from).collect::<Vec<_>>();
        match serde_json::to_string_pretty(&summaries) {
            Ok(json) => CallToolResult::success(vec![ContentBlock::text(json)]),
            Err(error) => tool_error(format!("failed to serialize host list: {error}")),
        }
    }

    #[tool(
        description = "Explicitly transfer the selected local source directory to a registered host and run the supplied command there as the configured SSH user. This is remote code execution with that user's permissions; call only when the user requested or confirmed this build."
    )]
    async fn remote_build(
        &self,
        Parameters(args): Parameters<RemoteBuildArgs>,
        context: RequestContext<RoleServer>,
    ) -> CallToolResult {
        if let Err(message) = validate_remote_build_args(&args) {
            return tool_error(message);
        }

        let host = match self.repo.get_host_by_name(&args.host) {
            Ok(host) => host,
            Err(error) => {
                return tool_error(format!("failed to find host '{}': {error}", args.host));
            }
        };

        let source = args.source.as_deref().unwrap_or(".");
        let defaults = build::BuildOptions::default();
        let options = build::BuildOptions {
            artifacts: args.artifacts,
            output: args.output.map(Into::into).unwrap_or(defaults.output),
            connect_timeout: std::time::Duration::from_secs(args.connect_timeout.unwrap_or(10)),
            transfer_timeout: std::time::Duration::from_secs(args.transfer_timeout.unwrap_or(300)),
            execution_timeout: std::time::Duration::from_secs(
                args.execution_timeout.unwrap_or(3600),
            ),
            max_artifact_bytes: args
                .max_artifact_bytes
                .unwrap_or(defaults.max_artifact_bytes),
            cancellation: context.ct.clone(),
            ssh_config: None,
        };
        match build::build_with_output(
            &host.ssh_alias,
            Path::new(source),
            &args.command,
            &args.excludes,
            args.keep,
            &options,
            BuildOutput::Stderr,
        )
        .await
        {
            Ok(result) => CallToolResult::success(vec![ContentBlock::text(format!(
                "Remote build completed on '{}'. Artifacts: {:?}. {}",
                host.name,
                result.artifacts,
                result
                    .retained_workspace
                    .map(|path| format!("Workspace retained at {path}."))
                    .unwrap_or_else(|| "Temporary workspace removed.".into())
            ))]),
            Err(error) => tool_error(format!("remote build failed on '{}': {error}", host.name)),
        }
    }
}

pub async fn serve(repo: Arc<HostRepository>) -> color_eyre::eyre::Result<()> {
    tracing::info!("Starting Igará MCP server over stdio");
    let service = IgaraMcpServer { repo }.serve(stdio()).await?;
    service.waiting().await?;
    Ok(())
}

fn validate_remote_build_args(args: &RemoteBuildArgs) -> Result<(), String> {
    if args.host.trim().is_empty() {
        return Err("host must be a non-empty registered host name".to_string());
    }

    if args.command.first().is_none_or(String::is_empty) {
        return Err("command must contain a non-empty executable".to_string());
    }

    if args.command.iter().any(|argument| argument.contains('\0')) {
        return Err("command arguments cannot contain a NUL byte".to_string());
    }

    if args.excludes.iter().any(|pattern| pattern.contains('\0')) {
        return Err("exclude patterns cannot contain a NUL byte".to_string());
    }

    Ok(())
}

fn tool_error(message: String) -> CallToolResult {
    CallToolResult::error(vec![ContentBlock::text(message)])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::run_migrations;
    use crate::models::{ConnectionMode, CreateHostOptions};
    use rusqlite::Connection;

    fn make_host() -> HostProfile {
        HostProfile::new(CreateHostOptions {
            name: "build-host".to_string(),
            hostname: "host.example.test".to_string(),
            user_name: "builder".to_string(),
            port: 22,
            connection_mode: ConnectionMode::Lan,
            proxy_jump: None,
            ssh_key_path: "/private/path/to/key".to_string(),
            tags: vec!["ci".to_string()],
        })
    }

    #[test]
    fn host_summary_contains_only_minimal_non_secret_fields() -> color_eyre::eyre::Result<()> {
        let host = make_host();
        let value = serde_json::to_value(HostSummary::from(&host))?;

        assert_eq!(value["name"], "build-host");
        assert_eq!(value["connection_mode"], "lan");
        assert_eq!(value["is_default"], false);
        assert_eq!(value.as_object().map(serde_json::Map::len), Some(3));
        assert!(!value.to_string().contains("host.example.test"));
        assert!(!value.to_string().contains("/private/path/to/key"));
        assert!(!value.to_string().contains("builder"));

        Ok(())
    }

    #[test]
    fn invalid_remote_build_inputs_are_rejected_before_ssh() {
        let args = RemoteBuildArgs {
            host: "build-host".to_string(),
            source: None,
            command: Vec::new(),
            excludes: Vec::new(),
            keep: false,
            artifacts: Vec::new(),
            output: None,
            connect_timeout: None,
            transfer_timeout: None,
            execution_timeout: None,
            max_artifact_bytes: None,
        };

        assert_eq!(
            validate_remote_build_args(&args),
            Err("command must contain a non-empty executable".to_string())
        );
    }

    #[test]
    fn server_lists_registered_hosts_as_summaries() -> color_eyre::eyre::Result<()> {
        let mut connection = Connection::open_in_memory()?;
        connection.execute("PRAGMA foreign_keys = ON", [])?;
        run_migrations(&mut connection)?;
        let repo = Arc::new(HostRepository::new(connection));
        repo.insert_host(&make_host())?;
        let server = IgaraMcpServer { repo };

        let result = server.list_hosts();
        let text = result
            .content
            .first()
            .and_then(|block| block.as_text())
            .map(|block| block.text.as_str())
            .unwrap_or_default();
        let summary: serde_json::Value = serde_json::from_str(text)?;

        assert_eq!(summary[0]["name"], "build-host");
        assert_eq!(summary[0]["connection_mode"], "lan");
        assert_eq!(summary[0].as_object().map(serde_json::Map::len), Some(3));
        assert!(!text.contains("host.example.test"));
        assert!(!text.contains("/private/path/to/key"));

        Ok(())
    }
}
