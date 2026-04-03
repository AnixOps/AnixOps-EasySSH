//! Application State Management
//!
//! This module manages the application state, including:
//! - Database connection
//! - Server and group lists
//! - UI state (selected items, focus, etc.)
//! - Dialog state

use crate::events::Key;
use crate::keybindings::{Action, KeyBindings};
use crate::theme::{ColorPalette, Theme};
use crate::ui::dialogs::{Dialog, DialogResult};
use crate::virtual_list::{
    render_virtual_group_list, render_virtual_server_list, GroupListItem, ServerListItem,
    VirtualListState,
};
use crossterm::event::{KeyCode, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
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
    /// Current theme
    pub theme: Theme,

    /// List of servers
    pub servers: Vec<ServerRecord>,
    /// List of groups
    pub groups: Vec<GroupRecord>,

    /// Virtual list state for server list
    pub server_list_state: VirtualListState,
    /// Currently selected server index in filtered list
    pub selected_server: usize,
    /// Currently selected group index (in sidebar, 0 = All)
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
    /// Clipboard content (internal)
    pub clipboard: Option<String>,
}

impl App {
    /// Create a new application instance
    pub fn new() -> AppResult<Self> {
        let state = AppState::new();
        let theme = Theme::default();

        Ok(Self {
            state,
            running: true,
            focus: Focus::ServerList,
            view_mode: ViewMode::Normal,
            input_mode: InputMode::Normal,
            key_bindings: KeyBindings::default(),
            theme,
            servers: Vec::new(),
            groups: Vec::new(),
            server_list_state: VirtualListState::new(0),
            selected_server: 0,
            selected_group: 0,
            filtered_servers: Vec::new(),
            search_query: String::new(),
            search_cursor: 0,
            dialog: None,
            status_message: String::new(),
            terminal_size: (80, 24),
            mouse_enabled: true,
            clipboard: None,
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

        // Update virtual list state
        self.server_list_state = VirtualListState::new(self.filtered_servers.len());
        self.server_list_state.selected = self.selected_server;

        // Ensure selection is valid
        self.selected_server = self
            .selected_server
            .min(self.filtered_servers.len().saturating_sub(1));
        self.selected_group = self.selected_group.min(self.groups.len());

        Ok(())
    }

    /// Get color palette from theme
    pub fn palette(&self) -> &ColorPalette {
        &self.theme.palette
    }

    /// Toggle between available themes
    pub fn toggle_theme(&mut self) {
        let themes = Theme::available_themes();
        let current_idx = themes
            .iter()
            .position(|&t| t == self.theme.name)
            .unwrap_or(0);
        let next_idx = (current_idx + 1) % themes.len();
        self.theme = Theme::new(themes[next_idx], self.theme.capability);
        self.set_status(format!("Theme: {}", self.theme.name));
    }

    /// Get clipboard content
    pub fn get_clipboard(&self) -> Option<&str> {
        self.clipboard.as_deref()
    }

    /// Set clipboard content
    pub fn set_clipboard(&mut self, content: String) {
        self.clipboard = Some(content);
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
            Some(Action::GoToFirst) => self.go_to_first(),
            Some(Action::GoToLast) => self.go_to_last(),
            Some(Action::PageUp) => self.page_up(),
            Some(Action::PageDown) => self.page_down(),
            Some(Action::Select) => self.select().await?,
            Some(Action::Search) => self.start_search(),
            Some(Action::NewServer) => self.new_server_dialog().await?,
            Some(Action::EditServer) => self.edit_server_dialog().await?,
            Some(Action::DeleteServer) => self.delete_server_confirm().await?,
            Some(Action::DuplicateServer) => self.duplicate_server().await?,
            Some(Action::NewGroup) => self.new_group_dialog().await?,
            Some(Action::EditGroup) => self.edit_group_dialog().await?,
            Some(Action::DeleteGroup) => self.delete_group_confirm().await?,
            Some(Action::Connect) => self.connect().await?,
            Some(Action::QuickConnect) => self.quick_connect().await?,
            Some(Action::Refresh) => {
                self.reload_data().await?;
                self.set_status("Data refreshed");
            }
            Some(Action::ToggleTheme) => self.toggle_theme(),
            Some(Action::Help) => self.show_help(),
            Some(Action::Copy) => self.copy_current(),
            Some(Action::Paste) => self.paste(),
            Some(Action::Cancel) | Some(Action::Back) => self.cancel(),
            None => {}
        }

        Ok(())
    }

    /// Handle mouse events with enhanced support
    pub async fn handle_mouse(&mut self, mouse: MouseEvent) -> AppResult<()> {
        let x = mouse.column;
        let y = mouse.row;

        // Calculate layout areas (similar to layout manager)
        let sidebar_width = (self.terminal_size.0 / 5).max(15).min(25);
        let detail_width = (self.terminal_size.0 / 4).max(20).min(35);
        let main_area_width = self
            .terminal_size
            .0
            .saturating_sub(sidebar_width + detail_width);

        match mouse.kind {
            MouseEventKind::Down(MouseButton::Left) => {
                // Check which area was clicked
                if x < sidebar_width && y > 0 {
                    // Sidebar click
                    self.focus = Focus::Sidebar;
                    let index = (y as usize).saturating_sub(2); // Adjust for header
                    let max_index = self.groups.len(); // +1 for "All"
                    if index <= max_index {
                        self.selected_group = index;
                        self.apply_group_filter();
                    }
                } else if x >= sidebar_width && x < sidebar_width + main_area_width && y > 0 {
                    // Server list click
                    self.focus = Focus::ServerList;
                    let index = (y as usize).saturating_sub(3); // Adjust for header + table header
                    if index < self.filtered_servers.len() {
                        self.selected_server = index;
                        self.server_list_state.selected = index;
                    }
                } else if x >= sidebar_width + main_area_width && y > 0 {
                    // Detail panel click
                    self.focus = Focus::DetailPanel;
                }
            }
            MouseEventKind::Down(MouseButton::Right) => {
                // Right click for context menu - could open quick actions
                if x >= sidebar_width && x < sidebar_width + main_area_width {
                    self.focus = Focus::ServerList;
                    // Future: Open context menu
                }
            }
            MouseEventKind::ScrollDown => match self.focus {
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
                        self.server_list_state.selected = self.selected_server;
                    }
                }
                _ => {}
            },
            MouseEventKind::ScrollUp => match self.focus {
                Focus::Sidebar => {
                    if self.selected_group > 0 {
                        self.selected_group -= 1;
                        self.apply_group_filter();
                    }
                }
                Focus::ServerList => {
                    if self.selected_server > 0 {
                        self.selected_server -= 1;
                        self.server_list_state.selected = self.selected_server;
                    }
                }
                _ => {}
            },
            _ => {}
        }

        Ok(())
    }

    /// Handle mouse double-click events
    pub async fn handle_mouse_double_click(
        &mut self,
        _x: u16,
        _y: u16,
        _button: crossterm::event::MouseButton,
    ) -> AppResult<()> {
        // Double-click on server list area connects to the selected server
        if self.focus == Focus::ServerList {
            self.connect().await?;
        }
        Ok(())
    }

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
                    self.server_list_state.navigate_up();
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
                    self.server_list_state.navigate_down();
                }
            }
            _ => {}
        }
    }

    fn go_to_first(&mut self) {
        match self.focus {
            Focus::Sidebar => {
                self.selected_group = 0;
                self.apply_group_filter();
            }
            Focus::ServerList => {
                self.selected_server = 0;
                self.server_list_state.go_to_first();
            }
            _ => {}
        }
    }

    fn go_to_last(&mut self) {
        match self.focus {
            Focus::Sidebar => {
                self.selected_group = self.groups.len();
            }
            Focus::ServerList => {
                self.selected_server = self.filtered_servers.len().saturating_sub(1);
                self.server_list_state.go_to_last();
            }
            _ => {}
        }
    }

    fn page_up(&mut self) {
        match self.focus {
            Focus::ServerList => {
                self.server_list_state.page_up();
                self.selected_server = self.server_list_state.selected;
            }
            _ => {}
        }
    }

    fn page_down(&mut self) {
        match self.focus {
            Focus::ServerList => {
                self.server_list_state.page_down();
                self.selected_server = self.server_list_state.selected;
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

    async fn quick_connect(&mut self) -> AppResult<()> {
        // Connect without changing focus - for power users
        self.connect().await
    }

    async fn duplicate_server(&mut self) -> AppResult<()> {
        if let Some(&index) = self.filtered_servers.get(self.selected_server) {
            if let Some(server) = self.servers.get(index).cloned() {
                let new_server = NewServer {
                    id: uuid::Uuid::new_v4().to_string(),
                    name: format!("{} (copy)", server.name),
                    host: server.host,
                    port: server.port,
                    username: server.username,
                    auth_type: server.auth_type.clone(),
                    identity_file: server.identity_file.clone(),
                    group_id: server.group_id.clone(),
                    status: "unknown".to_string(),
                };
                easyssh_core::add_server(&self.state, &new_server)?;
                self.set_status("Server duplicated");
                self.reload_data().await?;
            }
        }
        Ok(())
    }

    fn copy_current(&mut self) {
        if let Some(server) = self.get_selected_server() {
            let text = format!("{}@{}:{}", server.username, server.host, server.port);
            self.set_clipboard(text.clone());
            self.set_status(format!("Copied: {}", text));
        }
    }

    fn paste(&mut self) {
        // Paste functionality for search/filter
        if self.view_mode == ViewMode::Search {
            if let Some(clipboard) = &self.clipboard {
                self.search_query.push_str(clipboard);
                self.search_cursor = self.search_query.len();
                self.apply_search_filter();
                self.set_status("Pasted from clipboard");
            }
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
        self.set_status("Search: Type to filter servers (Esc to cancel, Enter to confirm)");
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
            KeyCode::Home => {
                self.search_cursor = 0;
            }
            KeyCode::End => {
                self.search_cursor = self.search_query.len();
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

        // Reset selection and virtual list state
        self.selected_server = 0;
        self.server_list_state = VirtualListState::new(self.filtered_servers.len());

        // Update status with results count
        self.status_message = format!("Found {} matches", self.filtered_servers.len());
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

        // Reset selection and virtual list state
        self.selected_server = 0;
        self.server_list_state = VirtualListState::new(self.filtered_servers.len());
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
                        color: "#4A90D9".to_string(),
                    };
                    easyssh_core::add_group(&self.state, &new_group)?;
                    self.set_status("Group added");
                } else {
                    // Update group
                    let update = easyssh_core::UpdateGroup {
                        id: data.id,
                        name: Some(data.name),
                        color: None,
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
