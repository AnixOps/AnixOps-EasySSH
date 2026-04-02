//! Confirmation Dialog
//!
//! Simple yes/no confirmation dialog for destructive actions.

use super::{Dialog, DialogResult};
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Margin, Rect},
    style::{Color, Modifier, Style},
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
            KeyCode::Left | KeyCode::Right => {
                self.confirmed = !self.confirmed;
                DialogResult::Continue
            }
            _ => DialogResult::Continue,
        }
    }

    fn render(&self, frame: &mut Frame, area: Rect) {
        // Clear background
        frame.render_widget(Clear, area);

        let block = Block::default()
            .title(self.title.as_str())
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Red));

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

        // Message
        let message_para = Paragraph::new(self.message.clone())
            .wrap(ratatui::widgets::Wrap { trim: true });
        frame.render_widget(message_para, chunks[0]);

        // Buttons
        let yes_style = if self.confirmed {
            Style::default()
                .fg(Color::Black)
                .bg(Color::White)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::White)
        };

        let no_style = if !self.confirmed {
            Style::default()
                .fg(Color::Black)
                .bg(Color::White)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::White)
        };

        let buttons = Paragraph::new(Line::from(vec![
            Span::styled(" [Y]es ", yes_style),
            Span::raw("   "),
            Span::styled(" [N]o ", no_style),
        ]))
        .alignment(Alignment::Center);

        frame.render_widget(buttons, chunks[1]);
    }

    fn is_valid(&self) -> bool {
        true // Confirmation is always valid
    }

    fn title(&self) -> &str {
        &self.title
    }
}
