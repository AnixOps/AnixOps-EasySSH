//! Dialogs Module for EasySSH Lite
//!
//! Provides modal dialogs for:
//! - Adding new servers
//! - Editing existing servers
//! - Managing groups
//! - Password prompts
//! - Master password management

use crate::viewmodels::{AppViewModel, GroupViewModel, ServerViewModel};
use egui::{Align, Button, Layout, Response, RichText, TextEdit, Ui, Vec2, Window};
use zeroize::Zeroize;

/// Dialog result type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DialogResult {
    Ok,
    Cancel,
    None,
}

/// Add Server Dialog state
pub struct AddServerDialog {
    pub open: bool,
    pub name: String,
    pub host: String,
    pub port: String,
    pub username: String,
    pub auth_type: String,
    pub identity_file: String,
    pub group_id: Option<String>,
    pub error_message: Option<String>,
}

impl Default for AddServerDialog {
    fn default() -> Self {
        Self {
            open: false,
            name: String::new(),
            host: String::new(),
            port: "22".to_string(),
            username: String::new(),
            auth_type: "agent".to_string(),
            identity_file: String::new(),
            group_id: None,
            error_message: None,
        }
    }
}

impl AddServerDialog {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn open(&mut self) {
        self.open = true;
        self.clear();
    }

    pub fn close(&mut self) {
        self.open = false;
    }

    fn clear(&mut self) {
        self.name.clear();
        self.host.clear();
        self.port = "22".to_string();
        self.username.clear();
        self.auth_type = "agent".to_string();
        self.identity_file.clear();
        self.group_id = None;
        self.error_message = None;
    }

    pub fn show(&mut self, ctx: &egui::Context, groups: &[GroupViewModel]) -> DialogResult {
        if !self.open {
            return DialogResult::None;
        }

        let mut result = DialogResult::None;
        let mut should_close = false;

        Window::new("Add New Server")
            .collapsible(false)
            .resizable(false)
            .min_width(400.0)
            .show(ctx, |ui| {
                ui.spacing_mut().item_spacing.y = 12.0;

                // Name
                ui.horizontal(|ui| {
                    ui.label("Name:");
                    ui.text_edit_singleline(&mut self.name);
                });

                // Host
                ui.horizontal(|ui| {
                    ui.label("Host:");
                    ui.text_edit_singleline(&mut self.host);
                });

                // Port and Username
                ui.horizontal(|ui| {
                    ui.label("Port:");
                    ui.add(TextEdit::singleline(&mut self.port).desired_width(60.0));
                    ui.label("Username:");
                    ui.text_edit_singleline(&mut self.username);
                });

                // Auth type
                ui.horizontal(|ui| {
                    ui.label("Auth Type:");
                    egui::ComboBox::from_id_source("auth_type")
                        .selected_text(match self.auth_type.as_str() {
                            "agent" => "SSH Agent",
                            "key" => "Private Key",
                            "password" => "Password",
                            _ => "SSH Agent",
                        })
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut self.auth_type, "agent".to_string(), "SSH Agent");
                            ui.selectable_value(&mut self.auth_type, "key".to_string(), "Private Key");
                            ui.selectable_value(&mut self.auth_type, "password".to_string(), "Password");
                        });
                });

                // Identity file (for key auth)
                if self.auth_type == "key" {
                    ui.horizontal(|ui| {
                        ui.label("Key File:");
                        ui.text_edit_singleline(&mut self.identity_file);
                        if ui.button("Browse").clicked() {
                            // File browser would be implemented here
                        }
                    });
                }

                // Group selection
                ui.horizontal(|ui| {
                    ui.label("Group:");
                    egui::ComboBox::from_id_source("group_select")
                        .selected_text(
                            self.group_id
                                .as_ref()
                                .and_then(|id| groups.iter().find(|g| &g.id == id))
                                .map(|g| g.name.clone())
                                .unwrap_or_else(|| "None".to_string()),
                        )
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut self.group_id, None, "None");
                            for group in groups {
                                ui.selectable_value(
                                    &mut self.group_id,
                                    Some(group.id.clone()),
                                    &group.name,
                                );
                            }
                        });
                });

                // Error message
                if let Some(ref error) = self.error_message {
                    ui.colored_label(egui::Color32::RED, error);
                }

                ui.separator();

                // Buttons
                ui.horizontal(|ui| {
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if ui.button("Cancel").clicked() {
                            should_close = true;
                            result = DialogResult::Cancel;
                        }
                        if ui.button("Add Server").clicked() {
                            if self.validate() {
                                should_close = true;
                                result = DialogResult::Ok;
                            }
                        }
                    });
                });
            });

        if should_close {
            self.open = false;
        }

        result
    }

    fn validate(&mut self) -> bool {
        if self.name.trim().is_empty() {
            self.error_message = Some("Name is required".to_string());
            return false;
        }
        if self.host.trim().is_empty() {
            self.error_message = Some("Host is required".to_string());
            return false;
        }
        if self.username.trim().is_empty() {
            self.error_message = Some("Username is required".to_string());
            return false;
        }
        if self.port.parse::<i64>().is_err() {
            self.error_message = Some("Invalid port number".to_string());
            return false;
        }

        self.error_message = None;
        true
    }

    pub fn get_server_data(&self) -> Option<(String, String, i64, String, String, Option<String>)> {
        if !self.validate_data() {
            return None;
        }

        Some((
            self.name.trim().to_string(),
            self.host.trim().to_string(),
            self.port.parse::<i64>().ok()?,
            self.username.trim().to_string(),
            self.auth_type.clone(),
            self.group_id.clone(),
        ))
    }

    fn validate_data(&self) -> bool {
        !self.name.trim().is_empty()
            && !self.host.trim().is_empty()
            && !self.username.trim().is_empty()
            && self.port.parse::<i64>().is_ok()
    }
}

/// Edit Server Dialog
pub struct EditServerDialog {
    pub open: bool,
    pub server_id: Option<String>,
    pub name: String,
    pub host: String,
    pub port: String,
    pub username: String,
    pub auth_type: String,
    pub identity_file: String,
    pub group_id: Option<String>,
    pub error_message: Option<String>,
}

impl Default for EditServerDialog {
    fn default() -> Self {
        Self {
            open: false,
            server_id: None,
            name: String::new(),
            host: String::new(),
            port: "22".to_string(),
            username: String::new(),
            auth_type: "agent".to_string(),
            identity_file: String::new(),
            group_id: None,
            error_message: None,
        }
    }
}

impl EditServerDialog {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn open_with_server(&mut self, server: &ServerViewModel, groups: &[GroupViewModel]) {
        self.open = true;
        self.server_id = Some(server.id.clone());
        self.name = server.name.clone();
        self.host = server.host.clone();
        self.port = server.port.to_string();
        self.username = server.username.clone();
        self.auth_type = server.auth_type.clone();
        self.group_id = server.group_id.clone();
        self.identity_file.clear();
        self.error_message = None;
    }

    pub fn close(&mut self) {
        self.open = false;
        self.server_id = None;
    }

    pub fn show(&mut self, ctx: &egui::Context, groups: &[GroupViewModel]) -> DialogResult {
        if !self.open {
            return DialogResult::None;
        }

        let mut result = DialogResult::None;
        let mut should_close = false;

        let title = format!("Edit Server: {}", self.name);

        Window::new(title)
            .collapsible(false)
            .resizable(false)
            .min_width(400.0)
            .show(ctx, |ui| {
                ui.spacing_mut().item_spacing.y = 12.0;

                // Name
                ui.horizontal(|ui| {
                    ui.label("Name:");
                    ui.text_edit_singleline(&mut self.name);
                });

                // Host
                ui.horizontal(|ui| {
                    ui.label("Host:");
                    ui.text_edit_singleline(&mut self.host);
                });

                // Port and Username
                ui.horizontal(|ui| {
                    ui.label("Port:");
                    ui.add(TextEdit::singleline(&mut self.port).desired_width(60.0));
                    ui.label("Username:");
                    ui.text_edit_singleline(&mut self.username);
                });

                // Auth type
                ui.horizontal(|ui| {
                    ui.label("Auth Type:");
                    egui::ComboBox::from_id_source("edit_auth_type")
                        .selected_text(match self.auth_type.as_str() {
                            "agent" => "SSH Agent",
                            "key" => "Private Key",
                            "password" => "Password",
                            _ => "SSH Agent",
                        })
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut self.auth_type, "agent".to_string(), "SSH Agent");
                            ui.selectable_value(&mut self.auth_type, "key".to_string(), "Private Key");
                            ui.selectable_value(&mut self.auth_type, "password".to_string(), "Password");
                        });
                });

                // Identity file (for key auth)
                if self.auth_type == "key" {
                    ui.horizontal(|ui| {
                        ui.label("Key File:");
                        ui.text_edit_singleline(&mut self.identity_file);
                        if ui.button("Browse").clicked() {
                            // File browser would be implemented here
                        }
                    });
                }

                // Group selection
                ui.horizontal(|ui| {
                    ui.label("Group:");
                    egui::ComboBox::from_id_source("edit_group_select")
                        .selected_text(
                            self.group_id
                                .as_ref()
                                .and_then(|id| groups.iter().find(|g| &g.id == id))
                                .map(|g| g.name.clone())
                                .unwrap_or_else(|| "None".to_string()),
                        )
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut self.group_id, None, "None");
                            for group in groups {
                                ui.selectable_value(
                                    &mut self.group_id,
                                    Some(group.id.clone()),
                                    &group.name,
                                );
                            }
                        });
                });

                // Error message
                if let Some(ref error) = self.error_message {
                    ui.colored_label(egui::Color32::RED, error);
                }

                ui.separator();

                // Buttons
                ui.horizontal(|ui| {
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if ui.button("Cancel").clicked() {
                            should_close = true;
                            result = DialogResult::Cancel;
                        }
                        if ui.button("Save Changes").clicked() {
                            if self.validate() {
                                should_close = true;
                                result = DialogResult::Ok;
                            }
                        }
                    });
                });
            });

        if should_close {
            self.open = false;
        }

        result
    }

    fn validate(&mut self) -> bool {
        if self.name.trim().is_empty() {
            self.error_message = Some("Name is required".to_string());
            return false;
        }
        if self.host.trim().is_empty() {
            self.error_message = Some("Host is required".to_string());
            return false;
        }
        if self.username.trim().is_empty() {
            self.error_message = Some("Username is required".to_string());
            return false;
        }
        if self.port.parse::<i64>().is_err() {
            self.error_message = Some("Invalid port number".to_string());
            return false;
        }

        self.error_message = None;
        true
    }

    pub fn get_server_data(&self) -> Option<(String, String, String, i64, String, String, Option<String>)> {
        if !self.validate_data() {
            return None;
        }

        Some((
            self.server_id.clone()?,
            self.name.trim().to_string(),
            self.host.trim().to_string(),
            self.port.parse::<i64>().ok()?,
            self.username.trim().to_string(),
            self.auth_type.clone(),
            self.group_id.clone(),
        ))
    }

    fn validate_data(&self) -> bool {
        self.server_id.is_some()
            && !self.name.trim().is_empty()
            && !self.host.trim().is_empty()
            && !self.username.trim().is_empty()
            && self.port.parse::<i64>().is_ok()
    }
}

/// Group Manager Dialog
pub struct GroupManagerDialog {
    pub open: bool,
    pub new_group_name: String,
    pub editing_group: Option<String>,
    pub edit_name: String,
    pub error_message: Option<String>,
}

impl Default for GroupManagerDialog {
    fn default() -> Self {
        Self {
            open: false,
            new_group_name: String::new(),
            editing_group: None,
            edit_name: String::new(),
            error_message: None,
        }
    }
}

impl GroupManagerDialog {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn open(&mut self) {
        self.open = true;
        self.new_group_name.clear();
        self.editing_group = None;
        self.error_message = None;
    }

    pub fn close(&mut self) {
        self.open = false;
    }

    pub fn show(&mut self, ctx: &egui::Context, groups: &[GroupViewModel]) -> GroupDialogAction {
        if !self.open {
            return GroupDialogAction::None;
        }

        let mut action = GroupDialogAction::None;
        let mut should_close = false;

        Window::new("Manage Groups")
            .collapsible(false)
            .resizable(false)
            .min_width(350.0)
            .show(ctx, |ui| {
                ui.spacing_mut().item_spacing.y = 8.0;

                // Add new group
                ui.heading("Add New Group");
                ui.horizontal(|ui| {
                    ui.text_edit_singleline(&mut self.new_group_name);
                    if ui.button("Add").clicked() && !self.new_group_name.trim().is_empty() {
                        action = GroupDialogAction::Add(self.new_group_name.trim().to_string());
                        self.new_group_name.clear();
                    }
                });

                ui.separator();

                // Existing groups
                ui.heading("Existing Groups");
                egui::ScrollArea::vertical()
                    .max_height(200.0)
                    .show(ui, |ui| {
                        for group in groups {
                            ui.horizontal(|ui| {
                                if self.editing_group.as_ref() == Some(&group.id) {
                                    // Editing mode
                                    ui.text_edit_singleline(&mut self.edit_name);
                                    if ui.button("Save").clicked() {
                                        if !self.edit_name.trim().is_empty() {
                                            action = GroupDialogAction::Update(
                                                group.id.clone(),
                                                self.edit_name.trim().to_string(),
                                            );
                                            self.editing_group = None;
                                        }
                                    }
                                    if ui.button("Cancel").clicked() {
                                        self.editing_group = None;
                                    }
                                } else {
                                    // Display mode
                                    ui.label(&group.name);
                                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                        if ui.button("Delete").clicked() {
                                            action = GroupDialogAction::Delete(group.id.clone());
                                        }
                                        if ui.button("Edit").clicked() {
                                            self.editing_group = Some(group.id.clone());
                                            self.edit_name = group.name.clone();
                                        }
                                    });
                                }
                            });
                        }
                    });

                // Error message
                if let Some(ref error) = self.error_message {
                    ui.colored_label(egui::Color32::RED, error);
                }

                ui.separator();

                // Close button
                ui.horizontal(|ui| {
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if ui.button("Close").clicked() {
                            should_close = true;
                        }
                    });
                });
            });

        if should_close {
            self.open = false;
        }

        action
    }

    pub fn set_error(&mut self, error: String) {
        self.error_message = Some(error);
    }

    pub fn clear_error(&mut self) {
        self.error_message = None;
    }
}

/// Group dialog actions
#[derive(Debug, Clone)]
pub enum GroupDialogAction {
    None,
    Add(String),
    Update(String, String), // id, new_name
    Delete(String),
}

/// Password Prompt Dialog
pub struct PasswordPromptDialog {
    pub open: bool,
    pub server_name: String,
    pub username: String,
    pub password: String,
    pub save_password: bool,
    pub show_password: bool,
}

impl Default for PasswordPromptDialog {
    fn default() -> Self {
        Self {
            open: false,
            server_name: String::new(),
            username: String::new(),
            password: String::new(),
            save_password: false,
            show_password: false,
        }
    }
}

impl PasswordPromptDialog {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn open_for_server(&mut self, server_name: &str, username: &str) {
        self.open = true;
        self.server_name = server_name.to_string();
        self.username = username.to_string();
        self.password.clear();
        self.save_password = false;
        self.show_password = false;
    }

    pub fn close(&mut self) {
        self.open = false;
    }

    pub fn show(&mut self, ctx: &egui::Context) -> PasswordDialogResult {
        if !self.open {
            return PasswordDialogResult::None;
        }

        let mut result = PasswordDialogResult::None;
        let mut should_close = false;

        let title = format!("Password Required: {}", self.server_name);

        Window::new(title)
            .collapsible(false)
            .resizable(false)
            .min_width(350.0)
            .show(ctx, |ui| {
                ui.spacing_mut().item_spacing.y = 12.0;

                ui.label(format!("Enter password for user '{}'", self.username));

                // Password input
                ui.horizontal(|ui| {
                    ui.label("Password:");
                    if self.show_password {
                        ui.text_edit_singleline(&mut self.password);
                    } else {
                        ui.add(egui::TextEdit::singleline(&mut self.password).password(true));
                    }
                    if ui
                        .button(if self.show_password { "Hide" } else { "Show" })
                        .clicked()
                    {
                        self.show_password = !self.show_password;
                    }
                });

                // Save password checkbox
                ui.checkbox(&mut self.save_password, "Save password to keychain");

                ui.separator();

                // Buttons
                ui.horizontal(|ui| {
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if ui.button("Cancel").clicked() {
                            should_close = true;
                            result = PasswordDialogResult::Cancel;
                        }
                        if ui.button("Connect").clicked() && !self.password.is_empty() {
                            should_close = true;
                            result = PasswordDialogResult::Ok {
                                password: self.password.clone(),
                                save_password: self.save_password,
                            };
                        }
                    });
                });
            });

        if should_close {
            self.open = false;
        }

        result
    }
}

/// Password dialog result
#[derive(Debug, Clone)]
pub enum PasswordDialogResult {
    None,
    Ok { password: String, save_password: bool },
    Cancel,
}

/// Delete confirmation dialog
pub struct DeleteConfirmDialog {
    pub open: bool,
    pub item_name: String,
    pub item_type: String,
}

impl Default for DeleteConfirmDialog {
    fn default() -> Self {
        Self {
            open: false,
            item_name: String::new(),
            item_type: "item".to_string(),
        }
    }
}

impl DeleteConfirmDialog {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn open_for(&mut self, item_name: &str, item_type: &str) {
        self.open = true;
        self.item_name = item_name.to_string();
        self.item_type = item_type.to_string();
    }

    pub fn close(&mut self) {
        self.open = false;
    }

    pub fn show(&mut self, ctx: &egui::Context) -> DialogResult {
        if !self.open {
            return DialogResult::None;
        }

        let mut result = DialogResult::None;
        let mut should_close = false;

        Window::new("Confirm Delete")
            .collapsible(false)
            .resizable(false)
            .min_width(300.0)
            .show(ctx, |ui| {
                ui.spacing_mut().item_spacing.y = 16.0;

                ui.label(
                    RichText::new(format!(
                        "Are you sure you want to delete this {}?",
                        self.item_type
                    ))
                    .size(16.0),
                );

                ui.colored_label(
                    egui::Color32::YELLOW,
                    format!("'{}'", self.item_name),
                );

                ui.label("This action cannot be undone.");

                ui.separator();

                // Buttons
                ui.horizontal(|ui| {
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if ui.button("Cancel").clicked() {
                            should_close = true;
                            result = DialogResult::Cancel;
                        }
                        if ui
                            .button(RichText::new("Delete").color(egui::Color32::RED))
                            .clicked()
                        {
                            should_close = true;
                            result = DialogResult::Ok;
                        }
                    });
                });
            });

        if should_close {
            self.open = false;
        }

        result
    }
}

/// Error dialog
pub struct ErrorDialog {
    pub open: bool,
    pub title: String,
    pub message: String,
}

impl Default for ErrorDialog {
    fn default() -> Self {
        Self {
            open: false,
            title: "Error".to_string(),
            message: String::new(),
        }
    }
}

impl ErrorDialog {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn open_with_message(&mut self, title: &str, message: &str) {
        self.open = true;
        self.title = title.to_string();
        self.message = message.to_string();
    }

    pub fn close(&mut self) {
        self.open = false;
    }

    pub fn show(&mut self, ctx: &egui::Context) -> DialogResult {
        if !self.open {
            return DialogResult::None;
        }

        let mut result = DialogResult::None;
        let mut should_close = false;

        Window::new(&self.title)
            .collapsible(false)
            .resizable(false)
            .min_width(300.0)
            .show(ctx, |ui| {
                ui.spacing_mut().item_spacing.y = 16.0;

                ui.colored_label(egui::Color32::RED, &self.message);

                ui.separator();

                ui.horizontal(|ui| {
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if ui.button("OK").clicked() {
                            should_close = true;
                            result = DialogResult::Ok;
                        }
                    });
                });
            });

        if should_close {
            self.open = false;
        }

        result
    }
}

/// Password strength indicator
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PasswordStrength {
    VeryWeak,    // 0-20
    Weak,        // 21-40
    Fair,        // 41-60
    Good,        // 61-80
    Strong,      // 81-100
}

impl PasswordStrength {
    pub fn from_score(score: u8) -> Self {
        match score {
            0..=20 => PasswordStrength::VeryWeak,
            21..=40 => PasswordStrength::Weak,
            41..=60 => PasswordStrength::Fair,
            61..=80 => PasswordStrength::Good,
            _ => PasswordStrength::Strong,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            PasswordStrength::VeryWeak => "Very Weak",
            PasswordStrength::Weak => "Weak",
            PasswordStrength::Fair => "Fair",
            PasswordStrength::Good => "Good",
            PasswordStrength::Strong => "Strong",
        }
    }

    pub fn color(&self) -> egui::Color32 {
        match self {
            PasswordStrength::VeryWeak => egui::Color32::RED,
            PasswordStrength::Weak => egui::Color32::from_rgb(255, 100, 0),
            PasswordStrength::Fair => egui::Color32::from_rgb(255, 200, 0),
            PasswordStrength::Good => egui::Color32::from_rgb(150, 220, 0),
            PasswordStrength::Strong => egui::Color32::GREEN,
        }
    }
}

/// Master Password dialog modes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MasterPasswordMode {
    /// First time setup - create new master password
    Setup,
    /// Verify on app startup
    Verify,
    /// Change existing password
    Change,
    /// Reset warning - data will be lost
    Reset,
}

impl std::fmt::Display for MasterPasswordMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MasterPasswordMode::Setup => write!(f, "Setup"),
            MasterPasswordMode::Verify => write!(f, "Verify"),
            MasterPasswordMode::Change => write!(f, "Change"),
            MasterPasswordMode::Reset => write!(f, "Reset"),
        }
    }
}

/// Master Password Dialog for EasySSH Lite
/// Handles: Setup, Verify, Change, and Reset modes
pub struct MasterPasswordDialog {
    pub open: bool,
    pub mode: MasterPasswordMode,
    pub password: String,
    pub confirm_password: String,
    pub old_password: String,
    pub error_message: Option<String>,
    pub password_strength: PasswordStrength,
    pub show_password: bool,
    pub show_confirm_password: bool,
    pub show_old_password: bool,
    pub attempt_count: u32,
    pub max_attempts: u32,
    pub last_input_time: f64,
    pub copy_paste_attempts: u32,
}

impl Default for MasterPasswordDialog {
    fn default() -> Self {
        Self {
            open: false,
            mode: MasterPasswordMode::Verify,
            password: String::new(),
            confirm_password: String::new(),
            old_password: String::new(),
            error_message: None,
            password_strength: PasswordStrength::VeryWeak,
            show_password: false,
            show_confirm_password: false,
            show_old_password: false,
            attempt_count: 0,
            max_attempts: 5,
            last_input_time: 0.0,
            copy_paste_attempts: 0,
        }
    }
}

impl MasterPasswordDialog {
    pub fn new() -> Self {
        Self::default()
    }

    /// Open dialog for first-time setup
    pub fn open_setup(&mut self) {
        self.open = true;
        self.mode = MasterPasswordMode::Setup;
        self.clear_all_fields();
        self.error_message = None;
        self.attempt_count = 0;
    }

    /// Open dialog for password verification on startup
    pub fn open_verify(&mut self) {
        self.open = true;
        self.mode = MasterPasswordMode::Verify;
        self.clear_all_fields();
        self.error_message = None;
        self.attempt_count = 0;
    }

    /// Open dialog for changing password
    pub fn open_change(&mut self) {
        self.open = true;
        self.mode = MasterPasswordMode::Change;
        self.clear_all_fields();
        self.error_message = None;
        self.attempt_count = 0;
    }

    /// Open dialog for reset warning
    pub fn open_reset(&mut self) {
        self.open = true;
        self.mode = MasterPasswordMode::Reset;
        self.clear_all_fields();
        self.error_message = None;
    }

    fn clear_all_fields(&mut self) {
        self.password.clear();
        self.confirm_password.clear();
        self.old_password.clear();
        self.password_strength = PasswordStrength::VeryWeak;
        self.show_password = false;
        self.show_confirm_password = false;
        self.show_old_password = false;
    }

    pub fn close(&mut self) {
        self.open = false;
        // Securely clear sensitive data
        self.password.zeroize();
        self.confirm_password.zeroize();
        self.old_password.zeroize();
    }

    /// Calculate password strength score
    fn calculate_strength(password: &str) -> (u8, Vec<&'static str>) {
        let mut score = 0u8;
        let mut feedback = Vec::new();

        // Length check
        if password.len() >= 8 {
            score += 20;
        } else {
            feedback.push("Password must be at least 8 characters");
        }
        if password.len() >= 12 {
            score += 10;
        }
        if password.len() >= 16 {
            score += 10;
        }

        // Character variety checks
        if password.chars().any(|c| c.is_ascii_lowercase()) {
            score += 15;
        } else {
            feedback.push("Add lowercase letters");
        }

        if password.chars().any(|c| c.is_ascii_uppercase()) {
            score += 15;
        } else {
            feedback.push("Add uppercase letters");
        }

        if password.chars().any(|c| c.is_ascii_digit()) {
            score += 15;
        } else {
            feedback.push("Add numbers");
        }

        if password.chars().any(|c| !c.is_alphanumeric()) {
            score += 15;
        } else {
            feedback.push("Add special characters");
        }

        (score, feedback)
    }

    fn update_strength(&mut self) {
        let (score, _) = Self::calculate_strength(&self.password);
        self.password_strength = PasswordStrength::from_score(score);
    }

    fn validate_setup(&mut self) -> bool {
        // Check minimum length
        if self.password.len() < 8 {
            self.error_message = Some("Password must be at least 8 characters long".to_string());
            return false;
        }

        // Check character requirements
        let has_lowercase = self.password.chars().any(|c| c.is_ascii_lowercase());
        let has_uppercase = self.password.chars().any(|c| c.is_ascii_uppercase());
        let has_digit = self.password.chars().any(|c| c.is_ascii_digit());
        let has_special = self.password.chars().any(|c| !c.is_alphanumeric());

        if !has_lowercase || !has_uppercase || !has_digit || !has_special {
            self.error_message = Some(
                "Password must contain lowercase, uppercase, numbers, and special characters".to_string()
            );
            return false;
        }

        // Check confirmation
        if self.password != self.confirm_password {
            self.error_message = Some("Passwords do not match".to_string());
            return false;
        }

        // Check strength
        let (score, _) = Self::calculate_strength(&self.password);
        if score < 60 {
            self.error_message = Some("Password is too weak. Please use a stronger password.".to_string());
            return false;
        }

        self.error_message = None;
        true
    }

    fn validate_verify(&mut self) -> bool {
        if self.password.is_empty() {
            self.error_message = Some("Please enter your master password".to_string());
            return false;
        }
        self.error_message = None;
        true
    }

    fn validate_change(&mut self) -> bool {
        if self.old_password.is_empty() {
            self.error_message = Some("Please enter your current password".to_string());
            return false;
        }

        if self.password.len() < 8 {
            self.error_message = Some("New password must be at least 8 characters long".to_string());
            return false;
        }

        let has_lowercase = self.password.chars().any(|c| c.is_ascii_lowercase());
        let has_uppercase = self.password.chars().any(|c| c.is_ascii_uppercase());
        let has_digit = self.password.chars().any(|c| c.is_ascii_digit());
        let has_special = self.password.chars().any(|c| !c.is_alphanumeric());

        if !has_lowercase || !has_uppercase || !has_digit || !has_special {
            self.error_message = Some(
                "New password must contain lowercase, uppercase, numbers, and special characters".to_string()
            );
            return false;
        }

        if self.password != self.confirm_password {
            self.error_message = Some("New passwords do not match".to_string());
            return false;
        }

        if self.old_password == self.password {
            self.error_message = Some("New password must be different from current password".to_string());
            return false;
        }

        let (score, _) = Self::calculate_strength(&self.password);
        if score < 60 {
            self.error_message = Some("New password is too weak".to_string());
            return false;
        }

        self.error_message = None;
        true
    }

    /// Show the dialog and return the result
    pub fn show(&mut self, ctx: &egui::Context) -> MasterPasswordDialogResult {
        if !self.open {
            return MasterPasswordDialogResult::None;
        }

        let mut result = MasterPasswordDialogResult::None;
        let mut should_close = false;

        let title = match self.mode {
            MasterPasswordMode::Setup => "Set Master Password",
            MasterPasswordMode::Verify => "Enter Master Password",
            MasterPasswordMode::Change => "Change Master Password",
            MasterPasswordMode::Reset => "Reset Master Password",
        };

        // Center the dialog
        let screen_rect = ctx.screen_rect();
        let window_size = egui::vec2(450.0, match self.mode {
            MasterPasswordMode::Setup => 420.0,
            MasterPasswordMode::Verify => 280.0,
            MasterPasswordMode::Change => 450.0,
            MasterPasswordMode::Reset => 320.0,
        });
        let window_pos = screen_rect.center() - window_size * 0.5;

        egui::Window::new(title)
            .collapsible(false)
            .resizable(false)
            .fixed_size(window_size)
            .fixed_pos(window_pos)
            .show(ctx, |ui| {
                ui.spacing_mut().item_spacing.y = 12.0;

                match self.mode {
                    MasterPasswordMode::Setup => {
                        self.show_setup_ui(ui, &mut result, &mut should_close);
                    }
                    MasterPasswordMode::Verify => {
                        self.show_verify_ui(ui, &mut result, &mut should_close);
                    }
                    MasterPasswordMode::Change => {
                        self.show_change_ui(ui, &mut result, &mut should_close);
                    }
                    MasterPasswordMode::Reset => {
                        self.show_reset_ui(ui, &mut result, &mut should_close);
                    }
                }
            });

        if should_close {
            self.close();
        }

        result
    }

    fn show_setup_ui(&mut self, ui: &mut Ui, result: &mut MasterPasswordDialogResult, should_close: &mut bool) {
        ui.label("Welcome to EasySSH Lite!");
        ui.label("Please set a master password to secure your SSH configurations.");
        ui.label("This password will be used to encrypt all your sensitive data.");
        ui.separator();

        // Password field
        ui.horizontal(|ui| {
            ui.label("Password:");
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if ui.button(if self.show_password { "Hide" } else { "Show" }).clicked() {
                    self.show_password = !self.show_password;
                }
                if self.show_password {
                    ui.text_edit_singleline(&mut self.password);
                } else {
                    ui.add(egui::TextEdit::singleline(&mut self.password).password(true));
                }
            });
        });

        // Update strength meter
        if ui.input(|i| i.pointer.any_click() || i.pointer.any_down()) {
            self.update_strength();
        }

        // Strength indicator
        ui.horizontal(|ui| {
            ui.label("Strength:");
            let strength_color = self.password_strength.color();
            ui.colored_label(strength_color, self.password_strength.as_str());
        });

        // Strength bar
        let (score, _) = Self::calculate_strength(&self.password);
        let progress = score as f32 / 100.0;
        ui.add(egui::ProgressBar::new(progress).fill(self.password_strength.color()));

        // Confirm password field
        ui.horizontal(|ui| {
            ui.label("Confirm:");
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if ui.button(if self.show_confirm_password { "Hide" } else { "Show" }).clicked() {
                    self.show_confirm_password = !self.show_confirm_password;
                }
                if self.show_confirm_password {
                    ui.text_edit_singleline(&mut self.confirm_password);
                } else {
                    ui.add(egui::TextEdit::singleline(&mut self.confirm_password).password(true));
                }
            });
        });

        // Requirements hint
        ui.collapsing("Password Requirements", |ui| {
            ui.label("• At least 8 characters");
            ui.label("• Uppercase letters (A-Z)");
            ui.label("• Lowercase letters (a-z)");
            ui.label("• Numbers (0-9)");
            ui.label("• Special characters (!@#$...)");
        });

        // Error message
        if let Some(ref error) = self.error_message {
            ui.colored_label(egui::Color32::RED, error);
        }

        ui.separator();

        // Buttons
        ui.horizontal(|ui| {
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if ui.button("Set Password").clicked() {
                    if self.validate_setup() {
                        *result = MasterPasswordDialogResult::SetPassword {
                            password: self.password.clone(),
                        };
                        *should_close = true;
                    }
                }
            });
        });
    }

    fn show_verify_ui(&mut self, ui: &mut Ui, result: &mut MasterPasswordDialogResult, should_close: &mut bool) {
        ui.label("Please enter your master password to unlock EasySSH.");
        ui.label("All your server configurations are securely encrypted.");
        ui.separator();

        // Attempt warning
        if self.attempt_count > 0 {
            let remaining = self.max_attempts.saturating_sub(self.attempt_count);
            ui.colored_label(
                egui::Color32::YELLOW,
                format!("Warning: {} failed attempts. {} attempts remaining.", self.attempt_count, remaining)
            );
        }

        // Password field
        ui.horizontal(|ui| {
            ui.label("Password:");
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if ui.button(if self.show_password { "Hide" } else { "Show" }).clicked() {
                    self.show_password = !self.show_password;
                }
                if self.show_password {
                    ui.text_edit_singleline(&mut self.password);
                } else {
                    ui.add(egui::TextEdit::singleline(&mut self.password).password(true));
                }
            });
        });

        // Error message
        if let Some(ref error) = self.error_message {
            ui.colored_label(egui::Color32::RED, error);
        }

        ui.separator();

        // Buttons
        ui.horizontal(|ui| {
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if ui.button("Unlock").clicked() {
                    if self.validate_verify() {
                        self.attempt_count += 1;
                        *result = MasterPasswordDialogResult::Verify {
                            password: self.password.clone(),
                            attempt: self.attempt_count,
                        };
                        if self.attempt_count >= self.max_attempts {
                            *result = MasterPasswordDialogResult::MaxAttemptsReached;
                        }
                        *should_close = true;
                    }
                }
            });
        });

        // Forgot password link
        ui.horizontal(|ui| {
            if ui.link("Forgot password?").clicked() {
                *result = MasterPasswordDialogResult::ForgotPassword;
            }
        });
    }

    fn show_change_ui(&mut self, ui: &mut Ui, result: &mut MasterPasswordDialogResult, should_close: &mut bool) {
        ui.label("Change your master password.");
        ui.label("All existing encrypted data will be re-encrypted with the new password.");
        ui.separator();

        // Old password field
        ui.horizontal(|ui| {
            ui.label("Current:");
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if ui.button(if self.show_old_password { "Hide" } else { "Show" }).clicked() {
                    self.show_old_password = !self.show_old_password;
                }
                if self.show_old_password {
                    ui.text_edit_singleline(&mut self.old_password);
                } else {
                    ui.add(egui::TextEdit::singleline(&mut self.old_password).password(true));
                }
            });
        });

        // New password field
        ui.horizontal(|ui| {
            ui.label("New:");
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if ui.button(if self.show_password { "Hide" } else { "Show" }).clicked() {
                    self.show_password = !self.show_password;
                }
                if self.show_password {
                    ui.text_edit_singleline(&mut self.password);
                } else {
                    ui.add(egui::TextEdit::singleline(&mut self.password).password(true));
                }
            });
        });

        // Update strength
        self.update_strength();

        // Strength indicator
        ui.horizontal(|ui| {
            ui.label("Strength:");
            let strength_color = self.password_strength.color();
            ui.colored_label(strength_color, self.password_strength.as_str());
        });

        // Confirm new password field
        ui.horizontal(|ui| {
            ui.label("Confirm:");
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if ui.button(if self.show_confirm_password { "Hide" } else { "Show" }).clicked() {
                    self.show_confirm_password = !self.show_confirm_password;
                }
                if self.show_confirm_password {
                    ui.text_edit_singleline(&mut self.confirm_password);
                } else {
                    ui.add(egui::TextEdit::singleline(&mut self.confirm_password).password(true));
                }
            });
        });

        // Error message
        if let Some(ref error) = self.error_message {
            ui.colored_label(egui::Color32::RED, error);
        }

        ui.separator();

        // Buttons
        ui.horizontal(|ui| {
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if ui.button("Cancel").clicked() {
                    *result = MasterPasswordDialogResult::Cancel;
                    *should_close = true;
                }
                if ui.button("Change Password").clicked() {
                    if self.validate_change() {
                        *result = MasterPasswordDialogResult::ChangePassword {
                            old_password: self.old_password.clone(),
                            new_password: self.password.clone(),
                        };
                        *should_close = true;
                    }
                }
            });
        });
    }

    fn show_reset_ui(&mut self, ui: &mut Ui, result: &mut MasterPasswordDialogResult, should_close: &mut bool) {
        // Warning icon and title
        ui.horizontal(|ui| {
            ui.colored_label(egui::Color32::RED, RichText::new("⚠").size(24.0));
            ui.heading("Reset Master Password");
        });

        ui.separator();

        ui.label(RichText::new("WARNING: This action cannot be undone!").strong());
        ui.colored_label(
            egui::Color32::RED,
            "Resetting your master password will permanently delete all encrypted data, including:"
        );

        ui.indent("warning_list", |ui| {
            ui.label("• All stored SSH passwords");
            ui.label("• All encrypted server configurations");
            ui.label("• All secure vault items");
            ui.label("• Your encrypted keychain data");
        });

        ui.colored_label(egui::Color32::YELLOW, "You will need to re-add all your servers manually.");

        ui.separator();

        // Confirmation checkboxes
        ui.label("Type \"DELETE\" to confirm you understand the consequences:");
        ui.text_edit_singleline(&mut self.password);

        // Error message
        if let Some(ref error) = self.error_message {
            ui.colored_label(egui::Color32::RED, error);
        }

        ui.separator();

        // Buttons
        ui.horizontal(|ui| {
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if ui.button("Cancel").clicked() {
                    *result = MasterPasswordDialogResult::Cancel;
                    *should_close = true;
                }
                if ui
                    .button(RichText::new("Reset Password").color(egui::Color32::RED))
                    .clicked()
                {
                    if self.password == "DELETE" {
                        *result = MasterPasswordDialogResult::ResetConfirmed;
                        *should_close = true;
                    } else {
                        self.error_message = Some("Please type DELETE to confirm".to_string());
                    }
                }
            });
        });
    }

    /// Get the password from a successful setup/verify
    pub fn get_password(&self) -> Option<String> {
        if self.password.is_empty() {
            None
        } else {
            Some(self.password.clone())
        }
    }

    /// Set error message (used by external verification failures)
    pub fn set_error(&mut self, error: String) {
        self.error_message = Some(error);
    }

    /// Check if max attempts reached
    pub fn is_max_attempts_reached(&self) -> bool {
        self.attempt_count >= self.max_attempts
    }

    /// Increment attempt counter (used when external verification fails)
    pub fn increment_attempt(&mut self) {
        self.attempt_count += 1;
    }
}

/// Master password dialog result types
#[derive(Debug, Clone)]
pub enum MasterPasswordDialogResult {
    None,
    /// Setup: New password set
    SetPassword { password: String },
    /// Verify: Password entered
    Verify { password: String, attempt: u32 },
    /// Change: Password change requested
    ChangePassword { old_password: String, new_password: String },
    /// Reset: User confirmed reset
    ResetConfirmed,
    /// Max attempts reached
    MaxAttemptsReached,
    /// User clicked forgot password
    ForgotPassword,
    /// User cancelled
    Cancel,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dialog_result() {
        assert_ne!(DialogResult::Ok, DialogResult::Cancel);
        assert_eq!(DialogResult::None, DialogResult::None);
    }

    #[test]
    fn test_add_server_dialog_validate() {
        let mut dialog = AddServerDialog::new();

        // Empty fields should fail
        assert!(!dialog.validate());
        assert!(dialog.error_message.is_some());

        // Fill in valid data
        dialog.name = "Test".to_string();
        dialog.host = "example.com".to_string();
        dialog.username = "user".to_string();
        dialog.port = "22".to_string();

        assert!(dialog.validate());
        assert!(dialog.error_message.is_none());

        // Invalid port
        dialog.port = "invalid".to_string();
        assert!(!dialog.validate());
    }

    #[test]
    fn test_password_dialog_result() {
        let result = PasswordDialogResult::Ok {
            password: "secret".to_string(),
            save_password: true,
        };

        match result {
            PasswordDialogResult::Ok { password, save_password } => {
                assert_eq!(password, "secret");
                assert!(save_password);
            }
            _ => panic!("Expected Ok variant"),
        }
    }

    #[test]
    fn test_group_dialog_action() {
        let action = GroupDialogAction::Add("New Group".to_string());
        match action {
            GroupDialogAction::Add(name) => assert_eq!(name, "New Group"),
            _ => panic!("Expected Add variant"),
        }

        let action = GroupDialogAction::Update("id123".to_string(), "Updated".to_string());
        match action {
            GroupDialogAction::Update(id, name) => {
                assert_eq!(id, "id123");
                assert_eq!(name, "Updated");
            }
            _ => panic!("Expected Update variant"),
        }
    }
}
