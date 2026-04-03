//! Confirmation Dialog
//!
//! Simple yes/no confirmation dialog for destructive actions.
//! Styled with theme support.

use super::{Dialog, DialogResult};
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

/// Actions that can be confirmed
#[derive(Debug, Clone)]
pub enum ConfirmAction {
    /// Delete a server
    DeleteServer(String),
    /// Delete a group
    DeleteGroup(String),
}

/// Simple confirmation dialog
pub struct ConfirmDialog {
    title: String,
    message: String,
    action: ConfirmAction,
    confirmed: bool,
}

impl ConfirmDialog {
    pub fn new(title: String, message: String, action: ConfirmAction) -> Self {
        Self {
            title,
            message,
            action,
            confirmed: false,
        }
    }
}

impl Dialog for ConfirmDialog {
    fn handle_key(&mut self, key: KeyEvent) -> DialogResult {
        match key.code {
            KeyCode::Char('y') | KeyCode::Char('Y') | KeyCode::Enter => {
                DialogResult::Confirm(self.action.clone())
            }
            KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => DialogResult::Cancel,
            KeyCode::Left | KeyCode::Right | KeyCode::Tab => {
                self.confirmed = !self.confirmed;
                DialogResult::Continue
            }
            _ => DialogResult::Continue,
        }
    }

    fn render(&self, frame: &mut Frame, area: Rect, theme: &crate::theme::ColorPalette) {
        // Clear background
        frame.render_widget(Clear, area);

        let block = Block::default()
            .title(format!(" {} ", self.title))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.accent_error))
            .style(Style::default().bg(theme.bg_dialog));

        let inner = block.inner(area);

        frame.render_widget(block, area);

        // Layout for content
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(2)
            .constraints([
                Constraint::Length(3),
                Constraint::Length(2),
                Constraint::Length(1),
            ])
            .split(inner);

        // Message with warning styling
        let message_para = Paragraph::new(Line::from(vec![
            Span::styled(
                "⚠ ",
                Style::default()
                    .fg(theme.accent_warning)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(self.message.clone(), Style::default().fg(theme.fg_primary)),
        ]))
        .wrap(ratatui::widgets::Wrap { trim: true });
        frame.render_widget(message_para, chunks[0]);

        // Buttons with theme styling
        let yes_style = if self.confirmed {
            Style::default()
                .fg(theme.bg_primary)
                .bg(theme.accent_success)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(theme.accent_success)
        };

        let no_style = if !self.confirmed {
            Style::default()
                .fg(theme.bg_primary)
                .bg(theme.accent_error)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(theme.accent_error)
        };

        let buttons = Paragraph::new(Line::from(vec![
            Span::styled(" [Y]es ", yes_style),
            Span::raw("   "),
            Span::styled(" [N]o ", no_style),
        ]))
        .alignment(Alignment::Center);

        frame.render_widget(buttons, chunks[1]);

        // Warning hint
        let hint = Paragraph::new(Span::styled(
            "⚠ This action cannot be undone",
            Style::default().fg(theme.accent_warning),
        ))
        .alignment(Alignment::Center);
        frame.render_widget(hint, chunks[2]);
    }

    fn is_valid(&self) -> bool {
        true // Confirmation is always valid
    }

    fn title(&self) -> &str {
        &self.title
    }
}
