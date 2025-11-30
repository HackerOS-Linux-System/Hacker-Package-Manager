use anyhow::Result;
use app::{App, InputMode, Package, Source, install_package, remove_package, search_packages};
use clap::Parser;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use futures::StreamExt;
use ratatui::{backend::Backend, Terminal};
use std::io;
use tokio::sync::mpsc;
use tokio::time;
use ui::ui;

mod app;
mod ui;

#[derive(Parser, Debug)]
#[command(name = "hpm")]
#[command(about = "Hacker Package Manager - CLI tool for managing packages from apt, snap, and flatpak")]
struct Args {
    /// Initial search query
    #[arg(short, long)]
    query: Option<String>,
}

pub enum AppMessage {
    SearchComplete(Result<Vec<Package>>),
    InstallComplete(Result<String>),
    RemoveComplete(Result<String>),
    Tick,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    let mut app = App::new();
    if let Some(query) = args.query {
        app.input = query;
        app.packages = search_packages(app.input.clone()).await?;
    }
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = ratatui::backend::CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    let res = run_app(&mut terminal, app).await;
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
) -> Result<()> {
    let mut event_stream = event::EventStream::new();
    let (update_tx, mut update_rx) = mpsc::channel::<AppMessage>(10);

    loop {
        terminal.draw(|f| ui(f, &mut app))?;

        tokio::select! {
            Some(event) = event_stream.next() => {
                if let Ok(Event::Key(key)) = event {
                    match app.input_mode {
                        InputMode::Normal => match key.code {
                            KeyCode::Char('q') => return Ok(()),
                            KeyCode::Char('e') => app.input_mode = InputMode::Editing,
                            KeyCode::Down | KeyCode::Char('j') => {
                                if let Some(selected) = app.package_list_state.selected() {
                                    let len = app.get_filtered_packages().len();
                                    if selected + 1 < len {
                                        app.package_list_state.select(Some(selected + 1));
                                    }
                                } else if !app.get_filtered_packages().is_empty() {
                                    app.package_list_state.select(Some(0));
                                }
                            }
                            KeyCode::Up | KeyCode::Char('k') => {
                                if let Some(selected) = app.package_list_state.selected() {
                                    if selected > 0 {
                                        app.package_list_state.select(Some(selected - 1));
                                    }
                                }
                            }
                            KeyCode::Enter => {
                                if app.input.is_empty() {
                                    app.message = "Enter a search query.".to_string();
                                    continue;
                                }
                                app.message = "Searching".to_string();
                                app.dot_count = 0;
                                let input = app.input.clone();
                                let tx = update_tx.clone();
                                let (cancel_tx, cancel_rx) = mpsc::channel::<()>(1);
                                tokio::spawn(async move {
                                    let res = search_packages(input).await;
                                    let _ = tx.send(AppMessage::SearchComplete(res)).await;
                                    let _ = cancel_tx.send(()).await;
                                });
                                let tx = update_tx.clone();
                                tokio::spawn(async move {
                                    let mut cancel_rx = cancel_rx;
                                    loop {
                                        tokio::select! {
                                            _ = cancel_rx.recv() => break,
                                             _ = time::sleep(time::Duration::from_millis(500)) => {
                                                 let _ = tx.send(AppMessage::Tick).await;
                                             }
                                        }
                                    }
                                });
                            }
                            KeyCode::Char('i') => {
                                if let Some(selected) = app.package_list_state.selected() {
                                    let filtered = app.get_filtered_packages();
                                    if let Some(pkg) = filtered.get(selected) {
                                        let pkg = pkg.clone();
                                        let tx = update_tx.clone();
                                        app.message = "Installing...".to_string();
                                        tokio::spawn(async move {
                                            let res = install_package(pkg).await;
                                            let _ = tx.send(AppMessage::InstallComplete(res)).await;
                                        });
                                    }
                                }
                            }
                            KeyCode::Char('r') => {
                                if let Some(selected) = app.package_list_state.selected() {
                                    let filtered = app.get_filtered_packages();
                                    if let Some(pkg) = filtered.get(selected) {
                                        let pkg = pkg.clone();
                                        let tx = update_tx.clone();
                                        app.message = "Removing...".to_string();
                                        tokio::spawn(async move {
                                            let res = remove_package(pkg).await;
                                            let _ = tx.send(AppMessage::RemoveComplete(res)).await;
                                        });
                                    }
                                }
                            }
                            KeyCode::Char('a') => {
                                app.selected_source = Source::Apt;
                                update_selection(&mut app);
                            }
                            KeyCode::Char('s') => {
                                app.selected_source = Source::Snap;
                                update_selection(&mut app);
                            }
                            KeyCode::Char('f') => {
                                app.selected_source = Source::Flatpak;
                                update_selection(&mut app);
                            }
                            KeyCode::Char('l') => {
                                app.selected_source = Source::All;
                                update_selection(&mut app);
                            }
                            _ => {}
                        },
                        InputMode::Editing => match key.code {
                            KeyCode::Enter => {
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
            Some(msg) = update_rx.recv() => {
                match msg {
                    AppMessage::SearchComplete(res) => {
                        match res {
                            Ok(pkgs) => {
                                app.packages = pkgs;
                                if app.packages.is_empty() {
                                    app.message = "No packages found.".to_string();
                                } else {
                                    app.message = String::new();
                                    update_selection(&mut app);
                                }
                            }
                            Err(e) => {
                                app.message = format!("Search failed: {}", e);
                            }
                        }
                    }
                    AppMessage::InstallComplete(res) => {
                        match res {
                            Ok(msg) => app.message = msg,
                            Err(e) => app.message = format!("Install failed: {}", e),
                        }
                    }
                    AppMessage::RemoveComplete(res) => {
                        match res {
                            Ok(msg) => app.message = msg,
                            Err(e) => app.message = format!("Remove failed: {}", e),
                        }
                    }
                    AppMessage::Tick => {
                        app.dot_count += 1;
                        app.message = "Searching".to_string() + &".".repeat(app.dot_count % 4);
                    }
                }
            }
        }
    }
}

fn update_selection(app: &mut App) {
    let filtered_len = app.get_filtered_packages().len();
    if filtered_len > 0 {
        app.package_list_state.select(Some(0));
    } else {
        app.package_list_state.select(None);
    }
}
