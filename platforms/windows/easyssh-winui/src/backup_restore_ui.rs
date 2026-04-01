//! Backup and Restore UI Module
//!
//! Features:
//! - Create manual backups with custom names
//! - Automatic scheduled backups
//! - Restore from backup points
//! - Backup history and management
//! - Export/Import backup files
//!
//! Feature flag: backup

#![allow(dead_code)]

use eframe::egui;
use std::path::PathBuf;
use std::time::SystemTime;

/// Backup restore dialog state
pub struct BackupRestoreDialog {
    /// Whether dialog is open
    pub is_open: bool,
    /// Active tab
    pub active_tab: BackupTab,
    /// Backup name for manual backup
    pub backup_name: String,
    /// Backup description
    pub backup_description: String,
    /// Include passwords in backup
    pub include_passwords: bool,
    /// Include SSH keys
    pub include_ssh_keys: bool,
    /// Include snippets
    pub include_snippets: bool,
    /// Include terminal history
    pub include_history: bool,
    /// Selected backup for restore
    pub selected_backup: Option<BackupItem>,
    /// Backup history list
    pub backup_history: Vec<BackupItem>,
    /// Show create backup dialog
    pub show_create_dialog: bool,
    /// Show restore confirmation
    pub show_restore_confirm: bool,
    /// Show delete confirmation
    pub show_delete_confirm: bool,
    /// Backup to delete
    pub pending_delete_backup: Option<String>,
    /// Import file path
    pub import_path: Option<PathBuf>,
    /// Export path
    pub export_path: Option<PathBuf>,
    /// Auto backup enabled
    pub auto_backup_enabled: bool,
    /// Auto backup interval (hours)
    pub auto_backup_interval: u32,
    /// Max auto backups to keep
    pub max_auto_backups: u32,
    /// Status message
    pub status_message: Option<(String, SystemTime)>,
    /// Progress (0-100)
    pub progress: f32,
    /// Is operation in progress
    pub is_processing: bool,
}

/// Backup tab selection
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BackupTab {
    Create,
    Restore,
    History,
    Settings,
}

/// Backup item in history
#[derive(Clone, Debug)]
pub struct BackupItem {
    pub id: String,
    pub name: String,
    pub description: String,
    pub created_at: SystemTime,
    pub size_bytes: u64,
    pub item_count: u32,
    pub includes_passwords: bool,
    pub includes_ssh_keys: bool,
    pub includes_snippets: bool,
    pub includes_history: bool,
    pub is_auto_backup: bool,
    pub file_path: Option<PathBuf>,
}

impl BackupItem {
    /// Format size for display
    pub fn format_size(&self) -> String {
        let bytes = self.size_bytes as f64;
        if bytes < 1024.0 {
            format!("{} B", bytes as u64)
        } else if bytes < 1024.0 * 1024.0 {
            format!("{:.1} KB", bytes / 1024.0)
        } else if bytes < 1024.0 * 1024.0 * 1024.0 {
            format!("{:.1} MB", bytes / (1024.0 * 1024.0))
        } else {
            format!("{:.1} GB", bytes / (1024.0 * 1024.0 * 1024.0))
        }
    }

    /// Format timestamp for display
    pub fn format_time(&self) -> String {
        let duration = SystemTime::now()
            .duration_since(self.created_at)
            .unwrap_or_default();

        if duration.as_secs() < 60 {
            "Just now".to_string()
        } else if duration.as_secs() < 3600 {
            format!("{} min ago", duration.as_secs() / 60)
        } else if duration.as_secs() < 86400 {
            format!("{} hours ago", duration.as_secs() / 3600)
        } else if duration.as_secs() < 604800 {
            format!("{} days ago", duration.as_secs() / 86400)
        } else {
            format!("{} weeks ago", duration.as_secs() / 604800)
        }
    }
}

impl Default for BackupRestoreDialog {
    fn default() -> Self {
        Self {
            is_open: false,
            active_tab: BackupTab::Create,
            backup_name: String::new(),
            backup_description: String::new(),
            include_passwords: false,
            include_ssh_keys: true,
            include_snippets: true,
            include_history: false,
            selected_backup: None,
            backup_history: Vec::new(),
            show_create_dialog: false,
            show_restore_confirm: false,
            show_delete_confirm: false,
            pending_delete_backup: None,
            import_path: None,
            export_path: None,
            auto_backup_enabled: false,
            auto_backup_interval: 24,
            max_auto_backups: 10,
            status_message: None,
            progress: 0.0,
            is_processing: false,
        }
    }
}

impl BackupRestoreDialog {
    /// Create new backup dialog with sample data
    pub fn new() -> Self {
        let mut dialog = Self::default();
        dialog.load_sample_backups();
        dialog
    }

    /// Load sample backup data for demonstration
    fn load_sample_backups(&mut self) {
        self.backup_history = vec![
            BackupItem {
                id: "backup_001".to_string(),
                name: "Before Migration".to_string(),
                description: "Full backup before server migration".to_string(),
                created_at: SystemTime::now() - std::time::Duration::from_secs(3600 * 2),
                size_bytes: 2_450_000,
                item_count: 45,
                includes_passwords: false,
                includes_ssh_keys: true,
                includes_snippets: true,
                includes_history: false,
                is_auto_backup: false,
                file_path: Some(PathBuf::from("/backups/before_migration.zip")),
            },
            BackupItem {
                id: "backup_002".to_string(),
                name: "Daily Auto-Backup".to_string(),
                description: "Automatic daily backup".to_string(),
                created_at: SystemTime::now() - std::time::Duration::from_secs(86400),
                size_bytes: 1_890_000,
                item_count: 42,
                includes_passwords: false,
                includes_ssh_keys: true,
                includes_snippets: true,
                includes_history: false,
                is_auto_backup: true,
                file_path: Some(PathBuf::from("/backups/auto_daily_001.zip")),
            },
            BackupItem {
                id: "backup_003".to_string(),
                name: "Weekly Snapshot".to_string(),
                description: "Weekly full backup with passwords".to_string(),
                created_at: SystemTime::now() - std::time::Duration::from_secs(604800),
                size_bytes: 3_120_000,
                item_count: 48,
                includes_passwords: true,
                includes_ssh_keys: true,
                includes_snippets: true,
                includes_history: true,
                is_auto_backup: false,
                file_path: Some(PathBuf::from("/backups/weekly_full.zip")),
            },
            BackupItem {
                id: "backup_004".to_string(),
                name: "Config Only".to_string(),
                description: "Servers and groups only".to_string(),
                created_at: SystemTime::now() - std::time::Duration::from_secs(1209600),
                size_bytes: 450_000,
                item_count: 35,
                includes_passwords: false,
                includes_ssh_keys: false,
                includes_snippets: false,
                includes_history: false,
                is_auto_backup: false,
                file_path: Some(PathBuf::from("/backups/config_only.zip")),
            },
        ];
    }

    /// Open the dialog
    pub fn open(&mut self) {
        self.is_open = true;
        self.active_tab = BackupTab::Create;
    }

    /// Close the dialog
    pub fn close(&mut self) {
        self.is_open = false;
        self.reset_state();
    }

    /// Reset dialog state
    pub fn reset_state(&mut self) {
        self.backup_name.clear();
        self.backup_description.clear();
        self.selected_backup = None;
        self.show_create_dialog = false;
        self.show_restore_confirm = false;
        self.show_delete_confirm = false;
        self.pending_delete_backup = None;
        self.progress = 0.0;
        self.is_processing = false;
    }

    /// Show status message
    pub fn show_status(&mut self, message: String) {
        self.status_message = Some((message, SystemTime::now()));
    }

    /// Clear expired status messages
    pub fn clear_expired_status(&mut self) {
        if let Some((_, time)) = self.status_message {
            if SystemTime::now().duration_since(time).unwrap_or_default().as_secs() > 5 {
                self.status_message = None;
            }
        }
    }

    /// Render the backup/restore dialog
    pub fn render(&mut self, ctx: &egui::Context) {
        if !self.is_open {
            return;
        }

        self.clear_expired_status();

        let window_width = 700.0;
        let window_height = 550.0;

        egui::Window::new("💾 Backup & Restore")
            .collapsible(false)
            .resizable(false)
            .fixed_size([window_width, window_height])
            .frame(egui::Frame {
                fill: egui::Color32::from_rgb(35, 38, 46),
                rounding: egui::Rounding::same(12.0),
                stroke: egui::Stroke::new(1.0, egui::Color32::from_rgb(64, 156, 255)),
                shadow: egui::epaint::Shadow {
                    blur: 24.0,
                    spread: 0.0,
                    offset: egui::Vec2::new(0.0, 8.0),
                    color: egui::Color32::from_black_alpha(100),
                },
                ..Default::default()
            })
            .show(ctx, |ui| {
                ui.vertical(|ui| {
                    // Header with tabs
                    self.render_header(ui);

                    ui.separator();

                    // Tab content
                    match self.active_tab {
                        BackupTab::Create => self.render_create_tab(ui),
                        BackupTab::Restore => self.render_restore_tab(ui),
                        BackupTab::History => self.render_history_tab(ui),
                        BackupTab::Settings => self.render_settings_tab(ui),
                    }

                    // Status bar
                    if let Some((ref msg, _)) = self.status_message {
                        ui.separator();
                        ui.horizontal(|ui| {
                            ui.label(
                                egui::RichText::new(msg)
                                    .small()
                                    .color(egui::Color32::from_rgb(100, 220, 150)),
                            );
                        });
                    }

                    // Progress bar if processing
                    if self.is_processing {
                        ui.add_space(8.0);
                        ui.add(
                            egui::ProgressBar::new(self.progress / 100.0)
                                .text("Processing...")
                                .desired_width(ui.available_width()),
                        );
                    }
                });
            });

        // Render confirmation dialogs
        if self.show_restore_confirm {
            self.render_restore_confirmation(ctx);
        }
        if self.show_delete_confirm {
            self.render_delete_confirmation(ctx);
        }
    }

    fn render_header(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.heading("Backup & Restore");

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("✕").clicked() {
                    self.close();
                }
            });
        });

        ui.add_space(10.0);

        // Tab buttons
        ui.horizontal(|ui| {
            let tabs = vec![
                (BackupTab::Create, "➕ Create", "Create new backup"),
                (BackupTab::Restore, "↺ Restore", "Restore from backup"),
                (BackupTab::History, "📜 History", "View backup history"),
                (BackupTab::Settings, "⚙ Settings", "Configure auto-backup"),
            ];

            for (tab, icon_label, tooltip) in tabs {
                let is_active = self.active_tab == tab;
                let btn = egui::Button::new(icon_label)
                    .fill(if is_active {
                        egui::Color32::from_rgb(64, 156, 255)
                    } else {
                        egui::Color32::from_rgb(50, 55, 65)
                    })
                    .min_size([100.0, 32.0].into());

                if ui.add(btn).on_hover_text(tooltip).clicked() {
                    self.active_tab = tab;
                }
            }
        });
    }

    fn render_create_tab(&mut self, ui: &mut egui::Ui) {
        ui.heading("Create New Backup");
        ui.add_space(15.0);

        // Quick backup button
        ui.horizontal(|ui| {
            if ui
                .add(
                    egui::Button::new("⚡ Quick Backup")
                        .fill(egui::Color32::from_rgb(72, 199, 116))
                        .min_size([140.0, 40.0].into()),
                )
                .on_hover_text("Create backup with default settings")
                .clicked()
            {
                self.create_quick_backup();
            }

            ui.label(
                egui::RichText::new("Create a full backup with one click")
                    .small()
                    .color(egui::Color32::from_rgb(150, 160, 175)),
            );
        });

        ui.add_space(20.0);
        ui.separator();
        ui.add_space(10.0);

        // Custom backup options
        ui.label(
            egui::RichText::new("Custom Backup Options")
                .strong()
                .size(16.0),
        );
        ui.add_space(15.0);

        // Backup name
        ui.horizontal(|ui| {
            ui.label("Name:");
            ui.add(
                egui::TextEdit::singleline(&mut self.backup_name)
                    .desired_width(300.0)
                    .hint_text("Enter backup name..."),
            );
        });

        ui.add_space(10.0);

        // Backup description
        ui.horizontal(|ui| {
            ui.label("Description:");
            ui.add(
                egui::TextEdit::singleline(&mut self.backup_description)
                    .desired_width(400.0)
                    .hint_text("Optional description..."),
            );
        });

        ui.add_space(15.0);

        // Backup contents
        ui.group(|ui| {
            ui.label(
                egui::RichText::new("Backup Contents")
                    .strong()
                    .size(14.0),
            );
            ui.add_space(10.0);

            ui.checkbox(&mut self.include_ssh_keys, "☐ SSH Keys & Identities");
            ui.checkbox(&mut self.include_snippets, "☐ Snippets & Commands");
            ui.checkbox(
                &mut self.include_passwords,
                "☐ Saved Passwords (encrypted)",
            );
            ui.checkbox(&mut self.include_history, "☐ Terminal History");
        });

        ui.add_space(20.0);

        // Create button
        ui.horizontal(|ui| {
            let can_create = !self.backup_name.is_empty();
            if ui
                .add_enabled(
                    can_create,
                    egui::Button::new("💾 Create Backup")
                        .fill(egui::Color32::from_rgb(64, 156, 255))
                        .min_size([140.0, 40.0].into()),
                )
                .clicked()
            {
                self.create_custom_backup();
            }

            if ui.button("Cancel").clicked() {
                self.backup_name.clear();
                self.backup_description.clear();
            }
        });
    }

    fn render_restore_tab(&mut self, ui: &mut egui::Ui) {
        ui.heading("Restore from Backup");
        ui.add_space(15.0);

        // Import backup file
        ui.group(|ui| {
            ui.label(
                egui::RichText::new("Import Backup File")
                    .strong()
                    .size(14.0),
            );
            ui.add_space(10.0);

            ui.horizontal(|ui| {
                if ui
                    .add(
                        egui::Button::new("📁 Import from File...")
                            .fill(egui::Color32::from_rgb(64, 156, 255))
                            .min_size([140.0, 36.0].into()),
                    )
                    .clicked()
                {
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("Backup files", &["zip", "json", "enc"])
                        .add_filter("All files", &["*"])
                        .pick_file()
                    {
                        self.import_path = Some(path);
                        self.show_status(format!("Selected: {:?}", self.import_path));
                    }
                }

                if let Some(ref path) = self.import_path {
                    ui.label(
                        egui::RichText::new(format!("Selected: {}", path.display()))
                            .small()
                            .color(egui::Color32::from_rgb(150, 160, 175)),
                    );
                }
            });
        });

        ui.add_space(20.0);

        // Recent backups list
        ui.label(
            egui::RichText::new("Recent Backups")
                .strong()
                .size(14.0),
        );
        ui.add_space(10.0);

        if self.backup_history.is_empty() {
            ui.label("No backups available.");
        } else {
            egui::ScrollArea::vertical()
                .max_height(250.0)
                .show(ui, |ui| {
                    for backup in &self.backup_history.clone() {
                        self.render_backup_list_item(ui, backup, true);
                    }
                });
        }
    }

    fn render_history_tab(&mut self, ui: &mut egui::Ui) {
        ui.heading("Backup History");
        ui.add_space(15.0);

        // Statistics
        ui.group(|ui| {
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new(format!("Total Backups: {}", self.backup_history.len()))
                        .strong(),
                );

                let total_size: u64 = self.backup_history.iter().map(|b| b.size_bytes).sum();
                let total_size_mb = total_size as f64 / (1024.0 * 1024.0);

                ui.label(format!("Total Size: {:.1} MB", total_size_mb));

                let auto_count = self
                    .backup_history
                    .iter()
                    .filter(|b| b.is_auto_backup)
                    .count();
                ui.label(format!("Auto-backups: {}", auto_count));
            });
        });

        ui.add_space(15.0);

        // Backup list
        if self.backup_history.is_empty() {
            ui.label("No backup history available.");
        } else {
            egui::ScrollArea::vertical()
                .max_height(350.0)
                .show(ui, |ui| {
                    for backup in &self.backup_history.clone() {
                        self.render_backup_list_item(ui, backup, false);
                    }
                });
        }

        ui.add_space(10.0);

        // Clear history button
        if ui
            .add(
                egui::Button::new("🗑 Clear History")
                    .fill(egui::Color32::from_rgb(180, 60, 60))
                    .min_size([120.0, 32.0].into()),
            )
            .clicked()
        {
            self.backup_history.clear();
            self.show_status("History cleared".to_string());
        }
    }

    fn render_settings_tab(&mut self, ui: &mut egui::Ui) {
        ui.heading("Auto-Backup Settings");
        ui.add_space(15.0);

        ui.group(|ui| {
            ui.horizontal(|ui| {
                ui.checkbox(&mut self.auto_backup_enabled, "Enable automatic backups");
            });

            if self.auto_backup_enabled {
                ui.add_space(15.0);

                ui.horizontal(|ui| {
                    ui.label("Backup interval:");
                    ui.add(
                        egui::DragValue::new(&mut self.auto_backup_interval)
                            .speed(1)
                            .range(1..=168)
                            .suffix(" hours"),
                    );
                });

                ui.add_space(10.0);

                ui.horizontal(|ui| {
                    ui.label("Max backups to keep:");
                    ui.add(
                        egui::DragValue::new(&mut self.max_auto_backups)
                            .speed(1)
                            .range(1..=100),
                    );
                });

                ui.add_space(15.0);

                // What to include in auto-backups
                ui.label("Include in auto-backups:");
                ui.checkbox(&mut self.include_ssh_keys, "SSH Keys");
                ui.checkbox(&mut self.include_snippets, "Snippets");
                ui.checkbox(&mut self.include_history, "Terminal History");
                ui.checkbox(
                    &mut false, // Auto backups shouldn't include passwords
                    "Saved Passwords (not recommended for auto-backup)",
                );

                ui.add_space(10.0);

                // Next backup estimate
                ui.label(
                    egui::RichText::new(format!(
                        "Next backup: in approximately {} hours",
                        self.auto_backup_interval
                    ))
                    .small()
                    .color(egui::Color32::from_rgb(150, 160, 175)),
                );
            }
        });

        ui.add_space(20.0);

        // Export settings
        ui.group(|ui| {
            ui.label(
                egui::RichText::new("Export/Import Settings")
                    .strong()
                    .size(14.0),
            );
            ui.add_space(10.0);

            ui.horizontal(|ui| {
                if ui.button("📤 Export Settings").clicked() {
                    // Export settings logic
                }
                if ui.button("📥 Import Settings").clicked() {
                    // Import settings logic
                }
            });
        });

        ui.add_space(20.0);

        // Save settings button
        if ui
            .add(
                egui::Button::new("💾 Save Settings")
                    .fill(egui::Color32::from_rgb(64, 156, 255))
                    .min_size([140.0, 36.0].into()),
            )
            .clicked()
        {
            self.show_status("Settings saved".to_string());
        }
    }

    fn render_backup_list_item(
        &mut self,
        ui: &mut egui::Ui,
        backup: &BackupItem,
        show_restore_button: bool,
    ) {
        let is_selected = self
            .selected_backup
            .as_ref()
            .map(|b| b.id == backup.id)
            .unwrap_or(false);

        let bg_color = if is_selected {
            egui::Color32::from_rgb(64, 120, 200)
        } else {
            egui::Color32::from_rgb(40, 45, 55)
        };

        egui::Frame::none()
            .fill(bg_color)
            .rounding(egui::Rounding::same(8.0))
            .inner_margin(egui::Margin::same(10.0))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        // Name and type
                        ui.horizontal(|ui| {
                            ui.label(
                                egui::RichText::new(&backup.name)
                                    .strong()
                                    .size(14.0),
                            );
                            if backup.is_auto_backup {
                                ui.label(
                                    egui::RichText::new("[AUTO]")
                                        .small()
                                        .color(egui::Color32::from_rgb(100, 220, 150)),
                                );
                            }
                        });

                        // Description
                        ui.label(
                            egui::RichText::new(&backup.description)
                                .small()
                                .color(egui::Color32::from_rgb(150, 160, 175)),
                        );

                        // Metadata
                        ui.horizontal(|ui| {
                            ui.label(
                                egui::RichText::new(format!(
                                    "{} • {} items • {}",
                                    backup.format_size(),
                                    backup.item_count,
                                    backup.format_time()
                                ))
                                .small()
                                .color(egui::Color32::from_rgb(120, 130, 145)),
                            );

                            // Content indicators
                            if backup.includes_ssh_keys {
                                ui.label(
                                    egui::RichText::new("🔑").small(),
                                );
                            }
                            if backup.includes_snippets {
                                ui.label(
                                    egui::RichText::new("📋").small(),
                                );
                            }
                            if backup.includes_passwords {
                                ui.label(
                                    egui::RichText::new("🔒").small(),
                                );
                            }
                        });
                    });

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("🗑").clicked() {
                            self.pending_delete_backup = Some(backup.id.clone());
                            self.show_delete_confirm = true;
                        }

                        if ui.button("📤").clicked() {
                            self.export_backup(backup);
                        }

                        if show_restore_button {
                            if ui
                                .add(
                                    egui::Button::new("↺ Restore")
                                        .fill(egui::Color32::from_rgb(64, 156, 255))
                                        .min_size([80.0, 28.0].into()),
                                )
                                .clicked()
                            {
                                self.selected_backup = Some(backup.clone());
                                self.show_restore_confirm = true;
                            }
                        }
                    });
                });
            });

        ui.add_space(4.0);
    }

    fn render_restore_confirmation(&mut self, ctx: &egui::Context) {
        let mut should_close = false;

        egui::Window::new("Confirm Restore")
            .collapsible(false)
            .resizable(false)
            .default_size([400.0, 250.0])
            .show(ctx, |ui| {
                ui.add_space(10.0);
                ui.label(
                    egui::RichText::new("⚠️ Warning")
                        .strong()
                        .size(18.0)
                        .color(egui::Color32::from_rgb(255, 200, 80)),
                );
                ui.add_space(15.0);

                ui.label("Restoring from backup will:");
                ui.label("• Replace current server configurations");
                ui.label("• Overwrite existing snippets");
                ui.label("• Restore saved passwords (if included)");
                ui.add_space(10.0);

                ui.label(
                    egui::RichText::new("This action cannot be undone!")
                        .color(egui::Color32::from_rgb(255, 100, 100))
                        .strong(),
                );

                ui.add_space(20.0);

                ui.horizontal(|ui| {
                    if ui.button("Cancel").clicked() {
                        should_close = true;
                    }

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui
                            .add(
                                egui::Button::new("↺ Restore")
                                    .fill(egui::Color32::from_rgb(64, 156, 255))
                                    .min_size([100.0, 32.0].into()),
                            )
                            .clicked()
                        {
                            self.perform_restore();
                            should_close = true;
                        }
                    });
                });
            });

        if should_close {
            self.show_restore_confirm = false;
        }
    }

    fn render_delete_confirmation(&mut self, ctx: &egui::Context) {
        let mut should_close = false;

        egui::Window::new("Confirm Delete")
            .collapsible(false)
            .resizable(false)
            .default_size([350.0, 200.0])
            .show(ctx, |ui| {
                ui.add_space(10.0);
                ui.label(
                    egui::RichText::new("🗑️ Delete Backup")
                        .strong()
                        .size(18.0),
                );
                ui.add_space(15.0);

                ui.label("Are you sure you want to delete this backup?");
                ui.add_space(10.0);
                ui.label(
                    egui::RichText::new("This action cannot be undone!")
                        .color(egui::Color32::from_rgb(255, 100, 100))
                        .strong(),
                );

                ui.add_space(20.0);

                ui.horizontal(|ui| {
                    if ui.button("Cancel").clicked() {
                        should_close = true;
                    }

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui
                            .add(
                                egui::Button::new("🗑 Delete")
                                    .fill(egui::Color32::from_rgb(180, 60, 60))
                                    .min_size([100.0, 32.0].into()),
                            )
                            .clicked()
                        {
                            self.perform_delete();
                            should_close = true;
                        }
                    });
                });
            });

        if should_close {
            self.show_delete_confirm = false;
            self.pending_delete_backup = None;
        }
    }

    // Action methods
    fn create_quick_backup(&mut self) {
        self.backup_name = format!(
            "Quick Backup {}",
            chrono::Local::now().format("%Y-%m-%d %H:%M")
        );
        self.backup_description = "Automatic quick backup".to_string();
        self.include_ssh_keys = true;
        self.include_snippets = true;
        self.include_passwords = false;
        self.create_custom_backup();
    }

    fn create_custom_backup(&mut self) {
        self.is_processing = true;
        self.progress = 0.0;

        // Simulate backup creation
        let new_backup = BackupItem {
            id: format!("backup_{:03}", self.backup_history.len() + 1),
            name: self.backup_name.clone(),
            description: self.backup_description.clone(),
            created_at: SystemTime::now(),
            size_bytes: 1_500_000 + (rand::random::<u64>() % 2_000_000),
            item_count: 35 + (rand::random::<u32>() % 20),
            includes_passwords: self.include_passwords,
            includes_ssh_keys: self.include_ssh_keys,
            includes_snippets: self.include_snippets,
            includes_history: self.include_history,
            is_auto_backup: false,
            file_path: Some(PathBuf::from(format!(
                "/backups/{}_{}.zip",
                self.backup_name.to_lowercase().replace(" ", "_"),
                chrono::Local::now().format("%Y%m%d_%H%M")
            ))),
        };

        self.backup_history.insert(0, new_backup);
        self.show_status(format!("Backup '{}' created successfully", self.backup_name));

        self.is_processing = false;
        self.progress = 100.0;

        // Clear form
        self.backup_name.clear();
        self.backup_description.clear();
    }

    fn perform_restore(&mut self) {
        if let Some(ref backup) = self.selected_backup {
            self.is_processing = true;
            self.progress = 50.0;

            // Simulate restore
            self.show_status(format!("Restored from: {}", backup.name));

            self.is_processing = false;
            self.progress = 100.0;
            self.selected_backup = None;
        }
    }

    fn perform_delete(&mut self) {
        if let Some(ref id) = self.pending_delete_backup {
            self.backup_history.retain(|b| &b.id != id);
            self.show_status("Backup deleted".to_string());
        }
    }

    fn export_backup(&mut self, backup: &BackupItem) {
        if let Some(ref path) = backup.file_path {
            if let Some(export_path) = rfd::FileDialog::new()
                .set_file_name(&format!("{}.zip", backup.name.to_lowercase().replace(" ", "_")))
                .save_file()
            {
                self.export_path = Some(export_path);
                self.show_status(format!("Exported: {}", backup.name));
            }
        }
    }
}

// Random helper for demo data
mod rand {
    pub fn random<T>() -> T
    where
        T: From<u64>,
    {
        T::from(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos() as u64,
        )
    }
}
