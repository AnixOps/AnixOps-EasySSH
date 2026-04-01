#![allow(dead_code)]

use eframe::egui;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Mutex;

use easyssh_core::{
    ExportFormat, ConfigConflictResolution as ConflictResolution, ImportResult, ImportFormat,
    get_supported_languages, get_current_language, set_language, get_language_display_name,
};
use crate::app_settings::SettingsManager;

/// UI Theme mode for application appearance
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum UiThemeMode {
    #[default]
    Dark,
    Light,
    System,
}

impl UiThemeMode {
    pub fn is_dark(&self) -> bool {
        matches!(self, UiThemeMode::Dark)
    }

    pub fn is_light(&self) -> bool {
        matches!(self, UiThemeMode::Light)
    }

    pub fn is_system(&self) -> bool {
        matches!(self, UiThemeMode::System)
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            UiThemeMode::Dark => "Dark",
            UiThemeMode::Light => "Light",
            UiThemeMode::System => "System",
        }
    }
}

/// Settings panel state and configuration
pub struct SettingsPanel {
    pub is_open: bool,
    pub active_tab: SettingsTab,
    pub show_import_dialog: bool,
    pub show_export_dialog: bool,
    pub show_import_result: bool,
    pub show_conflict_dialog: bool,
    pub show_cloud_sync_dialog: bool,
    pub import_file_path: Option<PathBuf>,
    pub import_format: ImportFormat,
    pub import_content: String,
    pub import_result: Option<ImportResult>,
    pub conflict_resolution: ConflictResolution,
    pub export_format: ExportFormat,
    pub export_password: String,
    pub export_include_secrets: bool,
    pub export_path: Option<PathBuf>,
    pub cloud_sync_enabled: bool,
    pub cloud_provider: CloudProvider,
    pub cloud_api_key: String,
    pub master_password: String,
    pub use_encryption: bool,
    pub selected_language: String,
    // UI Theme mode
    pub ui_theme_mode: UiThemeMode,
    // Accessibility settings
    pub high_contrast: bool,
    pub reduce_motion: bool,
    pub large_text: bool,
    // Terminal font settings
    pub terminal_font_family: String,
    pub terminal_font_size: f32,
    pub terminal_font_zoom: f32,
    // Terminal behavior settings
    pub terminal_use_webgl: bool,
    pub terminal_auto_scroll: bool,
    pub terminal_copy_on_select: bool,
    // Flag to notify main app that theme needs update
    pub pending_theme_change: Option<UiThemeMode>,
    // Flag to notify main app that accessibility needs update
    pub pending_accessibility_change: bool,
    // Flag to notify main app that font settings need update
    pub pending_font_change: bool,
    // Settings manager for persistence
    settings_manager: Option<Arc<SettingsManager>>,
    // Flag to notify main app that language needs update
    pub pending_language_change: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum SettingsTab {
    General,
    ImportExport,
    CloudSync,
    Security,
    Appearance,
    Themes,
}

#[derive(Clone, Debug, PartialEq)]
pub enum CloudProvider {
    None,
    Dropbox,
    GoogleDrive,
    OneDrive,
    Custom,
}

impl Default for SettingsPanel {
    fn default() -> Self {
        Self {
            is_open: false,
            active_tab: SettingsTab::General,
            show_import_dialog: false,
            show_export_dialog: false,
            show_import_result: false,
            show_conflict_dialog: false,
            show_cloud_sync_dialog: false,
            import_file_path: None,
            import_format: ImportFormat::AutoDetect,
            import_content: String::new(),
            import_result: None,
            conflict_resolution: ConflictResolution::Skip,
            export_format: ExportFormat::Json,
            export_password: String::new(),
            export_include_secrets: false,
            export_path: None,
            cloud_sync_enabled: false,
            cloud_provider: CloudProvider::None,
            cloud_api_key: String::new(),
            master_password: String::new(),
            use_encryption: false,
            selected_language: get_current_language(),
            // Default to dark theme (SSH clients typically use dark)
            ui_theme_mode: UiThemeMode::Dark,
            high_contrast: false,
            reduce_motion: false,
            large_text: false,
            // Default font settings
            terminal_font_family: "Cascadia Code".to_string(),
            terminal_font_size: 14.0,
            terminal_font_zoom: 1.0,
            // Default terminal behavior
            terminal_use_webgl: true,
            terminal_auto_scroll: true,
            terminal_copy_on_select: false,
            pending_theme_change: None,
            pending_accessibility_change: false,
            pending_font_change: false,
            settings_manager: None,
            pending_language_change: None,
        }
    }
}

impl SettingsPanel {
    pub fn open(&mut self) {
        self.is_open = true;
    }

    pub fn close(&mut self) {
        self.is_open = false;
    }

    pub fn toggle(&mut self) {
        self.is_open = !self.is_open;
    }

    /// Initialize the settings panel with the settings manager
    pub fn initialize(&mut self, settings_manager: Arc<SettingsManager>) {
        self.settings_manager = Some(settings_manager.clone());

        // Sync UI state with loaded settings
        let settings = settings_manager.get_settings();
        self.selected_language = settings.language;
        self.ui_theme_mode = match settings.theme_mode.as_str() {
            "light" => UiThemeMode::Light,
            "system" => UiThemeMode::System,
            _ => UiThemeMode::Dark,
        };
        self.high_contrast = settings.high_contrast;
        self.reduce_motion = settings.reduced_motion;
        self.large_text = settings.large_text;

        // Load font settings
        self.terminal_font_family = settings.terminal_font_family;
        self.terminal_font_size = settings.terminal_font_size;
        self.terminal_font_zoom = settings.terminal_font_zoom;
        self.terminal_use_webgl = settings.terminal_use_webgl;
        self.terminal_auto_scroll = settings.terminal_auto_scroll;
        self.terminal_copy_on_select = settings.terminal_copy_on_select;

        tracing::info!("Settings panel initialized with font: {} {}px (zoom: {})",
            self.terminal_font_family, self.terminal_font_size, self.terminal_font_zoom);
    }

    /// Check if there's a pending theme change
    pub fn has_pending_theme_change(&self) -> bool {
        self.pending_theme_change.is_some()
    }

    /// Take the pending theme change (if any)
    pub fn take_pending_theme_change(&mut self) -> Option<UiThemeMode> {
        self.pending_theme_change.take()
    }

    /// Check if there's a pending accessibility change
    pub fn has_pending_accessibility_change(&self) -> bool {
        self.pending_accessibility_change
    }

    /// Take the pending accessibility change flag
    pub fn take_pending_accessibility_change(&mut self) -> bool {
        let pending = self.pending_accessibility_change;
        self.pending_accessibility_change = false;
        pending
    }

    /// Check if there's a pending font change
    pub fn has_pending_font_change(&self) -> bool {
        self.pending_font_change
    }

    /// Take the pending font change flag
    pub fn take_pending_font_change(&mut self) -> bool {
        let pending = self.pending_font_change;
        self.pending_font_change = false;
        pending
    }

    /// Get current font settings
    pub fn get_font_settings(&self) -> (&str, f32, f32) {
        (&self.terminal_font_family, self.terminal_font_size, self.terminal_font_zoom)
    }

    /// Get current accessibility settings
    pub fn get_accessibility_settings(&self) -> (bool, bool, bool) {
        (self.high_contrast, self.reduce_motion, self.large_text)
    }

    /// Check if there's a pending language change
    pub fn has_pending_language_change(&self) -> bool {
        self.pending_language_change.is_some()
    }

    /// Take the pending language change (if any)
    pub fn take_pending_language_change(&mut self) -> Option<String> {
        self.pending_language_change.take()
    }

    /// Save all current settings to disk
    pub fn save_settings(&self) {
        if let Some(ref manager) = self.settings_manager {
            // Update all settings in the manager
            let theme_mode_str = match self.ui_theme_mode {
                UiThemeMode::Light => "light",
                UiThemeMode::Dark => "dark",
                UiThemeMode::System => "system",
            }.to_string();

            manager.set_theme_mode(theme_mode_str);
            manager.set_accessibility(self.high_contrast, self.reduce_motion, self.large_text);

            // Save terminal font and behavior settings
            manager.set_terminal_settings(
                self.terminal_font_family.clone(),
                self.terminal_font_size,
                self.terminal_font_zoom,
                self.terminal_use_webgl,
                self.terminal_auto_scroll,
                self.terminal_copy_on_select,
            );

            // Save to disk
            if let Err(e) = manager.force_save() {
                tracing::error!("Failed to save settings: {}", e);
            } else {
                tracing::info!("Settings saved successfully");
            }
        }
    }

    pub fn render(&mut self, ctx: &egui::Context, view_model: &Arc<Mutex<crate::viewmodels::AppViewModel>>) {
        if !self.is_open {
            return;
        }

        egui::Window::new("Settings")
            .collapsible(false)
            .resizable(true)
            .default_size([700.0, 500.0])
            .frame(egui::Frame {
                fill: egui::Color32::from_rgb(42, 48, 58),
                stroke: egui::Stroke::new(1.0, egui::Color32::from_rgb(60, 70, 85)),
                ..Default::default()
            })
            .show(ctx, |ui| {
                self.render_settings_content(ui, view_model);
            });

        // Render sub-dialogs
        if self.show_import_dialog {
            self.render_import_dialog(ctx, view_model);
        }
        if self.show_export_dialog {
            self.render_export_dialog(ctx, view_model);
        }
        if self.show_import_result {
            self.render_import_result(ctx);
        }
        if self.show_cloud_sync_dialog {
            self.render_cloud_sync_dialog(ctx);
        }
    }

    fn render_settings_content(&mut self, ui: &mut egui::Ui, _view_model: &Arc<Mutex<crate::viewmodels::AppViewModel>>) {
        ui.horizontal(|ui| {
            // Left sidebar - tabs
            ui.vertical(|ui| {
                ui.set_width(150.0);
                ui.set_min_height(400.0);

                self.render_tab_button(ui, "General", SettingsTab::General, "⚙");
                self.render_tab_button(ui, "Import/Export", SettingsTab::ImportExport, "📂");
                self.render_tab_button(ui, "Cloud Sync", SettingsTab::CloudSync, "☁");
                self.render_tab_button(ui, "Security", SettingsTab::Security, "🔒");
                self.render_tab_button(ui, "Appearance", SettingsTab::Appearance, "🎨");
                self.render_tab_button(ui, "Themes", SettingsTab::Themes, "🎭");

                ui.add_space(20.0);

                if ui.add(egui::Button::new("Close").min_size([120.0, 40.0].into())).clicked() {
                    self.close();
                }
            });

            ui.separator();

            // Right content area
            ui.vertical(|ui| {
                ui.set_width(500.0);

                match self.active_tab {
                    SettingsTab::General => self.render_general_tab(ui),
                    SettingsTab::ImportExport => self.render_import_export_tab(ui),
                    SettingsTab::CloudSync => self.render_cloud_sync_tab(ui),
                    SettingsTab::Security => self.render_security_tab(ui),
                    SettingsTab::Appearance => self.render_appearance_tab(ui),
                    SettingsTab::Themes => self.render_themes_placeholder(ui),
                }
            });
        });
    }

    fn render_tab_button(&mut self, ui: &mut egui::Ui, label: &str, tab: SettingsTab, icon: &str) {
        let is_active = self.active_tab == tab;
        let button = egui::Button::new(format!("{} {}", icon, label))
            .min_size([140.0, 40.0].into())
            .fill(if is_active {
                egui::Color32::from_rgb(64, 156, 255)
            } else {
                egui::Color32::TRANSPARENT
            });

        if ui.add(button).clicked() {
            self.active_tab = tab;
        }
    }

    fn render_general_tab(&mut self, ui: &mut egui::Ui) {
        ui.heading("General Settings");
        ui.add_space(20.0);

        ui.group(|ui| {
            ui.label("Application Settings");
            ui.add_space(10.0);

            // Language selection with working dropdown
            ui.horizontal(|ui| {
                ui.label("Language:");
                let current_display = get_language_display_name(&self.selected_language)
                    .unwrap_or(&self.selected_language);

                egui::ComboBox::from_id_source(egui::Id::new("language_selector"))
                    .selected_text(current_display)
                    .width(180.0)
                    .show_ui(ui, |ui| {
                        for (code, native_name, _english_name) in get_supported_languages() {
                            let is_selected = self.selected_language == *code;
                            if ui.selectable_label(is_selected, *native_name).clicked() {
                                if self.selected_language != *code {
                                    self.selected_language = code.to_string();

                                    // Apply the language change immediately
                                    if let Err(e) = set_language(code) {
                                        eprintln!("Failed to set language to {}: {}", code, e);
                                    } else {
                                        println!("Language changed to: {} ({})", native_name, code);

                                        // Update settings manager if available
                                        if let Some(ref manager) = self.settings_manager {
                                            if let Err(e) = manager.set_language(code.to_string()) {
                                                eprintln!("Failed to persist language setting: {}", e);
                                            } else {
                                                println!("Language setting persisted to disk");
                                            }
                                        }

                                        // Set pending language change for UI refresh
                                        self.pending_language_change = Some(code.to_string());
                                    }
                                }
                            }
                        }
                    });
            });

            ui.add_space(10.0);

            // Theme selection with working dropdown
            ui.horizontal(|ui| {
                ui.label("Theme:");
                egui::ComboBox::from_id_source(egui::Id::new("theme_selector"))
                    .selected_text(self.ui_theme_mode.display_name())
                    .width(180.0)
                    .show_ui(ui, |ui| {
                        let themes = [
                            UiThemeMode::Dark,
                            UiThemeMode::Light,
                            UiThemeMode::System,
                        ];
                        for theme in themes {
                            let is_selected = self.ui_theme_mode == theme;
                            if ui.selectable_label(is_selected, theme.display_name()).clicked() {
                                if self.ui_theme_mode != theme {
                                    self.pending_theme_change = Some(theme);
                                    self.ui_theme_mode = theme;
                                }
                            }
                        }
                    });
            });
        });

        ui.add_space(20.0);

        ui.group(|ui| {
            ui.label(egui::RichText::new("Terminal Settings").strong().size(16.0));
            ui.add_space(10.0);

            // Font Family Selection
            ui.horizontal(|ui| {
                ui.label("Font Family:");
                let available_fonts = [
                    ("Cascadia Code", "Cascadia Code"),
                    ("JetBrains Mono", "JetBrains Mono"),
                    ("Fira Code", "Fira Code"),
                    ("Consolas", "Consolas"),
                    ("Monaco", "Monaco"),
                    ("Source Code Pro", "Source Code Pro"),
                    ("Ubuntu Mono", "Ubuntu Mono"),
                ];

                egui::ComboBox::from_id_source(egui::Id::new("font_family_selector"))
                    .selected_text(&self.terminal_font_family)
                    .width(200.0)
                    .show_ui(ui, |ui| {
                        for (value, display) in available_fonts {
                            let is_selected = self.terminal_font_family == value;
                            if ui.selectable_label(is_selected, display).clicked() {
                                if self.terminal_font_family != value {
                                    self.terminal_font_family = value.to_string();
                                    self.pending_font_change = true;
                                    tracing::info!("Font family changed to: {}", value);
                                }
                            }
                        }
                    });
            });

            ui.add_space(10.0);

            // Font Size Control
            ui.horizontal(|ui| {
                ui.label("Font Size:");
                let response = ui.add(
                    egui::DragValue::new(&mut self.terminal_font_size)
                        .speed(0.5)
                        .range(8.0..=32.0)
                        .suffix(" px")
                );
                if response.changed() {
                    self.pending_font_change = true;
                    tracing::info!("Font size changed to: {}px", self.terminal_font_size);
                }

                ui.add_space(10.0);

                // Reset button
                if ui.button("Reset to 14px").clicked() {
                    self.terminal_font_size = 14.0;
                    self.pending_font_change = true;
                    tracing::info!("Font size reset to default");
                }
            });

            ui.add_space(10.0);

            // Font Zoom Control
            ui.horizontal(|ui| {
                ui.label("Font Zoom:");
                let response = ui.add(
                    egui::Slider::new(&mut self.terminal_font_zoom, 0.5..=2.0)
                        .text("x")
                        .step_by(0.1)
                );
                if response.changed() {
                    self.pending_font_change = true;
                    tracing::info!("Font zoom changed to: {}", self.terminal_font_zoom);
                }

                ui.add_space(10.0);

                // Reset button
                if ui.button("Reset").clicked() {
                    self.terminal_font_zoom = 1.0;
                    self.pending_font_change = true;
                    tracing::info!("Font zoom reset to default");
                }
            });

            ui.add_space(15.0);
            ui.separator();
            ui.add_space(10.0);

            // Terminal Behavior Settings (with actual state binding)
            ui.label("Terminal Behavior:");
            ui.add_space(5.0);

            if ui.checkbox(&mut self.terminal_use_webgl, "Use WebGL terminal (faster)").changed() {
                self.pending_font_change = true;
                tracing::info!("WebGL setting changed to: {}", self.terminal_use_webgl);
            }

            if ui.checkbox(&mut self.terminal_auto_scroll, "Auto-scroll terminal output").changed() {
                self.pending_font_change = true;
                tracing::info!("Auto-scroll changed to: {}", self.terminal_auto_scroll);
            }

            if ui.checkbox(&mut self.terminal_copy_on_select, "Copy on select").changed() {
                self.pending_font_change = true;
                tracing::info!("Copy on select changed to: {}", self.terminal_copy_on_select);
            }

            ui.add_space(15.0);

            // Preview of current settings
            ui.separator();
            ui.add_space(5.0);
            let effective_size = self.terminal_font_size * self.terminal_font_zoom;
            ui.label(egui::RichText::new(format!(
                "Preview: {} at {:.1}px (effective: {:.1}px)",
                self.terminal_font_family, self.terminal_font_size, effective_size
            )).color(egui::Color32::from_rgb(100, 180, 255)).size(12.0));
        });

        // Save button
        ui.add_space(15.0);
        ui.horizontal(|ui| {
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.add(
                    egui::Button::new("💾 Save Settings")
                        .min_size([140.0, 36.0].into())
                        .fill(egui::Color32::from_rgb(64, 156, 255))
                ).clicked() {
                    self.save_settings();
                }
            });
        });
    }

    fn render_import_export_tab(&mut self, ui: &mut egui::Ui) {
        ui.heading("Import & Export Configuration");
        ui.add_space(20.0);

        // Import Section
        ui.group(|ui| {
            ui.label(egui::RichText::new("Import Configuration").strong().size(16.0));
            ui.add_space(10.0);

            ui.label("Import servers, groups, and settings from various formats:");
            ui.add_space(5.0);

            ui.horizontal(|ui| {
                ui.label("Format:");
                egui::ComboBox::from_id_source(egui::Id::new("import_format"))
                    .selected_text(format!("{:?}", self.import_format))
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut self.import_format, ImportFormat::AutoDetect, "Auto-detect");
                        ui.selectable_value(&mut self.import_format, ImportFormat::Json, "JSON (.json)");
                        ui.selectable_value(&mut self.import_format, ImportFormat::JsonEncrypted, "Encrypted JSON (.json.enc)");
                        ui.selectable_value(&mut self.import_format, ImportFormat::Csv, "CSV (.csv)");
                        ui.selectable_value(&mut self.import_format, ImportFormat::SshConfig, "SSH Config (.ssh/config)");
                    });
            });

            ui.add_space(10.0);

            ui.horizontal(|ui| {
                ui.label("Conflict Resolution:");
                ui.radio_value(&mut self.conflict_resolution, ConflictResolution::Skip, "Skip existing");
                ui.radio_value(&mut self.conflict_resolution, ConflictResolution::Overwrite, "Overwrite");
                ui.radio_value(&mut self.conflict_resolution, ConflictResolution::Merge, "Merge");
            });

            ui.add_space(10.0);

            if ui.add(egui::Button::new("📁 Import from File...")
                .min_size([150.0, 40.0].into())
                .fill(egui::Color32::from_rgb(64, 156, 255)))
                .clicked() {
                self.show_import_dialog = true;
            }

            if ui.button("Import from ~/.ssh/config").clicked() {
                // Try to import from default SSH config location
                self.import_from_ssh_config();
            }
        });

        ui.add_space(20.0);

        // Export Section
        ui.group(|ui| {
            ui.label(egui::RichText::new("Export Configuration").strong().size(16.0));
            ui.add_space(10.0);

            ui.label("Export your configuration for backup or migration:");
            ui.add_space(5.0);

            ui.horizontal(|ui| {
                ui.label("Format:");
                egui::ComboBox::from_id_source(egui::Id::new("export_format"))
                    .selected_text(self.export_format.to_string())
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut self.export_format, ExportFormat::Json, "JSON");
                        ui.selectable_value(&mut self.export_format, ExportFormat::JsonEncrypted, "Encrypted JSON");
                        ui.selectable_value(&mut self.export_format, ExportFormat::Csv, "CSV (servers only)");
                        ui.selectable_value(&mut self.export_format, ExportFormat::SshConfig, "SSH Config");
                    });
            });

            if self.export_format == ExportFormat::JsonEncrypted {
                ui.add_space(10.0);
                ui.horizontal(|ui| {
                    ui.label("Password:");
                    ui.add(egui::TextEdit::singleline(&mut self.export_password)
                        .password(true)
                        .desired_width(200.0));
                });
                ui.checkbox(&mut self.export_include_secrets, "Include passwords (encrypted)");
            }

            ui.add_space(10.0);

            if ui.add(egui::Button::new("💾 Export to File...")
                .min_size([150.0, 40.0].into())
                .fill(egui::Color32::from_rgb(72, 199, 116)))
                .clicked() {
                self.show_export_dialog = true;
            }
        });
    }

    fn render_cloud_sync_tab(&mut self, ui: &mut egui::Ui) {
        ui.heading("Cloud Synchronization (Pro)");
        ui.add_space(20.0);

        ui.group(|ui| {
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("Enable Cloud Sync").strong());
                ui.checkbox(&mut self.cloud_sync_enabled, "");
            });

            if self.cloud_sync_enabled {
                ui.add_space(10.0);

                ui.horizontal(|ui| {
                    ui.label("Provider:");
                    egui::ComboBox::from_id_source(egui::Id::new("cloud_provider"))
                        .selected_text(format!("{:?}", self.cloud_provider))
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut self.cloud_provider, CloudProvider::Dropbox, "Dropbox");
                            ui.selectable_value(&mut self.cloud_provider, CloudProvider::GoogleDrive, "Google Drive");
                            ui.selectable_value(&mut self.cloud_provider, CloudProvider::OneDrive, "OneDrive");
                            ui.selectable_value(&mut self.cloud_provider, CloudProvider::Custom, "Custom (S3/WebDAV)");
                        });
                });

                ui.add_space(10.0);

                ui.horizontal(|ui| {
                    ui.label("API Key/Token:");
                    ui.add(egui::TextEdit::singleline(&mut self.cloud_api_key)
                        .password(true)
                        .desired_width(300.0));
                });

                ui.add_space(10.0);

                ui.horizontal(|ui| {
                    if ui.button("🔗 Connect").clicked() {
                        self.show_cloud_sync_dialog = true;
                    }
                    if ui.button("↻ Sync Now").clicked() {
                        // Trigger sync
                    }
                });

                ui.add_space(10.0);

                ui.label("Last sync: Never");
                ui.label("Status: Not connected");
            }
        });

        ui.add_space(20.0);

        ui.group(|ui| {
            ui.label("Sync Options");
            ui.add_space(10.0);

            ui.checkbox(&mut true, "Sync servers");
            ui.checkbox(&mut true, "Sync groups");
            ui.checkbox(&mut true, "Sync snippets");
            ui.checkbox(&mut false, "Sync identities (secure)");
            ui.checkbox(&mut true, "Auto-sync on change");
        });
    }

    fn render_security_tab(&mut self, ui: &mut egui::Ui) {
        ui.heading("Security Settings");
        ui.add_space(20.0);

        ui.group(|ui| {
            ui.label("Master Password");
            ui.add_space(10.0);

            ui.horizontal(|ui| {
                ui.label("Current Password:");
                ui.add(egui::TextEdit::singleline(&mut self.master_password)
                    .password(true)
                    .desired_width(200.0));
            });

            ui.add_space(10.0);

            if ui.button("🔒 Set/Change Master Password").clicked() {
                // Change master password
            }

            ui.checkbox(&mut self.use_encryption, "Enable database encryption");
        });

        ui.add_space(20.0);

        ui.group(|ui| {
            ui.label("Keychain Integration");
            ui.add_space(10.0);

            ui.checkbox(&mut true, "Store passwords in system keychain");
            ui.checkbox(&mut true, "Lock after inactivity (15 min)");

            if ui.button("🔑 Manage SSH Keys").clicked() {
                // Open SSH key manager
            }
        });
    }

    fn render_appearance_tab(&mut self, ui: &mut egui::Ui) {
        ui.heading("Appearance");
        ui.add_space(20.0);

        ui.group(|ui| {
            ui.label("UI Theme Mode");
            ui.add_space(10.0);

            ui.horizontal(|ui| {
                // Dark mode button
                let dark_selected = self.ui_theme_mode == UiThemeMode::Dark;
                if ui.selectable_label(dark_selected, "🌙 Dark").clicked() && !dark_selected {
                    self.ui_theme_mode = UiThemeMode::Dark;
                    self.pending_theme_change = Some(UiThemeMode::Dark);
                }

                // Light mode button
                let light_selected = self.ui_theme_mode == UiThemeMode::Light;
                if ui.selectable_label(light_selected, "☀ Light").clicked() && !light_selected {
                    self.ui_theme_mode = UiThemeMode::Light;
                    self.pending_theme_change = Some(UiThemeMode::Light);
                }

                // System mode button
                let system_selected = self.ui_theme_mode == UiThemeMode::System;
                if ui.selectable_label(system_selected, "🖥 System").clicked() && !system_selected {
                    self.ui_theme_mode = UiThemeMode::System;
                    self.pending_theme_change = Some(UiThemeMode::System);
                }
            });

            ui.add_space(10.0);

            // Show current theme status
            let theme_status = match self.ui_theme_mode {
                UiThemeMode::Dark => "Current: Dark Mode",
                UiThemeMode::Light => "Current: Light Mode",
                UiThemeMode::System => "Current: Following System",
            };
            ui.label(egui::RichText::new(theme_status).color(egui::Color32::from_rgb(100, 180, 255)));
        });

        ui.add_space(20.0);

        ui.group(|ui| {
            ui.label("Accessibility");
            ui.add_space(10.0);

            // FIX: Bind checkbox directly to self field and use changed() for immediate update
            if ui.checkbox(&mut self.high_contrast, "High contrast mode").changed() {
                self.pending_accessibility_change = true;
                tracing::info!("High contrast mode changed to: {}", self.high_contrast);
            }

            if ui.checkbox(&mut self.reduce_motion, "Reduce motion").changed() {
                self.pending_accessibility_change = true;
                tracing::info!("Reduce motion changed to: {}", self.reduce_motion);
            }

            if ui.checkbox(&mut self.large_text, "Large text").changed() {
                self.pending_accessibility_change = true;
                tracing::info!("Large text changed to: {}", self.large_text);
            }
        });

        ui.add_space(20.0);

        ui.label("For advanced terminal theme options, use the Theme Gallery (🎨 button in toolbar)");
    }

    /// Placeholder for themes tab - actual rendering is done from main.rs
    fn render_themes_placeholder(&mut self, ui: &mut egui::Ui) {
        ui.heading("Terminal Themes");
        ui.add_space(20.0);

        ui.label("Click the 🎨 Theme Gallery button in the toolbar to:");
        ui.add_space(10.0);
        ui.label("• Browse 10+ built-in themes (One Dark, Dracula, Solarized, etc.)");
        ui.label("• Create and edit custom themes");
        ui.label("• Import VS Code themes");
        ui.label("• Configure dynamic day/night switching");
        ui.label("• Set terminal background images");
        ui.label("• Customize cursor style and fonts");
        ui.label("• Enable semantic syntax highlighting");

        ui.add_space(20.0);
        ui.label("The Theme Gallery provides a visual preview of all themes!");
    }

    fn render_import_dialog(&mut self, ctx: &egui::Context, view_model: &Arc<Mutex<crate::viewmodels::AppViewModel>>) {
        egui::Window::new("Import Configuration")
            .collapsible(false)
            .resizable(false)
            .default_size([500.0, 400.0])
            .show(ctx, |ui| {
                ui.label("Select a file to import:");
                ui.add_space(10.0);

                if let Some(ref path) = self.import_file_path {
                    ui.label(format!("Selected: {}", path.display()));
                } else {
                    ui.label("No file selected");
                }

                ui.add_space(10.0);

                if ui.button("📁 Browse...").clicked() {
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("All supported formats", &["json", "csv", "enc"])
                        .add_filter("JSON", &["json"])
                        .add_filter("CSV", &["csv"])
                        .add_filter("SSH Config", &["config", "", "*"])
                        .pick_file() {
                        self.import_file_path = Some(path);
                    }
                }

                ui.add_space(20.0);

                // Password field for encrypted imports
                if self.import_format == ImportFormat::JsonEncrypted {
                    ui.horizontal(|ui| {
                        ui.label("Password:");
                        ui.add(egui::TextEdit::singleline(&mut self.master_password)
                            .password(true)
                            .desired_width(200.0));
                    });
                    ui.add_space(10.0);
                }

                ui.separator();

                ui.horizontal(|ui| {
                    if ui.button("Cancel").clicked() {
                        self.show_import_dialog = false;
                        self.import_file_path = None;
                    }

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let can_import = self.import_file_path.is_some() &&
                            (self.import_format != ImportFormat::JsonEncrypted || !self.master_password.is_empty());

                        if ui.add_enabled(can_import, egui::Button::new("Import")).clicked() {
                            self.perform_import(view_model);
                            self.show_import_dialog = false;
                            self.show_import_result = true;
                        }
                    });
                });
            });
    }

    fn render_export_dialog(&mut self, ctx: &egui::Context, view_model: &Arc<Mutex<crate::viewmodels::AppViewModel>>) {
        egui::Window::new("Export Configuration")
            .collapsible(false)
            .resizable(false)
            .default_size([500.0, 400.0])
            .show(ctx, |ui| {
                ui.label("Choose where to save the export:");
                ui.add_space(10.0);

                let default_filename = match self.export_format {
                    ExportFormat::Json => "easyssh-export.json",
                    ExportFormat::JsonEncrypted => "easyssh-export.json.enc",
                    ExportFormat::Csv => "easyssh-servers.csv",
                    ExportFormat::SshConfig => "config",
                };

                ui.label(format!("Default filename: {}", default_filename));
                ui.add_space(10.0);

                if ui.button("💾 Save As...").clicked() {
                    let mut dialog = rfd::FileDialog::new();

                    dialog = match self.export_format {
                        ExportFormat::Json => dialog.add_filter("JSON", &["json"]),
                        ExportFormat::JsonEncrypted => dialog.add_filter("Encrypted JSON", &["enc", "json"]),
                        ExportFormat::Csv => dialog.add_filter("CSV", &["csv"]),
                        ExportFormat::SshConfig => dialog.add_filter("SSH Config", &["config", ""]),
                    };

                    if let Some(path) = dialog.set_file_name(default_filename).save_file() {
                        self.export_path = Some(path);
                        self.perform_export(view_model);
                        self.show_export_dialog = false;
                    }
                }

                ui.add_space(20.0);
                ui.separator();

                ui.horizontal(|ui| {
                    if ui.button("Cancel").clicked() {
                        self.show_export_dialog = false;
                    }
                });
            });
    }

    fn render_import_result(&mut self, ctx: &egui::Context) {
        egui::Window::new("Import Result")
            .collapsible(false)
            .resizable(false)
            .default_size([400.0, 300.0])
            .show(ctx, |ui| {
                if let Some(ref result) = self.import_result {
                    ui.label(egui::RichText::new("Import Complete!").strong().size(18.0));
                    ui.add_space(15.0);

                    ui.label(format!("✓ Servers imported: {}", result.servers_imported));
                    ui.label(format!("⊘ Servers skipped: {}", result.servers_skipped));
                    if result.servers_merged > 0 {
                        ui.label(format!("⟲ Servers merged: {}", result.servers_merged));
                    }
                    ui.label(format!("✓ Groups imported: {}", result.groups_imported));
                    ui.label(format!("✓ Identities imported: {}", result.identities_imported));
                    ui.label(format!("✓ Snippets imported: {}", result.snippets_imported));
                    ui.label(format!("✓ Tags imported: {}", result.tags_imported));

                    if !result.errors.is_empty() {
                        ui.add_space(10.0);
                        ui.separator();
                        ui.label(egui::RichText::new("Errors:").color(egui::Color32::RED));
                        for error in &result.errors {
                            ui.label(egui::RichText::new(format!("• {}", error)).color(egui::Color32::RED).small());
                        }
                    }
                } else {
                    ui.label("No import result available.");
                }

                ui.add_space(20.0);
                ui.separator();

                if ui.button("OK").clicked() {
                    self.show_import_result = false;
                    self.import_result = None;
                    self.import_file_path = None;
                    self.import_content.clear();
                }
            });
    }

    fn render_cloud_sync_dialog(&mut self, ctx: &egui::Context) {
        egui::Window::new("Cloud Sync Setup")
            .collapsible(false)
            .resizable(false)
            .default_size([400.0, 300.0])
            .show(ctx, |ui| {
                ui.label(format!("Connecting to {:?}...", self.cloud_provider));
                ui.add_space(20.0);
                ui.label("This feature will be available in the Pro version.");
                ui.add_space(20.0);

                if ui.button("OK").clicked() {
                    self.show_cloud_sync_dialog = false;
                }
            });
    }

    // Action methods
    fn perform_import(&mut self, view_model: &Arc<Mutex<crate::viewmodels::AppViewModel>>) {
        if let Some(ref path) = self.import_file_path {
            // Read file content
            match std::fs::read_to_string(path) {
                Ok(content) => {
                    self.import_content = content;

                    // Detect format if auto
                    if self.import_format == ImportFormat::AutoDetect {
                        self.import_format = self.detect_format(path);
                    }

                    // Get database from view model
                    let result = view_model.lock().unwrap()
                        .import_config(&self.import_content, self.import_format.clone(), self.conflict_resolution.clone());

                    self.import_result = result.ok();
                }
                Err(e) => {
                    self.import_result = Some(ImportResult {
                        errors: vec![format!("Failed to read file: {}", e)],
                        ..Default::default()
                    });
                }
            }
        }
    }

    fn perform_export(&mut self, view_model: &Arc<Mutex<crate::viewmodels::AppViewModel>>) {
        if let Some(ref path) = self.export_path {
            let result = view_model.lock().unwrap().export_config(
                self.export_format.clone(),
                &self.export_password,
                self.export_include_secrets,
            );

            match result {
                Ok(content) => {
                    if let Err(e) = std::fs::write(path, content) {
                        eprintln!("Failed to write export: {}", e);
                    }
                }
                Err(e) => {
                    eprintln!("Failed to export: {}", e);
                }
            }
        }
    }

    fn import_from_ssh_config(&mut self) {
        let home_dir = dirs::home_dir();
        if let Some(home) = home_dir {
            let ssh_config = home.join(".ssh").join("config");
            if ssh_config.exists() {
                self.import_file_path = Some(ssh_config);
                self.import_format = ImportFormat::SshConfig;
            }
        }
    }

    fn detect_format(&self, path: &PathBuf) -> ImportFormat {
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        match ext {
            "json" => {
                // Check if encrypted
                let content = std::fs::read_to_string(path).unwrap_or_default();
                if content.contains("salt") && content.contains("data") {
                    ImportFormat::JsonEncrypted
                } else {
                    ImportFormat::Json
                }
            }
            "csv" => ImportFormat::Csv,
            "enc" => ImportFormat::JsonEncrypted,
            "config" => ImportFormat::SshConfig,
            _ => ImportFormat::AutoDetect,
        }
    }
}
