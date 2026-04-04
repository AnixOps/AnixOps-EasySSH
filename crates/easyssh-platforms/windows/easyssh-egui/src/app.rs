//! EasySSH Application - Main egui App
//!
//! Implements the main application logic with:
//! - Terminal tabs management
//! - Sidebar connection list
//! - Search panel
//! - Clipboard integration

use std::collections::HashMap;

use egui::{Color32, Context, Id, Key};
use eframe::Storage;
use serde::{Deserialize, Serialize};
use tracing::{debug, info};

use crate::terminal::view::TerminalView;
use crate::platform::windows::WindowsPlatform;

/// Main EasySSH application
pub struct EasySSHApp {
    /// Active terminal views (key: connection_id-session_id)
    terminals: HashMap<String, TerminalView>,
    /// Currently active terminal ID
    active_terminal: Option<String>,
    /// Sidebar visibility
    sidebar_visible: bool,
    /// Search panel visibility
    search_visible: bool,
    /// Search query
    search_query: String,
    /// Use regex for search
    search_use_regex: bool,
    /// Connection list (placeholder)
    connections: Vec<ConnectionInfo>,
    /// Platform implementation
    platform: WindowsPlatform,
    /// App state for persistence
    state: AppState,
}

/// Connection info for sidebar display
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionInfo {
    /// Connection ID
    pub id: String,
    /// Connection name
    pub name: String,
    /// Host address
    pub host: String,
    /// Connection status
    pub status: ConnectionStatus,
}

/// Connection status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConnectionStatus {
    /// Not connected
    Disconnected,
    /// Connecting
    Connecting,
    /// Connected and active
    Connected,
    /// Connection failed
    Failed,
}

/// Persisted app state
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct AppState {
    /// Sidebar visible
    sidebar_visible: bool,
    /// Active connection ID
    active_connection_id: Option<String>,
    /// Recent connections
    recent_connections: Vec<String>,
}

impl EasySSHApp {
    /// Create new application
    pub fn new(cc: &eframe::CreationContext) -> Self {
        // Load persisted state
        let state: AppState = cc.storage
            .and_then(|s| eframe::get_value(s, "easyssh_state"))
            .unwrap_or_default();

        info!("Creating EasySSH App");

        Self {
            terminals: HashMap::new(),
            active_terminal: None,
            sidebar_visible: state.sidebar_visible,
            search_visible: false,
            search_query: String::new(),
            search_use_regex: false,
            connections: Self::create_demo_connections(),
            platform: WindowsPlatform::new(),
            state,
        }
    }

    /// Create demo connections for testing
    fn create_demo_connections() -> Vec<ConnectionInfo> {
        vec![
            ConnectionInfo {
                id: "demo-1".to_string(),
                name: "Local Server".to_string(),
                host: "localhost:22".to_string(),
                status: ConnectionStatus::Disconnected,
            },
            ConnectionInfo {
                id: "demo-2".to_string(),
                name: "Production Server".to_string(),
                host: "prod.example.com:22".to_string(),
                status: ConnectionStatus::Disconnected,
            },
            ConnectionInfo {
                id: "demo-3".to_string(),
                name: "Dev Server".to_string(),
                host: "dev.example.com:22".to_string(),
                status: ConnectionStatus::Disconnected,
            },
        ]
    }

    /// Create a new terminal for a connection
    pub fn create_terminal(&mut self, connection_id: &str) {
        let session_id = uuid::Uuid::new_v4().to_string();
        let terminal_key = format!("{}-{}", connection_id, session_id);

        let terminal = TerminalView::new(connection_id, &session_id);
        self.terminals.insert(terminal_key.clone(), terminal);
        self.active_terminal = Some(terminal_key.clone());

        info!("Created terminal: {}", terminal_key);
    }

    /// Close a terminal
    pub fn close_terminal(&mut self, terminal_id: &str) {
        if let Some(terminal) = self.terminals.remove(terminal_id) {
            // Drop terminal to clean up handles (SYSTEM_INVARIANTS.md)
            drop(terminal);
            debug!("Closed terminal: {}", terminal_id);
        }

        // Update active terminal
        if self.active_terminal.as_ref() == Some(&terminal_id.to_string()) {
            self.active_terminal = self.terminals.keys().next().cloned();
        }
    }

    /// Handle global keyboard shortcuts
    fn handle_global_shortcuts(&mut self, ctx: &Context) {
        ctx.input(|i| {
            // Ctrl+T - New terminal
            if i.key_pressed(Key::T) && i.modifiers.ctrl {
                // Create terminal for active connection or first connection
                let conn_id = self.connections.first()
                    .map(|c| c.id.clone())
                    .unwrap_or_else(|| "default".to_string());
                self.create_terminal(&conn_id);
            }

            // Ctrl+W - Close terminal
            if i.key_pressed(Key::W) && i.modifiers.ctrl {
                if let Some(active) = &self.active_terminal.clone() {
                    self.close_terminal(active);
                }
            }

            // Ctrl+B - Toggle sidebar
            if i.key_pressed(Key::B) && i.modifiers.ctrl {
                self.sidebar_visible = !self.sidebar_visible;
            }

            // Ctrl+F - Toggle search
            if i.key_pressed(Key::F) && i.modifiers.ctrl {
                self.search_visible = !self.search_visible;
                if self.search_visible {
                    // Focus search panel
                    ctx.memory_mut(|mem| mem.request_focus(Id::new("search_panel")));
                }
            }

            // Ctrl+Tab - Next terminal
            if i.key_pressed(Key::Tab) && i.modifiers.ctrl && !i.modifiers.shift {
                self.next_terminal();
            }

            // Ctrl+Shift+Tab - Previous terminal
            if i.key_pressed(Key::Tab) && i.modifiers.ctrl && i.modifiers.shift {
                self.prev_terminal();
            }
        });
    }

    /// Switch to next terminal
    fn next_terminal(&mut self) {
        if self.terminals.is_empty() {
            return;
        }

        let keys: Vec<String> = self.terminals.keys().cloned().collect();
        if let Some(active) = &self.active_terminal {
            let current_idx = keys.iter().position(|k| k == active).unwrap_or(0);
            let next_idx = (current_idx + 1) % keys.len();
            self.active_terminal = Some(keys[next_idx].clone());
        } else {
            self.active_terminal = keys.first().cloned();
        }
    }

    /// Switch to previous terminal
    fn prev_terminal(&mut self) {
        if self.terminals.is_empty() {
            return;
        }

        let keys: Vec<String> = self.terminals.keys().cloned().collect();
        if let Some(active) = &self.active_terminal {
            let current_idx = keys.iter().position(|k| k == active).unwrap_or(0);
            let prev_idx = if current_idx == 0 {
                keys.len() - 1
            } else {
                current_idx - 1
            };
            self.active_terminal = Some(keys[prev_idx].clone());
        } else {
            self.active_terminal = keys.first().cloned();
        }
    }

    /// Show sidebar panel
    fn show_sidebar(&mut self, ctx: &Context) {
        egui::SidePanel::left("sidebar")
            .default_width(200.0)
            .min_width(150.0)
            .max_width(300.0)
            .show(ctx, |ui| {
                ui.heading("Connections");
                ui.separator();

                // Connection list - collect actions first to avoid borrow conflicts
                let mut actions: Vec<(String, bool)> = Vec::new();
                for conn in &self.connections {
                    let is_active = self.active_terminal.as_ref()
                        .map(|t| t.starts_with(&conn.id))
                        .unwrap_or(false);

                    let response = ui.selectable_label(is_active, &conn.name);

                    if response.clicked() {
                        // Check if terminal exists for this connection
                        let existing = self.terminals.keys()
                            .find(|k| k.starts_with(&conn.id))
                            .cloned();
                        actions.push((conn.id.clone(), existing.is_some()));
                    }

                    // Show status indicator
                    let status_color = match conn.status {
                        ConnectionStatus::Connected => Color32::GREEN,
                        ConnectionStatus::Connecting => Color32::YELLOW,
                        ConnectionStatus::Failed => Color32::RED,
                        ConnectionStatus::Disconnected => Color32::GRAY,
                    };

                    ui.painter().circle_filled(
                        response.rect.right_top() - egui::vec2(10.0, -15.0),
                        5.0,
                        status_color,
                    );
                }

                // Execute actions after iteration
                for (conn_id, has_existing) in actions {
                    if has_existing {
                        // Switch to existing terminal
                        let existing = self.terminals.keys()
                            .find(|k| k.starts_with(&conn_id))
                            .cloned();
                        self.active_terminal = existing;
                    } else {
                        self.create_terminal(&conn_id);
                    }
                }

                ui.separator();

                // Terminal tabs - collect close actions first
                let mut terminal_to_close: Option<String> = None;
                let mut terminal_to_switch: Option<String> = None;
                for terminal_id in self.terminals.keys() {
                    let is_active = self.active_terminal.as_ref() == Some(terminal_id);

                    // Extract connection name for display
                    let display_name = terminal_id.split('-').next().unwrap_or("Terminal");

                    let response = ui.selectable_label(is_active, display_name);

                    if response.clicked() {
                        terminal_to_switch = Some(terminal_id.clone());
                    }

                    // Close button on hover
                    if response.hovered() {
                        let close_rect = egui::Rect::from_min_size(
                            response.rect.right_top() - egui::vec2(20.0, 0.0),
                            egui::vec2(20.0, response.rect.height()),
                        );

                        if ui.put(close_rect, egui::Button::new("X")).clicked() {
                            terminal_to_close = Some(terminal_id.clone());
                        }
                    }
                }

                // Execute actions after iteration
                if let Some(id) = terminal_to_switch {
                    self.active_terminal = Some(id);
                }
                if let Some(id) = terminal_to_close {
                    self.close_terminal(&id);
                }
            });
    }

    /// Show search panel
    fn show_search_panel(&mut self, ctx: &Context) {
        if !self.search_visible {
            return;
        }

        egui::TopBottomPanel::top("search_panel")
            .default_height(40.0)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label("Search:");

                    let response = ui.text_edit_singleline(&mut self.search_query);

                    if response.changed() {
                        // Update search in active terminal
                        if let Some(active) = &self.active_terminal {
                            if let Some(terminal) = self.terminals.get_mut(active) {
                                if !self.search_query.is_empty() {
                                    terminal.start_search(&self.search_query, self.search_use_regex);
                                } else {
                                    terminal.clear_search();
                                }
                            }
                        }
                    }

                    // Regex toggle
                    ui.checkbox(&mut self.search_use_regex, "Regex");

                    // Navigation buttons
                    if ui.button("Prev").clicked() {
                        if let Some(active) = &self.active_terminal {
                            if let Some(terminal) = self.terminals.get_mut(active) {
                                terminal.prev_search_result();
                            }
                        }
                    }

                    if ui.button("Next").clicked() {
                        if let Some(active) = &self.active_terminal {
                            if let Some(terminal) = self.terminals.get_mut(active) {
                                terminal.next_search_result();
                            }
                        }
                    }

                    // Close button
                    if ui.button("Close").clicked() {
                        self.search_visible = false;
                        if let Some(active) = &self.active_terminal {
                            if let Some(terminal) = self.terminals.get_mut(active) {
                                terminal.clear_search();
                            }
                        }
                    }
                });
            });
    }

    /// Show main terminal area
    fn show_terminal_area(&mut self, ctx: &Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            if self.terminals.is_empty() {
                // Show welcome screen
                ui.centered_and_justified(|ui| {
                    ui.heading("EasySSH Standard");
                    ui.label("Press Ctrl+T to create a new terminal");
                    ui.label("Or select a connection from the sidebar");

                    if ui.button("Create Terminal").clicked() {
                        let conn_id = self.connections.first()
                            .map(|c| c.id.clone())
                            .unwrap_or_else(|| "default".to_string());
                        self.create_terminal(&conn_id);
                    }
                });
            } else if let Some(active_id) = &self.active_terminal.clone() {
                // Show active terminal with Key-Driven Reset pattern
                // The ID includes connection_id-session_id per SYSTEM_INVARIANTS.md
                egui::ScrollArea::vertical().show(ui, |ui| {
                    // Use terminal ID as egui key for Key-Driven Reset
                    let terminal_key = Id::new(active_id);

                    ui.push_id(terminal_key, |ui| {
                        if let Some(terminal) = self.terminals.get_mut(active_id) {
                            terminal.show(ui);
                        }
                    });
                });
            }
        });
    }
}

impl eframe::App for EasySSHApp {
    fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        // Handle global shortcuts
        self.handle_global_shortcuts(ctx);

        // Show UI panels
        if self.sidebar_visible {
            self.show_sidebar(ctx);
        }

        self.show_search_panel(ctx);
        self.show_terminal_area(ctx);

        // Request continuous repaint for smooth terminal rendering
        ctx.request_repaint_after(std::time::Duration::from_millis(16));
    }

    fn save(&mut self, storage: &mut dyn Storage) {
        // Persist state
        self.state.sidebar_visible = self.sidebar_visible;
        self.state.active_connection_id = self.active_terminal.as_ref()
            .and_then(|t| t.split('-').next())
            .map(|s| s.to_string());

        eframe::set_value(storage, "easyssh_state", &self.state);
    }

    fn persist_egui_memory(&self) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper to create EasySSHApp for testing
    fn create_test_app() -> EasySSHApp {
        EasySSHApp {
            terminals: HashMap::new(),
            active_terminal: None,
            sidebar_visible: true,
            search_visible: false,
            search_query: String::new(),
            search_use_regex: false,
            connections: EasySSHApp::create_demo_connections(),
            platform: WindowsPlatform::new(),
            state: AppState::default(),
        }
    }

    #[test]
    fn test_app_creation() {
        let app = create_test_app();
        assert!(app.terminals.is_empty());
        assert!(app.sidebar_visible);
    }

    #[test]
    fn test_create_terminal() {
        let mut app = create_test_app();
        app.create_terminal("conn-123");

        assert_eq!(app.terminals.len(), 1);
        assert!(app.active_terminal.is_some());
    }

    #[test]
    fn test_close_terminal() {
        let mut app = create_test_app();
        app.create_terminal("conn-123");

        let terminal_id = app.active_terminal.clone().unwrap();
        app.close_terminal(&terminal_id);

        assert!(app.terminals.is_empty());
        assert!(app.active_terminal.is_none());
    }

    #[test]
    fn test_terminal_navigation() {
        let mut app = create_test_app();
        app.create_terminal("conn-1");
        app.create_terminal("conn-2");
        app.create_terminal("conn-3");

        // Should have 3 terminals
        assert_eq!(app.terminals.len(), 3);

        // Navigate
        app.next_terminal();
        assert!(app.active_terminal.is_some());

        app.prev_terminal();
        assert!(app.active_terminal.is_some());
    }

    #[test]
    fn test_key_format() {
        let mut app = create_test_app();
        app.create_terminal("my-connection");

        // Key should follow {connection_id}-{session_id} format
        for key in app.terminals.keys() {
            assert!(key.contains("-"));
            assert!(key.starts_with("my-connection"));
        }
    }

    #[test]
    fn test_persistence_state() {
        let mut state = AppState::default();
        state.sidebar_visible = false;
        state.active_connection_id = Some("conn-123".to_string());

        assert!(!state.sidebar_visible);
        assert_eq!(state.active_connection_id, Some("conn-123".to_string()));
    }
}