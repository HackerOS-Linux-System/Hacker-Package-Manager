use anyhow::{Context, Result};
use clap::Parser;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use futures::StreamExt;
use ratatui::{
    backend::Backend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Frame, Terminal,
};
use std::io;
use std::process::Command;
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

        if self.input.is_empty() {
            self.message = "Enter a search query.".to_string();
            return Ok(());
        }

        // Search APT
        let apt_output = Command::new("apt-cache")
            .arg("search")
            .arg("--names-only")
            .arg(&self.input)
            .output()
            .context("Failed to execute apt-cache search")?;
        if apt_output.status.success() {
            let apt_str = String::from_utf8_lossy(&apt_output.stdout);
            for line in apt_str.lines() {
                if let Some((name, desc)) = line.split_once(" - ") {
                    self.packages.push(Package {
                        name: name.to_string(),
                        source: Source::Apt,
                        description: desc.to_string(),
                    });
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
            for line in snap_str.lines().skip(1) {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 5 {
                    let name = parts[0].to_string();
                    let description = parts[4..].join(" ");
                    self.packages.push(Package {
                        name,
                        source: Source::Snap,
                        description,
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
            for line in flatpak_str.lines().skip(1) {
                let parts: Vec<&str> = line.split('\t').collect();
                if parts.len() >= 4 {
                    let name = parts[2].to_string(); // Application ID for install
                    let description = format!("{} - {}", parts[0], parts[1]);
                    self.packages.push(Package {
                        name,
                        source: Source::Flatpak,
                        description,
                    });
                }
            }
        }

        // Filter by selected source if needed, but we collect all and filter in UI
        if self.packages.is_empty() {
            self.message = "No packages found.".to_string();
        } else {
            self.package_list_state.select(Some(0));
        }
        Ok(())
    }

    fn install_package(&mut self) -> Result<()> {
        if let Some(selected) = self.package_list_state.selected() {
            if let Some(pkg) = self.packages.get(selected) {
                let (cmd, args) = match pkg.source {
                    Source::Apt => ("apt", vec!["install", "-y", &pkg.name]),
                    Source::Snap => ("snap", vec!["install", &pkg.name]),
                    Source::Flatpak => ("flatpak", vec!["install", "-y", &pkg.name]),
                };

                let status = Command::new("sudo")
                    .arg(cmd)
                    .args(&args)
                    .status()
                    .context("Failed to install package")?;

                self.message = if status.success() {
                    format!("Installed {} from {}", pkg.name, pkg.source.as_str())
                } else {
                    format!("Failed to install {} from {}", pkg.name, pkg.source.as_str())
                };
            }
        }
        Ok(())
    }

    fn remove_package(&mut self) -> Result<()> {
        if let Some(selected) = self.package_list_state.selected() {
            if let Some(pkg) = self.packages.get(selected) {
                let (cmd, args) = match pkg.source {
                    Source::Apt => ("apt", vec!["remove", "-y", &pkg.name]),
                    Source::Snap => ("snap", vec!["remove", &pkg.name]),
                    Source::Flatpak => ("flatpak", vec!["uninstall", "-y", &pkg.name]),
                };

                let status = Command::new("sudo")
                    .arg(cmd)
                    .args(&args)
                    .status()
                    .context("Failed to remove package")?;

                self.message = if status.success() {
                    format!("Removed {} from {}", pkg.name, pkg.source.as_str())
                } else {
                    format!("Failed to remove {} from {}", pkg.name, pkg.source.as_str())
                };
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
    let backend = ratatui::backend::CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let (tx, mut rx) = mpsc::channel(100);

    let res = run_app(&mut terminal, app, tx, &mut rx).await;

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

async fn run_app(
    terminal: &mut Terminal<impl Backend>,
    mut app: App,
    _tx: mpsc::Sender<Message>,
    _rx: &mut mpsc::Receiver<Message>,
) -> Result<()> {
    let mut event_stream = event::EventStream::new();

    loop {
        terminal.draw(|f| ui(f, &mut app))?;

        if let Some(Ok(event)) = event_stream.next().await {
            if let Event::Key(key) = event {
                match app.input_mode {
                    InputMode::Normal => match key.code {
                        KeyCode::Char('q') => return Ok(()),
                        KeyCode::Char('e') => app.input_mode = InputMode::Editing,
                        KeyCode::Down => {
                            if let Some(selected) = app.package_list_state.selected() {
                                let len = app
                                    .packages
                                    .iter()
                                    .filter(|p| p.source == app.selected_source)
                                    .count();
                                if selected + 1 < len {
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
                        KeyCode::Char(c) => app.input.push(c),
                        KeyCode::Backspace => {
                            app.input.pop();
                        }
                        KeyCode::Esc => app.input_mode = InputMode::Normal,
                        _ => {}
                    },
                }
            }
        }
    }
}

fn ui(f: &mut Frame, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(1), Constraint::Length(3)])
        .split(f.area());

    let (msg, style) = match app.input_mode {
        InputMode::Normal => (
            vec![
                Span::raw("Press "),
                Span::styled("q", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(" to exit, "),
                Span::styled("e", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(" to edit query, "),
                Span::styled("Enter", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(" to search."),
            ],
            Style::default().add_modifier(Modifier::RAPID_BLINK),
        ),
        InputMode::Editing => (
            vec![
                Span::raw("Press "),
                Span::styled("Esc", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(" to cancel, "),
                Span::styled("Enter", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(" to search."),
            ],
            Style::default(),
        ),
    };

    let mut text = vec![Line::from(msg)];
    if !app.message.is_empty() {
        text.push(Line::from(Span::styled(
            app.message.clone(),
            Style::default().fg(Color::Yellow),
        )));
    }
    let help_message = Paragraph::new(text).style(style);
    f.render_widget(help_message, chunks[2]);

    let input = Paragraph::new(app.input.as_str())
        .style(match app.input_mode {
            InputMode::Normal => Style::default(),
            InputMode::Editing => Style::default().fg(Color::Yellow),
        })
        .block(Block::default().borders(Borders::ALL).title("Search Query"));
    f.render_widget(input, chunks[0]);

    if let InputMode::Editing = app.input_mode {
        let cursor_x = chunks[0].x + app.input.len() as u16 + 1;
        let cursor_y = chunks[0].y + 1;
        f.set_cursor_position((cursor_x, cursor_y));
    }

    let filtered_packages: Vec<Package> = app
        .packages
        .iter()
        .filter(|p| p.source == app.selected_source)
        .cloned()
        .collect();

    let items: Vec<ListItem> = filtered_packages
        .iter()
        .map(|p| {
            ListItem::new(Line::from(vec![
                Span::styled(&p.name, Style::default().fg(Color::Green)),
                Span::raw(" - "),
                Span::raw(&p.description),
            ]))
        })
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!(
                    "Packages [{}] (a/s/f to switch, i:install, r:remove)",
                    app.selected_source.as_str()
                )),
        )
        .highlight_style(Style::default().bg(Color::DarkGray).add_modifier(Modifier::BOLD))
        .highlight_symbol(">> ");

    let mut state = ListState::default();
    state.select(app.package_list_state.selected());
    if !filtered_packages.is_empty() {
        let offset = app
            .packages
            .iter()
            .filter(|p| p.source == app.selected_source)
            .take(app.package_list_state.selected().unwrap_or(0))
            .count();
        state.select(Some(offset));
    }

    f.render_stateful_widget(list, chunks[1], &mut state);
}
