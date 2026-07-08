use crate::bootstrap;
use crate::models::HostProfile;
use crate::repo::HostRepository;
use crate::services::{self, SharedService};
use crate::ssh;
use color_eyre::eyre::Result;
use crossterm::event::{self, Event, KeyCode};
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::widgets::{Block, Borders, Cell, Paragraph, Row, Table, TableState};
use std::io::{self, Stdout};
use std::sync::Arc;
use std::time::Duration;

struct App {
    hosts: Vec<HostProfile>,
    repo: Arc<HostRepository>,
    table_state: TableState,
    status: String,
}

impl App {
    fn new(hosts: Vec<HostProfile>, repo: Arc<HostRepository>) -> Self {
        let mut table_state = TableState::default();
        if !hosts.is_empty() {
            table_state.select(Some(0));
        }
        Self {
            hosts,
            repo,
            table_state,
            status:
                "Arrows: navigate. s: ssh, b: bootstrap, h: health, p: postgres, v: valkey, q: quit"
                    .to_string(),
        }
    }

    fn next(&mut self) {
        let i = match self.table_state.selected() {
            Some(i) => {
                if i >= self.hosts.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.table_state.select(Some(i));
    }

    fn previous(&mut self) {
        let i = match self.table_state.selected() {
            Some(i) => {
                if i == 0 {
                    self.hosts.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.table_state.select(Some(i));
    }

    fn selected_host(&self) -> Option<&HostProfile> {
        self.table_state.selected().map(|i| &self.hosts[i])
    }

    async fn refresh_hosts(&mut self) -> Result<()> {
        self.hosts = self.repo.list_hosts()?;
        Ok(())
    }
}

pub async fn run(hosts: Vec<HostProfile>, repo: Arc<HostRepository>) -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new(hosts, repo);

    loop {
        terminal.draw(|frame| {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(3),
                    Constraint::Min(5),
                    Constraint::Length(1),
                ])
                .split(frame.area());

            let header = Paragraph::new("Igará — remote host manager")
                .block(Block::default().title("Header").borders(Borders::ALL));
            frame.render_widget(header, chunks[0]);

            let rows = app.hosts.iter().map(|host| {
                let health_status = match host.health {
                    Some(ref h) => {
                        if h.is_up {
                            "UP"
                        } else {
                            "DOWN"
                        }
                    }
                    None => "UNKNOWN",
                };
                Row::new(vec![
                    Cell::from(host.name.clone()),
                    Cell::from(host.hostname.clone()),
                    Cell::from(host.user_name.clone()),
                    Cell::from(host.connection_mode.to_string()),
                    Cell::from(health_status),
                ])
            });
            let table = Table::new(
                rows,
                [
                    Constraint::Length(15),
                    Constraint::Length(20),
                    Constraint::Length(12),
                    Constraint::Length(10),
                    Constraint::Length(10),
                ],
            )
            .header(Row::new(vec!["Name", "Host", "User", "Mode", "Health"]))
            .block(Block::default().title("Hosts").borders(Borders::ALL))
            .highlight_symbol(">> ")
            .row_highlight_style(
                ratatui::style::Style::default().add_modifier(ratatui::style::Modifier::REVERSED),
            );

            frame.render_stateful_widget(table, chunks[1], &mut app.table_state);

            frame.render_widget(Paragraph::new(app.status.as_str()), chunks[2]);
        })?;

        if event::poll(Duration::from_millis(250))?
            && let Event::Key(key) = event::read()?
            && key.kind == event::KeyEventKind::Press
        {
            match key.code {
                KeyCode::Char('q') => break,
                KeyCode::Down | KeyCode::Char('j') => app.next(),
                KeyCode::Up | KeyCode::Char('k') => app.previous(),
                KeyCode::Char('s') => {
                    if let Some(host) = app.selected_host() {
                        let alias = host.ssh_alias.clone();
                        // Exit TUI temporarily
                        disable_raw_mode()?;
                        execute!(terminal.backend_mut(), LeaveAlternateScreen)?;

                        println!("Connecting to {}...", alias);
                        let mut child = std::process::Command::new("ssh").arg(&alias).spawn()?;
                        child.wait()?;

                        // Re-enter TUI
                        enable_raw_mode()?;
                        execute!(terminal.backend_mut(), EnterAlternateScreen)?;
                        terminal.clear()?;
                    }
                }
                KeyCode::Char('p') => {
                    if let Some(host) = app.selected_host() {
                        let host_clone = host.clone();
                        app.status = format!("Deploying Postgres to {}...", host.name);
                        tokio::spawn(async move {
                            let _ = services::deploy_service(&host_clone, SharedService::Postgres)
                                .await;
                        });
                    }
                }
                KeyCode::Char('v') => {
                    if let Some(host) = app.selected_host() {
                        let host_clone = host.clone();
                        app.status = format!("Deploying Valkey to {}...", host.name);
                        tokio::spawn(async move {
                            let _ =
                                services::deploy_service(&host_clone, SharedService::Valkey).await;
                        });
                    }
                }
                KeyCode::Char('h') => {
                    if let Some(host) = app.selected_host() {
                        let alias = host.ssh_alias.clone();
                        let host_id = host.id.clone();
                        let repo = Arc::clone(&app.repo);
                        app.status = format!("Checking health for {}...", host.name);
                        tokio::spawn(async move {
                            let is_up = ssh::test_ssh(&alias).await.is_ok();
                            let _ = repo.update_health(&host_id, is_up);
                        });
                    }
                }
                KeyCode::Char('b') => {
                    if let Some(host) = app.selected_host() {
                        let alias = host.ssh_alias.clone();
                        let host_name = host.name.clone();
                        app.status = format!("Bootstrapping {} in background...", host_name);
                        tokio::spawn(async move {
                            if let Ok(result) = bootstrap::inspect(&alias).await {
                                let _ = bootstrap::apply(&alias, result.platform_family).await;
                            }
                        });
                    }
                }
                KeyCode::Char('r') => {
                    app.refresh_hosts().await?;
                    app.status = "Refreshed hosts".to_string();
                }
                _ => {}
            }
        }

        // Auto-refresh to see health updates (optional, but good for UX)
        app.refresh_hosts().await?;
    }

    restore_terminal(terminal)?;
    Ok(())
}

fn restore_terminal(mut terminal: Terminal<CrosstermBackend<Stdout>>) -> Result<()> {
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::run_migrations;
    use crate::models::{ConnectionMode, CreateHostOptions};
    use crate::repo::HostRepository;
    use rusqlite::Connection;

    fn setup_repo() -> Arc<HostRepository> {
        let mut conn = Connection::open_in_memory().unwrap();
        conn.execute("PRAGMA foreign_keys = ON", []).unwrap();
        run_migrations(&mut conn).unwrap();
        Arc::new(HostRepository::new(conn))
    }

    fn make_host(name: &str) -> HostProfile {
        HostProfile::new(CreateHostOptions {
            name: name.to_string(),
            hostname: format!("{}.example.com", name),
            user_name: "root".to_string(),
            port: 22,
            connection_mode: ConnectionMode::Lan,
            proxy_jump: None,
            ssh_key_path: format!("/tmp/{}", name),
            tags: vec![],
        })
    }

    #[test]
    fn test_app_new_empty() {
        let app = App::new(vec![], setup_repo());
        assert!(app.hosts.is_empty());
        assert!(app.table_state.selected().is_none());
    }

    #[test]
    fn test_app_new_selects_first() {
        let hosts = vec![make_host("alpha"), make_host("beta")];
        let app = App::new(hosts, setup_repo());
        assert_eq!(app.table_state.selected(), Some(0));
        assert_eq!(app.selected_host().unwrap().name, "alpha");
    }

    #[test]
    fn test_app_navigation_cycles_forward() {
        let hosts = vec![make_host("a"), make_host("b"), make_host("c")];
        let mut app = App::new(hosts, setup_repo());

        assert_eq!(app.selected_host().unwrap().name, "a");
        app.next();
        assert_eq!(app.selected_host().unwrap().name, "b");
        app.next();
        assert_eq!(app.selected_host().unwrap().name, "c");
        app.next();
        assert_eq!(app.selected_host().unwrap().name, "a");
    }

    #[test]
    fn test_app_navigation_cycles_backward() {
        let hosts = vec![make_host("a"), make_host("b"), make_host("c")];
        let mut app = App::new(hosts, setup_repo());

        app.previous();
        assert_eq!(app.selected_host().unwrap().name, "c");
        app.previous();
        assert_eq!(app.selected_host().unwrap().name, "b");
        app.previous();
        assert_eq!(app.selected_host().unwrap().name, "a");
    }

    #[test]
    fn test_app_navigation_single_host() {
        let hosts = vec![make_host("only")];
        let mut app = App::new(hosts, setup_repo());

        assert_eq!(app.selected_host().unwrap().name, "only");
        app.next();
        assert_eq!(app.selected_host().unwrap().name, "only");
        app.previous();
        assert_eq!(app.selected_host().unwrap().name, "only");
    }

    #[test]
    fn test_app_status_default() {
        let app = App::new(vec![], setup_repo());
        assert!(app.status.contains("Arrows"));
        assert!(app.status.contains("q: quit"));
    }
}
