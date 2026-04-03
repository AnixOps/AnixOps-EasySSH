//! Detail Panel Component
//!
//! Renders the right panel with server details.
//! Shows:
//! - Server name and connection info
//! - Authentication method
//! - Current status
//! - Additional metadata
//! - Theme-aware styling

use crate::app::{App, Focus};
use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};

pub fn render(frame: &mut Frame, area: Rect, app: &App, theme: &crate::theme::ColorPalette) {
    // Determine border style based on focus
    let is_focused = app.focus == Focus::DetailPanel;
    let border_style = if is_focused {
        Style::default().fg(theme.border_focused)
    } else {
        Style::default().fg(theme.border_unfocused)
    };

    let block = Block::default()
        .title(" Details ")
        .borders(Borders::ALL)
        .border_style(border_style)
        .style(Style::default().bg(theme.bg_primary));

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

        let status_color = theme.server_status_color(&server.status);

        let status_symbol = match server.status.as_str() {
            "online" => "●",
            "offline" => "○",
            "error" => "✗",
            "connecting" => "⟳",
            _ => "?",
        };

        vec![
            Line::from(""),
            Line::from(vec![Span::styled(
                &server.name,
                Style::default()
                    .fg(theme.accent_primary)
                    .add_modifier(Modifier::BOLD)
                    .add_modifier(Modifier::UNDERLINED),
            )]),
            Line::from(""),
            Line::from(vec![
                Span::styled(
                    "Host: ",
                    Style::default()
                        .add_modifier(Modifier::BOLD)
                        .fg(theme.fg_secondary),
                ),
                Span::styled(&server.host, Style::default().fg(theme.fg_primary)),
            ]),
            Line::from(vec![
                Span::styled(
                    "Port: ",
                    Style::default()
                        .add_modifier(Modifier::BOLD)
                        .fg(theme.fg_secondary),
                ),
                Span::styled(
                    server.port.to_string(),
                    Style::default().fg(theme.fg_primary),
                ),
            ]),
            Line::from(vec![
                Span::styled(
                    "Username: ",
                    Style::default()
                        .add_modifier(Modifier::BOLD)
                        .fg(theme.fg_secondary),
                ),
                Span::styled(&server.username, Style::default().fg(theme.fg_primary)),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled(
                    "Auth: ",
                    Style::default()
                        .add_modifier(Modifier::BOLD)
                        .fg(theme.fg_secondary),
                ),
                Span::styled(auth_display, Style::default().fg(theme.accent_secondary)),
            ]),
            if let Some(ref identity) = server.identity_file {
                Line::from(vec![
                    Span::styled(
                        "Key: ",
                        Style::default()
                            .add_modifier(Modifier::BOLD)
                            .fg(theme.fg_secondary),
                    ),
                    Span::styled(identity, Style::default().fg(theme.fg_muted)),
                ])
            } else {
                Line::from("")
            },
            Line::from(""),
            Line::from(vec![
                Span::styled(
                    "Group: ",
                    Style::default()
                        .add_modifier(Modifier::BOLD)
                        .fg(theme.fg_secondary),
                ),
                Span::styled(group_name, Style::default().fg(theme.accent_info)),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled(
                    "Status: ",
                    Style::default()
                        .add_modifier(Modifier::BOLD)
                        .fg(theme.fg_secondary),
                ),
                Span::styled(
                    format!("{} {}", status_symbol, &server.status),
                    Style::default().fg(status_color),
                ),
            ]),
            Line::from(""),
            Line::from(vec![Span::styled(
                "Press Enter to connect  |  e to edit  |  d to delete",
                Style::default().fg(theme.fg_muted),
            )]),
            Line::from(""),
            Line::from(vec![
                Span::styled(
                    "ID: ",
                    Style::default()
                        .add_modifier(Modifier::BOLD)
                        .fg(theme.fg_muted),
                ),
                Span::styled(&server.id, Style::default().fg(theme.fg_muted)),
            ]),
        ]
    } else {
        vec![
            Line::from(""),
            Line::from(Span::styled(
                "No server selected",
                Style::default()
                    .fg(theme.fg_muted)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from(Span::styled(
                "Select a server from the list to view details",
                Style::default().fg(theme.fg_muted),
            )),
            Line::from(""),
            Line::from(vec![Span::styled(
                "Keyboard Shortcuts:",
                Style::default()
                    .add_modifier(Modifier::BOLD)
                    .fg(theme.fg_secondary),
            )]),
            Line::from(vec![
                Span::styled("  ↑/k  ", Style::default().fg(theme.accent_info)),
                Span::styled("Navigate up", Style::default().fg(theme.fg_primary)),
            ]),
            Line::from(vec![
                Span::styled("  ↓/j  ", Style::default().fg(theme.accent_info)),
                Span::styled("Navigate down", Style::default().fg(theme.fg_primary)),
            ]),
            Line::from(vec![
                Span::styled("  n    ", Style::default().fg(theme.accent_info)),
                Span::styled("Add new server", Style::default().fg(theme.fg_primary)),
            ]),
            Line::from(vec![
                Span::styled("  ?    ", Style::default().fg(theme.accent_info)),
                Span::styled("Show help", Style::default().fg(theme.fg_primary)),
            ]),
        ]
    };

    let paragraph = Paragraph::new(content)
        .block(block)
        .wrap(Wrap { trim: true });

    frame.render_widget(paragraph, area);
}
