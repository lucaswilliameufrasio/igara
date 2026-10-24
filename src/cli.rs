use clap::{Args, Parser, Subcommand};
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[command(name = "igara")]
#[command(version)]
#[command(about = "Remote host, build, and docker context manager")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    Host {
        #[command(subcommand)]
        command: HostCommands,
    },
    Ssh {
        #[command(subcommand)]
        command: SshCommands,
    },
    Docker {
        #[command(subcommand)]
        command: DockerCommands,
    },
    Bootstrap {
        #[command(subcommand)]
        command: BootstrapCommands,
    },
    Build(BuildArgs),
    Mcp {
        #[command(subcommand)]
        command: McpCommands,
    },
    Tui,
}

#[derive(Debug, Subcommand)]
pub enum McpCommands {
    Serve,
}

#[derive(Debug, Args)]
pub struct BuildArgs {
    #[arg(value_name = "HOST")]
    pub host: String,
    #[arg(long, default_value = ".")]
    pub source: PathBuf,
    #[arg(long = "exclude", value_name = "PATTERN")]
    pub excludes: Vec<String>,
    #[arg(long)]
    pub keep: bool,
    #[arg(last = true, required = true, value_name = "COMMAND")]
    pub command: Vec<String>,
}

#[derive(Debug, Subcommand)]
pub enum HostCommands {
    Add(AddHostArgs),
    List,
    Show { name: String },
    Remove { name: String },
    Use { name: String },
    BootstrapAll,
    HealthAll,
}

#[derive(Debug, Args)]
pub struct AddHostArgs {
    #[arg(long)]
    pub name: String,
    #[arg(long)]
    pub hostname: String,
    #[arg(long)]
    pub user: String,
    #[arg(long, default_value_t = 22)]
    pub port: u16,
    #[arg(long, default_value = "tailscale")]
    pub mode: String,
    #[arg(long)]
    pub proxy_jump: Option<String>,
    #[arg(long, value_delimiter = ',', default_value = "")]
    pub tags: Vec<String>,
    #[arg(long, default_value_t = false)]
    pub install_key: bool,
    #[arg(long, default_value_t = false)]
    pub create_docker_context: bool,
}

#[derive(Debug, Subcommand)]
pub enum SshCommands {
    Test { alias: String },
    InstallKey { alias: String },
    RenderConfig { alias: String },
}

#[derive(Debug, Subcommand)]
pub enum DockerCommands {
    CreateContext { name: String },
    Use { name: String },
    RemoveContext { name: String },
    Info { name: String },
    Ps { name: String },
}

#[derive(Debug, Subcommand)]
pub enum BootstrapCommands {
    Inspect { name: String },
    Apply { name: String },
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[test]
    fn build_command_accepts_explicit_source_options_and_command_arguments() {
        let cli = Cli::try_parse_from([
            "igara",
            "build",
            "beefy",
            "--source",
            "./my-project",
            "--keep",
            "--exclude",
            "generated",
            "--",
            "cargo",
            "build",
            "--release",
        ])
        .unwrap();

        let Some(Commands::Build(args)) = cli.command else {
            panic!("expected build command");
        };

        assert_eq!(args.host, "beefy");
        assert_eq!(args.source, PathBuf::from("./my-project"));
        assert_eq!(args.excludes, vec!["generated"]);
        assert!(args.keep);
        assert_eq!(args.command, vec!["cargo", "build", "--release"]);
    }

    #[test]
    fn build_command_requires_a_command_after_separator() {
        let result = Cli::try_parse_from(["igara", "build", "beefy"]);

        assert!(result.is_err());
    }

    #[test]
    fn mcp_serve_command_is_available() {
        let cli = Cli::try_parse_from(["igara", "mcp", "serve"]).unwrap();

        assert!(matches!(
            cli.command,
            Some(Commands::Mcp {
                command: McpCommands::Serve
            })
        ));
    }
}
