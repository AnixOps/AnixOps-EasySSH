//! Proxy Jump UI
//!
//! Configuration interface for SSH ProxyJump (bastion/jump host) chains.
//! Allows users to set up multi-hop SSH connections through intermediate servers.

use eframe::egui;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use uuid::Uuid;

/// Proxy jump configuration for a server
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProxyJumpConfig {
    pub id: String,
    pub name: String,
    pub target_server_id: String,
    pub target_server_name: String,
    pub jump_chain: Vec<JumpHop>,
    pub enabled: bool,
    pub created_at: chrono::DateTime<chrono::Local>,
    pub updated_at: chrono::DateTime<chrono::Local>,
}

impl ProxyJumpConfig {
    pub fn new(target_server_id: String, target_server_name: String) -> Self {
        let now = chrono::Local::now();
        Self {
            id: Uuid::new_v4().to_string(),
            name: format!("Proxy to {}", target_server_name),
            target_server_id,
            target_server_name,
            jump_chain: Vec::new(),
            enabled: true,
            created_at: now,
            updated_at: now,
        }
    }

    /// Build SSH ProxyJump string (e.g., "jump1,jump2,target")
    pub fn build_jump_string(&self) -> Option<String> {
        if self.jump_chain.is_empty() {
            return None;
        }

        let hops: Vec<String> = self
            .jump_chain
            .iter()
            .map(|hop| format!("{}@{}:{}", hop.username, hop.host, hop.port))
            .collect();

        Some(hops.join(","))
    }

    /// Validate the jump chain
    pub fn validate(&self) -> Vec<String> {
        let mut errors = Vec::new();

        if self.jump_chain.is_empty() {
            errors.push("Jump chain is empty".to_string());
        }

        for (i, hop) in self.jump_chain.iter().enumerate() {
            if hop.host.is_empty() {
                errors.push(format!("Hop {}: Host is required", i + 1));
            }
            if hop.username.is_empty() {
                errors.push(format!("Hop {}: Username is required", i + 1));
            }
            if hop.port == 0 || hop.port > 65535 {
                errors.push(format!("Hop {}: Invalid port number", i + 1));
            }
        }

        errors
    }

    pub fn update_timestamp(&mut self) {
        self.updated_at = chrono::Local::now();
    }
}

/// Individual jump hop (intermediate server)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct JumpHop {
    pub id: String,
    pub host: String,
    pub port: u16,
    pub username: String,
    pub auth_type: JumpAuthType,
    pub password: Option<String>,
    pub identity_file: Option<String>,
    pub name: String, // Display name
}

impl JumpHop {
    pub fn new(name: String, host: String, port: u16, username: String) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            host,
            port,
            username,
            auth_type: JumpAuthType::Password,
            password: None,
            identity_file: None,
            name,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum JumpAuthType {
    Password,
    Key,
    Agent,
}

impl Default for JumpAuthType {
    fn default() -> Self {
        JumpAuthType::Password
    }
}

impl std::fmt::Display for JumpAuthType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            JumpAuthType::Password => write!(f, "Password"),
            JumpAuthType::Key => write!(f, "SSH Key"),
            JumpAuthType::Agent => write!(f, "SSH Agent"),
        }
    }
}

/// Proxy Jump UI state
pub struct ProxyJumpUI {
    pub is_open: bool,
    pub configs: Vec<ProxyJumpConfig>,
    pub selected_config_id: Option<String>,
    pub servers_list: Vec<ServerInfo>, // Available servers
    pub search_query: String,
    pub show_add_dialog: bool,
    pub show_edit_hop_dialog: bool,
    pub editing_hop_index: Option<usize>,
    pub new_config_form: NewConfigForm,
    pub hop_form: HopForm,
    pub action_message: Option<(String, chrono::DateTime<chrono::Local>)>,
}

#[derive(Clone, Debug)]
pub struct ServerInfo {
    pub id: String,
    pub name: String,
    pub host: String,
    pub port: i64,
    pub username: String,
}

#[derive(Default)]
pub struct NewConfigForm {
    pub name: String,
    pub target_server_id: String,
}

#[derive(Default)]
pub struct HopForm {
    pub name: String,
    pub host: String,
    pub port: String,
    pub username: String,
    pub auth_type: JumpAuthType,
    pub password: String,
    pub identity_file: String,
}

impl Default for ProxyJumpUI {
    fn default() -> Self {
        Self {
            is_open: false,
            configs: Vec::new(),
            selected_config_id: None,
            servers_list: Vec::new(),
            search_query: String::new(),
            show_add_dialog: false,
            show_edit_hop_dialog: false,
            editing_hop_index: None,
            new_config_form: NewConfigForm::default(),
            hop_form: HopForm::default(),
            action_message: None,
        }
    }
}

impl ProxyJumpUI {
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

    /// Load available servers from view model
    pub fn load_servers(&mut self, servers: Vec<ServerInfo>) {
        self.servers_list = servers;
    }

    /// Add a new proxy jump configuration
    pub fn add_config(&mut self, config: ProxyJumpConfig) -> String {
        let id = config.id.clone();
        self.configs.push(config);
        id
    }

    /// Remove a configuration
    pub fn remove_config(&mut self, config_id: &str) -> Option<ProxyJumpConfig> {
        if let Some(index) = self.configs.iter().position(|c| c.id == config_id) {
            Some(self.configs.remove(index))
        } else {
            None
        }
    }

    /// Get configuration by ID
    pub fn get_config(&self, config_id: &str) -> Option<&ProxyJumpConfig> {
        self.configs.iter().find(|c| c.id == config_id)
    }

    /// Get mutable configuration by ID
    pub fn get_config_mut(&mut self, config_id: &str) -> Option<&mut ProxyJumpConfig> {
        self.configs.iter_mut().find(|c| c.id == config_id)
    }

    /// Get filtered configurations
    pub fn get_filtered_configs(&self) -> Vec<&ProxyJumpConfig> {
        if self.search_query.is_empty() {
            self.configs.iter().collect()
        } else {
            let query = self.search_query.to_lowercase();
            self.configs
                .iter()
                .filter(|c| {
                    c.name.to_lowercase().contains(&query)
                        || c.target_server_name.to_lowercase().contains(&query)
                        || c.jump_chain.iter().any(|h| {
                            h.name.to_lowercase().contains(&query)
                                || h.host.to_lowercase().contains(&query)
                        })
                })
                .collect()
        }
    }

    /// Start adding a new hop to selected config
    pub fn start_add_hop(&mut self) {
        self.editing_hop_index = None;
        self.hop_form = HopForm::default();
        self.hop_form.port = "22".to_string();
        self.show_edit_hop_dialog = true;
    }

    /// Start editing an existing hop
    pub fn start_edit_hop(&mut self, hop_index: usize) {
        if let Some(ref config_id) = self.selected_config_id {
            // First, collect the hop data we need
            let hop_data = self.get_config(config_id).and_then(|config| {
                config.jump_chain.get(hop_index).map(|hop| {
                    (hop.name.clone(), hop.host.clone(), hop.port, hop.username.clone(),
                     hop.auth_type.clone(), hop.password.clone(), hop.identity_file.clone())
                })
            });

            // Now update self with the hop data
            if let Some((name, host, port, username, auth_type, password, identity_file)) = hop_data {
                self.editing_hop_index = Some(hop_index);
                self.hop_form = HopForm {
                    name,
                    host,
                    port: port.to_string(),
                    username,
                    auth_type,
                    password: password.unwrap_or_default(),
                    identity_file: identity_file.unwrap_or_default(),
                };
                self.show_edit_hop_dialog = true;
            }
        }
    }

    /// Save hop from form
    pub fn save_hop(&mut self) -> Result<(), String> {
        let port: u16 = self
            .hop_form
            .port
            .parse()
            .map_err(|_| "Invalid port number".to_string())?;

        let hop = JumpHop {
            id: Uuid::new_v4().to_string(),
            name: self.hop_form.name.clone(),
            host: self.hop_form.host.clone(),
            port,
            username: self.hop_form.username.clone(),
            auth_type: self.hop_form.auth_type.clone(),
            password: if self.hop_form.password.is_empty() {
                None
            } else {
                Some(self.hop_form.password.clone())
            },
            identity_file: if self.hop_form.identity_file.is_empty() {
                None
            } else {
                Some(self.hop_form.identity_file.clone())
            },
        };

        let editing_index = self.editing_hop_index;
        let config_id = self.selected_config_id.clone()
            .ok_or_else(|| "No configuration selected".to_string())?;

        if let Some(config) = self.get_config_mut(&config_id) {
            match editing_index {
                Some(index) => {
                    if index < config.jump_chain.len() {
                        config.jump_chain[index] = hop;
                    }
                }
                None => {
                    config.jump_chain.push(hop);
                }
            }
            config.update_timestamp();
            self.show_message("Hop saved successfully".to_string());
            self.show_edit_hop_dialog = false;
            Ok(())
        } else {
            Err("Configuration not found".to_string())
        }
    }

    /// Remove a hop from the chain
    pub fn remove_hop(&mut self, hop_index: usize) {
        let config_id = self.selected_config_id.clone();
        if let Some(id) = config_id {
            if let Some(config) = self.get_config_mut(&id) {
                if hop_index < config.jump_chain.len() {
                    config.jump_chain.remove(hop_index);
                    config.update_timestamp();
                    self.show_message("Hop removed".to_string());
                }
            }
        }
    }

    /// Move hop up in the chain
    pub fn move_hop_up(&mut self, hop_index: usize) {
        if hop_index == 0 {
            return;
        }
        let config_id = self.selected_config_id.clone();
        if let Some(id) = config_id {
            if let Some(config) = self.get_config_mut(&id) {
                if hop_index < config.jump_chain.len() {
                    config.jump_chain.swap(hop_index, hop_index - 1);
                    config.update_timestamp();
                }
            }
        }
    }

    /// Move hop down in the chain
    pub fn move_hop_down(&mut self, hop_index: usize) {
        let config_id = self.selected_config_id.clone();
        if let Some(id) = config_id {
            if let Some(config) = self.get_config_mut(&id) {
                if hop_index + 1 < config.jump_chain.len() {
                    config.jump_chain.swap(hop_index, hop_index + 1);
                    config.update_timestamp();
                }
            }
        }
    }

    /// Show action message
    pub fn show_message(&mut self, message: String) {
        self.action_message = Some((message, chrono::Local::now()));
    }

    /// Clear expired message
    pub fn clear_expired_message(&mut self) {
        if let Some((_, timestamp)) = self.action_message {
            let elapsed = chrono::Local::now().signed_duration_since(timestamp);
            if elapsed.num_seconds() > 3 {
                self.action_message = None;
            }
        }
    }

    /// Export configurations to JSON
    pub fn export_configs(&self, path: &std::path::Path) -> Result<(), String> {
        match serde_json::to_string_pretty(&self.configs) {
            Ok(json) => match std::fs::write(path, json) {
                Ok(_) => Ok(()),
                Err(e) => Err(format!("Failed to write file: {}", e)),
            },
            Err(e) => Err(format!("Failed to serialize: {}", e)),
        }
    }

    /// Import configurations from JSON
    pub fn import_configs(&mut self, path: &std::path::Path) -> Result<usize, String> {
        match std::fs::read_to_string(path) {
            Ok(content) => match serde_json::from_str::<Vec<ProxyJumpConfig>>(&content) {
                Ok(imported) => {
                    let count = imported.len();
                    for mut config in imported {
                        config.id = Uuid::new_v4().to_string();
                        config.created_at = chrono::Local::now();
                        config.updated_at = chrono::Local::now();
                        self.configs.push(config);
                    }
                    Ok(count)
                }
                Err(e) => Err(format!("Failed to parse JSON: {}", e)),
            },
            Err(e) => Err(format!("Failed to read file: {}", e)),
        }
    }

    /// Render the Proxy Jump configuration window
    pub fn render(&mut self, ctx: &egui::Context) {
        if !self.is_open {
            return;
        }

        self.clear_expired_message();

        egui::Window::new("Proxy Jump Configuration")
            .collapsible(false)
            .resizable(true)
            .default_size([800.0, 600.0])
            .frame(egui::Frame {
                fill: egui::Color32::from_rgb(42, 48, 58),
                stroke: egui::Stroke::new(1.0, egui::Color32::from_rgb(60, 70, 85)),
                ..Default::default()
            })
            .show(ctx, |ui| {
                self.render_content(ui);
            });

        // Render dialogs
        if self.show_add_dialog {
            self.render_add_dialog(ctx);
        }
        if self.show_edit_hop_dialog {
            self.render_edit_hop_dialog(ctx);
        }
    }

    fn render_content(&mut self, ui: &mut egui::Ui) {
        // Header
        ui.horizontal(|ui| {
            ui.heading("Proxy Jump / Bastion Host");
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("✕ Close").clicked() {
                    self.close();
                }
                if ui.button("📥 Import").clicked() {
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("JSON", &["json"])
                        .pick_file()
                    {
                        match self.import_configs(&path) {
                            Ok(count) => self.show_message(format!("Imported {} configs", count)),
                            Err(e) => self.show_message(format!("Import failed: {}", e)),
                        }
                    }
                }
                if ui.button("📤 Export").clicked() {
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("JSON", &["json"])
                        .set_file_name("proxy-jump-configs.json")
                        .save_file()
                    {
                        match self.export_configs(&path) {
                            Ok(_) => self.show_message("Configs exported".to_string()),
                            Err(e) => self.show_message(format!("Export failed: {}", e)),
                        }
                    }
                }
                if ui.button("➕ New Chain").clicked() {
                    self.show_add_dialog = true;
                    self.new_config_form = NewConfigForm::default();
                }
            });
        });

        ui.add_space(10.0);

        // Info text
        ui.label(
            egui::RichText::new(
                "Configure SSH ProxyJump chains to connect through bastion/jump hosts. \
                 The connection will be routed through each hop in sequence.",
            )
            .size(12.0)
            .color(egui::Color32::from_rgb(150, 150, 150)),
        );

        ui.add_space(10.0);
        ui.separator();
        ui.add_space(10.0);

        // Two-column layout
        ui.horizontal(|ui| {
            // Left: Configuration list
            ui.vertical(|ui| {
                ui.set_width(300.0);
                self.render_config_list(ui);
            });

            ui.separator();

            // Right: Configuration details / hop chain
            ui.vertical(|ui| {
                ui.set_width(450.0);
                self.render_config_details(ui);
            });
        });

        // Status message
        if let Some((ref message, _)) = self.action_message {
            ui.add_space(10.0);
            ui.label(
                egui::RichText::new(message)
                    .color(egui::Color32::from_rgb(100, 200, 100))
                    .size(12.0),
            );
        }
    }

    fn render_config_list(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.add(
                egui::TextEdit::singleline(&mut self.search_query)
                    .hint_text("🔍 Search...")
                    .desired_width(200.0),
            );
        });

        ui.add_space(10.0);

        // Clone the IDs we need to render
        let config_ids: Vec<String> = if self.search_query.is_empty() {
            self.configs.iter().map(|c| c.id.clone()).collect()
        } else {
            let query = self.search_query.to_lowercase();
            self.configs
                .iter()
                .filter(|c| {
                    c.name.to_lowercase().contains(&query)
                        || c.target_server_name.to_lowercase().contains(&query)
                        || c.jump_chain.iter().any(|h| {
                            h.name.to_lowercase().contains(&query)
                                || h.host.to_lowercase().contains(&query)
                        })
                })
                .map(|c| c.id.clone())
                .collect()
        };

        if config_ids.is_empty() {
            ui.label("No proxy jump configurations.");
        } else {
            egui::ScrollArea::vertical().max_height(400.0).show(ui, |ui| {
                for id in config_ids {
                    if let Some(config) = self.get_config(&id).cloned() {
                        self.render_config_item(ui, &config);
                    }
                }
            });
        }
    }

    fn render_config_item(&mut self, ui: &mut egui::Ui, config: &ProxyJumpConfig) {
        let is_selected = self
            .selected_config_id
            .as_ref()
            .map(|id| id == &config.id)
            .unwrap_or(false);

        let status_color = if config.enabled {
            egui::Color32::from_rgb(72, 199, 116)
        } else {
            egui::Color32::from_rgb(150, 150, 150)
        };

        let frame = egui::Frame::group(ui.style())
            .inner_margin(8.0)
            .fill(if is_selected {
                egui::Color32::from_rgb(60, 70, 85)
            } else {
                egui::Color32::TRANSPARENT
            });

        frame.show(ui, |ui| {
            ui.set_min_width(ui.available_width());

            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new(if config.enabled { "●" } else { "○" })
                        .color(status_color)
                        .size(16.0),
                );

                ui.vertical(|ui| {
                    ui.label(
                        egui::RichText::new(&config.name)
                            .strong()
                            .size(14.0),
                    );

                    ui.label(
                        egui::RichText::new(format!(
                            "Target: {} | {} hop(s)",
                            config.target_server_name,
                            config.jump_chain.len()
                        ))
                        .size(11.0)
                        .color(egui::Color32::from_rgb(150, 150, 150)),
                    );
                });
            });
        });

        if ui.interact(ui.min_rect(), egui::Id::new(&config.id), egui::Sense::click()).clicked() {
            self.selected_config_id = Some(config.id.clone());
        }
    }

    fn render_config_details(&mut self, ui: &mut egui::Ui) {
        let selected_config = self.selected_config_id.clone();

        if let Some(ref config_id) = selected_config {
            if let Some(config) = self.get_config(config_id).cloned() {
                // Config header
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new(&config.name).strong().size(16.0));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let mut enabled = config.enabled;
                        if ui.checkbox(&mut enabled, "Enabled").clicked() {
                            if let Some(c) = self.get_config_mut(config_id) {
                                c.enabled = enabled;
                                c.update_timestamp();
                            }
                        }
                    });
                });

                ui.add_space(5.0);

                ui.label(
                    egui::RichText::new(format!("Target: {}", config.target_server_name))
                        .size(12.0)
                        .color(egui::Color32::from_rgb(150, 150, 150)),
                );

                // Validation
                let errors = config.validate();
                if !errors.is_empty() {
                    ui.add_space(10.0);
                    ui.group(|ui| {
                        ui.label(
                            egui::RichText::new("⚠ Validation Issues")
                                .color(egui::Color32::from_rgb(255, 193, 7)),
                        );
                        for error in &errors {
                            ui.label(
                                egui::RichText::new(format!("• {}", error))
                                    .size(11.0)
                                    .color(egui::Color32::from_rgb(255, 150, 100)),
                            );
                        }
                    });
                }

                // Jump chain visualization
                ui.add_space(15.0);
                ui.separator();
                ui.add_space(10.0);

                let target_server_name = config.target_server_name.clone();
                let has_hops = !config.jump_chain.is_empty();

                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("Jump Chain").strong().size(14.0));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("➕ Add Hop").clicked() {
                            self.start_add_hop();
                        }
                        if ui.button("🗑 Delete").clicked() {
                            if let Some(id) = self.selected_config_id.take() {
                                self.remove_config(&id);
                            }
                        }
                    });
                });

                ui.add_space(10.0);

                if !has_hops {
                    ui.label("No hops configured. Add at least one jump host.");
                } else {
                    // Visual chain representation
                    let chain_len = config.jump_chain.len();
                    ui.vertical(|ui| {
                        for (i, hop) in config.jump_chain.iter().enumerate() {
                            self.render_hop_node(ui, i, hop, chain_len);
                        }
                    });
                }

                // Generated jump string
                if let Some(jump_string) = config.build_jump_string() {
                    ui.add_space(15.0);
                    ui.separator();
                    ui.add_space(10.0);

                    ui.label(egui::RichText::new("Generated SSH Config").strong().size(12.0));
                    ui.add_space(5.0);

                    ui.group(|ui| {
                        ui.set_min_width(ui.available_width());
                        ui.monospace(format!("ProxyJump {}", jump_string));
                    });
                }
            } else {
                ui.label("Selected configuration not found.");
            }
        } else {
            ui.vertical_centered(|ui| {
                ui.add_space(100.0);
                ui.label("Select a configuration to view details");
            });
        }
    }

    fn render_hop_node(&mut self, ui: &mut egui::Ui, index: usize, hop: &JumpHop, total: usize) {
        let is_first = index == 0;
        let is_last = index == total - 1;

        ui.horizontal(|ui| {
            // Connection arrow (except for first)
            if !is_first {
                ui.label("↑");
            } else {
                ui.label(" ");
            }

            ui.vertical(|ui| {
                ui.group(|ui| {
                    ui.set_min_width(350.0);

                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new(format!("Hop {}: {}", index + 1, hop.name))
                                .strong()
                                .size(13.0),
                        );

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            // Actions
                            if ui.button("🗑").on_hover_text("Remove hop").clicked() {
                                self.remove_hop(index);
                            }
                            if ui.button("✎").on_hover_text("Edit hop").clicked() {
                                self.start_edit_hop(index);
                            }
                            if !is_first && ui.button("↑").on_hover_text("Move up").clicked() {
                                self.move_hop_up(index);
                            }
                            if !is_last && ui.button("↓").on_hover_text("Move down").clicked() {
                                self.move_hop_down(index);
                            }
                        });
                    });

                    ui.add_space(5.0);

                    ui.label(
                        egui::RichText::new(format!(
                            "{}@{}:{} | Auth: {}",
                            hop.username,
                            hop.host,
                            hop.port,
                            hop.auth_type
                        ))
                        .size(11.0)
                        .color(egui::Color32::from_rgb(150, 150, 150)),
                    );
                });
            });
        });

        ui.add_space(5.0);
    }

    fn render_add_dialog(&mut self, ctx: &egui::Context) {
        egui::Window::new("New Proxy Jump Configuration")
            .collapsible(false)
            .resizable(false)
            .default_size([400.0, 250.0])
            .show(ctx, |ui| {
                ui.label("Create a new proxy jump chain");
                ui.add_space(15.0);

                ui.horizontal(|ui| {
                    ui.label("Name:");
                    ui.add(
                        egui::TextEdit::singleline(&mut self.new_config_form.name)
                            .hint_text("My Proxy Chain")
                            .desired_width(250.0),
                    );
                });

                ui.add_space(10.0);

                ui.label("Target Server:");
                ui.add_space(5.0);

                // Server selection
                egui::ScrollArea::vertical().max_height(100.0).show(ui, |ui| {
                    for server in &self.servers_list {
                        let is_selected = self.new_config_form.target_server_id == server.id;
                        if ui
                            .selectable_label(is_selected, format!("{} ({}@{}", server.name, server.username, server.host))
                            .clicked()
                        {
                            self.new_config_form.target_server_id = server.id.clone();
                        }
                    }
                });

                ui.add_space(20.0);

                ui.horizontal(|ui| {
                    if ui.button("Cancel").clicked() {
                        self.show_add_dialog = false;
                    }

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let can_create = !self.new_config_form.name.is_empty()
                            && !self.new_config_form.target_server_id.is_empty();

                        if ui.add_enabled(can_create, egui::Button::new("Create")).clicked() {
                            if let Some(server) = self
                                .servers_list
                                .iter()
                                .find(|s| s.id == self.new_config_form.target_server_id)
                            {
                                let config = ProxyJumpConfig::new(
                                    server.id.clone(),
                                    server.name.clone(),
                                );
                                let id = self.add_config(config);
                                self.selected_config_id = Some(id);
                                self.show_add_dialog = false;
                                self.show_message("Configuration created".to_string());
                            }
                        }
                    });
                });
            });
    }

    fn render_edit_hop_dialog(&mut self, ctx: &egui::Context) {
        let title = if self.editing_hop_index.is_some() {
            "Edit Hop"
        } else {
            "Add New Hop"
        };

        egui::Window::new(title)
            .collapsible(false)
            .resizable(false)
            .default_size([400.0, 400.0])
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label("Name:");
                    ui.add(
                        egui::TextEdit::singleline(&mut self.hop_form.name)
                            .hint_text("Bastion 1")
                            .desired_width(300.0),
                    );
                });

                ui.add_space(10.0);

                ui.horizontal(|ui| {
                    ui.label("Host:");
                    ui.add(
                        egui::TextEdit::singleline(&mut self.hop_form.host)
                            .hint_text("jump.example.com")
                            .desired_width(300.0),
                    );
                });

                ui.add_space(10.0);

                ui.horizontal(|ui| {
                    ui.label("Port:");
                    ui.add(
                        egui::TextEdit::singleline(&mut self.hop_form.port)
                            .desired_width(80.0),
                    );
                });

                ui.add_space(10.0);

                ui.horizontal(|ui| {
                    ui.label("Username:");
                    ui.add(
                        egui::TextEdit::singleline(&mut self.hop_form.username)
                            .hint_text("admin")
                            .desired_width(200.0),
                    );
                });

                ui.add_space(10.0);

                ui.label("Authentication:");
                ui.add_space(5.0);

                ui.horizontal(|ui| {
                    ui.radio_value(&mut self.hop_form.auth_type, JumpAuthType::Password, "Password");
                    ui.radio_value(&mut self.hop_form.auth_type, JumpAuthType::Key, "SSH Key");
                    ui.radio_value(&mut self.hop_form.auth_type, JumpAuthType::Agent, "SSH Agent");
                });

                ui.add_space(10.0);

                match self.hop_form.auth_type {
                    JumpAuthType::Password => {
                        ui.horizontal(|ui| {
                            ui.label("Password:");
                            ui.add(
                                egui::TextEdit::singleline(&mut self.hop_form.password)
                                    .password(true)
                                    .desired_width(250.0),
                            );
                        });
                    }
                    JumpAuthType::Key => {
                        ui.horizontal(|ui| {
                            ui.label("Identity File:");
                            ui.add(
                                egui::TextEdit::singleline(&mut self.hop_form.identity_file)
                                    .hint_text("~/.ssh/id_rsa")
                                    .desired_width(250.0),
                            );
                            if ui.button("📁").clicked() {
                                if let Some(path) = rfd::FileDialog::new().pick_file() {
                                    self.hop_form.identity_file = path.to_string_lossy().to_string();
                                }
                            }
                        });
                    }
                    JumpAuthType::Agent => {
                        ui.label("Will use SSH agent for authentication");
                    }
                }

                ui.add_space(20.0);

                ui.horizontal(|ui| {
                    if ui.button("Cancel").clicked() {
                        self.show_edit_hop_dialog = false;
                    }

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let can_save = !self.hop_form.name.is_empty()
                            && !self.hop_form.host.is_empty()
                            && !self.hop_form.username.is_empty();

                        if ui.add_enabled(can_save, egui::Button::new("Save")).clicked() {
                            if let Err(e) = self.save_hop() {
                                self.show_message(format!("Error: {}", e));
                            }
                        }
                    });
                });
            });
    }
}
