//! Server List Component
//!
//! Renders the central server list area.
//! Features:
//! - Server name, host, and port display
//! - Selection highlighting
//! - Status indicators
//! - Group affiliation

use crate::app::{App, Focus};
use easyssh_core::ServerStatus;
use ratatui::{
    layout::{Constraint, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Row, Table},
    Frame,
};

pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    // Determine border style based on focus
    let is_focused = app.focus == Focus::ServerList;
    let border_style = if is_focused {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default().fg(Color::Gray)
    };

    let block = Block::default()
        .title(" Servers ")
        .borders(Borders::ALL)
        .border_style(border_style);

    // Build table rows
    let mut rows: Vec<Row> = Vec::new();

    for (display_index, &server_index) in app.filtered_servers.iter().enumerate() {
        if let Some(server) = app.servers.get(server_index) {
            let is_selected = app.selected_server == display_index;

            // Determine row style based on selection
            let style = if is_selected && is_focused {
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::White)
                    .add_modifier(Modifier::BOLD)
            } else if is_selected {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };

            // Status symbol
            let status_symbol = status_to_symbol(&server.status);
            let status_color = status_to_color(&server.status);

            // Selection indicator
            let indicator = if is_selected { "> " } else { "  " };

            // Get group name
            let group_name = server
                .group_id
                .as_ref()
                .and_then(|gid| {
                    app.groups
                        .iter()
                        .find(|g| &g.id == gid)
                        .map(|g| g.name.as_str())
                })
                .unwrap_or("-");

            // Build row cells
            let cells = vec![
                Cell::from(Line::from(vec![
                    Span::styled(indicator, style),
                    Span::styled(status_symbol, style.fg(status_color)),
                ])),
                Cell::from(server.name.clone()).style(style),
                Cell::from(format!(
                    "{}@{}:{}",
                    server.username, server.host, server.port
                ))
                .style(style),
                Cell::from(group_name.to_string()).style(style.fg(Color::Gray)),
            ];

            rows.push(Row::new(cells));
        }
    }

    // If no servers, show empty message
    if rows.is_empty() {
        let empty_style = Style::default().fg(Color::Gray);
        rows.push(Row::new(vec![
            Cell::from(""),
            Cell::from("No servers configured").style(empty_style),
            Cell::from(""),
            Cell::from(""),
        ]));
    }

    // Create table
    let table = Table::new(
        rows,
        [
            Constraint::Length(4),  // Indicator + status
            Constraint::Length(20), // Name
            Constraint::Length(30), // Connection string
            Constraint::Length(15), // Group
        ],
    )
    .header(
        Row::new(vec!["", "Name", "Connection", "Group"])
            .style(Style::default().add_modifier(Modifier::BOLD)),
    )
    .block(block);

    frame.render_widget(table, area);
}

/// Convert server status to symbol
fn status_to_symbol(status: &str) -> &'static str {
    match status {
        "online" => "●",
        "offline" => "○",
        "error" => "✗",
        "connecting" => "⟳",
        _ => "?",
    }
}

/// Convert server status to color
fn status_to_color(status: &str) -> Color {
    match status {
        "online" => Color::Green,
        "offline" => Color::Gray,
        "error" => Color::Red,
        "connecting" => Color::Yellow,
        _ => Color::White,
    }
}
