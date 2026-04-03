//! Help Dialog
//!
//! Displays keyboard shortcuts and usage information.
//! Styled with theme support for consistent UI.

use super::{Dialog, DialogResult};
use crate::keybindings::KeyBindings;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Frame,
};

/// Help information dialog
pub struct HelpDialog;

impl HelpDialog {
    pub fn new() -> Self {
        Self
    }
}

impl Dialog for HelpDialog {
    fn handle_key(&mut self, key: KeyEvent) -> DialogResult {
        match key.code {
            KeyCode::Esc | KeyCode::Enter | KeyCode::Char('q') | KeyCode::Char('Q') => {
                DialogResult::Cancel
            }
            _ => DialogResult::Continue,
        }
    }

    fn render(&self, frame: &mut Frame, area: Rect, theme: &crate::theme::ColorPalette) {
        frame.render_widget(Clear, area);

        let block = Block::default()
            .title(" Keyboard Shortcuts ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.accent_success))
            .style(Style::default().bg(theme.bg_dialog));

        let inner = block.inner(area);
        frame.render_widget(block, area);

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(2)
            .constraints([
                Constraint::Length(3),
                Constraint::Min(0),
                Constraint::Length(1),
            ])
            .split(inner);

        // Header
        let header = Paragraph::new(vec![
            Line::from(vec![Span::styled(
                "EasySSH Lite TUI",
                Style::default()
                    .fg(theme.accent_primary)
                    .add_modifier(Modifier::BOLD),
            )]),
            Line::from(vec![
                Span::styled("Use ", Style::default().fg(theme.fg_secondary)),
                Span::styled(
                    "hjkl",
                    Style::default()
                        .fg(theme.accent_info)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(" or ", Style::default().fg(theme.fg_secondary)),
                Span::styled(
                    "arrows",
                    Style::default()
                        .fg(theme.accent_info)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(" for navigation", Style::default().fg(theme.fg_secondary)),
            ]),
        ])
        .alignment(Alignment::Center);
        frame.render_widget(header, chunks[0]);

        // Key bindings
        let help_entries = KeyBindings::get_help_entries();

        let mut lines: Vec<Line> = Vec::new();
        lines.push(Line::from(""));

        // Category styling function
        let category_header = |name: &str| -> Line {
            Line::from(vec![Span::styled(
                format!("  {}  ", name),
                Style::default()
                    .add_modifier(Modifier::BOLD)
                    .fg(theme.accent_secondary)
                    .bg(theme.bg_highlight),
            )])
        };

        // Navigation section
        lines.push(category_header("Navigation"));
        lines.push(Line::from(""));

        for (action, key, desc) in &help_entries {
            if matches!(
                action,
                crate::keybindings::Action::NavigateUp
                    | crate::keybindings::Action::NavigateDown
                    | crate::keybindings::Action::NavigateLeft
                    | crate::keybindings::Action::NavigateRight
                    | crate::keybindings::Action::GoToFirst
                    | crate::keybindings::Action::GoToLast
                    | crate::keybindings::Action::PageUp
                    | crate::keybindings::Action::PageDown
                    | crate::keybindings::Action::Select
                    | crate::keybindings::Action::Cancel
            ) {
                lines.push(Line::from(vec![
                    Span::styled(
                        format!("  {:<15}", key),
                        Style::default().fg(theme.accent_info),
                    ),
                    Span::raw(*desc),
                ]));
            }
        }

        lines.push(Line::from(""));
        lines.push(category_header("Server Operations"));
        lines.push(Line::from(""));

        for (action, key, desc) in &help_entries {
            if matches!(
                action,
                crate::keybindings::Action::NewServer
                    | crate::keybindings::Action::EditServer
                    | crate::keybindings::Action::DeleteServer
                    | crate::keybindings::Action::DuplicateServer
                    | crate::keybindings::Action::Connect
                    | crate::keybindings::Action::QuickConnect
                    | crate::keybindings::Action::Search
                    | crate::keybindings::Action::Refresh
            ) {
                lines.push(Line::from(vec![
                    Span::styled(
                        format!("  {:<15}", key),
                        Style::default().fg(theme.accent_info),
                    ),
                    Span::raw(*desc),
                ]));
            }
        }

        lines.push(Line::from(""));
        lines.push(category_header("Group Operations"));
        lines.push(Line::from(""));

        for (action, key, desc) in &help_entries {
            if matches!(
                action,
                crate::keybindings::Action::NewGroup
                    | crate::keybindings::Action::EditGroup
                    | crate::keybindings::Action::DeleteGroup
            ) {
                lines.push(Line::from(vec![
                    Span::styled(
                        format!("  {:<15}", key),
                        Style::default().fg(theme.accent_info),
                    ),
                    Span::raw(*desc),
                ]));
            }
        }

        lines.push(Line::from(""));
        lines.push(category_header("Other"));
        lines.push(Line::from(""));

        for (action, key, desc) in &help_entries {
            if matches!(
                action,
                crate::keybindings::Action::Help
                    | crate::keybindings::Action::ToggleTheme
                    | crate::keybindings::Action::Quit
            ) {
                lines.push(Line::from(vec![
                    Span::styled(
                        format!("  {:<15}", key),
                        Style::default().fg(theme.accent_info),
                    ),
                    Span::raw(*desc),
                ]));
            }
        }

        let content = Paragraph::new(lines).wrap(Wrap { trim: true });
        frame.render_widget(content, chunks[1]);

        // Footer
        let footer = Paragraph::new("Press Enter, Esc, or q to close this help")
            .style(Style::default().fg(theme.fg_muted))
            .alignment(Alignment::Center);
        frame.render_widget(footer, chunks[2]);
    }

    fn is_valid(&self) -> bool {
        true
    }

    fn title(&self) -> &str {
        "Help"
    }
}
