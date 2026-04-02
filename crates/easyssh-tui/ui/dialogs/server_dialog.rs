//! Server Dialog
//!
//! Form dialog for creating or editing servers.
//! Fields:
//! - Name (required)
//! - Host (required)
//! - Port (default: 22)
//! - Username (required)
//! - Authentication method (agent/password/key)
//! - Group

use super::{Dialog, DialogResult, ServerData};
use crate::ui::dialogs::{handle_dialog_keys, handle_text_input};
use crossterm::event::{KeyCode, KeyEvent};
use easyssh_core::{AuthMethod, ServerRecord};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

/// Fields in the server dialog
#[derive(Debug, Clone, Copy, PartialEq)]
enum Field {
    Name,
    Host,
    Port,
    Username,
    AuthMethod,
    IdentityFile,
    Group,
}

impl Field {
    fn next(&self) -> Self {
        match self {
            Field::Name => Field::Host,
            Field::Host => Field::Port,
            Field::Port => Field::Username,
            Field::Username => Field::AuthMethod,
            Field::AuthMethod => Field::IdentityFile,
            Field::IdentityFile => Field::Group,
            Field::Group => Field::Name,
        }
    }

    fn prev(&self) -> Self {
        match self {
            Field::Name => Field::Group,
            Field::Host => Field::Name,
            Field::Port => Field::Host,
            Field::Username => Field::Port,
            Field::AuthMethod => Field::Username,
            Field::IdentityFile => Field::AuthMethod,
            Field::Group => Field::IdentityFile,
        }
    }
}

/// Server creation/editing dialog
pub struct ServerDialog {
    title: String,
    data: ServerData,
    focused_field: Field,
    groups: Vec<easyssh_core::GroupRecord>,
    cursor_name: usize,
    cursor_host: usize,
    cursor_port: usize,
    cursor_username: usize,
    cursor_identity: usize,
}

impl ServerDialog {
    pub fn new(
        title: String,
        server: Option<ServerRecord>,
        groups: Vec<easyssh_core::GroupRecord>,
    ) -> Self {
        let data = if let Some(server) = server {
            ServerData {
                id: server.id,
                name: server.name,
                host: server.host,
                port: server.port as u16,
                username: server.username,
                auth_method: easyssh_core::AuthMethod::from_db_string(
                    &server.auth_type,
                    server.identity_file.as_deref(),
                ),
                identity_file: server.identity_file,
                group_id: server.group_id,
            }
        } else {
            ServerData::default()
        };

        Self {
            title,
            data,
            focused_field: Field::Name,
            groups,
            cursor_name: 0,
            cursor_host: 0,
            cursor_port: 0,
            cursor_username: 0,
            cursor_identity: 0,
        }
    }

    fn is_identity_required(&self) -> bool {
        matches!(self.data.auth_method, AuthMethod::PrivateKey { .. })
    }
}

impl Dialog for ServerDialog {
    fn handle_key(&mut self, key: KeyEvent) -> DialogResult {
        // Handle dialog navigation
        match key.code {
            KeyCode::Tab => {
                self.focused_field = self.focused_field.next();
                return DialogResult::Continue;
            }
            KeyCode::BackTab => {
                self.focused_field = self.focused_field.prev();
                return DialogResult::Continue;
            }
            KeyCode::Esc => return DialogResult::Cancel,
            KeyCode::Enter => {
                if self.is_valid() {
                    return DialogResult::ServerData(self.data.clone());
                }
            }
            _ => {}
        }

        // Handle field-specific input
        match self.focused_field {
            Field::Name => {
                handle_text_input(key, &mut self.data.name, &mut self.cursor_name);
            }
            Field::Host => {
                handle_text_input(key, &mut self.data.host, &mut self.cursor_host);
            }
            Field::Port => {
                match key.code {
                    KeyCode::Char(c) if c.is_ascii_digit() => {
                        let port_str = self.data.port.to_string();
                        if port_str.len() < 5 {
                            self.data.port = (self.data.port * 10 + c.to_digit(10).unwrap() as u16)
                                .min(65535);
                        }
                    }
                    KeyCode::Backspace => {
                        self.data.port /= 10;
                    }
                    _ => {}
                }
            }
            Field::Username => {
                handle_text_input(key, &mut self.data.username, &mut self.cursor_username);
            }
            Field::AuthMethod => {
                match key.code {
                    KeyCode::Char('1') => {
                        self.data.auth_method = AuthMethod::Agent;
                        self.data.identity_file = None;
                    }
                    KeyCode::Char('2') => {
                        self.data.auth_method = AuthMethod::Password {
                            password: String::new(),
                        };
                        self.data.identity_file = None;
                    }
                    KeyCode::Char('3') => {
                        self.data.auth_method = AuthMethod::PrivateKey {
                            key_path: self.data.identity_file.clone().unwrap_or_default(),
                            passphrase: None,
                        };
                    }
                    _ => {}
                }
            }
            Field::IdentityFile => {
                let mut path = self.data.identity_file.clone().unwrap_or_default();
                handle_text_input(key, &mut path, &mut self.cursor_identity);
                self.data.identity_file = Some(path);

                // Update auth method if needed
                if let AuthMethod::PrivateKey { .. } = &self.data.auth_method {
                    self.data.auth_method = AuthMethod::PrivateKey {
                        key_path: self.data.identity_file.clone().unwrap_or_default(),
                        passphrase: None,
                    };
                }
            }
            Field::Group => {
                match key.code {
                    KeyCode::Char('0') => {
                        self.data.group_id = None;
                    }
                    KeyCode::Char(c) if c.is_ascii_digit() => {
                        let idx = (c.to_digit(10).unwrap() as usize).saturating_sub(1);
                        if let Some(group) = self.groups.get(idx) {
                            self.data.group_id = Some(group.id.clone());
                        }
                    }
                    _ => {}
                }
            }
        }

        DialogResult::Continue
    }

    fn render(&self, frame: &mut Frame, area: Rect) {
        frame.render_widget(Clear, area);

        let block = Block::default()
            .title(self.title.as_str())
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan));

        let inner = block.inner(area);
        frame.render_widget(block, area);

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(2)
            .constraints([
                Constraint::Length(3), // Name
                Constraint::Length(3), // Host
                Constraint::Length(3), // Port
                Constraint::Length(3), // Username
                Constraint::Length(5), // Auth method
                Constraint::Length(3), // Group
                Constraint::Length(2), // Help
            ])
            .split(inner);

        // Helper function to render field
        fn render_field(
            frame: &mut Frame,
            area: Rect,
            label: &str,
            value: &str,
            is_focused: bool,
            cursor_pos: usize,
        ) {
            let style = if is_focused {
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::White)
            } else {
                Style::default()
            };

            let text = if is_focused {
                Line::from(vec![
                    Span::styled(label, Style::default().add_modifier(Modifier::BOLD)),
                    Span::raw(" "),
                    Span::styled(value, style),
                ])
            } else {
                Line::from(vec![
                    Span::styled(label, Style::default().add_modifier(Modifier::BOLD)),
                    Span::raw(" "),
                    Span::raw(value),
                ])
            };

            let para = Paragraph::new(text);
            frame.render_widget(para, area);

            if is_focused {
                frame.set_cursor(
                    area.x + label.len() as u16 + 1 + cursor_pos as u16,
                    area.y,
                );
            }
        }

        // Render each field
        render_field(
            frame,
            chunks[0],
            "Name:",
            &self.data.name,
            self.focused_field == Field::Name,
            self.cursor_name,
        );

        render_field(
            frame,
            chunks[1],
            "Host:",
            &self.data.host,
            self.focused_field == Field::Host,
            self.cursor_host,
        );

        render_field(
            frame,
            chunks[2],
            "Port:",
            &self.data.port.to_string(),
            self.focused_field == Field::Port,
            self.cursor_port,
        );

        render_field(
            frame,
            chunks[3],
            "User:",
            &self.data.username,
            self.focused_field == Field::Username,
            self.cursor_username,
        );

        // Auth method field
        let auth_style = if self.focused_field == Field::AuthMethod {
            Style::default().fg(Color::Black).bg(Color::White)
        } else {
            Style::default()
        };

        let auth_text = format!(
            "Auth: [1] Agent {}  [2] Password {}  [3] Key {}",
            if matches!(self.data.auth_method, AuthMethod::Agent) {
                "(*)"
            } else {
                "( )"
            },
            if matches!(self.data.auth_method, AuthMethod::Password { .. }) {
                "(*)"
            } else {
                "( )"
            },
            if matches!(self.data.auth_method, AuthMethod::PrivateKey { .. }) {
                "(*)"
            } else {
                "( )"
            },
        );

        let auth_para = Paragraph::new(Line::from(vec![
            Span::styled("Auth:", Style::default().add_modifier(Modifier::BOLD)),
            Span::styled(auth_text.trim_start_matches("Auth:"), auth_style),
        ]));
        frame.render_widget(auth_para, chunks[4]);

        // Identity file (only if key auth selected)
        if self.is_identity_required() {
            render_field(
                frame,
                chunks[5],
                "Key:",
                self.data.identity_file.as_deref().unwrap_or(""),
                self.focused_field == Field::IdentityFile,
                self.cursor_identity,
            );
        }

        // Group selection
        let group_style = if self.focused_field == Field::Group {
            Style::default().fg(Color::Black).bg(Color::White)
        } else {
            Style::default()
        };

        let mut group_text = String::from("Group: [0] None ");
        for (idx, group) in self.groups.iter().enumerate() {
            let is_selected = self.data.group_id.as_ref() == Some(&group.id);
            group_text.push_str(&format!(
                "[{}] {}{} ",
                idx + 1,
                group.name,
                if is_selected { "(*)" } else { "( )" }
            ));
        }

        let group_para = Paragraph::new(Line::from(vec![
            Span::styled("Group:", Style::default().add_modifier(Modifier::BOLD)),
            Span::styled(group_text.trim_start_matches("Group:"), group_style),
        ]));
        frame.render_widget(group_para, chunks[5]);

        // Help text
        let help_para = Paragraph::new("Tab: Next field  |  Enter: Save  |  Esc: Cancel")
            .style(Style::default().fg(Color::Gray));
        frame.render_widget(help_para, chunks[6]);
    }

    fn is_valid(&self) -> bool {
        !self.data.name.trim().is_empty()
            && !self.data.host.trim().is_empty()
            && !self.data.username.trim().is_empty()
            && self.data.port > 0
            && self.data.port <= 65535
    }

    fn title(&self) -> &str {
        &self.title
    }
}
