#![allow(dead_code)]

//! Enterprise Password Vault UI for Windows
//!
//! Provides a complete user interface for managing passwords, SSH keys,
//! API keys, TOTP codes, and secure notes with enterprise security features.

use crate::design::Theme;
use crate::viewmodels::{ConnectionViewModel, ServerViewModel};
use easyssh_core::vault::{
    EnterpriseVault, VaultItemType, SecurityLevel, HardwareAuthMethod,
    VaultItemMetadata, PasswordGeneratorConfig, PasswordStrength, SecurityAuditResult,
    AutofillConfig, UnlockOptions, TrustedContact, EmergencyAccessLevel, InvitationStatus, NoteFormat,
};
use zeroize::Zeroize;

/// Enterprise Vault Window State
pub struct EnterpriseVaultWindow {
    pub open: bool,
    pub vault: Option<EnterpriseVault>,
    pub active_tab: VaultTab,
    pub theme: Theme,

    // Unlock dialog state
    pub unlock_dialog_open: bool,
    pub master_password_input: String,
    pub use_biometric: bool,
    pub unlock_error: Option<String>,

    // Password generator state
    pub generator_config: PasswordGeneratorConfig,
    pub generated_password: String,
    pub password_strength: Option<PasswordStrength>,

    // Add item dialog state
    pub add_dialog_open: bool,
    pub new_item_type: VaultItemType,
    pub new_item_name: String,
    pub new_username: String,
    pub new_password: String,
    pub new_url: String,
    pub new_notes: String,
    pub new_ssh_private_key: String,
    pub new_ssh_public_key: String,
    pub new_api_key: String,
    pub new_totp_secret: String,

    // List view state
    pub search_query: String,
    pub selected_folder: Option<String>,
    pub selected_item: Option<String>,
    pub items: Vec<VaultItemMetadata>,
    pub filtered_items: Vec<VaultItemMetadata>,

    // Security audit state
    pub audit_result: Option<SecurityAuditResult>,
    pub audit_in_progress: bool,

    // TOTP state
    pub totp_codes: HashMap<String, String>,
    pub totp_time_remaining: u8,

    // Hardware auth state
    pub available_hardware_methods: Vec<HardwareAuthMethod>,
    pub selected_hardware_method: Option<HardwareAuthMethod>,

    // Emergency access state
    pub trusted_contacts: Vec<TrustedContact>,
    pub new_contact_name: String,
    pub new_contact_email: String,
    pub new_contact_access_level: EmergencyAccessLevel,

    // Settings
    pub autofill_config: AutofillConfig,
    pub auto_lock_timeout: u32,

    // Password reveal toggle
    pub show_password: bool,
    pub show_ssh_key: bool,
}

use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VaultTab {
    AllItems,
    Passwords,
    SshKeys,
    ApiKeys,
    Totp,
    SecureNotes,
    Generator,
    SecurityAudit,
    EmergencyAccess,
    Settings,
}

impl std::fmt::Display for VaultTab {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VaultTab::AllItems => write!(f, "All Items"),
            VaultTab::Passwords => write!(f, "Passwords"),
            VaultTab::SshKeys => write!(f, "SSH Keys"),
            VaultTab::ApiKeys => write!(f, "API Keys"),
            VaultTab::Totp => write!(f, "2FA Codes"),
            VaultTab::SecureNotes => write!(f, "Secure Notes"),
            VaultTab::Generator => write!(f, "Generator"),
            VaultTab::SecurityAudit => write!(f, "Security Audit"),
            VaultTab::EmergencyAccess => write!(f, "Emergency Access"),
            VaultTab::Settings => write!(f, "Settings"),
        }
    }
}

impl EnterpriseVaultWindow {
    /// Create new vault window
    pub fn new(theme: Theme) -> Self {
        let vault = EnterpriseVault::new().ok();
        let generator_config = PasswordGeneratorConfig::default();

        Self {
            open: false,
            vault,
            active_tab: VaultTab::AllItems,
            theme,
            unlock_dialog_open: false,
            master_password_input: String::new(),
            use_biometric: true,
            unlock_error: None,
            generator_config,
            generated_password: String::new(),
            password_strength: None,
            add_dialog_open: false,
            new_item_type: VaultItemType::Password,
            new_item_name: String::new(),
            new_username: String::new(),
            new_password: String::new(),
            new_url: String::new(),
            new_notes: String::new(),
            new_ssh_private_key: String::new(),
            new_ssh_public_key: String::new(),
            new_api_key: String::new(),
            new_totp_secret: String::new(),
            search_query: String::new(),
            selected_folder: None,
            selected_item: None,
            items: Vec::new(),
            filtered_items: Vec::new(),
            audit_result: None,
            audit_in_progress: false,
            totp_codes: HashMap::new(),
            totp_time_remaining: 30,
            available_hardware_methods: Vec::new(),
            selected_hardware_method: None,
            trusted_contacts: Vec::new(),
            new_contact_name: String::new(),
            new_contact_email: String::new(),
            new_contact_access_level: EmergencyAccessLevel::ViewOnly,
            autofill_config: AutofillConfig::default(),
            auto_lock_timeout: 15,
            show_password: false,
            show_ssh_key: false,
        }
    }

    /// Show the vault window
    pub fn show(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if !self.open {
            return;
        }

        let window_title = "Enterprise Password Vault";

        egui::Window::new(window_title)
            .open(&mut self.open)
            .min_size([900.0, 600.0])
            .default_size([1000.0, 700.0])
            .show(ctx, |ui| {
                // Check if vault is unlocked
                if let Some(vault) = &self.vault {
                    if !vault.is_unlocked() && !self.unlock_dialog_open {
                        self.unlock_dialog_open = true;
                    }
                }

                // Show unlock dialog if needed
                if self.unlock_dialog_open {
                    self.show_unlock_dialog(ctx);
                }

                // Main vault UI
                if self.vault.as_ref().map(|v| v.is_unlocked()).unwrap_or(false) {
                    self.show_main_vault_ui(ui);
                }
            });

        // Update TOTP codes periodically
        self.update_totp_codes();
    }

    /// Show unlock dialog
    fn show_unlock_dialog(&mut self, ctx: &egui::Context) {
        egui::Window::new("Unlock Vault")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                ui.heading("Enter Master Password");
                ui.add_space(16.0);

                // Master password input
                ui.label("Master Password:");
                ui.add(
                    egui::TextEdit::singleline(&mut self.master_password_input)
                        .password(true)
                        .hint_text("Enter your master password...")
                );

                ui.add_space(8.0);

                // Biometric option
                if !self.available_hardware_methods.is_empty() {
                    ui.checkbox(&mut self.use_biometric, "Use Windows Hello (Biometric)");
                    ui.add_space(8.0);
                }

                // Error message
                if let Some(error) = &self.unlock_error {
                    ui.colored_label(egui::Color32::RED, format!("Error: {}", error));
                    ui.add_space(8.0);
                }

                ui.add_space(16.0);

                // Buttons
                ui.horizontal(|ui| {
                    if ui.button("🔓 Unlock").clicked() {
                        self.attempt_unlock();
                    }

                    if ui.button("Cancel").clicked() {
                        self.unlock_dialog_open = false;
                        self.open = false;
                    }
                });

                ui.add_space(8.0);
                ui.label(egui::RichText::new("🔒 Your vault is encrypted with AES-256-GCM")
                    .small()
                    .color(egui::Color32::GRAY));
            });
    }

    /// Attempt to unlock the vault
    fn attempt_unlock(&mut self) {
        if let Some(vault) = &self.vault {
            let options = UnlockOptions {
                master_password: Some(self.master_password_input.clone()),
                biometric: self.use_biometric,
                hardware_key: self.selected_hardware_method,
                pin: None,
                timeout_minutes: Some(self.auto_lock_timeout),
            };

            match vault.unlock(options) {
                Ok(true) => {
                    self.unlock_dialog_open = false;
                    self.unlock_error = None;
                    self.master_password_input.zeroize();
                    self.refresh_items();
                }
                Ok(false) => {
                    self.unlock_error = Some("Invalid password".to_string());
                }
                Err(e) => {
                    self.unlock_error = Some(e.to_string());
                }
            }
        }
    }

    /// Show main vault UI
    fn show_main_vault_ui(&mut self, ui: &mut egui::Ui) {
        egui::SidePanel::left("vault_sidebar")
            .exact_width(180.0)
            .show_inside(ui, |ui| {
                ui.heading("Vault");
                ui.add_space(16.0);

                // Tab buttons
                let tabs = vec![
                    VaultTab::AllItems,
                    VaultTab::Passwords,
                    VaultTab::SshKeys,
                    VaultTab::ApiKeys,
                    VaultTab::Totp,
                    VaultTab::SecureNotes,
                ];

                for tab in tabs {
                    let is_active = self.active_tab == tab;
                    let button = egui::Button::new(format!("{} {}",
                        self.tab_icon(&tab),
                        tab
                    ))
                    .fill(if is_active {
                        self.theme.accent_color
                    } else {
                        egui::Color32::TRANSPARENT
                    });

                    if ui.add_sized([160.0, 32.0], button).clicked() {
                        self.active_tab = tab;
                        self.refresh_items();
                    }
                }

                ui.add_space(24.0);
                ui.separator();
                ui.add_space(16.0);

                // Tools section
                ui.label(egui::RichText::new("TOOLS").small().strong());
                ui.add_space(8.0);

                let tools = vec![
                    VaultTab::Generator,
                    VaultTab::SecurityAudit,
                    VaultTab::EmergencyAccess,
                    VaultTab::Settings,
                ];

                for tab in tools {
                    let is_active = self.active_tab == tab;
                    let button = egui::Button::new(format!("{} {}",
                        self.tab_icon(&tab),
                        tab
                    ))
                    .fill(if is_active {
                        self.theme.accent_color
                    } else {
                        egui::Color32::TRANSPARENT
                    });

                    if ui.add_sized([160.0, 32.0], button).clicked() {
                        self.active_tab = tab;
                    }
                }

                ui.add_space(24.0);

                // Lock vault button
                if ui.button("🔒 Lock Vault").clicked() {
                    if let Some(vault) = &self.vault {
                        vault.lock();
                    }
                }
            });

        // Main content area
        egui::CentralPanel::default().show_inside(ui, |ui| {
            match self.active_tab {
                VaultTab::AllItems |
                VaultTab::Passwords |
                VaultTab::SshKeys |
                VaultTab::ApiKeys |
                VaultTab::Totp |
                VaultTab::SecureNotes => {
                    self.show_items_view(ui);
                }
                VaultTab::Generator => {
                    self.show_generator_tab(ui);
                }
                VaultTab::SecurityAudit => {
                    self.show_security_audit_tab(ui);
                }
                VaultTab::EmergencyAccess => {
                    self.show_emergency_access_tab(ui);
                }
                VaultTab::Settings => {
                    self.show_settings_tab(ui);
                }
            }
        });
    }

    /// Get icon for tab
    fn tab_icon(&self, tab: &VaultTab) -> &'static str {
        match tab {
            VaultTab::AllItems => "📦",
            VaultTab::Passwords => "🔐",
            VaultTab::SshKeys => "🔑",
            VaultTab::ApiKeys => "🌐",
            VaultTab::Totp => "⏱️",
            VaultTab::SecureNotes => "📝",
            VaultTab::Generator => "⚡",
            VaultTab::SecurityAudit => "🛡️",
            VaultTab::EmergencyAccess => "🚨",
            VaultTab::Settings => "⚙️",
        }
    }

    /// Show items list view
    fn show_items_view(&mut self, ui: &mut egui::Ui) {
        // Search bar
        ui.horizontal(|ui| {
            ui.add(
                egui::TextEdit::singleline(&mut self.search_query)
                    .hint_text("🔍 Search vault...")
                    .desired_width(300.0)
            );

            if ui.button("🔍").clicked() {
                self.search_items();
            }

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("➕ Add Item").clicked() {
                    self.add_dialog_open = true;
                    self.reset_add_dialog();
                }
            });
        });

        ui.add_space(8.0);
        ui.separator();
        ui.add_space(8.0);

        // Items list
        egui::ScrollArea::vertical().show(ui, |ui| {
            if self.filtered_items.is_empty() {
                ui.centered_and_justified(|ui| {
                    ui.label("No items found. Click 'Add Item' to create one.");
                });
            } else {
                for item in &self.filtered_items.clone() {
                    self.show_item_card(ui, item);
                }
            }
        });

        // Add item dialog
        if self.add_dialog_open {
            self.show_add_dialog(ui.ctx());
        }
    }

    /// Show item card
    fn show_item_card(&mut self, ui: &mut egui::Ui, item: &VaultItemMetadata) {
        let response = egui::Frame::group(ui.style())
            .fill(self.theme.card_background)
            .rounding(8.0)
            .show(ui, |ui| {
                ui.set_min_width(ui.available_width());

                ui.horizontal(|ui| {
                    // Icon
                    ui.label(self.item_type_icon(&item.item_type));

                    // Name and info
                    ui.vertical(|ui| {
                        ui.label(egui::RichText::new(&item.name).strong());

                        let meta_text = if let Some(last_accessed) = item.last_accessed {
                            format!("Accessed {} times, last: {}",
                                item.access_count,
                                last_accessed.format("%Y-%m-%d %H:%M")
                            )
                        } else {
                            format!("Created: {}", item.created_at.format("%Y-%m-%d"))
                        };
                        ui.label(egui::RichText::new(meta_text).small().color(egui::Color32::GRAY));

                        // Tags
                        if !item.tags.is_empty() {
                            ui.horizontal_wrapped(|ui| {
                                for tag in &item.tags {
                                    ui.label(
                                        egui::RichText::new(format!("#{}", tag))
                                            .small()
                                            .color(self.theme.accent_color)
                                    );
                                }
                            });
                        }
                    });

                    // Actions
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        // Copy button based on type
                        match item.item_type {
                            VaultItemType::Password => {
                                if ui.button("📋 Copy Password").clicked() {
                                    self.copy_password(&item.id);
                                }
                                if ui.button("👤 Copy Username").clicked() {
                                    self.copy_username(&item.id);
                                }
                            }
                            VaultItemType::SshKey => {
                                if ui.button("📋 Copy Public Key").clicked() {
                                    self.copy_ssh_public_key(&item.id);
                                }
                            }
                            VaultItemType::ApiKey => {
                                if ui.button("📋 Copy API Key").clicked() {
                                    self.copy_api_key(&item.id);
                                }
                            }
                            VaultItemType::TOTP => {
                                if let Some(code) = self.totp_codes.get(&item.id) {
                                    ui.monospace(format!("{} ({}s)", code, self.totp_time_remaining));
                                }
                                if ui.button("🔄 Refresh").clicked() {
                                    self.refresh_totp(&item.id);
                                }
                            }
                            _ => {}
                        }

                        // View/Edit button
                        if ui.button("👁️ View").clicked() {
                            self.selected_item = Some(item.id.clone());
                            self.show_item_detail_dialog(ui.ctx(), &item.id);
                        }
                    });
                });
            });

        if response.clicked() {
            self.selected_item = Some(item.id.clone());
        }
    }

    /// Get icon for item type
    fn item_type_icon(&self, item_type: &VaultItemType) -> &'static str {
        match item_type {
            VaultItemType::Password => "🔐",
            VaultItemType::SshKey => "🔑",
            VaultItemType::ApiKey => "🌐",
            VaultItemType::Certificate => "📜",
            VaultItemType::SecureNote => "📝",
            VaultItemType::TOTP => "⏱️",
            VaultItemType::CreditCard => "💳",
            VaultItemType::BankAccount => "🏦",
            VaultItemType::Identity => "🆔",
            VaultItemType::SoftwareLicense => "📄",
        }
    }

    /// Show add item dialog
    fn show_add_dialog(&mut self, ctx: &egui::Context) {
        egui::Window::new("Add New Item")
            .open(&mut self.add_dialog_open)
            .default_size([500.0, 400.0])
            .show(ctx, |ui| {
                // Item type selector
                ui.label("Item Type:");
                egui::ComboBox::from_id(egui::Id::new("item_type_selector"))
                    .selected_text(format!("{} {}",
                        self.item_type_icon(&self.new_item_type),
                        self.new_item_type
                    ))
                    .show_ui(ui, |ui| {
                        let types = vec![
                            VaultItemType::Password,
                            VaultItemType::SshKey,
                            VaultItemType::ApiKey,
                            VaultItemType::TOTP,
                            VaultItemType::SecureNote,
                        ];
                        for t in types {
                            ui.selectable_value(&mut self.new_item_type, t, format!("{} {}",
                                self.item_type_icon(&t),
                                t
                            ));
                        }
                    });

                ui.add_space(16.0);

                // Common fields
                ui.label("Name:");
                ui.text_edit_singleline(&mut self.new_item_name);

                ui.add_space(8.0);

                // Type-specific fields
                match self.new_item_type {
                    VaultItemType::Password => {
                        ui.label("Username:");
                        ui.text_edit_singleline(&mut self.new_username);
                        ui.add_space(8.0);

                        ui.label("Password:");
                        ui.horizontal(|ui| {
                            ui.add(
                                egui::TextEdit::singleline(&mut self.new_password)
                                    .password(!self.show_password)
                            );
                            if ui.button(if self.show_password { "🙈" } else { "👁️" }).clicked() {
                                self.show_password = !self.show_password;
                            }
                        });

                        // Password strength indicator
                        if !self.new_password.is_empty() {
                            let strength = vault::EnterpriseVault::analyze_password_strength(&self.new_password);
                            self.show_strength_indicator(ui, &strength);
                        }

                        ui.add_space(8.0);
                        if ui.button("⚡ Generate Strong Password").clicked() {
                            if let Ok(password) = vault::EnterpriseVault::generate_password_with_config(&self.generator_config) {
                                self.new_password = password;
                            }
                        }

                        ui.add_space(8.0);
                        ui.label("URL:");
                        ui.text_edit_singleline(&mut self.new_url);
                    }
                    VaultItemType::SshKey => {
                        ui.label("Private Key:");
                        ui.add(
                            egui::TextEdit::multiline(&mut self.new_ssh_private_key)
                                .desired_rows(5)
                                .font(egui::TextStyle::Monospace)
                        );
                        if ui.button("📂 Import from File").clicked() {
                            // Would open file picker
                        }

                        ui.add_space(8.0);
                        ui.label("Public Key:");
                        ui.add(
                            egui::TextEdit::multiline(&mut self.new_ssh_public_key)
                                .desired_rows(3)
                                .font(egui::TextStyle::Monospace)
                        );
                    }
                    VaultItemType::ApiKey => {
                        ui.label("API Key:");
                        ui.add(
                            egui::TextEdit::multiline(&mut self.new_api_key)
                                .desired_rows(3)
                                .font(egui::TextStyle::Monospace)
                        );
                        ui.add_space(8.0);
                        ui.label("Endpoint URL:");
                        ui.text_edit_singleline(&mut self.new_url);
                    }
                    VaultItemType::TOTP => {
                        ui.label("TOTP Secret:");
                        ui.horizontal(|ui| {
                            ui.add(
                                egui::TextEdit::singleline(&mut self.new_totp_secret)
                                    .password(!self.show_password)
                            );
                            if ui.button(if self.show_password { "🙈" } else { "👁️" }).clicked() {
                                self.show_password = !self.show_password;
                            }
                        });
                        ui.add_space(8.0);
                        ui.label("Or scan QR code...");
                    }
                    _ => {}
                }

                ui.add_space(8.0);
                ui.label("Notes:");
                ui.add(
                    egui::TextEdit::multiline(&mut self.new_notes)
                        .desired_rows(3)
                );

                ui.add_space(16.0);

                // Buttons
                ui.horizontal(|ui| {
                    if ui.button("💾 Save").clicked() {
                        self.save_new_item();
                    }
                    if ui.button("Cancel").clicked() {
                        self.add_dialog_open = false;
                    }
                });
            });
    }

    /// Show password strength indicator
    fn show_strength_indicator(&self, ui: &mut egui::Ui, strength: &PasswordStrength) {
        let (color, label) = if strength.score >= 80 {
            (egui::Color32::GREEN, "Strong")
        } else if strength.score >= 60 {
            (egui::Color32::YELLOW, "Good")
        } else if strength.score >= 40 {
            (egui::Color32::from_rgb(255, 165, 0), "Fair")
        } else {
            (egui::Color32::RED, "Weak")
        };

        ui.horizontal(|ui| {
            ui.label("Password Strength:");
            ui.colored_label(color, format!("{} ({} bits)", label, strength.entropy_bits as u32));
        });

        // Progress bar
        let progress = strength.score as f32 / 100.0;
        let bar_color = if strength.score >= 80 {
            egui::Color32::GREEN
        } else if strength.score >= 60 {
            egui::Color32::YELLOW
        } else if strength.score >= 40 {
            egui::Color32::from_rgb(255, 165, 0)
        } else {
            egui::Color32::RED
        };

        ui.add(
            egui::ProgressBar::new(progress)
                .fill(bar_color)
                .desired_width(200.0)
        );
    }

    /// Show item detail dialog
    fn show_item_detail_dialog(&mut self, ctx: &egui::Context, item_id: &str) {
        // This would show a detailed view of the item
        // For now, just a placeholder
    }

    /// Show password generator tab
    fn show_generator_tab(&mut self, ui: &mut egui::Ui) {
        ui.heading("Password Generator");
        ui.add_space(16.0);

        // Configuration
        egui::Frame::group(ui.style())
            .show(ui, |ui| {
                ui.label("Configuration");
                ui.add_space(8.0);

                ui.horizontal(|ui| {
                    ui.label("Length:");
                    ui.add(egui::Slider::new(&mut self.generator_config.length, 8..=128));
                });

                ui.checkbox(&mut self.generator_config.include_uppercase, "Include Uppercase (A-Z)");
                ui.checkbox(&mut self.generator_config.include_lowercase, "Include Lowercase (a-z)");
                ui.checkbox(&mut self.generator_config.include_numbers, "Include Numbers (0-9)");
                ui.checkbox(&mut self.generator_config.include_symbols, "Include Symbols (!@#$...)");
                ui.checkbox(&mut self.generator_config.exclude_ambiguous, "Exclude Ambiguous Characters (0, O, l, 1)");
                ui.checkbox(&mut self.generator_config.pronounceable, "Generate Memorable Passphrase");

                if self.generator_config.pronounceable {
                    ui.horizontal(|ui| {
                        ui.label("Words:");
                        ui.add(egui::Slider::new(&mut self.generator_config.word_count, 3..=10));
                    });
                    ui.label("Separator:");
                    ui.text_edit_singleline(&mut self.generator_config.word_separator);
                }
            });

        ui.add_space(16.0);

        // Generate button
        if ui.button("⚡ Generate Password").clicked() {
            if let Ok(password) = vault::EnterpriseVault::generate_password_with_config(&self.generator_config) {
                self.generated_password = password;
                self.password_strength = Some(vault::EnterpriseVault::analyze_password_strength(&self.generated_password));
            }
        }

        ui.add_space(16.0);

        // Result
        if !self.generated_password.is_empty() {
            egui::Frame::group(ui.style())
                .show(ui, |ui| {
                    ui.label("Generated Password:");
                    ui.add_space(8.0);

                    ui.horizontal(|ui| {
                        ui.add(
                            egui::TextEdit::singleline(&mut self.generated_password)
                                .desired_width(400.0)
                                .font(egui::TextStyle::Monospace)
                        );

                        if ui.button("📋 Copy").clicked() {
                            ui.output_mut(|o| o.copied_text = self.generated_password.clone());
                        }
                    });

                    // Show strength
                    if let Some(strength) = &self.password_strength {
                        self.show_strength_indicator(ui, strength);
                        ui.add_space(8.0);
                        ui.label(format!("Estimated crack time: {}", strength.crack_time_display));

                        if !strength.weaknesses.is_empty() {
                            ui.add_space(8.0);
                            ui.label("Potential weaknesses:");
                            for weakness in &strength.weaknesses {
                                ui.label(format!("  ⚠️ {}", weakness));
                            }
                        }
                    }
                });
        }

        // History
        ui.add_space(24.0);
        ui.separator();
        ui.add_space(16.0);
        ui.label(egui::RichText::new("Recently Generated").small().strong());
    }

    /// Show security audit tab
    fn show_security_audit_tab(&mut self, ui: &mut egui::Ui) {
        ui.heading("Security Audit");
        ui.add_space(16.0);

        // Run audit button
        if !self.audit_in_progress {
            if ui.button("🔍 Run Security Audit").clicked() {
                self.run_security_audit();
            }
        } else {
            ui.spinner();
            ui.label("Running audit...");
        }

        ui.add_space(16.0);

        // Results
        if let Some(audit) = &self.audit_result {
            // Overall score
            ui.horizontal(|ui| {
                ui.label("Security Score:");
                let score_color = if audit.overall_score >= 80 {
                    egui::Color32::GREEN
                } else if audit.overall_score >= 60 {
                    egui::Color32::YELLOW
                } else {
                    egui::Color32::RED
                };
                ui.heading(egui::RichText::new(format!("{}/100", audit.overall_score))
                    .color(score_color));
            });

            ui.add_space(16.0);

            // Issues summary
            egui::Frame::group(ui.style())
                .show(ui, |ui| {
                    ui.label("Summary");
                    ui.add_space(8.0);

                    let issues = vec![
                        ("Weak Passwords", audit.weak_passwords.len()),
                        ("Reused Passwords", audit.duplicate_passwords.len()),
                        ("Old Passwords (>90 days)", audit.old_passwords.len()),
                        ("Missing 2FA", audit.missing_2fa.len()),
                        ("Expired Items", audit.expired_items.len()),
                    ];

                    for (name, count) in issues {
                        let color = if count > 0 { egui::Color32::RED } else { egui::Color32::GREEN };
                        ui.horizontal(|ui| {
                            ui.label(name);
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                ui.colored_label(color, count.to_string());
                            });
                        });
                    }
                });

            ui.add_space(16.0);

            // Recommendations
            if !audit.recommendations.is_empty() {
                ui.label("Recommendations:");
                for rec in &audit.recommendations {
                    ui.horizontal(|ui| {
                        ui.label("💡");
                        ui.label(rec);
                    });
                }
            }
        }
    }

    /// Show emergency access tab
    fn show_emergency_access_tab(&mut self, ui: &mut egui::Ui) {
        ui.heading("Emergency Access");
        ui.add_space(8.0);
        ui.label("Grant trusted contacts access to your vault in case of emergency.");
        ui.add_space(16.0);

        // Add contact form
        egui::Frame::group(ui.style())
            .show(ui, |ui| {
                ui.label("Add Trusted Contact");
                ui.add_space(8.0);

                ui.label("Name:");
                ui.text_edit_singleline(&mut self.new_contact_name);

                ui.add_space(8.0);
                ui.label("Email:");
                ui.text_edit_singleline(&mut self.new_contact_email);

                ui.add_space(8.0);
                ui.label("Access Level:");
                egui::ComboBox::from_id(egui::Id::new("access_level"))
                    .selected_text(format!("{:?}", self.new_contact_access_level))
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut self.new_contact_access_level, EmergencyAccessLevel::ViewOnly, "View Only");
                        ui.selectable_value(&mut self.new_contact_access_level, EmergencyAccessLevel::ViewAndExport, "View & Export");
                        ui.selectable_value(&mut self.new_contact_access_level, EmergencyAccessLevel::FullAccess, "Full Access");
                    });

                ui.add_space(16.0);
                if ui.button("➕ Add Contact").clicked() {
                    self.add_trusted_contact();
                }
            });

        ui.add_space(16.0);

        // List of trusted contacts
        ui.label("Trusted Contacts:");
        if self.trusted_contacts.is_empty() {
            ui.label("No trusted contacts added yet.");
        } else {
            for contact in &self.trusted_contacts.clone() {
                egui::Frame::group(ui.style())
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.vertical(|ui| {
                                ui.label(egui::RichText::new(&contact.name).strong());
                                ui.label(egui::RichText::new(&contact.email).small());
                                ui.label(format!("Access: {:?}", contact.access_level));
                            });

                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                if ui.button("🗑️ Remove").clicked() {
                                    self.remove_trusted_contact(&contact.id);
                                }
                                // Show status
                                let status_color = match contact.invitation_status {
                                    InvitationStatus::Accepted => egui::Color32::GREEN,
                                    InvitationStatus::Pending => egui::Color32::YELLOW,
                                    _ => egui::Color32::GRAY,
                                };
                                ui.colored_label(status_color, format!("{:?}", contact.invitation_status));
                            });
                        });
                    });
            }
        }
    }

    /// Show settings tab
    fn show_settings_tab(&mut self, ui: &mut egui::Ui) {
        ui.heading("Vault Settings");
        ui.add_space(16.0);

        // Auto-lock settings
        egui::Frame::group(ui.style())
            .show(ui, |ui| {
                ui.label("Security");
                ui.add_space(8.0);

                ui.horizontal(|ui| {
                    ui.label("Auto-lock after:");
                    ui.add(egui::Slider::new(&mut self.auto_lock_timeout, 1..=60).text("minutes"));
                });

                if ui.button("🔒 Lock Now").clicked() {
                    if let Some(vault) = &self.vault {
                        vault.lock();
                    }
                }
            });

        ui.add_space(16.0);

        // Autofill settings
        egui::Frame::group(ui.style())
            .show(ui, |ui| {
                ui.label("Autofill");
                ui.add_space(8.0);

                ui.checkbox(&mut self.autofill_config.enabled, "Enable Autofill");
                ui.checkbox(&mut self.autofill_config.show_autofill_button, "Show Autofill Button");
                ui.checkbox(&mut self.autofill_config.auto_submit, "Auto-submit after fill");
                ui.checkbox(&mut self.autofill_config.require_biometric, "Require biometric for autofill");
            });

        ui.add_space(16.0);

        // Hardware security
        egui::Frame::group(ui.style())
            .show(ui, |ui| {
                ui.label("Hardware Security");
                ui.add_space(8.0);

                if self.available_hardware_methods.is_empty() {
                    ui.label("No hardware authentication devices detected.");
                    ui.label("Connect a YubiKey or enable Windows Hello for enhanced security.");
                } else {
                    ui.label("Available methods:");
                    for method in &self.available_hardware_methods {
                        ui.label(format!("  ✅ {}", method));
                    }
                }
            });

        ui.add_space(16.0);

        // Export/Import
        egui::Frame::group(ui.style())
            .show(ui, |ui| {
                ui.label("Backup & Export");
                ui.add_space(8.0);

                ui.horizontal(|ui| {
                    if ui.button("📥 Export Vault").clicked() {
                        // Export functionality
                    }
                    if ui.button("📤 Import Vault").clicked() {
                        // Import functionality
                    }
                });
            });
    }

    // Helper methods

    fn refresh_items(&mut self) {
        if let Some(vault) = &self.vault {
            match vault.list_items() {
                Ok(items) => {
                    self.items = items;
                    self.filter_items();
                }
                Err(_) => {}
            }
        }
    }

    fn filter_items(&mut self) {
        self.filtered_items = self.items.clone();

        // Filter by type
        match self.active_tab {
            VaultTab::Passwords => {
                self.filtered_items.retain(|i| i.item_type == VaultItemType::Password);
            }
            VaultTab::SshKeys => {
                self.filtered_items.retain(|i| i.item_type == VaultItemType::SshKey);
            }
            VaultTab::ApiKeys => {
                self.filtered_items.retain(|i| i.item_type == VaultItemType::ApiKey);
            }
            VaultTab::Totp => {
                self.filtered_items.retain(|i| i.item_type == VaultItemType::TOTP);
            }
            VaultTab::SecureNotes => {
                self.filtered_items.retain(|i| i.item_type == VaultItemType::SecureNote);
            }
            _ => {}
        }

        // Filter by search
        if !self.search_query.is_empty() {
            let query = self.search_query.to_lowercase();
            self.filtered_items.retain(|i| {
                i.name.to_lowercase().contains(&query)
                    || i.tags.iter().any(|t| t.to_lowercase().contains(&query))
                    || i.notes.as_ref().map(|n| n.to_lowercase().contains(&query)).unwrap_or(false)
            });
        }
    }

    fn search_items(&mut self) {
        self.filter_items();
    }

    fn copy_password(&mut self, item_id: &str) {
        if let Some(vault) = &self.vault {
            if let Ok(Some((_, entry))) = vault.get_password(item_id) {
                // Copy to clipboard
            }
        }
    }

    fn copy_username(&mut self, item_id: &str) {
        if let Some(vault) = &self.vault {
            if let Ok(Some((_, entry))) = vault.get_password(item_id) {
                // Copy to clipboard
            }
        }
    }

    fn copy_ssh_public_key(&mut self, item_id: &str) {
        if let Some(vault) = &self.vault {
            if let Ok(Some((_, entry))) = vault.get_ssh_key(item_id) {
                // Copy to clipboard
            }
        }
    }

    fn copy_api_key(&mut self, item_id: &str) {
        if let Some(vault) = &self.vault {
            if let Ok(Some((_, entry))) = vault.get_api_key(item_id) {
                // Copy to clipboard
            }
        }
    }

    fn refresh_totp(&mut self, item_id: &str) {
        if let Some(vault) = &self.vault {
            if let Ok(Some(code)) = vault.generate_totp_code(item_id) {
                self.totp_codes.insert(item_id.to_string(), code);
            }
        }
    }

    fn update_totp_codes(&mut self) {
        // Update TOTP codes every 30 seconds
        // This would be triggered by a timer in the main loop
    }

    fn run_security_audit(&mut self) {
        self.audit_in_progress = true;

        if let Some(vault) = &self.vault {
            match vault.run_security_audit() {
                Ok(result) => {
                    self.audit_result = Some(result);
                }
                Err(_) => {}
            }
        }

        self.audit_in_progress = false;
    }

    fn save_new_item(&mut self) {
        if let Some(vault) = &self.vault {
            let result = match self.new_item_type {
                VaultItemType::Password => {
                    vault.add_password(
                        &self.new_item_name,
                        &self.new_username,
                        &self.new_password,
                        Some(&self.new_url),
                        self.selected_folder.as_deref(),
                    )
                }
                VaultItemType::SshKey => {
                    vault.add_ssh_key(
                        &self.new_item_name,
                        &self.new_ssh_private_key,
                        &self.new_ssh_public_key,
                        None,
                        None,
                    )
                }
                VaultItemType::ApiKey => {
                    vault.add_api_key(
                        &self.new_item_name,
                        &self.new_api_key,
                        None,
                        Some(&self.new_url),
                    )
                }
                VaultItemType::TOTP => {
                    vault.add_totp(
                        &self.new_item_name,
                        &self.new_totp_secret,
                        None,
                        None,
                    )
                }
                VaultItemType::SecureNote => {
                    vault.add_secure_note(
                        &self.new_item_name,
                        &self.new_notes,
                        vault::NoteFormat::PlainText,
                    )
                }
                _ => Ok(String::new()),
            };

            if result.is_ok() {
                self.add_dialog_open = false;
                self.reset_add_dialog();
                self.refresh_items();
            }
        }
    }

    fn reset_add_dialog(&mut self) {
        self.new_item_type = VaultItemType::Password;
        self.new_item_name.clear();
        self.new_username.clear();
        self.new_password.clear();
        self.new_url.clear();
        self.new_notes.clear();
        self.new_ssh_private_key.clear();
        self.new_ssh_public_key.clear();
        self.new_api_key.clear();
        self.new_totp_secret.clear();
        self.show_password = false;
    }

    fn add_trusted_contact(&mut self) {
        if let Some(vault) = &self.vault {
            match vault.add_trusted_contact(
                &self.new_contact_name,
                &self.new_contact_email,
                self.new_contact_access_level,
            ) {
                Ok(_) => {
                    self.new_contact_name.clear();
                    self.new_contact_email.clear();
                    self.refresh_trusted_contacts();
                }
                Err(_) => {}
            }
        }
    }

    fn remove_trusted_contact(&mut self, contact_id: &str) {
        if let Some(vault) = &self.vault {
            if vault.remove_trusted_contact(contact_id).is_ok() {
                self.refresh_trusted_contacts();
            }
        }
    }

    fn refresh_trusted_contacts(&mut self) {
        if let Some(vault) = &self.vault {
            match vault.list_trusted_contacts() {
                Ok(contacts) => {
                    self.trusted_contacts = contacts;
                }
                Err(_) => {}
            }
        }
    }
}

impl Default for EnterpriseVaultWindow {
    fn default() -> Self {
        Self::new(Theme::default())
    }
}
