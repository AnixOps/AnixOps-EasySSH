//! Cloud Sync UI
//!
//! Provides configuration interface for cloud synchronization of SSH configurations.
//! Supports multiple cloud providers and manual/automatic sync options.

use eframe::egui;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Instant;
use uuid::Uuid;

/// Cloud provider types
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum CloudProviderType {
    Dropbox,
    GoogleDrive,
    OneDrive,
    WebDav,
    S3,
    Custom,
}

impl std::fmt::Display for CloudProviderType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CloudProviderType::Dropbox => write!(f, "Dropbox"),
            CloudProviderType::GoogleDrive => write!(f, "Google Drive"),
            CloudProviderType::OneDrive => write!(f, "OneDrive"),
            CloudProviderType::WebDav => write!(f, "WebDAV"),
            CloudProviderType::S3 => write!(f, "Amazon S3"),
            CloudProviderType::Custom => write!(f, "Custom"),
        }
    }
}

/// Cloud sync configuration
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CloudSyncConfig {
    pub id: String,
    pub provider: CloudProviderType,
    pub name: String,
    pub enabled: bool,
    pub auto_sync: bool,
    pub sync_interval_minutes: u32,
    pub api_token: String,
    pub refresh_token: String,
    pub folder_path: String,
    pub last_sync: Option<chrono::DateTime<chrono::Local>>,
    pub last_sync_status: SyncStatus,
    pub sync_stats: SyncStatistics,
    // Provider-specific settings
    pub custom_settings: HashMap<String, String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum SyncStatus {
    Never,
    Pending,
    InProgress,
    Success,
    Failed(String),
    Conflict,
}

impl Default for SyncStatus {
    fn default() -> Self {
        SyncStatus::Never
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct SyncStatistics {
    pub servers_synced: u32,
    pub groups_synced: u32,
    pub keys_synced: u32,
    pub snippets_synced: u32,
    pub total_syncs: u32,
    pub failed_syncs: u32,
}

impl CloudSyncConfig {
    pub fn new(provider: CloudProviderType, name: String) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            provider,
            name,
            enabled: false,
            auto_sync: false,
            sync_interval_minutes: 30,
            api_token: String::new(),
            refresh_token: String::new(),
            folder_path: "/EasySSH".to_string(),
            last_sync: None,
            last_sync_status: SyncStatus::Never,
            sync_stats: SyncStatistics::default(),
            custom_settings: HashMap::new(),
        }
    }

    pub fn formatted_last_sync(&self) -> String {
        match self.last_sync {
            Some(time) => time.format("%Y-%m-%d %H:%M").to_string(),
            None => "Never".to_string(),
        }
    }

    pub fn status_color(&self) -> egui::Color32 {
        match self.last_sync_status {
            SyncStatus::Success => egui::Color32::from_rgb(72, 199, 116),
            SyncStatus::Failed(_) => egui::Color32::from_rgb(255, 100, 100),
            SyncStatus::InProgress => egui::Color32::from_rgb(100, 180, 255),
            SyncStatus::Pending => egui::Color32::from_rgb(255, 193, 7),
            SyncStatus::Conflict => egui::Color32::from_rgb(255, 150, 50),
            SyncStatus::Never => egui::Color32::from_rgb(150, 150, 150),
        }
    }
}

/// Cloud Sync UI state
pub struct CloudSyncUI {
    pub is_open: bool,
    pub configs: Vec<CloudSyncConfig>,
    pub selected_config_id: Option<String>,
    pub show_add_dialog: bool,
    pub show_provider_setup: bool,
    pub show_sync_log: bool,
    pub new_provider: CloudProviderType,
    pub new_config_name: String,
    pub action_message: Option<(String, Instant)>,
    pub sync_log: Vec<SyncLogEntry>,
    // Sync options
    pub sync_servers: bool,
    pub sync_groups: bool,
    pub sync_keys: bool,
    pub sync_snippets: bool,
    pub encrypt_sync: bool,
    pub master_password: String,
}

#[derive(Clone, Debug)]
pub struct SyncLogEntry {
    pub timestamp: chrono::DateTime<chrono::Local>,
    pub message: String,
    pub level: LogLevel,
}

#[derive(Clone, Debug, PartialEq)]
pub enum LogLevel {
    Info,
    Warning,
    Error,
    Success,
}

impl Default for CloudSyncUI {
    fn default() -> Self {
        Self {
            is_open: false,
            configs: Vec::new(),
            selected_config_id: None,
            show_add_dialog: false,
            show_provider_setup: false,
            show_sync_log: false,
            new_provider: CloudProviderType::Dropbox,
            new_config_name: String::new(),
            action_message: None,
            sync_log: Vec::new(),
            sync_servers: true,
            sync_groups: true,
            sync_keys: true,
            sync_snippets: true,
            encrypt_sync: true,
            master_password: String::new(),
        }
    }
}

impl CloudSyncUI {
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

    /// Add a new cloud sync configuration
    pub fn add_config(&mut self, config: CloudSyncConfig) -> String {
        let id = config.id.clone();
        self.configs.push(config);
        id
    }

    /// Remove a configuration
    pub fn remove_config(&mut self, config_id: &str) -> Option<CloudSyncConfig> {
        if let Some(index) = self.configs.iter().position(|c| c.id == config_id) {
            Some(self.configs.remove(index))
        } else {
            None
        }
    }

    /// Get configuration by ID
    pub fn get_config(&self, config_id: &str) -> Option<&CloudSyncConfig> {
        self.configs.iter().find(|c| c.id == config_id)
    }

    /// Get mutable configuration by ID
    pub fn get_config_mut(&mut self, config_id: &str) -> Option<&mut CloudSyncConfig> {
        self.configs.iter_mut().find(|c| c.id == config_id)
    }

    /// Start sync for a configuration
    pub fn start_sync(&mut self, config_id: &str) {
        if let Some(config) = self.get_config_mut(config_id) {
            config.last_sync_status = SyncStatus::InProgress;
            self.log_info(format!("Starting sync for '{}'...", config.name));
        }
    }

    /// Complete sync successfully
    pub fn complete_sync(&mut self, config_id: &str, stats: SyncStatistics) {
        if let Some(config) = self.get_config_mut(config_id) {
            config.last_sync_status = SyncStatus::Success;
            config.last_sync = Some(chrono::Local::now());
            config.sync_stats = stats.clone();
            config.sync_stats.total_syncs += 1;

            self.log_success(format!(
                "Sync completed: {} servers, {} groups, {} keys, {} snippets",
                stats.servers_synced,
                stats.groups_synced,
                stats.keys_synced,
                stats.snippets_synced
            ));
        }
    }

    /// Fail sync with error
    pub fn fail_sync(&mut self, config_id: &str, error: String) {
        if let Some(config) = self.get_config_mut(config_id) {
            config.last_sync_status = SyncStatus::Failed(error.clone());
            config.sync_stats.failed_syncs += 1;
            self.log_error(format!("Sync failed for '{}': {}", config.name, error));
        }
    }

    /// Add log entry
    fn add_log_entry(&mut self, message: String, level: LogLevel) {
        self.sync_log.push(SyncLogEntry {
            timestamp: chrono::Local::now(),
            message,
            level,
        });
        // Keep only last 100 entries
        if self.sync_log.len() > 100 {
            self.sync_log.remove(0);
        }
    }

    pub fn log_info(&mut self, message: String) {
        self.add_log_entry(message, LogLevel::Info);
    }

    pub fn log_warning(&mut self, message: String) {
        self.add_log_entry(message, LogLevel::Warning);
    }

    pub fn log_error(&mut self, message: String) {
        self.add_log_entry(message, LogLevel::Error);
    }

    pub fn log_success(&mut self, message: String) {
        self.add_log_entry(message, LogLevel::Success);
    }

    /// Show action message
    pub fn show_message(&mut self, message: String) {
        self.action_message = Some((message, Instant::now()));
    }

    /// Clear expired message
    pub fn clear_expired_message(&mut self) {
        if let Some((_, timestamp)) = self.action_message {
            if timestamp.elapsed().as_secs() > 3 {
                self.action_message = None;
            }
        }
    }

    /// Get default folder path for provider
    pub fn default_folder_for_provider(provider: &CloudProviderType) -> String {
        match provider {
            CloudProviderType::Dropbox => "/Apps/EasySSH".to_string(),
            CloudProviderType::GoogleDrive => "/EasySSH".to_string(),
            CloudProviderType::OneDrive => "/EasySSH".to_string(),
            _ => "/easyssh".to_string(),
        }
    }

    /// Export configurations
    pub fn export_configs(&self, path: &std::path::Path) -> Result<(), String> {
        match serde_json::to_string_pretty(&self.configs) {
            Ok(json) => match std::fs::write(path, json) {
                Ok(_) => Ok(()),
                Err(e) => Err(format!("Failed to write file: {}", e)),
            },
            Err(e) => Err(format!("Failed to serialize: {}", e)),
        }
    }

    /// Import configurations
    pub fn import_configs(&mut self, path: &std::path::Path) -> Result<usize, String> {
        match std::fs::read_to_string(path) {
            Ok(content) => match serde_json::from_str::<Vec<CloudSyncConfig>>(&content) {
                Ok(imported) => {
                    let count = imported.len();
                    for mut config in imported {
                        config.id = Uuid::new_v4().to_string();
                        // Clear sensitive data
                        config.api_token.clear();
                        config.refresh_token.clear();
                        self.configs.push(config);
                    }
                    Ok(count)
                }
                Err(e) => Err(format!("Failed to parse JSON: {}", e)),
            },
            Err(e) => Err(format!("Failed to read file: {}", e)),
        }
    }

    /// Render the cloud sync configuration window
    pub fn render(&mut self, ctx: &egui::Context) {
        if !self.is_open {
            return;
        }

        self.clear_expired_message();

        egui::Window::new("Cloud Synchronization")
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
        if self.show_sync_log {
            self.render_sync_log(ctx);
        }
    }

    fn render_content(&mut self, ui: &mut egui::Ui) {
        // Header
        ui.horizontal(|ui| {
            ui.heading("Cloud Sync");
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("✕ Close").clicked() {
                    self.close();
                }
                if ui.button("📋 Sync Log").clicked() {
                    self.show_sync_log = true;
                }
                if ui.button("➕ Add Provider").clicked() {
                    self.show_add_dialog = true;
                    self.new_config_name.clear();
                }
            });
        });

        ui.add_space(10.0);

        // Info text
        ui.label(
            egui::RichText::new(
                "Synchronize your SSH configurations across multiple devices using cloud storage providers.",
            )
            .size(12.0)
            .color(egui::Color32::from_rgb(150, 150, 150)),
        );

        ui.add_space(10.0);
        ui.separator();
        ui.add_space(10.0);

        // Two-column layout
        ui.horizontal(|ui| {
            // Left: Provider list
            ui.vertical(|ui| {
                ui.set_width(300.0);
                self.render_provider_list(ui);
            });

            ui.separator();

            // Right: Configuration details
            ui.vertical(|ui| {
                ui.set_width(450.0);
                self.render_provider_details(ui);
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

    fn render_provider_list(&mut self, ui: &mut egui::Ui) {
        ui.label(egui::RichText::new("Configured Providers").strong().size(14.0));
        ui.add_space(10.0);

        if self.configs.is_empty() {
            ui.label("No cloud providers configured.");
            ui.add_space(10.0);
            if ui.button("➕ Add your first provider").clicked() {
                self.show_add_dialog = true;
            }
        } else {
            egui::ScrollArea::vertical().max_height(400.0).show(ui, |ui| {
                for config in self.configs.clone() {
                    self.render_provider_item(ui, &config);
                }
            });
        }
    }

    fn render_provider_item(&mut self, ui: &mut egui::Ui, config: &CloudSyncConfig) {
        let is_selected = self
            .selected_config_id
            .as_ref()
            .map(|id| id == &config.id)
            .unwrap_or(false);

        let status_icon = match config.last_sync_status {
            SyncStatus::Success => "✓",
            SyncStatus::Failed(_) => "✗",
            SyncStatus::InProgress => "⟳",
            SyncStatus::Pending => "⏳",
            SyncStatus::Conflict => "⚠",
            SyncStatus::Never => "○",
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
                // Status indicator
                ui.label(
                    egui::RichText::new(status_icon)
                        .color(config.status_color())
                        .size(16.0),
                );

                ui.vertical(|ui| {
                    ui.label(
                        egui::RichText::new(&config.name)
                            .strong()
                            .size(14.0),
                    );

                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new(config.provider.to_string())
                                .size(11.0)
                                .color(egui::Color32::from_rgb(150, 150, 150)),
                        );

                        if config.enabled {
                            ui.label(
                                egui::RichText::new("● Active")
                                    .size(10.0)
                                    .color(egui::Color32::from_rgb(72, 199, 116)),
                            );
                        }

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(
                                egui::RichText::new(config.formatted_last_sync())
                                    .size(10.0)
                                    .color(egui::Color32::from_rgb(150, 150, 150)),
                            );
                        });
                    });
                });
            });
        });

        if ui.interact(ui.min_rect(), egui::Id::new(&config.id), egui::Sense::click()).clicked() {
            self.selected_config_id = Some(config.id.clone());
        }
    }

    fn render_provider_details(&mut self, ui: &mut egui::Ui) {
        if let Some(ref config_id) = self.selected_config_id {
            if let Some(config) = self.get_config(config_id).cloned() {
                // Header
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new(&config.name).strong().size(16.0));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let mut enabled = config.enabled;
                        if ui.checkbox(&mut enabled, "Enabled").clicked() {
                            if let Some(c) = self.get_config_mut(config_id) {
                                c.enabled = enabled;
                            }
                        }
                    });
                });

                ui.label(
                    egui::RichText::new(format!("Provider: {}", config.provider))
                        .size(12.0)
                        .color(egui::Color32::from_rgb(150, 150, 150)),
                );

                ui.add_space(15.0);

                // Connection settings
                ui.group(|ui| {
                    ui.label(egui::RichText::new("Connection Settings").strong());
                    ui.add_space(10.0);

                    match config.provider {
                        CloudProviderType::Dropbox |
                        CloudProviderType::GoogleDrive |
                        CloudProviderType::OneDrive => {
                            if config.api_token.is_empty() {
                                ui.horizontal(|ui| {
                                    ui.label("Not connected");
                                    if ui.button("🔗 Connect").clicked() {
                                        self.log_info(format!("Initiating OAuth flow for {:?}", config.provider));
                                        // Would trigger OAuth flow
                                    }
                                });
                            } else {
                                ui.horizontal(|ui| {
                                    ui.label("✓ Connected");
                                    if ui.button("🔌 Disconnect").clicked() {
                                        if let Some(c) = self.get_config_mut(config_id) {
                                            c.api_token.clear();
                                            c.refresh_token.clear();
                                        }
                                        self.show_message("Disconnected".to_string());
                                    }
                                });
                            }
                        }
                        _ => {
                            // For other providers, show API key field
                            ui.horizontal(|ui| {
                                ui.label("API Key/Token:");
                                let mut token = config.api_token.clone();
                                if ui
                                    .add(
                                        egui::TextEdit::singleline(&mut token)
                                            .password(true)
                                            .desired_width(250.0),
                                    )
                                    .changed()
                                {
                                    if let Some(c) = self.get_config_mut(config_id) {
                                        c.api_token = token;
                                    }
                                }
                            });
                        }
                    }

                    ui.add_space(10.0);

                    // Folder path
                    ui.horizontal(|ui| {
                        ui.label("Sync Folder:");
                        let mut path = config.folder_path.clone();
                        if ui
                            .add(egui::TextEdit::singleline(&mut path).desired_width(250.0))
                            .changed()
                        {
                            if let Some(c) = self.get_config_mut(config_id) {
                                c.folder_path = path;
                            }
                        }
                    });
                });

                ui.add_space(15.0);

                // Sync options
                ui.group(|ui| {
                    ui.label(egui::RichText::new("Sync Options").strong());
                    ui.add_space(10.0);

                    let mut auto_sync = config.auto_sync;
                    if ui.checkbox(&mut auto_sync, "Enable automatic sync").changed() {
                        if let Some(c) = self.get_config_mut(config_id) {
                            c.auto_sync = auto_sync;
                        }
                    }

                    if auto_sync {
                        ui.horizontal(|ui| {
                            ui.label("Interval:");
                            let mut interval = config.sync_interval_minutes;
                            if ui
                                .add(
                                    egui::DragValue::new(&mut interval)
                                        .speed(5)
                                        .range(5..=1440)
                                        .suffix(" min"),
                                )
                                .changed()
                            {
                                if let Some(c) = self.get_config_mut(config_id) {
                                    c.sync_interval_minutes = interval;
                                }
                            }
                        });
                    }

                    ui.add_space(10.0);

                    // What to sync
                    ui.label("Synchronize:");
                    ui.checkbox(&mut self.sync_servers, "Servers");
                    ui.checkbox(&mut self.sync_groups, "Groups");
                    ui.checkbox(&mut self.sync_keys, "SSH Keys (encrypted)");
                    ui.checkbox(&mut self.sync_snippets, "Snippets");

                    ui.add_space(10.0);

                    ui.checkbox(&mut self.encrypt_sync, "Encrypt cloud data with master password");
                });

                ui.add_space(15.0);

                // Statistics
                ui.group(|ui| {
                    ui.label(egui::RichText::new("Sync Statistics").strong());
                    ui.add_space(5.0);
                    ui.label(format!("Total syncs: {}", config.sync_stats.total_syncs));
                    ui.label(format!("Failed syncs: {}", config.sync_stats.failed_syncs));
                    if config.sync_stats.total_syncs > 0 {
                        let success_rate = ((config.sync_stats.total_syncs - config.sync_stats.failed_syncs) as f32
                            / config.sync_stats.total_syncs as f32)
                            * 100.0;
                        ui.label(format!("Success rate: {:.1}%", success_rate));
                    }
                });

                ui.add_space(15.0);

                // Action buttons
                ui.horizontal(|ui| {
                    if ui.button("🗑 Delete").clicked() {
                        if let Some(id) = self.selected_config_id.take() {
                            self.remove_config(&id);
                        }
                    }

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui
                            .add_enabled(
                                config.enabled && !config.api_token.is_empty(),
                                egui::Button::new("↻ Sync Now"),
                            )
                            .clicked()
                        {
                            self.start_sync(config_id);
                            // Simulate completion after a delay (in real implementation, this would be async)
                            // self.complete_sync(config_id, SyncStatistics::default());
                        }

                        if ui.button("💾 Save Changes").clicked() {
                            self.show_message("Configuration saved".to_string());
                        }
                    });
                });
            } else {
                ui.label("Selected provider not found.");
            }
        } else {
            ui.vertical_centered(|ui| {
                ui.add_space(100.0);
                ui.label("Select a provider to view and configure settings");
            });
        }
    }

    fn render_add_dialog(&mut self, ctx: &egui::Context) {
        egui::Window::new("Add Cloud Provider")
            .collapsible(false)
            .resizable(false)
            .default_size([400.0, 300.0])
            .show(ctx, |ui| {
                ui.label("Select a cloud provider to synchronize with:");
                ui.add_space(15.0);

                // Provider selection
                let providers = [
                    CloudProviderType::Dropbox,
                    CloudProviderType::GoogleDrive,
                    CloudProviderType::OneDrive,
                    CloudProviderType::WebDav,
                    CloudProviderType::S3,
                    CloudProviderType::Custom,
                ];

                for provider in providers {
                    let is_selected = self.new_provider == provider;
                    if ui
                        .selectable_label(is_selected, format!("{} {}", provider_icon(&provider), provider))
                        .clicked()
                    {
                        self.new_provider = provider;
                    }
                }

                ui.add_space(15.0);

                ui.horizontal(|ui| {
                    ui.label("Name:");
                    ui.add(
                        egui::TextEdit::singleline(&mut self.new_config_name)
                            .hint_text("My Cloud Sync")
                            .desired_width(250.0),
                    );
                });

                ui.add_space(20.0);

                ui.horizontal(|ui| {
                    if ui.button("Cancel").clicked() {
                        self.show_add_dialog = false;
                    }

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let can_create = !self.new_config_name.is_empty();

                        if ui.add_enabled(can_create, egui::Button::new("Add")).clicked() {
                            let mut config = CloudSyncConfig::new(
                                self.new_provider.clone(),
                                self.new_config_name.clone(),
                            );
                            config.folder_path = Self::default_folder_for_provider(&self.new_provider);
                            let id = self.add_config(config);
                            self.selected_config_id = Some(id);
                            self.show_add_dialog = false;
                            self.new_config_name.clear();
                            self.show_message("Provider added".to_string());
                        }
                    });
                });
            });
    }

    fn render_sync_log(&mut self, ctx: &egui::Context) {
        egui::Window::new("Sync Log")
            .collapsible(false)
            .resizable(true)
            .default_size([600.0, 400.0])
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.heading("Synchronization Log");
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("Clear").clicked() {
                            self.sync_log.clear();
                        }
                        if ui.button("Close").clicked() {
                            self.show_sync_log = false;
                        }
                    });
                });

                ui.add_space(10.0);

                if self.sync_log.is_empty() {
                    ui.label("No log entries yet.");
                } else {
                    egui::ScrollArea::vertical().max_height(300.0).show(ui, |ui| {
                        for entry in self.sync_log.iter().rev() {
                            let color = match entry.level {
                                LogLevel::Info => egui::Color32::from_rgb(150, 150, 150),
                                LogLevel::Warning => egui::Color32::from_rgb(255, 193, 7),
                                LogLevel::Error => egui::Color32::from_rgb(255, 100, 100),
                                LogLevel::Success => egui::Color32::from_rgb(72, 199, 116),
                            };

                            let icon = match entry.level {
                                LogLevel::Info => "ℹ",
                                LogLevel::Warning => "⚠",
                                LogLevel::Error => "✗",
                                LogLevel::Success => "✓",
                            };

                            ui.label(
                                egui::RichText::new(format!(
                                    "[{}] {} {}",
                                    entry.timestamp.format("%H:%M:%S"),
                                    icon,
                                    entry.message
                                ))
                                .color(color)
                                .size(12.0),
                            );
                        }
                    });
                }
            });
    }
}

fn provider_icon(provider: &CloudProviderType) -> &'static str {
    match provider {
        CloudProviderType::Dropbox => "📦",
        CloudProviderType::GoogleDrive => "📁",
        CloudProviderType::OneDrive => "☁",
        CloudProviderType::WebDav => "🌐",
        CloudProviderType::S3 => "🪣",
        CloudProviderType::Custom => "⚙",
    }
}
