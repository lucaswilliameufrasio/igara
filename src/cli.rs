use clap::{Args, Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(name = "igara")]
#[command(version)]
#[command(about = "Remote host bootstrap and docker context manager")]
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
    Tui,
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
