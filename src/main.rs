use anyhow::{Context, Result};
use clap::Parser;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use futures::StreamExt;
use ratatui::{
    backend::{Backend, CrosstermBackend},
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Frame, Terminal,
};
use std::io;
use std::process::{Command, Stdio};
use tokio::sync::mpsc;

#[derive(Parser, Debug)]
#[command(name = "hpm")]
#[command(about = "Hacker Package Manager - CLI tool for managing packages from apt, snap, and flatpak")]
struct Args {
    /// Initial search query
    #[arg(short, long)]
    query: Option<String>,
}

enum InputMode {
    Normal,
    Editing,
}

enum Message {
    Quit,
    Input(KeyCode),
}

struct App {
    input: String,
    input_mode: InputMode,
    packages: Vec<Package>,
    package_list_state: ListState,
    selected_source: Source,
    message: String,
}

#[derive(Clone)]
struct Package {
    name: String,
    source: Source,
    description: String,
}

#[derive(Clone, PartialEq)]
enum Source {
    Apt,
    Snap,
    Flatpak,
}

impl Source {
    fn as_str(&self) -> &'static str {
        match self {
            Source::Apt => "APT",
            Source::Snap => "SNAP",
            Source::Flatpak => "FLATPAK",
        }
    }
}

impl App {
    fn new() -> App {
        App {
            input: String::new(),
            input_mode: InputMode::Normal,
            packages: Vec::new(),
            package_list_state: ListState::default(),
            selected_source: Source::Apt,
            message: String::new(),
        }
    }

    fn search_packages(&mut self) -> Result<()> {
        self.packages.clear();
        self.message.clear();

        // Search APT
        let apt_output = Command::new("apt")
            .arg("search")
            .arg(&self.input)
            .output()
            .context("Failed to execute apt search")?;
        if apt_output.status.success() {
            let apt_str = String::from_utf8_lossy(&apt_output.stdout);
            for line in apt_str.lines() {
                if line.contains('/') {
                    let parts: Vec<&str> = line.splitn(2, ' ').collect();
                    if parts.len() >= 2 {
                        self.packages.push(Package {
                            name: parts[0].to_string(),
                            source: Source::Apt,
                            description: parts[1].to_string(),
                        });
                    }
                }
            }
        }

        // Search Snap
        let snap_output = Command::new("snap")
            .arg("find")
            .arg(&self.input)
            .output()
            .context("Failed to execute snap find")?;
        if snap_output.status.success() {
            let snap_str = String::from_utf8_lossy(&snap_output.stdout);
            for line in snap_str.lines().skip(1) {  // Skip header
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 2 {
                    self.packages.push(Package {
                        name: parts[0].to_string(),
                        source: Source::Snap,
                        description: parts[1..].join(" "),
                    });
                }
            }
        }

        // Search Flatpak
        let flatpak_output = Command::new("flatpak")
            .arg("search")
            .arg(&self.input)
            .output()
            .context("Failed to execute flatpak search")?;
        if flatpak_output.status.success() {
            let flatpak_str = String::from_utf8_lossy(&flatpak_output.stdout);
            for line in flatpak_str.lines().skip(1) {  // Skip header if present
                let parts: Vec<&str> = line.split('\t').collect();
                if parts.len() >= 3 {
                    self.packages.push(Package {
                        name: parts[1].to_string(),  // Application ID or name
                        source: Source::Flatpak,
                        description: parts[2].to_string(),
                    });
                }
            }
        }

        if self.packages.is_empty() {
            self.message = "No packages found.".to_string();
        }

        self.package_list_state.select(Some(0));
        Ok(())
    }

    fn install_package(&mut self) -> Result<()> {
        if let Some(selected) = self.package_list_state.selected() {
            if let Some(pkg) = self.packages.get(selected) {
                let cmd = match pkg.source {
                    Source::Apt => vec!["sudo", "apt", "install", "-y", &pkg.name],
                    Source::Snap => vec!["sudo", "snap", "install", &pkg.name],
                    Source::Flatpak => vec!["flatpak", "install", "-y", &pkg.name],
                };

                let status = Command::new(cmd[0])
                    .args(&cmd[1..])
                    .status()
                    .context("Failed to install package")?;

                if status.success() {
                    self.message = format!("Installed {} from {}", pkg.name, pkg.source.as_str());
                } else {
                    self.message = format!("Failed to install {} from {}", pkg.name, pkg.source.as_str());
                }
            }
        }
        Ok(())
    }

    fn remove_package(&mut self) -> Result<()> {
        if let Some(selected) = self.package_list_state.selected() {
            if let Some(pkg) = self.packages.get(selected) {
                let cmd = match pkg.source {
                    Source::Apt => vec!["sudo", "apt", "remove", "-y", &pkg.name],
                    Source::Snap => vec!["sudo", "snap", "remove", &pkg.name],
                    Source::Flatpak => vec!["flatpak", "uninstall", "-y", &pkg.name],
                };

                let status = Command::new(cmd[0])
                    .args(&cmd[1..])
                    .status()
                    .context("Failed to remove package")?;

                if status.success() {
                    self.message = format!("Removed {} from {}", pkg.name, pkg.source.as_str());
                } else {
                    self.message = format!("Failed to remove {} from {}", pkg.name, pkg.source.as_str());
                }
            }
        }
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    let mut app = App::new();
    if let Some(query) = args.query {
        app.input = query;
        app.search_packages()?;
    }

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let (tx, mut rx) = mpsc::channel(100);

    let res = run_app(&mut terminal, app, tx, &mut rx).await;

    // restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("{err:?}");
    }

    Ok(())
}

async fn run_app<B: Backend>(
    terminal: &mut Terminal<B>,
    mut app: App,
    tx: mpsc::Sender<Message>,
    rx: &mut mpsc::Receiver<Message>,
) -> Result<()> {
    let mut event_stream = event::EventStream::new();

    loop {
        terminal.draw(|f| ui(f, &mut app))?;

        tokio::select! {
            Some(event) = event_stream.next() => {
                if let Ok(Event::Key(key)) = event {
                    match app.input_mode {
                        InputMode::Normal => match key.code {
                            KeyCode::Char('q') => return Ok(()),
                            KeyCode::Char('e') => {
                                app.input_mode = InputMode::Editing;
                            }
                            KeyCode::Down => {
                                if let Some(selected) = app.package_list_state.selected() {
                                    if selected < app.packages.len() - 1 {
                                        app.package_list_state.select(Some(selected + 1));
                                    }
                                }
                            }
                            KeyCode::Up => {
                                if let Some(selected) = app.package_list_state.selected() {
                                    if selected > 0 {
                                        app.package_list_state.select(Some(selected - 1));
                                    }
                                }
                            }
                            KeyCode::Enter => {
                                app.search_packages()?;
                            }
                            KeyCode::Char('i') => {
                                app.install_package()?;
                            }
                            KeyCode::Char('r') => {
                                app.remove_package()?;
                            }
                            KeyCode::Char('a') => app.selected_source = Source::Apt,
                            KeyCode::Char('s') => app.selected_source = Source::Snap,
                            KeyCode::Char('f') => app.selected_source = Source::Flatpak,
                            _ => {}
                        },
                        InputMode::Editing => match key.code {
                            KeyCode::Enter => {
                                app.search_packages()?;
                                app.input_mode = InputMode::Normal;
                            }
                            KeyCode::Char(c) => {
                                app.input.push(c);
                            }
                            KeyCode::Backspace => {
                                app.input.pop();
                            }
                            KeyCode::Esc => {
                                app.input_mode = InputMode::Normal;
                            }
                            _ => {}
                        },
                    }
                }
            }
            Some(msg) = rx.recv() => {
                match msg {
                    Message::Quit => break,
                    Message::Input(_) => {},  // Handle if needed
                }
            }
        }
    }
    Ok(())
}

fn ui<B: Backend>(f: &mut Frame<B>, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(
            [
                Constraint::Length(3),
                Constraint::Min(1),
                Constraint::Length(3),
            ]
            .as_ref(),
        )
        .split(f.area());

    let (msg, style) = match app.input_mode {
        InputMode::Normal => (
            vec![
                Span::raw("Press "),
                Span::styled("q", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(" to exit, "),
                Span::styled("e", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(" to start editing."),
            ],
            Style::default().add_modifier(Modifier::RAPID_BLINK),
        ),
        InputMode::Editing => (
            vec![
                Span::raw("Press "),
                Span::styled("Esc", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(" to stop editing, "),
                Span::styled("Enter", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(" to search."),
            ],
            Style::default(),
        ),
    };
    let mut text = vec![Line::from(msg)];
    if !app.message.is_empty() {
        text.push(Line::from(Span::raw(app.message.clone())));
    }
    let help_message = Paragraph::new(text);
    f.render_widget(help_message, chunks[2]);

    let input = Paragraph::new(app.input.as_str())
        .style(match app.input_mode {
            InputMode::Normal => Style::default(),
            InputMode::Editing => Style::default().fg(Color::Yellow),
        })
        .block(Block::default().borders(Borders::ALL).title("Search Query"));
    f.render_widget(input, chunks[0]);

    match app.input_mode {
        InputMode::Editing => {
            f.set_cursor_position((chunks[0].x + app.input.len() as u16 + 1, chunks[0].y + 1))
        }
        _ => {}
    }

    let filtered_packages: Vec<ListItem> = app
        .packages
        .iter()
        .filter(|p| p.source == app.selected_source)
        .map(|p| {
            ListItem::new(vec![Line::from(vec![
                Span::styled(p.name.clone(), Style::default().fg(Color::Green)),
                Span::raw(" - "),
                Span::raw(p.description.clone()),
                Span::raw(" ("),
                Span::styled(p.source.as_str().to_string(), Style::default().fg(Color::Blue)),
                Span::raw(")"),
            ])])
        })
        .collect();

    let packages_list = List::new(filtered_packages)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!("Packages (Source: {}) - i: install, r: remove", app.selected_source.as_str())),
        )
        .highlight_style(
            Style::default()
                .bg(Color::LightGreen)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol(">> ");

    f.render_stateful_widget(packages_list, chunks[1], &mut app.package_list_state);
}
