use crate::models::HostProfile;
use color_eyre::eyre::Result;
use crossterm::event::{self, Event, KeyCode};
use crossterm::execute;
use crossterm::terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen};
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::widgets::{Block, Borders, Cell, Paragraph, Row, Table};
use ratatui::Terminal;
use std::io::{self, Stdout};
use std::time::Duration;

pub fn run(hosts: Vec<HostProfile>) -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    loop {
        terminal.draw(|frame| {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Length(3), Constraint::Min(5), Constraint::Length(1)])
                .split(frame.area());

            let header = Paragraph::new("Igará — remote host manager")
                .block(Block::default().title("Header").borders(Borders::ALL));
            frame.render_widget(header, chunks[0]);

            let rows = hosts.iter().map(|host| {
                Row::new(vec![
                    Cell::from(host.name.clone()),
                    Cell::from(host.hostname.clone()),
                    Cell::from(host.user_name.clone()),
                    Cell::from(host.connection_mode.to_string()),
                ])
            });
            let table = Table::new(
                rows,
                [
                    Constraint::Length(18),
                    Constraint::Length(26),
                    Constraint::Length(16),
                    Constraint::Length(12),
                ],
            )
            .header(Row::new(vec!["Name", "Host", "User", "Mode"]))
            .block(Block::default().title("Hosts").borders(Borders::ALL));
            frame.render_widget(table, chunks[1]);

            frame.render_widget(Paragraph::new("q: quit"), chunks[2]);
        })?;

        if event::poll(Duration::from_millis(250))? {
            if let Event::Key(key) = event::read()? {
                if key.code == KeyCode::Char('q') {
                    break;
                }
            }
        }
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
