//! Application State Management
//!
//! This module manages the application state, including:
//! - Database connection
//! - Server and group lists
//! - UI state (selected items, focus, etc.)
//! - Dialog state

use crate::events::Key;
use crate::keybindings::{Action, KeyBindings};
use crate::ui::dialogs::{Dialog, DialogResult};
use crossterm::event::{KeyCode, KeyModifiers, MouseEvent};
use easyssh_core::{
    connect_server, delete_group, delete_server, get_db_path, get_groups, get_servers,
    init_database, update_server, AppState, AuthMethod, GroupRecord, NewGroup, NewServer,
    ServerRecord, ServerStatus,
};
use std::io;

pub type AppResult<T> = Result<T, Box<dyn std::error::Error>>;

/// Focus area of the UI
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Focus {
    /// Sidebar with groups
    Sidebar,
    /// Server list
    ServerList,
    /// Detail panel
    DetailPanel,
    /// Dialog overlay
    Dialog,
}

/// Current view mode
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ViewMode {
    /// Normal browsing mode
    Normal,
    /// Search/filter mode
    Search,
    /// Dialog open
    DialogOpen,
}

/// Input mode for text entry
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum InputMode {
    /// Normal navigation
    Normal,
    /// Editing text
    Editing,
}

/// Application state
pub struct App {
    /// Core application state
    pub state: AppState,
    /// Whether the app is running
    pub running: bool,
    /// Current focus area
    pub focus: Focus,
    /// Current view mode
    pub view_mode: ViewMode,
    /// Input mode
    pub input_mode: InputMode,
    /// Key bindings
    pub key_bindings: KeyBindings,

    /// List of servers
    pub servers: Vec<ServerRecord>,
    /// List of groups
    pub groups: Vec<GroupRecord>,
    /// Currently selected server index
    pub selected_server: usize,
    /// Currently selected group index (in sidebar)
    pub selected_group: usize,
    /// Filtered server indices (for search)
    pub filtered_servers: Vec<usize>,

    /// Search query
    pub search_query: String,
    /// Search cursor position
    pub search_cursor: usize,

    /// Current dialog (if any)
    pub dialog: Option<Box<dyn Dialog>>,
    /// Status message
    pub status_message: String,
    /// Terminal size
    pub terminal_size: (u16, u16),

    /// Whether mouse support is enabled
    pub mouse_enabled: bool,
}

impl App {
    /// Create a new application instance
    pub fn new() -> AppResult<Self> {
        let state = AppState::new();

        Ok(Self {
            state,
            running: true,
            focus: Focus::ServerList,
            view_mode: ViewMode::Normal,
            input_mode: InputMode::Normal,
            key_bindings: KeyBindings::default(),
            servers: Vec::new(),
            groups: Vec::new(),
            selected_server: 0,
            selected_group: 0,
            filtered_servers: Vec::new(),
            search_query: String::new(),
            search_cursor: 0,
            dialog: None,
            status_message: String::new(),
            terminal_size: (80, 24),
            mouse_enabled: true,
        })
    }

    /// Initialize the application
    pub async fn init(&mut self) -> AppResult<()> {
        // Initialize database
        init_database(&self.state)?;

        // Load data
        self.reload_data().await?;

        // Enable mouse capture
        if self.mouse_enabled {
            crossterm::execute!(io::stdout(), crossterm::event::EnableMouseCapture)?;
        }

        // Enter alternate screen
        crossterm::execute!(io::stdout(), crossterm::terminal::EnterAlternateScreen)?;
        crossterm::terminal::enable_raw_mode()?;

        self.set_status("EasySSH Lite TUI - Press '?' for help");
        Ok(())
    }

    /// Clean up terminal state
    pub fn cleanup(&self) -> AppResult<()> {
        crossterm::terminal::disable_raw_mode()?;
        crossterm::execute!(
            io::stdout(),
            crossterm::terminal::LeaveAlternateScreen,
            crossterm::event::DisableMouseCapture
        )?;
        Ok(())
    }

    /// Reload all data from database
    pub async fn reload_data(&mut self) -> AppResult<()> {
        self.servers = get_servers(&self.state)?;
        self.groups = get_groups(&self.state)?;

        // Reset filtered list
        self.filtered_servers = (0..self.servers.len()).collect();

        // Ensure selection is valid
        self.selected_server = self
            .selected_server
            .min(self.filtered_servers.len().saturating_sub(1));
        self.selected_group = self.selected_group.min(self.groups.len().saturating_sub(1));

        Ok(())
    }

    /// Tick handler for periodic updates
    pub async fn tick(&mut self) -> AppResult<()> {
        // Periodic updates can go here
        Ok(())
    }

    /// Handle key events
    pub async fn handle_key(&mut self, key: Key) -> AppResult<()> {
        // Handle dialog input first if dialog is open
        if self.view_mode == ViewMode::DialogOpen {
            if let Some(dialog) = &mut self.dialog {
                match dialog.handle_key(key.clone()) {
                    DialogResult::Continue => return Ok(()),
                    DialogResult::Confirm(result) => {
                        self.handle_dialog_confirm(DialogResult::Confirm(result))
                            .await?;
                        return Ok(());
                    }
                    DialogResult::Cancel => {
                        self.close_dialog();
                        return Ok(());
                    }
                    DialogResult::ServerData(_) | DialogResult::GroupData(_) => {
                        // These should be handled by the specific dialog implementations
                        // For now, just close the dialog
                        self.close_dialog();
                        return Ok(());
                    }
                }
            }
        }

        // Handle search input
        if self.view_mode == ViewMode::Search {
            return self.handle_search_key(key).await;
        }

        // Handle normal navigation
        match self.key_bindings.get_action(key) {
            Some(Action::Quit) => self.quit(),
            Some(Action::NavigateUp) => self.navigate_up(),
            Some(Action::NavigateDown) => self.navigate_down(),
            Some(Action::NavigateLeft) => self.navigate_left(),
            Some(Action::NavigateRight) => self.navigate_right(),
            Some(Action::Select) => self.select().await?,
            Some(Action::Search) => self.start_search(),
            Some(Action::NewServer) => self.new_server_dialog().await?,
            Some(Action::EditServer) => self.edit_server_dialog().await?,
            Some(Action::DeleteServer) => self.delete_server_confirm().await?,
            Some(Action::NewGroup) => self.new_group_dialog().await?,
            Some(Action::EditGroup) => self.edit_group_dialog().await?,
            Some(Action::DeleteGroup) => self.delete_group_confirm().await?,
            Some(Action::Connect) => self.connect().await?,
            Some(Action::Help) => self.show_help(),
            Some(Action::Cancel) | Some(Action::Back) => self.cancel(),
            None => {}
        }

        Ok(())
    }

    /// Handle mouse events
    pub async fn handle_mouse(&mut self, mouse: MouseEvent) -> AppResult<()> {
        use crossterm::event::{MouseButton, MouseEventKind};

        match mouse.kind {
            MouseEventKind::Down(MouseButton::Left) => {
                // Check if click is in sidebar area (left 20 columns)
                let x = mouse.column;
                let y = mouse.row;

                // Simple hit testing based on layout
                if x < 20 && y > 1 {
                    // Sidebar click
                    self.focus = Focus::Sidebar;
                    let index = (y as usize).saturating_sub(3); // Adjust for header
                    if index < self.groups.len() + 1 {
                        // +1 for "All" group
                        self.selected_group = index;
                        self.apply_group_filter();
                    }
                } else if x >= 20 && x < self.terminal_size.0.saturating_sub(30) && y > 1 {
                    // Server list click
                    self.focus = Focus::ServerList;
                    let index = (y as usize).saturating_sub(3);
                    if index < self.filtered_servers.len() {
                        self.selected_server = index;
                    }
                }
            }
            MouseEventKind::ScrollDown => {
                self.navigate_down();
            }
            MouseEventKind::ScrollUp => {
                self.navigate_up();
            }
            _ => {}
        }

        Ok(())
    }

    /// Handle terminal resize
    pub async fn handle_resize(&mut self, width: u16, height: u16) -> AppResult<()> {
        self.terminal_size = (width, height);
        Ok(())
    }

    // Navigation methods

    fn navigate_up(&mut self) {
        match self.focus {
            Focus::Sidebar => {
                if self.selected_group > 0 {
                    self.selected_group -= 1;
                    self.apply_group_filter();
                }
            }
            Focus::ServerList => {
                if self.selected_server > 0 {
                    self.selected_server -= 1;
                }
            }
            _ => {}
        }
    }

    fn navigate_down(&mut self) {
        match self.focus {
            Focus::Sidebar => {
                let max = self.groups.len();
                if self.selected_group < max {
                    self.selected_group += 1;
                    self.apply_group_filter();
                }
            }
            Focus::ServerList => {
                let max = self.filtered_servers.len().saturating_sub(1);
                if self.selected_server < max {
                    self.selected_server += 1;
                }
            }
            _ => {}
        }
    }

    fn navigate_left(&mut self) {
        match self.focus {
            Focus::ServerList | Focus::DetailPanel => {
                self.focus = Focus::Sidebar;
            }
            _ => {}
        }
    }

    fn navigate_right(&mut self) {
        match self.focus {
            Focus::Sidebar => {
                self.focus = Focus::ServerList;
            }
            Focus::ServerList => {
                self.focus = Focus::DetailPanel;
            }
            _ => {}
        }
    }

    async fn select(&mut self) -> AppResult<()> {
        match self.focus {
            Focus::Sidebar => {
                self.focus = Focus::ServerList;
                self.apply_group_filter();
            }
            Focus::ServerList => {
                self.connect().await?;
            }
            _ => {}
        }
        Ok(())
    }

    fn cancel(&mut self) {
        match self.view_mode {
            ViewMode::Search => {
                self.view_mode = ViewMode::Normal;
                self.search_query.clear();
                self.search_cursor = 0;
                self.filtered_servers = (0..self.servers.len()).collect();
            }
            _ => {
                if self.focus == Focus::DetailPanel {
                    self.focus = Focus::ServerList;
                } else if self.focus == Focus::ServerList {
                    self.focus = Focus::Sidebar;
                }
            }
        }
    }

    fn quit(&mut self) {
        self.running = false;
    }

    // Search methods

    fn start_search(&mut self) {
        self.view_mode = ViewMode::Search;
        self.search_query.clear();
        self.search_cursor = 0;
        self.set_status("Search: Type to filter servers");
    }

    async fn handle_search_key(&mut self, key: Key) -> AppResult<()> {
        match key.code {
            KeyCode::Char(c) => {
                self.search_query.insert(self.search_cursor, c);
                self.search_cursor += 1;
                self.apply_search_filter();
            }
            KeyCode::Backspace => {
                if self.search_cursor > 0 {
                    self.search_cursor -= 1;
                    self.search_query.remove(self.search_cursor);
                    self.apply_search_filter();
                }
            }
            KeyCode::Left => {
                if self.search_cursor > 0 {
                    self.search_cursor -= 1;
                }
            }
            KeyCode::Right => {
                if self.search_cursor < self.search_query.len() {
                    self.search_cursor += 1;
                }
            }
            KeyCode::Esc => {
                self.cancel();
            }
            KeyCode::Enter => {
                self.view_mode = ViewMode::Normal;
                self.focus = Focus::ServerList;
            }
            _ => {}
        }

        Ok(())
    }

    fn apply_search_filter(&mut self) {
        let query = self.search_query.to_lowercase();
        self.filtered_servers = self
            .servers
            .iter()
            .enumerate()
            .filter(|(_, s)| {
                s.name.to_lowercase().contains(&query)
                    || s.host.to_lowercase().contains(&query)
                    || s.username.to_lowercase().contains(&query)
            })
            .map(|(i, _)| i)
            .collect();
        self.selected_server = 0;
    }

    fn apply_group_filter(&mut self) {
        if self.selected_group == 0 {
            // "All" selected
            self.filtered_servers = (0..self.servers.len()).collect();
        } else if let Some(group) = self.groups.get(self.selected_group - 1) {
            // Specific group selected
            self.filtered_servers = self
                .servers
                .iter()
                .enumerate()
                .filter(|(_, s)| {
                    s.group_id
                        .as_ref()
                        .map(|id| id == &group.id)
                        .unwrap_or(false)
                })
                .map(|(i, _)| i)
                .collect();
        }
        self.selected_server = 0;
    }

    // Action methods

    async fn connect(&mut self) -> AppResult<()> {
        if let Some(&index) = self.filtered_servers.get(self.selected_server) {
            if let Some(server) = self.servers.get(index) {
                let server_name = server.name.clone();
                let server_id = server.id.clone();
                self.set_status(format!("Connecting to {}...", server_name));

                // Restore terminal before connecting
                self.cleanup()?;

                // Attempt connection
                match connect_server(&self.state, &server_id) {
                    Ok(_) => {
                        println!("\nConnection closed.");
                    }
                    Err(e) => {
                        eprintln!("\nConnection failed: {}", e);
                    }
                }

                // Wait for user to press Enter
                println!("Press Enter to return to EasySSH...");
                let _ = std::io::stdin().read_line(&mut String::new());

                // Re-initialize terminal
                self.reinit_terminal()?;
                self.set_status("Returned from connection");
            }
        }
        Ok(())
    }

    fn reinit_terminal(&self) -> AppResult<()> {
        crossterm::execute!(io::stdout(), crossterm::terminal::EnterAlternateScreen)?;
        crossterm::terminal::enable_raw_mode()?;
        if self.mouse_enabled {
            crossterm::execute!(io::stdout(), crossterm::event::EnableMouseCapture)?;
        }
        Ok(())
    }

    // Dialog methods

    async fn new_server_dialog(&mut self) -> AppResult<()> {
        let dialog = crate::ui::dialogs::ServerDialog::new(
            "New Server".to_string(),
            None,
            self.groups.clone(),
        );
        self.dialog = Some(Box::new(dialog));
        self.view_mode = ViewMode::DialogOpen;
        Ok(())
    }

    async fn edit_server_dialog(&mut self) -> AppResult<()> {
        if let Some(&index) = self.filtered_servers.get(self.selected_server) {
            if let Some(server) = self.servers.get(index).cloned() {
                let dialog = crate::ui::dialogs::ServerDialog::new(
                    "Edit Server".to_string(),
                    Some(server),
                    self.groups.clone(),
                );
                self.dialog = Some(Box::new(dialog));
                self.view_mode = ViewMode::DialogOpen;
            }
        }
        Ok(())
    }

    async fn delete_server_confirm(&mut self) -> AppResult<()> {
        if let Some(&index) = self.filtered_servers.get(self.selected_server) {
            if let Some(server) = self.servers.get(index) {
                let dialog = crate::ui::dialogs::ConfirmDialog::new(
                    "Delete Server".to_string(),
                    format!("Are you sure you want to delete '{}'?", server.name),
                    crate::ui::dialogs::ConfirmAction::DeleteServer(server.id.clone()),
                );
                self.dialog = Some(Box::new(dialog));
                self.view_mode = ViewMode::DialogOpen;
            }
        }
        Ok(())
    }

    async fn new_group_dialog(&mut self) -> AppResult<()> {
        let dialog = crate::ui::dialogs::GroupDialog::new("New Group".to_string(), None);
        self.dialog = Some(Box::new(dialog));
        self.view_mode = ViewMode::DialogOpen;
        Ok(())
    }

    async fn edit_group_dialog(&mut self) -> AppResult<()> {
        if self.selected_group > 0 {
            if let Some(group) = self.groups.get(self.selected_group - 1).cloned() {
                let dialog =
                    crate::ui::dialogs::GroupDialog::new("Edit Group".to_string(), Some(group));
                self.dialog = Some(Box::new(dialog));
                self.view_mode = ViewMode::DialogOpen;
            }
        }
        Ok(())
    }

    async fn delete_group_confirm(&mut self) -> AppResult<()> {
        if self.selected_group > 0 {
            if let Some(group) = self.groups.get(self.selected_group - 1) {
                let dialog = crate::ui::dialogs::ConfirmDialog::new(
                    "Delete Group".to_string(),
                    format!("Are you sure you want to delete '{}'?", group.name),
                    crate::ui::dialogs::ConfirmAction::DeleteGroup(group.id.clone()),
                );
                self.dialog = Some(Box::new(dialog));
                self.view_mode = ViewMode::DialogOpen;
            }
        }
        Ok(())
    }

    async fn handle_dialog_confirm(&mut self, result: DialogResult) -> AppResult<()> {
        match result {
            DialogResult::Confirm(action) => match action {
                crate::ui::dialogs::ConfirmAction::DeleteServer(id) => {
                    delete_server(&self.state, &id)?;
                    self.set_status("Server deleted");
                    self.reload_data().await?;
                }
                crate::ui::dialogs::ConfirmAction::DeleteGroup(id) => {
                    delete_group(&self.state, &id)?;
                    self.set_status("Group deleted");
                    self.reload_data().await?;
                }
            },
            DialogResult::ServerData(data) => {
                if data.id.is_empty() {
                    // New server
                    let new_server = NewServer {
                        id: uuid::Uuid::new_v4().to_string(),
                        name: data.name,
                        host: data.host,
                        port: data.port as i64,
                        username: data.username,
                        auth_type: match data.auth_method {
                            AuthMethod::Agent => "agent".to_string(),
                            AuthMethod::Password { .. } => "password".to_string(),
                            AuthMethod::PrivateKey { .. } => "key".to_string(),
                        },
                        identity_file: data.identity_file,
                        group_id: data.group_id,
                        status: "unknown".to_string(),
                    };
                    easyssh_core::add_server(&self.state, &new_server)?;
                    self.set_status("Server added");
                } else {
                    // Update server
                    let update = easyssh_core::UpdateServer {
                        id: data.id,
                        name: Some(data.name),
                        host: Some(data.host),
                        port: Some(data.port as i64),
                        username: Some(data.username),
                        auth_type: Some(match data.auth_method {
                            AuthMethod::Agent => "agent".to_string(),
                            AuthMethod::Password { .. } => "password".to_string(),
                            AuthMethod::PrivateKey { .. } => "key".to_string(),
                        }),
                        identity_file: data.identity_file,
                        group_id: data.group_id,
                        status: None,
                    };
                    update_server(&self.state, &update)?;
                    self.set_status("Server updated");
                }
                self.reload_data().await?;
            }
            DialogResult::GroupData(data) => {
                if data.id.is_empty() {
                    // New group
                    let new_group = NewGroup {
                        id: uuid::Uuid::new_v4().to_string(),
                        name: data.name,
                    };
                    easyssh_core::add_group(&self.state, &new_group)?;
                    self.set_status("Group added");
                } else {
                    // Update group
                    let update = easyssh_core::UpdateGroup {
                        id: data.id,
                        name: Some(data.name),
                    };
                    easyssh_core::update_group(&self.state, &update)?;
                    self.set_status("Group updated");
                }
                self.reload_data().await?;
            }
            _ => {}
        }

        self.close_dialog();
        Ok(())
    }

    fn close_dialog(&mut self) {
        self.dialog = None;
        self.view_mode = ViewMode::Normal;
    }

    fn show_help(&mut self) {
        let dialog = crate::ui::dialogs::HelpDialog::new();
        self.dialog = Some(Box::new(dialog));
        self.view_mode = ViewMode::DialogOpen;
    }

    // Utility methods

    fn set_status<S: Into<String>>(&mut self, message: S) {
        self.status_message = message.into();
    }

    /// Get the currently selected server
    pub fn get_selected_server(&self) -> Option<&ServerRecord> {
        self.filtered_servers
            .get(self.selected_server)
            .and_then(|&index| self.servers.get(index))
    }

    /// Get the currently selected group
    pub fn get_selected_group(&self) -> Option<&GroupRecord> {
        if self.selected_group == 0 {
            None // "All" pseudo-group
        } else {
            self.groups.get(self.selected_group - 1)
        }
    }

    /// Get server count for a group
    pub fn get_group_server_count(&self, group_id: &str) -> usize {
        self.servers
            .iter()
            .filter(|s| {
                s.group_id
                    .as_ref()
                    .map(|id| id == group_id)
                    .unwrap_or(false)
            })
            .count()
    }

    /// Get total server count
    pub fn get_total_server_count(&self) -> usize {
        self.servers.len()
    }

    /// Get filtered server count
    pub fn get_filtered_server_count(&self) -> usize {
        self.filtered_servers.len()
    }
}
