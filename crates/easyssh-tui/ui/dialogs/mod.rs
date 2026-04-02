//! Dialogs Module
//!
//! Provides dialog components for:
//! - Server creation/editing
//! - Group management
//! - Confirmation dialogs
//! - Help display
//!
//! All dialogs support theme-aware rendering for consistent styling.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

pub mod confirm_dialog;
pub mod group_dialog;
pub mod help_dialog;
pub mod server_dialog;

pub use confirm_dialog::{ConfirmAction, ConfirmDialog};
pub use group_dialog::GroupDialog;
pub use help_dialog::HelpDialog;
pub use server_dialog::ServerDialog;

use ratatui::{layout::Rect, widgets::Clear, Frame};

/// Result of a dialog interaction
#[derive(Debug, Clone)]
pub enum DialogResult {
    /// Dialog should continue handling input
    Continue,
    /// Dialog confirmed with optional data
    Confirm(ConfirmAction),
    /// Dialog was cancelled
    Cancel,
    /// Server data for creation/update
    ServerData(ServerData),
    /// Group data for creation/update
    GroupData(GroupData),
}

/// Server data for create/update operations
#[derive(Debug, Clone)]
pub struct ServerData {
    pub id: String,
    pub name: String,
    pub host: String,
    pub port: u16,
    pub username: String,
    pub auth_method: easyssh_core::AuthMethod,
    pub identity_file: Option<String>,
    pub group_id: Option<String>,
}

impl Default for ServerData {
    fn default() -> Self {
        Self {
            id: String::new(),
            name: String::new(),
            host: String::new(),
            port: 22,
            username: String::new(),
            auth_method: easyssh_core::AuthMethod::Agent,
            identity_file: None,
            group_id: None,
        }
    }
}

/// Group data for create/update operations
#[derive(Debug, Clone)]
pub struct GroupData {
    pub id: String,
    pub name: String,
    pub color: String,
}

impl Default for GroupData {
    fn default() -> Self {
        Self {
            id: String::new(),
            name: String::new(),
            color: "#4A90D9".to_string(),
        }
    }
}

/// Trait for dialog components
pub trait Dialog: Send {
    /// Handle keyboard input
    fn handle_key(&mut self, key: KeyEvent) -> DialogResult;

    /// Render the dialog with theme support
    fn render(&self, frame: &mut Frame, area: Rect, theme: &crate::theme::ColorPalette);

    /// Check if dialog is complete (ready to submit)
    fn is_valid(&self) -> bool;

    /// Get dialog title
    fn title(&self) -> &str;
}

/// Common key handling for dialogs
pub fn handle_dialog_keys(
    key: KeyEvent,
    focused_field: &mut usize,
    field_count: usize,
) -> Option<DialogResult> {
    match key.code {
        KeyCode::Tab => {
            *focused_field = (*focused_field + 1) % field_count;
            Some(DialogResult::Continue)
        }
        KeyCode::BackTab => {
            if *focused_field == 0 {
                *focused_field = field_count - 1;
            } else {
                *focused_field -= 1;
            }
            Some(DialogResult::Continue)
        }
        KeyCode::Esc => Some(DialogResult::Cancel),
        _ => None,
    }
}

/// Handle text input for a field
pub fn handle_text_input(key: KeyEvent, content: &mut String, cursor: &mut usize) {
    match key.code {
        KeyCode::Char(c) => {
            content.insert(*cursor, c);
            *cursor += 1;
        }
        KeyCode::Backspace => {
            if *cursor > 0 {
                *cursor -= 1;
                content.remove(*cursor);
            }
        }
        KeyCode::Delete => {
            if *cursor < content.len() {
                content.remove(*cursor);
            }
        }
        KeyCode::Left => {
            if *cursor > 0 {
                *cursor -= 1;
            }
        }
        KeyCode::Right => {
            if *cursor < content.len() {
                *cursor += 1;
            }
        }
        KeyCode::Home => {
            *cursor = 0;
        }
        KeyCode::End => {
            *cursor = content.len();
        }
        KeyCode::Ctrl('a') => {
            *cursor = 0;
        }
        KeyCode::Ctrl('e') => {
            *cursor = content.len();
        }
        KeyCode::Ctrl('k') => {
            content.truncate(*cursor);
        }
        KeyCode::Ctrl('u') => {
            content.clear();
            *cursor = 0;
        }
        _ => {}
    }
}
