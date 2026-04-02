//! Detail Panel Component
//!
//! Renders the right panel with server details.
//! Shows:
//! - Server name and connection info
//! - Authentication method
//! - Current status
//! - Additional metadata

use crate::app::{App, Focus};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};

pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    // Determine border style based on focus
    let is_focused = app.focus == Focus::DetailPanel;
    let border_style = if is_focused {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default().fg(Color::Gray)
    };

    let block = Block::default()
        .title(" Details ")
        .borders(Borders::ALL)
        .border_style(border_style);

    // Get selected server
    let content = if let Some(server) = app.get_selected_server() {
        let group_name = server
            .group_id
            .as_ref()
            .and_then(|gid| {
                app.groups
                    .iter()
                    .find(|g| &g.id == gid)
                    .map(|g| g.name.as_str())
            })
            .unwrap_or("Ungrouped");

        let auth_display = match server.auth_type.as_str() {
            "agent" => "SSH Agent",
            "password" => "Password",
            "key" => "Private Key",
            _ => &server.auth_type,
        };

        let status_color = match server.status.as_str() {
            "online" => Color::Green,
            "offline" => Color::Gray,
            "error" => Color::Red,
            "connecting" => Color::Yellow,
            _ => Color::White,
        };

        let status_symbol = match server.status.as_str() {
            "online" => "●",
            "offline" => "○",
            "error" => "✗",
            "connecting" => "⟳",
            _ => "?",
        };

        vec![
            Line::from(vec![
                Span::styled("Name: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(&server.name),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("Host: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(&server.host),
            ]),
            Line::from(vec![
                Span::styled("Port: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(server.port.to_string()),
            ]),
            Line::from(vec![
                Span::styled("Username: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(&server.username),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("Auth: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(auth_display),
            ]),
            if let Some(ref identity) = server.identity_file {
                Line::from(vec![
                    Span::styled("Key: ", Style::default().add_modifier(Modifier::BOLD)),
                    Span::raw(identity),
                ])
            } else {
                Line::from("")
            },
            Line::from(""),
            Line::from(vec![
                Span::styled("Group: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(group_name),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("Status: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::styled(
                    format!("{} {}", status_symbol, &server.status),
                    Style::default().fg(status_color),
                ),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("ID: ", Style::default().add_modifier(Modifier::BOLD).fg(Color::Gray)),
                Span::styled(&server.id, Style::default().fg(Color::Gray)),
            ]),
        ]
    } else {
        vec![
            Line::from(""),
            Line::from(
                Span::styled(
                    "No server selected",
                    Style::default().fg(Color::Gray),
                )
            ),
            Line::from(""),
            Line::from(
                Span::styled(
                    "Select a server from the list to view details",
                    Style::default().fg(Color::Gray),
                )
            ),
        ]
    };

    let paragraph = Paragraph::new(content)
        .block(block)
        .wrap(Wrap { trim: true });

    frame.render_widget(paragraph, area);
}
