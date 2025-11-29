use crate::app::{App, InputMode, Source};
use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
    Frame,
};

pub fn ui(f: &mut Frame, app: &mut App) {
    let chunks = Layout::default()
    .direction(Direction::Vertical)
    .constraints([Constraint::Length(3), Constraint::Min(1), Constraint::Length(5)])
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
                              Span::raw(" to search, "),
                              Span::styled("a/s/f/l", Style::default().add_modifier(Modifier::BOLD)),
                              Span::raw(" to switch source (APT/SNAP/FLATPAK/ALL), "),
                              Span::styled("i", Style::default().add_modifier(Modifier::BOLD)),
                              Span::raw(" to install, "),
                              Span::styled("r", Style::default().add_modifier(Modifier::BOLD)),
                              Span::raw(" to remove, "),
                              Span::styled("j/k", Style::default().add_modifier(Modifier::BOLD)),
                              Span::raw(" or arrows to navigate."),
            ],
            Style::default(),
        ),
        InputMode::Editing => (
            vec![
                Span::raw("Press "),
                               Span::styled("Esc", Style::default().add_modifier(Modifier::BOLD)),
                               Span::raw(" to cancel, "),
                               Span::styled("Enter", Style::default().add_modifier(Modifier::BOLD)),
                               Span::raw(" to confirm editing."),
            ],
            Style::default(),
        ),
    };
    let mut text = vec![Line::from(msg)];
    if !app.message.is_empty() {
        text.push(Line::from(""));
        text.push(Line::from(Span::styled(
            app.message.clone(),
                                          Style::default().fg(Color::Yellow),
        )));
    }
    let help_message = Paragraph::new(text).style(style).wrap(Wrap::default());
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
    let filtered_packages = app.get_filtered_packages();
    let items: Vec<ListItem> = filtered_packages
    .iter()
    .map(|p| {
        ListItem::new(Line::from(vec![
            Span::styled(&p.name, Style::default().fg(Color::Green)),
                                 Span::raw(" ("),
                                 Span::styled(p.source.as_str(), Style::default().fg(Color::Blue)),
                                 Span::raw(") - "),
                                 Span::raw(&p.description),
        ]))
    })
    .collect();
    let list = List::new(items)
    .block(
        Block::default()
        .borders(Borders::ALL)
        .title(format!(
            "Packages [{}] (a/s/f/l to switch, i:install, r:remove)",
                       app.selected_source.as_str()
        )),
    )
    .highlight_style(Style::default().bg(Color::DarkGray).add_modifier(Modifier::BOLD))
    .highlight_symbol(">> ");
    f.render_stateful_widget(list, chunks[1], &mut app.package_list_state);
}
