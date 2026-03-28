mod bootstrap;
mod cli;
mod config;
mod db;
mod docker;
mod errors;
mod models;
mod repo;
mod services;
mod ssh;
mod tui;

use clap::Parser;
use cli::{BootstrapCommands, Cli, Commands, DockerCommands, HostCommands, SshCommands};
use color_eyre::eyre::Result;
use config::AppPaths;
use models::{ConnectionMode, HostProfile};
use repo::HostRepository;
use std::sync::Arc;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let cli = Cli::parse();
    let paths = AppPaths::resolve()?;
    paths.ensure()?;
    let conn = db::open(&paths.db_file)?;
    let repo = Arc::new(HostRepository::new(conn));

    match cli.command {
        Some(Commands::Host { command }) => match command {
            HostCommands::Add(args) => {
                let mode: ConnectionMode = args.mode.parse()?;
                let key_path = ssh::key_path(&paths, &args.name);
                ssh::generate_host_key(&key_path, &format!("igara:{}", args.name))?;
                let ssh_block = ssh::render_ssh_block(
                    &args.name,
                    &args.hostname,
                    &args.user,
                    args.port,
                    &key_path,
                    args.proxy_jump.as_deref(),
                );
                ssh::upsert_managed_block(&paths.ssh_config_path()?, &args.name, &ssh_block)?;

                let host = HostProfile::new(
                    args.name.clone(),
                    args.hostname,
                    args.user,
                    args.port,
                    mode,
                    args.proxy_jump.clone(),
                    key_path.display().to_string(),
                    args.tags.into_iter().filter(|t| !t.is_empty()).collect(),
                );
                repo.insert_host(&host)?;

                if args.install_key {
                    ssh::install_public_key(&host.ssh_alias, &key_path.with_extension("pub")).await?;
                }
                if args.create_docker_context {
                    docker::create_context(&host.docker_context_name, &host.ssh_alias)?;
                }

                println!("Host {} added", host.name);
            }
            HostCommands::List => {
                for host in repo.list_hosts()? {
                    let health_status = match host.health {
                        Some(h) => if h.is_up { "UP" } else { "DOWN" },
                        None => "UNKNOWN",
                    };
                    println!(
                        "{}\t{}\t{}\t{}\t{}\t{}",
                        host.name,
                        host.hostname,
                        host.user_name,
                        host.connection_mode,
                        health_status,
                        if host.is_default { "(default)" } else { "" }
                    );
                }
            }
            HostCommands::Show { name } => {
                let host = repo.get_host_by_name(&name)?;
                println!("{}", serde_json::to_string_pretty(&host)?);
            }
            HostCommands::Remove { name } => {
                let host = repo.get_host_by_name(&name)?;
                let removed = repo.delete_host(&name)?;
                if removed {
                    let _ = docker::remove_context(&host.docker_context_name);
                    let _ = ssh::remove_managed_block(&paths.ssh_config_path()?, &host.ssh_alias);
                    println!("Host {} removed", host.name);
                }
            }
            HostCommands::Use { name } => {
                repo.set_default_host(&name)?;
                let host = repo.get_host_by_name(&name)?;
                docker::use_context(&host.docker_context_name)?;
                println!("Using host {}", host.name);
            }
            HostCommands::BootstrapAll => {
                let hosts = repo.list_hosts()?;
                let mut handles = Vec::new();
                for host in hosts {
                    handles.push(tokio::spawn(async move {
                        println!("Bootstrapping {}...", host.name);
                        match bootstrap::inspect(&host.ssh_alias).await {
                            Ok(result) => {
                                if let Err(e) = bootstrap::apply(&host.ssh_alias, result.platform_family).await {
                                    eprintln!("Failed to bootstrap {}: {}", host.name, e);
                                } else {
                                    println!("Bootstrap successful for {}", host.name);
                                }
                            }
                            Err(e) => eprintln!("Failed to inspect {}: {}", host.name, e),
                        }
                    }));
                }
                futures::future::join_all(handles).await;
            }
            HostCommands::HealthAll => {
                let hosts = repo.list_hosts()?;
                let mut handles = Vec::new();
                for host in hosts {
                    let repo_clone = Arc::clone(&repo);
                    handles.push(tokio::spawn(async move {
                        let is_up = ssh::test_ssh(&host.ssh_alias).await.is_ok();
                        let _ = repo_clone.update_health(&host.id, is_up);
                        println!("Host {}: {}", host.name, if is_up { "UP" } else { "DOWN" });
                    }));
                }
                futures::future::join_all(handles).await;
            }
        },
        Some(Commands::Ssh { command }) => match command {
            SshCommands::Test { alias } => {
                ssh::test_ssh(&alias).await?;
                println!("SSH connection to {} successful", alias);
            }
            SshCommands::InstallKey { alias } => {
                let host = repo.get_host_by_name(&alias)?;
                ssh::install_public_key(&host.ssh_alias, &std::path::PathBuf::from(&host.ssh_key_path).with_extension("pub")).await?;
                println!("Public key installed on {}", alias);
            }
            SshCommands::RenderConfig { alias } => {
                let host = repo.get_host_by_name(&alias)?;
                let block = ssh::render_ssh_block(
                    &host.name,
                    &host.hostname,
                    &host.user_name,
                    host.port,
                    &std::path::PathBuf::from(&host.ssh_key_path),
                    host.proxy_jump.as_deref(),
                );
                println!("{}", block);
            }
        },
        Some(Commands::Docker { command }) => match command {
            DockerCommands::CreateContext { name } => {
                let host = repo.get_host_by_name(&name)?;
                docker::create_context(&host.docker_context_name, &host.ssh_alias)?;
                println!("Docker context {} created", host.docker_context_name);
            }
            DockerCommands::Use { name } => {
                let host = repo.get_host_by_name(&name)?;
                docker::use_context(&host.docker_context_name)?;
                println!("Using docker context {}", host.docker_context_name);
            }
            DockerCommands::RemoveContext { name } => {
                let host = repo.get_host_by_name(&name)?;
                docker::remove_context(&host.docker_context_name)?;
                println!("Docker context {} removed", host.docker_context_name);
            }
            DockerCommands::Info { name } => {
                let host = repo.get_host_by_name(&name)?;
                docker::run_docker_command(&host.ssh_alias, &["info"])?;
            }
            DockerCommands::Ps { name } => {
                let host = repo.get_host_by_name(&name)?;
                docker::run_docker_command(&host.ssh_alias, &["ps"])?;
            }
        },
        Some(Commands::Bootstrap { command }) => match command {
            BootstrapCommands::Inspect { name } => {
                let host = repo.get_host_by_name(&name)?;
                let result = bootstrap::inspect(&host.ssh_alias).await?;
                println!(
                    "{}",
                    serde_json::to_string_pretty(&serde_json::json!({
                        "platform_family": result.platform_family.to_string(),
                        "has_docker": result.has_docker,
                        "has_tailscale": result.has_tailscale,
                        "has_buildx": result.has_buildx,
                        "uname_m": result.uname_m,
                        "os_release": result.os_release,
                    }))?
                );
            }
            BootstrapCommands::Apply { name } => {
                let host = repo.get_host_by_name(&name)?;
                let result = bootstrap::inspect(&host.ssh_alias).await?;
                bootstrap::apply(&host.ssh_alias, result.platform_family).await?;
                println!("Bootstrap applied to {}", host.name);
            }
        },
        Some(Commands::Tui) => {
            let hosts = repo.list_hosts()?;
            tui::run(hosts, Arc::clone(&repo)).await?;
        }
        None => {
            let hosts = repo.list_hosts()?;
            tui::run(hosts, Arc::clone(&repo)).await?;
        }
    }

    Ok(())
}
