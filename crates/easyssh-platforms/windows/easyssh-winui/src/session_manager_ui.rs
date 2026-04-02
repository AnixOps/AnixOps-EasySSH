//! Session Manager UI
//!
//! Provides session management interface for viewing, saving, restoring,
//! and managing SSH sessions with full persistence support.

use eframe::egui;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use uuid::Uuid;

/// Session information for UI display
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SessionInfo {
    pub id: String,
    pub name: String,
    pub server_id: String,
    pub server_name: String,
    pub host: String,
    pub username: String,
    #[serde(skip)]
    pub created_at: chrono::DateTime<chrono::Local>,
    #[serde(skip)]
    pub last_accessed: chrono::DateTime<chrono::Local>,
    pub is_active: bool,
    pub tab_index: usize,
    pub output_buffer: String,
    pub current_path: String,
    pub environment_vars: HashMap<String, String>,
}

impl SessionInfo {
    pub fn new(server_id: String, server_name: String, host: String, username: String) -> Self {
        let now = chrono::Local::now();
        Self {
            id: Uuid::new_v4().to_string(),
            name: format!(
                "{}@{} {}",
                username,
                host,
                chrono::Local::now().format("%H:%M")
            ),
            server_id,
            server_name,
            host,
            username,
            created_at: now,
            last_accessed: now,
            is_active: true,
            tab_index: 0,
            output_buffer: String::new(),
            current_path: String::from("~"),
            environment_vars: HashMap::new(),
        }
    }

    pub fn duration_since_created(&self) -> Duration {
        let now = chrono::Local::now();
        let diff = now.signed_duration_since(self.created_at);
        Duration::from_secs(diff.num_seconds().max(0) as u64)
    }

    pub fn duration_since_accessed(&self) -> Duration {
        let now = chrono::Local::now();
        let diff = now.signed_duration_since(self.last_accessed);
        Duration::from_secs(diff.num_seconds().max(0) as u64)
    }

    pub fn formatted_duration(&self) -> String {
        let duration = self.duration_since_created();
        let hours = duration.as_secs() / 3600;
        let minutes = (duration.as_secs() % 3600) / 60;
        if hours > 0 {
            format!("{}h {}m", hours, minutes)
        } else {
            format!("{}m", minutes)
        }
    }
}

/// Session manager UI state
pub struct SessionManagerUI {
    pub is_open: bool,
    pub sessions: Vec<SessionInfo>,
    pub selected_session_id: Option<String>,
    pub search_query: String,
    pub show_save_dialog: bool,
    pub show_restore_dialog: bool,
    pub new_session_name: String,
    pub filter_active_only: bool,
    pub sort_by: SessionSortBy,
    pub auto_save_enabled: bool,
    pub auto_save_interval_minutes: u32,
    pub last_auto_save: Option<chrono::DateTime<chrono::Local>>,
    pub show_session_details: bool,
    pub export_path: Option<std::path::PathBuf>,
    pub import_path: Option<std::path::PathBuf>,
    pub action_message: Option<(String, chrono::DateTime<chrono::Local>)>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum SessionSortBy {
    LastAccessed,
    CreatedAt,
    Name,
    ServerName,
}

impl Default for SessionManagerUI {
    fn default() -> Self {
        Self {
            is_open: false,
            sessions: Vec::new(),
            selected_session_id: None,
            search_query: String::new(),
            show_save_dialog: false,
            show_restore_dialog: false,
            new_session_name: String::new(),
            filter_active_only: false,
            sort_by: SessionSortBy::LastAccessed,
            auto_save_enabled: true,
            auto_save_interval_minutes: 5,
            last_auto_save: None,
            show_session_details: false,
            export_path: None,
            import_path: None,
            action_message: None,
        }
    }
}

impl SessionManagerUI {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn open(&mut self) {
        self.is_open = true;
    }

    pub fn close(&mut self) {
        self.is_open = false;
    }

    pub fn toggle(&mut self) {
        self.is_open = !self.is_open;
    }

    /// Add a new session
    pub fn add_session(&mut self, mut session: SessionInfo) -> String {
        // Find next available tab index
        let max_tab = self.sessions.iter().map(|s| s.tab_index).max().unwrap_or(0);
        session.tab_index = max_tab + 1;

        let id = session.id.clone();
        self.sessions.push(session);
        self.sort_sessions();
        id
    }

    /// Remove a session by ID
    pub fn remove_session(&mut self, session_id: &str) -> Option<SessionInfo> {
        if let Some(index) = self.sessions.iter().position(|s| s.id == session_id) {
            let session = self.sessions.remove(index);
            // Renumber remaining tabs
            for (idx, s) in self.sessions.iter_mut().enumerate() {
                s.tab_index = idx + 1;
            }
            Some(session)
        } else {
            None
        }
    }

    /// Get a session by ID
    pub fn get_session(&self, session_id: &str) -> Option<&SessionInfo> {
        self.sessions.iter().find(|s| s.id == session_id)
    }

    /// Get mutable session by ID
    pub fn get_session_mut(&mut self, session_id: &str) -> Option<&mut SessionInfo> {
        self.sessions.iter_mut().find(|s| s.id == session_id)
    }

    /// Update session last accessed time
    pub fn touch_session(&mut self, session_id: &str) {
        if let Some(session) = self.get_session_mut(session_id) {
            session.last_accessed = chrono::Local::now();
        }
    }

    /// Mark session as active/inactive
    pub fn set_session_active(&mut self, session_id: &str, active: bool) {
        if let Some(session) = self.get_session_mut(session_id) {
            session.is_active = active;
            if active {
                session.last_accessed = chrono::Local::now();
            }
        }
    }

    /// Sort sessions based on current sort criteria
    fn sort_sessions(&mut self) {
        match self.sort_by {
            SessionSortBy::LastAccessed => {
                self.sessions
                    .sort_by(|a, b| b.last_accessed.cmp(&a.last_accessed));
            }
            SessionSortBy::CreatedAt => {
                self.sessions
                    .sort_by(|a, b| b.created_at.cmp(&a.created_at));
            }
            SessionSortBy::Name => {
                self.sessions.sort_by(|a, b| a.name.cmp(&b.name));
            }
            SessionSortBy::ServerName => {
                self.sessions
                    .sort_by(|a, b| a.server_name.cmp(&b.server_name));
            }
        }
    }

    /// Get filtered and sorted sessions
    pub fn get_filtered_sessions(&self) -> Vec<&SessionInfo> {
        let filtered: Vec<&SessionInfo> = self
            .sessions
            .iter()
            .filter(|s| {
                if self.filter_active_only && !s.is_active {
                    return false;
                }
                if !self.search_query.is_empty() {
                    let query = self.search_query.to_lowercase();
                    s.name.to_lowercase().contains(&query)
                        || s.server_name.to_lowercase().contains(&query)
                        || s.host.to_lowercase().contains(&query)
                        || s.username.to_lowercase().contains(&query)
                } else {
                    true
                }
            })
            .collect();
        filtered
    }

    /// Check if auto-save is due
    pub fn should_auto_save(&self) -> bool {
        if !self.auto_save_enabled {
            return false;
        }
        match self.last_auto_save {
            None => true,
            Some(last) => {
                let now = chrono::Local::now();
                let diff = now.signed_duration_since(last);
                diff.num_seconds() >= (self.auto_save_interval_minutes as i64 * 60)
            }
        }
    }

    /// Mark auto-save as completed
    pub fn mark_auto_saved(&mut self) {
        self.last_auto_save = Some(chrono::Local::now());
    }

    /// Export sessions to JSON
    pub fn export_sessions(&self, path: &std::path::Path) -> Result<(), String> {
        let export_data: Vec<SerializableSession> = self
            .sessions
            .iter()
            .map(|s| SerializableSession::from(s.clone()))
            .collect();

        match serde_json::to_string_pretty(&export_data) {
            Ok(json) => match std::fs::write(path, json) {
                Ok(_) => Ok(()),
                Err(e) => Err(format!("Failed to write file: {}", e)),
            },
            Err(e) => Err(format!("Failed to serialize: {}", e)),
        }
    }

    /// Import sessions from JSON
    pub fn import_sessions(&mut self, path: &std::path::Path) -> Result<usize, String> {
        match std::fs::read_to_string(path) {
            Ok(content) => match serde_json::from_str::<Vec<SerializableSession>>(&content) {
                Ok(imported) => {
                    let count = imported.len();
                    for session_data in imported {
                        let mut session: SessionInfo = session_data.into();
                        session.id = Uuid::new_v4().to_string(); // New ID
                        session.created_at = chrono::Local::now();
                        session.last_accessed = chrono::Local::now();
                        self.add_session(session);
                    }
                    Ok(count)
                }
                Err(e) => Err(format!("Failed to parse JSON: {}", e)),
            },
            Err(e) => Err(format!("Failed to read file: {}", e)),
        }
    }

    /// Show action message
    pub fn show_message(&mut self, message: String) {
        self.action_message = Some((message, chrono::Local::now()));
    }

    /// Clear expired message
    pub fn clear_expired_message(&mut self) {
        if let Some((_, timestamp)) = self.action_message {
            let now = chrono::Local::now();
            let diff = now.signed_duration_since(timestamp);
            if diff.num_seconds() > 3 {
                self.action_message = None;
            }
        }
    }

    /// Render the session manager window
    pub fn render(&mut self, ctx: &egui::Context) {
        if !self.is_open {
            return;
        }

        self.clear_expired_message();

        let action_msg = self.action_message.clone();

        egui::Window::new("Session Manager")
            .collapsible(false)
            .resizable(true)
            .default_size([800.0, 600.0])
            .frame(egui::Frame {
                fill: egui::Color32::from_rgb(42, 48, 58),
                stroke: egui::Stroke::new(1.0, egui::Color32::from_rgb(60, 70, 85)),
                ..Default::default()
            })
            .show(ctx, |ui| {
                self.render_content(ui, action_msg.as_ref());
            });

        // Render dialogs
        if self.show_save_dialog {
            self.render_save_dialog(ctx);
        }
        if self.show_restore_dialog {
            self.render_restore_dialog(ctx);
        }
    }

    fn render_content(
        &mut self,
        ui: &mut egui::Ui,
        action_message: Option<&(String, chrono::DateTime<chrono::Local>)>,
    ) {
        // Toolbar
        let mut should_close = false;
        let mut should_import = false;
        let mut should_export = false;
        ui.horizontal(|ui| {
            ui.heading("Sessions");
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("✕ Close").clicked() {
                    should_close = true;
                }
                if ui.button("📥 Import").clicked() {
                    should_import = true;
                }
                if ui.button("📤 Export").clicked() {
                    should_export = true;
                }
            });
        });

        if should_close {
            self.close();
            return;
        }

        if should_import {
            self.import_path = rfd::FileDialog::new()
                .add_filter("JSON", &["json"])
                .pick_file();
            if let Some(ref path) = self.import_path.clone() {
                match self.import_sessions(path) {
                    Ok(count) => self.show_message(format!("Imported {} sessions", count)),
                    Err(e) => self.show_message(format!("Import failed: {}", e)),
                }
            }
        }

        if should_export {
            self.export_path = rfd::FileDialog::new()
                .add_filter("JSON", &["json"])
                .set_file_name("sessions.json")
                .save_file();
            if let Some(ref path) = self.export_path.clone() {
                match self.export_sessions(path) {
                    Ok(_) => self.show_message("Sessions exported".to_string()),
                    Err(e) => self.show_message(format!("Export failed: {}", e)),
                }
            }
        }

        ui.add_space(10.0);

        // Filters and search
        let current_sort = self.sort_by.clone();
        let mut new_sort: Option<SessionSortBy> = None;
        ui.horizontal(|ui| {
            ui.add(
                egui::TextEdit::singleline(&mut self.search_query)
                    .hint_text("🔍 Search sessions...")
                    .desired_width(200.0),
            );

            ui.checkbox(&mut self.filter_active_only, "Active only");

            ui.label("Sort by:");
            egui::ComboBox::from_id_source("session_sort")
                .selected_text(format!("{:?}", current_sort))
                .width(120.0)
                .show_ui(ui, |ui| {
                    let options = [
                        SessionSortBy::LastAccessed,
                        SessionSortBy::CreatedAt,
                        SessionSortBy::Name,
                        SessionSortBy::ServerName,
                    ];
                    for option in options {
                        if ui
                            .selectable_label(current_sort == option, format!("{:?}", option))
                            .clicked()
                            && current_sort != option
                        {
                            new_sort = Some(option);
                        }
                    }
                });

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.checkbox(&mut self.auto_save_enabled, "Auto-save");
            });
        });

        if let Some(sort) = new_sort {
            self.sort_by = sort;
            self.sort_sessions();
        }

        ui.add_space(10.0);
        ui.separator();
        ui.add_space(10.0);

        // Session list
        let filtered: Vec<SessionInfo> =
            self.get_filtered_sessions().into_iter().cloned().collect();
        let total_count = self.sessions.len();
        let active_count = self.sessions.iter().filter(|s| s.is_active).count();

        ui.label(format!(
            "Showing {} of {} sessions ({} active)",
            filtered.len(),
            total_count,
            active_count
        ));
        ui.add_space(5.0);

        let selected_id = self.selected_session_id.clone();
        let mut new_selection: Option<String> = None;
        egui::ScrollArea::vertical()
            .max_height(400.0)
            .show(ui, |ui| {
                for session in &filtered {
                    Self::render_session_item(ui, session, &selected_id, &mut new_selection);
                }
            });

        if let Some(id) = new_selection {
            self.selected_session_id = Some(id);
        }

        ui.add_space(10.0);
        ui.separator();

        // Action buttons for selected session
        if let Some(ref selected_id) = self.selected_session_id.clone() {
            if let Some(session) = self.get_session(selected_id).cloned() {
                let session_id_clone = selected_id.clone();
                let mut should_delete = false;
                let mut should_save = false;
                let mut should_show_details = false;
                let mut should_disconnect = false;
                let mut should_reconnect = false;

                ui.horizontal(|ui| {
                    ui.label(format!("Selected: {}", session.name));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("🗑 Delete").clicked() {
                            should_delete = true;
                        }
                        if ui.button("💾 Save Snapshot").clicked() {
                            should_save = true;
                        }
                        if ui.button("📋 Details").clicked() {
                            should_show_details = true;
                        }
                        if session.is_active {
                            if ui.button("⏸ Disconnect").clicked() {
                                should_disconnect = true;
                            }
                        } else {
                            if ui.button("▶ Reconnect").clicked() {
                                should_reconnect = true;
                            }
                        }
                    });
                });

                // Handle actions outside the closure
                if should_delete {
                    self.remove_session(&session_id_clone);
                    self.selected_session_id = None;
                }
                if should_save {
                    self.show_save_dialog = true;
                }
                if should_show_details {
                    self.show_session_details = true;
                }
                if should_disconnect {
                    self.set_session_active(&session_id_clone, false);
                }
                if should_reconnect {
                    self.set_session_active(&session_id_clone, true);
                }
            }
        }

        // Status message
        if let Some((ref message, _)) = action_message {
            ui.add_space(10.0);
            ui.label(
                egui::RichText::new(message)
                    .color(egui::Color32::from_rgb(100, 200, 100))
                    .size(12.0),
            );
        }
    }

    fn render_session_item(
        ui: &mut egui::Ui,
        session: &SessionInfo,
        selected_session_id: &Option<String>,
        new_selection: &mut Option<String>,
    ) {
        let is_selected = selected_session_id
            .as_ref()
            .map(|id| id == &session.id)
            .unwrap_or(false);

        let status_color = if session.is_active {
            egui::Color32::from_rgb(72, 199, 116) // Green
        } else {
            egui::Color32::from_rgb(150, 150, 150) // Gray
        };

        let frame = egui::Frame::group(ui.style())
            .inner_margin(8.0)
            .fill(if is_selected {
                egui::Color32::from_rgb(60, 70, 85)
            } else {
                egui::Color32::TRANSPARENT
            });

        let response = frame.show(ui, |ui| {
            ui.set_min_width(ui.available_width());

            ui.horizontal(|ui| {
                // Status indicator
                ui.label(egui::RichText::new("●").color(status_color).size(16.0));

                ui.vertical(|ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new(&session.name).strong().size(14.0));
                        ui.label(
                            egui::RichText::new(format!("Tab #{}", session.tab_index))
                                .size(11.0)
                                .color(egui::Color32::from_rgb(150, 150, 150)),
                        );
                    });

                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new(format!(
                                "{}@{} ({})",
                                session.username, session.host, session.server_name
                            ))
                            .size(11.0)
                            .color(egui::Color32::from_rgb(150, 150, 150)),
                        );

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(
                                egui::RichText::new(session.formatted_duration())
                                    .size(11.0)
                                    .color(egui::Color32::from_rgb(150, 150, 150)),
                            );
                        });
                    });
                });
            });
        });

        // Click to select
        if ui
            .interact(
                response.response.rect,
                egui::Id::new(&session.id),
                egui::Sense::click(),
            )
            .clicked()
        {
            *new_selection = Some(session.id.clone());
        }
    }

    fn render_save_dialog(&mut self, ctx: &egui::Context) {
        let mut should_cancel = false;
        let mut should_save = false;

        egui::Window::new("Save Session Snapshot")
            .collapsible(false)
            .resizable(false)
            .default_size([400.0, 200.0])
            .show(ctx, |ui| {
                ui.label("Enter a name for this session snapshot:");
                ui.add_space(10.0);

                ui.add(
                    egui::TextEdit::singleline(&mut self.new_session_name)
                        .hint_text("Session name")
                        .desired_width(350.0),
                );

                ui.add_space(20.0);

                // Pre-collect the data we need
                let can_save = !self.new_session_name.is_empty();

                ui.horizontal(|ui| {
                    if ui.button("Cancel").clicked() {
                        should_cancel = true;
                    }

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui
                            .add_enabled(can_save, egui::Button::new("Save"))
                            .clicked()
                        {
                            should_save = true;
                        }
                    });
                });
            });

        // Handle actions outside the closure
        if should_cancel {
            self.show_save_dialog = false;
            self.new_session_name.clear();
        }
        if should_save {
            if let Some(ref id) = self.selected_session_id.clone() {
                let new_name = self.new_session_name.clone();
                if let Some(session) = self.get_session_mut(id) {
                    session.name = new_name;
                }
            }
            self.show_save_dialog = false;
            self.new_session_name.clear();
            self.show_message("Session saved".to_string());
        }
    }

    fn render_restore_dialog(&mut self, ctx: &egui::Context) {
        // Collect session IDs before rendering
        let saved_session_ids: Vec<(String, String)> = self
            .sessions
            .iter()
            .filter(|s| !s.is_active)
            .map(|s| (s.id.clone(), s.name.clone()))
            .collect();

        let mut should_close = false;
        let mut sessions_to_restore: Vec<(String, String)> = Vec::new();

        egui::Window::new("Restore Session")
            .collapsible(false)
            .resizable(false)
            .default_size([400.0, 300.0])
            .show(ctx, |ui| {
                ui.label("Select a saved session to restore:");
                ui.add_space(10.0);

                if saved_session_ids.is_empty() {
                    ui.label("No saved sessions available.");
                } else {
                    egui::ScrollArea::vertical()
                        .max_height(200.0)
                        .show(ui, |ui| {
                            for (id, name) in &saved_session_ids {
                                if ui.button(name).clicked() {
                                    sessions_to_restore.push((id.clone(), name.clone()));
                                }
                            }
                        });
                }

                ui.add_space(10.0);

                if ui.button("Close").clicked() {
                    should_close = true;
                }
            });

        // Handle actions outside the closure
        for (id, name) in sessions_to_restore {
            self.set_session_active(&id, true);
            self.show_restore_dialog = false;
            self.show_message(format!("Restored: {}", name));
        }
        if should_close {
            self.show_restore_dialog = false;
        }
    }
}

/// Serializable session for import/export
#[derive(Serialize, Deserialize)]
struct SerializableSession {
    pub name: String,
    pub server_id: String,
    pub server_name: String,
    pub host: String,
    pub username: String,
    pub tab_index: usize,
    pub current_path: String,
    pub environment_vars: HashMap<String, String>,
}

impl From<SessionInfo> for SerializableSession {
    fn from(s: SessionInfo) -> Self {
        Self {
            name: s.name,
            server_id: s.server_id,
            server_name: s.server_name,
            host: s.host,
            username: s.username,
            tab_index: s.tab_index,
            current_path: s.current_path,
            environment_vars: s.environment_vars,
        }
    }
}

impl From<SerializableSession> for SessionInfo {
    fn from(s: SerializableSession) -> Self {
        SessionInfo::new(s.server_id, s.server_name, s.host, s.username)
    }
}
