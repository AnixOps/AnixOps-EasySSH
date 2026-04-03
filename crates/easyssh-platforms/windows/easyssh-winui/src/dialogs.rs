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

/// Animation state for master password dialog
#[derive(Debug, Clone, Copy)]
pub struct DialogAnimation {
    pub open_progress: f32,
    pub error_shake: f32,
    pub success_pulse: f32,
    pub target_progress: f32,
}

impl Default for DialogAnimation {
    fn default() -> Self {
        Self {
            open_progress: 0.0,
            error_shake: 0.0,
            success_pulse: 0.0,
            target_progress: 1.0,
        }
    }
}

impl DialogAnimation {
    pub fn update(&mut self, ctx: &egui::Context, dt: f32) {
        // Smooth open animation
        let speed = 8.0;
        self.open_progress = egui::lerp(
            self.open_progress..=self.target_progress,
            (speed * dt).min(1.0),
        );

        // Decay error shake
        self.error_shake *= (1.0 - 10.0 * dt).max(0.0);

        // Decay success pulse
        self.success_pulse *= (1.0 - 5.0 * dt).max(0.0);

        // Request continuous updates during animation
        if self.open_progress < 1.0 || self.error_shake > 0.01 || self.success_pulse > 0.01 {
            ctx.request_repaint_after(std::time::Duration::from_millis(16));
        }
    }

    pub fn trigger_error(&mut self) {
        self.error_shake = 1.0;
    }

    pub fn trigger_success(&mut self) {
        self.success_pulse = 1.0;
    }

    pub fn get_scale(&self) -> f32 {
        0.9 + 0.1 * self.open_progress
    }

    pub fn get_alpha(&self) -> f32 {
        self.open_progress
    }

    pub fn get_shake_offset(&self) -> egui::Vec2 {
        if self.error_shake > 0.01 {
            let shake_x = (self.error_shake * 10.0).sin() * self.error_shake * 10.0;
            egui::vec2(shake_x, 0.0)
        } else {
            egui::Vec2::ZERO
        }
    }
}

/// Color picker preset for groups
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GroupColor {
    Default,
    Blue,
    Green,
    Red,
    Orange,
    Purple,
    Pink,
    Cyan,
    Yellow,
}

impl GroupColor {
    pub fn as_color32(&self) -> egui::Color32 {
        match self {
            GroupColor::Default => egui::Color32::GRAY,
            GroupColor::Blue => egui::Color32::from_rgb(59, 130, 246),
            GroupColor::Green => egui::Color32::from_rgb(34, 197, 94),
            GroupColor::Red => egui::Color32::from_rgb(239, 68, 68),
            GroupColor::Orange => egui::Color32::from_rgb(249, 115, 22),
            GroupColor::Purple => egui::Color32::from_rgb(168, 85, 247),
            GroupColor::Pink => egui::Color32::from_rgb(236, 72, 153),
            GroupColor::Cyan => egui::Color32::from_rgb(6, 182, 212),
            GroupColor::Yellow => egui::Color32::from_rgb(234, 179, 8),
        }
    }

    pub fn all_colors() -> &'static [GroupColor] {
        &[
            GroupColor::Blue,
            GroupColor::Green,
            GroupColor::Red,
            GroupColor::Orange,
            GroupColor::Purple,
            GroupColor::Pink,
            GroupColor::Cyan,
            GroupColor::Yellow,
        ]
    }

    pub fn name(&self) -> &'static str {
        match self {
            GroupColor::Default => "Default",
            GroupColor::Blue => "Blue",
            GroupColor::Green => "Green",
            GroupColor::Red => "Red",
            GroupColor::Orange => "Orange",
            GroupColor::Purple => "Purple",
            GroupColor::Pink => "Pink",
            GroupColor::Cyan => "Cyan",
            GroupColor::Yellow => "Yellow",
        }
    }
}

/// Dialog result type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DialogResult {
    Ok,
    Cancel,
    None,
}

/// Group color picker dialog
pub struct GroupColorPicker {
    pub open: bool,
    pub selected_color: GroupColor,
}

impl Default for GroupColorPicker {
    fn default() -> Self {
        Self {
            open: false,
            selected_color: GroupColor::Default,
        }
    }
}

impl GroupColorPicker {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn open(&mut self, current_color: GroupColor) {
        self.open = true;
        self.selected_color = current_color;
    }

    pub fn close(&mut self) {
        self.open = false;
    }

    pub fn show(&mut self, ctx: &egui::Context) -> Option<GroupColor> {
        if !self.open {
            return None;
        }

        let mut selected = None;
        let mut should_close = false;

        egui::Window::new("Choose Group Color")
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                ui.label("Select a color for this group:");
                ui.add_space(8.0);

                // Color grid
                let colors = GroupColor::all_colors();
                let colors_per_row = 4;

                for row in colors.chunks(colors_per_row) {
                    ui.horizontal(|ui| {
                        for color in row {
                            let is_selected = self.selected_color == *color;
                            let color_value = color.as_color32();

                            let button_size = egui::vec2(48.0, 48.0);
                            let (rect, response) =
                                ui.allocate_exact_size(button_size, egui::Sense::click());

                            // Draw color circle
                            let painter = ui.painter();
                            let center = rect.center();
                            let radius = if is_selected { 20.0 } else { 18.0 };

                            // Shadow for depth
                            painter.circle_filled(
                                center + egui::vec2(0.0, 2.0),
                                radius,
                                color_value.linear_multiply(0.7),
                            );
                            painter.circle_filled(center, radius, color_value);

                            // Selection indicator
                            if is_selected {
                                painter.circle_stroke(
                                    center,
                                    radius + 3.0,
                                    egui::Stroke::new(3.0, egui::Color32::WHITE),
                                );
                                painter.circle_stroke(
                                    center,
                                    radius + 3.0,
                                    egui::Stroke::new(
                                        1.0,
                                        egui::Color32::BLACK.linear_multiply(0.3),
                                    ),
                                );
                            }

                            // Checkmark for selected
                            if is_selected {
                                let check_color = if color_value.r() as u16
                                    + color_value.g() as u16
                                    + color_value.b() as u16
                                    > 384
                                {
                                    egui::Color32::BLACK
                                } else {
                                    egui::Color32::WHITE
                                };
                                painter.text(
                                    center,
                                    egui::Align2::CENTER_CENTER,
                                    "✓",
                                    egui::FontId::proportional(20.0),
                                    check_color,
                                );
                            }

                            if response.clicked() {
                                self.selected_color = *color;
                                selected = Some(*color);
                                should_close = true;
                            }

                            // Tooltip
                            response.on_hover_text(color.name());
                        }
                    });
                    ui.add_space(8.0);
                }

                ui.separator();

                ui.horizontal(|ui| {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("Cancel").clicked() {
                            should_close = true;
                            selected = None;
                        }
                    });
                });
            });

        if should_close {
            self.close();
        }

        selected
    }
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
                            ui.selectable_value(
                                &mut self.auth_type,
                                "agent".to_string(),
                                "SSH Agent",
                            );
                            ui.selectable_value(
                                &mut self.auth_type,
                                "key".to_string(),
                                "Private Key",
                            );
                            ui.selectable_value(
                                &mut self.auth_type,
                                "password".to_string(),
                                "Password",
                            );
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
                            ui.selectable_value(
                                &mut self.auth_type,
                                "agent".to_string(),
                                "SSH Agent",
                            );
                            ui.selectable_value(
                                &mut self.auth_type,
                                "key".to_string(),
                                "Private Key",
                            );
                            ui.selectable_value(
                                &mut self.auth_type,
                                "password".to_string(),
                                "Password",
                            );
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

    pub fn get_server_data(
        &self,
    ) -> Option<(String, String, String, i64, String, String, Option<String>)> {
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
    Ok {
        password: String,
        save_password: bool,
    },
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

                ui.colored_label(egui::Color32::YELLOW, format!("'{}'", self.item_name));

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
    VeryWeak, // 0-20
    Weak,     // 21-40
    Fair,     // 41-60
    Good,     // 61-80
    Strong,   // 81-100
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
    // Animation support
    pub animation: DialogAnimation,
    pub opening_time: f64,
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
            animation: DialogAnimation::default(),
            opening_time: 0.0,
        }
    }
}

impl MasterPasswordDialog {
    pub fn new() -> Self {
        Self::default()
    }

    /// Open dialog for first-time setup
    pub fn open_setup(&mut self, ctx: &egui::Context) {
        self.open = true;
        self.mode = MasterPasswordMode::Setup;
        self.clear_all_fields();
        self.error_message = None;
        self.attempt_count = 0;
        self.opening_time = ctx.input(|i| i.time);
        self.animation = DialogAnimation::default();
        self.animation.target_progress = 1.0;
    }

    /// Open dialog for password verification on startup
    pub fn open_verify(&mut self, ctx: &egui::Context) {
        self.open = true;
        self.mode = MasterPasswordMode::Verify;
        self.clear_all_fields();
        self.error_message = None;
        self.attempt_count = 0;
        self.opening_time = ctx.input(|i| i.time);
        self.animation = DialogAnimation::default();
        self.animation.target_progress = 1.0;
    }

    /// Open dialog for changing password
    pub fn open_change(&mut self, ctx: &egui::Context) {
        self.open = true;
        self.mode = MasterPasswordMode::Change;
        self.clear_all_fields();
        self.error_message = None;
        self.attempt_count = 0;
        self.opening_time = ctx.input(|i| i.time);
        self.animation = DialogAnimation::default();
        self.animation.target_progress = 1.0;
    }

    /// Open dialog for reset warning
    pub fn open_reset(&mut self, ctx: &egui::Context) {
        self.open = true;
        self.mode = MasterPasswordMode::Reset;
        self.clear_all_fields();
        self.error_message = None;
        self.opening_time = ctx.input(|i| i.time);
        self.animation = DialogAnimation::default();
        self.animation.target_progress = 1.0;
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
                "Password must contain lowercase, uppercase, numbers, and special characters"
                    .to_string(),
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
            self.error_message =
                Some("Password is too weak. Please use a stronger password.".to_string());
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
            self.error_message =
                Some("New password must be at least 8 characters long".to_string());
            return false;
        }

        let has_lowercase = self.password.chars().any(|c| c.is_ascii_lowercase());
        let has_uppercase = self.password.chars().any(|c| c.is_ascii_uppercase());
        let has_digit = self.password.chars().any(|c| c.is_ascii_digit());
        let has_special = self.password.chars().any(|c| !c.is_alphanumeric());

        if !has_lowercase || !has_uppercase || !has_digit || !has_special {
            self.error_message = Some(
                "New password must contain lowercase, uppercase, numbers, and special characters"
                    .to_string(),
            );
            return false;
        }

        if self.password != self.confirm_password {
            self.error_message = Some("New passwords do not match".to_string());
            return false;
        }

        if self.old_password == self.password {
            self.error_message =
                Some("New password must be different from current password".to_string());
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

        // Update animation
        let dt = ctx.input(|i| i.stable_dt).max(0.001).min(0.1);
        self.animation.update(ctx, dt);

        // Trigger error animation when error message is set
        if self.error_message.is_some() && self.animation.error_shake < 0.01 {
            self.animation.trigger_error();
        }

        let mut result = MasterPasswordDialogResult::None;
        let mut should_close = false;

        let title = match self.mode {
            MasterPasswordMode::Setup => "Set Master Password",
            MasterPasswordMode::Verify => "Enter Master Password",
            MasterPasswordMode::Change => "Change Master Password",
            MasterPasswordMode::Reset => "Reset Master Password",
        };

        // Center the dialog with animation
        let screen_rect = ctx.screen_rect();
        let window_size = egui::vec2(
            450.0,
            match self.mode {
                MasterPasswordMode::Setup => 420.0,
                MasterPasswordMode::Verify => 280.0,
                MasterPasswordMode::Change => 450.0,
                MasterPasswordMode::Reset => 320.0,
            },
        );
        let base_pos = screen_rect.center() - window_size * 0.5;
        let shake_offset = self.animation.get_shake_offset();
        let animated_pos = base_pos + shake_offset;

        // Apply scale transform through layer manipulation
        let scale = self.animation.get_scale();
        let alpha = self.animation.get_alpha();

        egui::Window::new(title)
            .collapsible(false)
            .resizable(false)
            .fixed_size(window_size)
            .fixed_pos(animated_pos)
            .title_bar(true)
            .frame(egui::Frame::window(&ctx.style()).multiply_with_opacity(alpha))
            .show(ctx, |ui| {
                ui.spacing_mut().item_spacing.y = 12.0;

                // Apply scale to content
                let available_size = ui.available_size();
                let centered_pos =
                    ui.cursor().min + (available_size - available_size * scale) * 0.5;

                ui.allocate_ui_at_rect(
                    egui::Rect::from_min_size(centered_pos, available_size * scale),
                    |ui| {
                        ui.set_clip_rect(ui.max_rect());

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
                    },
                );
            });

        if should_close {
            // Fade out animation before closing
            if self.animation.target_progress > 0.0 {
                self.animation.target_progress = 0.0;
                ctx.request_repaint_after(std::time::Duration::from_millis(16));
            } else if self.animation.open_progress < 0.01 {
                self.close();
            }
        }

        result
    }

    fn show_setup_ui(
        &mut self,
        ui: &mut Ui,
        result: &mut MasterPasswordDialogResult,
        should_close: &mut bool,
    ) {
        ui.label("Welcome to EasySSH Lite!");
        ui.label("Please set a master password to secure your SSH configurations.");
        ui.label("This password will be used to encrypt all your sensitive data.");
        ui.separator();

        // Password field
        ui.horizontal(|ui| {
            ui.label("Password:");
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if ui
                    .button(if self.show_password { "Hide" } else { "Show" })
                    .clicked()
                {
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
                if ui
                    .button(if self.show_confirm_password {
                        "Hide"
                    } else {
                        "Show"
                    })
                    .clicked()
                {
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

    fn show_verify_ui(
        &mut self,
        ui: &mut Ui,
        result: &mut MasterPasswordDialogResult,
        should_close: &mut bool,
    ) {
        ui.label("Please enter your master password to unlock EasySSH.");
        ui.label("All your server configurations are securely encrypted.");
        ui.separator();

        // Attempt warning
        if self.attempt_count > 0 {
            let remaining = self.max_attempts.saturating_sub(self.attempt_count);
            ui.colored_label(
                egui::Color32::YELLOW,
                format!(
                    "Warning: {} failed attempts. {} attempts remaining.",
                    self.attempt_count, remaining
                ),
            );
        }

        // Password field
        ui.horizontal(|ui| {
            ui.label("Password:");
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if ui
                    .button(if self.show_password { "Hide" } else { "Show" })
                    .clicked()
                {
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

    fn show_change_ui(
        &mut self,
        ui: &mut Ui,
        result: &mut MasterPasswordDialogResult,
        should_close: &mut bool,
    ) {
        ui.label("Change your master password.");
        ui.label("All existing encrypted data will be re-encrypted with the new password.");
        ui.separator();

        // Old password field
        ui.horizontal(|ui| {
            ui.label("Current:");
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if ui
                    .button(if self.show_old_password {
                        "Hide"
                    } else {
                        "Show"
                    })
                    .clicked()
                {
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
                if ui
                    .button(if self.show_password { "Hide" } else { "Show" })
                    .clicked()
                {
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
                if ui
                    .button(if self.show_confirm_password {
                        "Hide"
                    } else {
                        "Show"
                    })
                    .clicked()
                {
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

    fn show_reset_ui(
        &mut self,
        ui: &mut Ui,
        result: &mut MasterPasswordDialogResult,
        should_close: &mut bool,
    ) {
        // Warning icon and title
        ui.horizontal(|ui| {
            ui.colored_label(egui::Color32::RED, RichText::new("⚠").size(24.0));
            ui.heading("Reset Master Password");
        });

        ui.separator();

        ui.label(RichText::new("WARNING: This action cannot be undone!").strong());
        ui.colored_label(
            egui::Color32::RED,
            "Resetting your master password will permanently delete all encrypted data, including:",
        );

        ui.indent("warning_list", |ui| {
            ui.label("• All stored SSH passwords");
            ui.label("• All encrypted server configurations");
            ui.label("• All secure vault items");
            ui.label("• Your encrypted keychain data");
        });

        ui.colored_label(
            egui::Color32::YELLOW,
            "You will need to re-add all your servers manually.",
        );

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
        self.animation.trigger_error();
    }

    /// Trigger error animation
    pub fn trigger_error_animation(&mut self) {
        self.animation.trigger_error();
    }

    /// Trigger success animation
    pub fn trigger_success_animation(&mut self) {
        self.animation.trigger_success();
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
    SetPassword {
        password: String,
    },
    /// Verify: Password entered
    Verify {
        password: String,
        attempt: u32,
    },
    /// Change: Password change requested
    ChangePassword {
        old_password: String,
        new_password: String,
    },
    /// Reset: User confirmed reset
    ResetConfirmed,
    /// Max attempts reached
    MaxAttemptsReached,
    /// User clicked forgot password
    ForgotPassword,
    /// User cancelled
    Cancel,
}

/// Parsed SSH config host entry for preview
#[derive(Debug, Clone)]
pub struct ParsedSshHost {
    pub name: String,
    pub host: String,
    pub port: i64,
    pub username: String,
    pub auth_type: String,
    pub identity_file: Option<String>,
    pub group_name: Option<String>,
    pub selected: bool,
    pub exists: bool,
    pub warnings: Vec<String>,
}

/// Import state for the dialog
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImportState {
    Idle,
    Loading,
    Parsing,
    Previewing,
    Importing,
    Completed,
    Error,
}

/// Import Config Dialog for importing ~/.ssh/config files
pub struct ImportConfigDialog {
    pub open: bool,
    pub file_path: Option<std::path::PathBuf>,
    pub parsed_hosts: Vec<ParsedSshHost>,
    pub import_state: ImportState,
    pub conflict_resolution: easyssh_core::ConfigConflictResolution,
    pub error_message: Option<String>,
    pub import_result: Option<easyssh_core::ImportResult>,
    pub progress: f32,
    pub search_filter: String,
    pub show_existing_only: bool,
    pub show_conflicts_only: bool,
}

impl Default for ImportConfigDialog {
    fn default() -> Self {
        Self {
            open: false,
            file_path: None,
            parsed_hosts: Vec::new(),
            import_state: ImportState::Idle,
            conflict_resolution: easyssh_core::ConfigConflictResolution::Skip,
            error_message: None,
            import_result: None,
            progress: 0.0,
            search_filter: String::new(),
            show_existing_only: false,
            show_conflicts_only: false,
        }
    }
}

impl ImportConfigDialog {
    pub fn new() -> Self {
        Self::default()
    }

    /// Open the dialog, optionally with a pre-selected file path
    pub fn open(&mut self) {
        self.open = true;
        self.reset_state();
        // Auto-load default SSH config path
        self.load_default_ssh_config();
    }

    /// Open with a specific file path
    pub fn open_with_path(&mut self, path: std::path::PathBuf) {
        self.open = true;
        self.reset_state();
        self.file_path = Some(path);
    }

    /// Reset all state
    fn reset_state(&mut self) {
        self.file_path = None;
        self.parsed_hosts.clear();
        self.import_state = ImportState::Idle;
        self.error_message = None;
        self.import_result = None;
        self.progress = 0.0;
        self.search_filter.clear();
        self.show_existing_only = false;
        self.show_conflicts_only = false;
    }

    /// Close the dialog
    pub fn close(&mut self) {
        self.open = false;
        self.reset_state();
    }

    /// Load the default SSH config path (~/.ssh/config)
    fn load_default_ssh_config(&mut self) {
        if let Some(home) = dirs::home_dir() {
            let ssh_config = home.join(".ssh").join("config");
            if ssh_config.exists() {
                self.file_path = Some(ssh_config);
            }
        }
    }

    /// Parse the SSH config file content
    fn parse_ssh_config(&mut self, content: &str) {
        self.parsed_hosts.clear();
        let mut current_host: Option<ParsedSshHost> = None;

        for line in content.lines() {
            let line = line.trim();

            // Skip comments and empty lines
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            // Parse key-value pairs
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() < 2 {
                continue;
            }

            let key = parts[0].to_lowercase();
            let value = parts[1..].join(" ");

            match key.as_str() {
                "host" => {
                    // Save previous host if exists
                    if let Some(host) = current_host.take() {
                        if !host.name.starts_with('*') {
                            // Skip wildcard patterns
                            self.parsed_hosts.push(host);
                        }
                    }

                    // Start new host
                    current_host = Some(ParsedSshHost {
                        name: value.clone(),
                        host: value.clone(), // Will be updated if HostName is present
                        port: 22,
                        username: String::from("root"),
                        auth_type: String::from("password"),
                        identity_file: None,
                        group_name: None,
                        selected: true, // Default to selected
                        exists: false,  // Will be checked later
                        warnings: Vec::new(),
                    });
                }
                "hostname" => {
                    if let Some(ref mut host) = current_host {
                        host.host = value;
                    }
                }
                "port" => {
                    if let Some(ref mut host) = current_host {
                        host.port = value.parse().unwrap_or(22);
                    }
                }
                "user" => {
                    if let Some(ref mut host) = current_host {
                        host.username = value;
                    }
                }
                "identityfile" => {
                    if let Some(ref mut host) = current_host {
                        let expanded_path = if value.starts_with('~') {
                            if let Some(home) = dirs::home_dir() {
                                value.replacen('~', &home.to_string_lossy(), 1)
                            } else {
                                value.clone()
                            }
                        } else {
                            value.clone()
                        };
                        host.identity_file = Some(expanded_path);
                        host.auth_type = String::from("key");

                        // Add warning if identity file doesn't exist
                        if !std::path::Path::new(&host.identity_file.as_ref().unwrap()).exists() {
                            host.warnings.push("Identity file not found".to_string());
                        }
                    }
                }
                "forwardagent" => {
                    if let Some(ref mut host) = current_host {
                        if value.to_lowercase() == "yes" {
                            host.auth_type = String::from("agent");
                        }
                    }
                }
                _ => {}
            }
        }

        // Don't forget the last host
        if let Some(host) = current_host {
            if !host.name.starts_with('*') {
                self.parsed_hosts.push(host);
            }
        }
    }

    /// Check existing servers and mark conflicts
    fn check_existing_servers(&mut self, existing_servers: &[crate::viewmodels::ServerViewModel]) {
        // Build a lookup map by (host, username)
        let existing_map: std::collections::HashMap<(String, String), bool> = existing_servers
            .iter()
            .map(|s| ((s.host.clone(), s.username.clone()), true))
            .collect();

        // Also check by name
        let existing_names: std::collections::HashSet<String> =
            existing_servers.iter().map(|s| s.name.clone()).collect();

        for host in &mut self.parsed_hosts {
            // Check if exists by host+username or by name
            host.exists = existing_map.contains_key(&(host.host.clone(), host.username.clone()))
                || existing_names.contains(&host.name);

            if host.exists {
                host.warnings.push("Already exists".to_string());
                // Default: skip existing servers
                if self.conflict_resolution == easyssh_core::ConfigConflictResolution::Skip {
                    host.selected = false;
                }
            }
        }
    }

    /// Select all hosts
    pub fn select_all(&mut self) {
        for host in &mut self.parsed_hosts {
            host.selected = true;
        }
    }

    /// Deselect all hosts
    pub fn deselect_all(&mut self) {
        for host in &mut self.parsed_hosts {
            host.selected = false;
        }
    }

    /// Select only new (non-existing) hosts
    pub fn select_new_only(&mut self) {
        for host in &mut self.parsed_hosts {
            host.selected = !host.exists;
        }
    }

    /// Get filtered hosts for display
    fn get_filtered_hosts(&self) -> Vec<&ParsedSshHost> {
        self.parsed_hosts
            .iter()
            .filter(|h| {
                // Search filter
                if !self.search_filter.is_empty() {
                    let filter = self.search_filter.to_lowercase();
                    if !h.name.to_lowercase().contains(&filter)
                        && !h.host.to_lowercase().contains(&filter)
                        && !h.username.to_lowercase().contains(&filter)
                    {
                        return false;
                    }
                }

                // Show existing only filter
                if self.show_existing_only && !h.exists {
                    return false;
                }

                // Show conflicts only filter
                if self.show_conflicts_only && !h.exists {
                    return false;
                }

                true
            })
            .collect()
    }

    /// Get count statistics
    fn get_stats(&self) -> (usize, usize, usize, usize) {
        let total = self.parsed_hosts.len();
        let selected = self.parsed_hosts.iter().filter(|h| h.selected).count();
        let new_count = self.parsed_hosts.iter().filter(|h| !h.exists).count();
        let existing_count = self.parsed_hosts.iter().filter(|h| h.exists).count();
        (total, selected, new_count, existing_count)
    }

    /// Show the dialog
    pub fn show(
        &mut self,
        ctx: &egui::Context,
        view_model: &std::sync::Arc<std::sync::Mutex<crate::viewmodels::AppViewModel>>,
    ) -> ImportConfigDialogResult {
        if !self.open {
            return ImportConfigDialogResult::None;
        }

        let mut result = ImportConfigDialogResult::None;
        let mut should_close = false;

        // Calculate window size based on content
        let window_width = 700.0;
        let window_height = 550.0;

        Window::new("Import SSH Configuration")
            .collapsible(false)
            .resizable(true)
            .default_size([window_width, window_height])
            .min_size([500.0, 400.0])
            .frame(egui::Frame {
                fill: egui::Color32::from_rgb(42, 48, 58),
                stroke: egui::Stroke::new(1.0, egui::Color32::from_rgb(60, 70, 85)),
                ..Default::default()
            })
            .show(ctx, |ui| {
                self.render_content(ui, view_model, &mut result, &mut should_close);
            });

        // Handle state transitions
        if self.import_state == ImportState::Loading && self.file_path.is_some() {
            self.load_and_parse_file(view_model);
        }

        if should_close {
            self.close();
        }

        result
    }

    /// Render the dialog content
    fn render_content(
        &mut self,
        ui: &mut Ui,
        view_model: &std::sync::Arc<std::sync::Mutex<crate::viewmodels::AppViewModel>>,
        result: &mut ImportConfigDialogResult,
        should_close: &mut bool,
    ) {
        ui.spacing_mut().item_spacing.y = 10.0;

        // Header with file selection
        ui.group(|ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new("SSH Config File:").strong());

                if let Some(ref path) = self.file_path {
                    ui.label(path.display().to_string());
                } else {
                    ui.label("No file selected");
                }

                if ui
                    .add(Button::new("📁 Browse...").min_size([80.0, 28.0].into()))
                    .clicked()
                {
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("SSH Config", &["config", "", "*"])
                        .add_filter("All Files", &["*"])
                        .set_title("Select SSH Config File")
                        .pick_file()
                    {
                        self.file_path = Some(path);
                        self.import_state = ImportState::Loading;
                    }
                }

                // Quick load default button
                if self.file_path.is_none() || !self.is_default_path() {
                    if ui.button("Load ~/.ssh/config").clicked() {
                        self.load_default_ssh_config();
                        if self.file_path.is_some() {
                            self.import_state = ImportState::Loading;
                        }
                    }
                }
            });
        });

        // Progress indicator
        if self.import_state == ImportState::Loading
            || self.import_state == ImportState::Parsing
            || self.import_state == ImportState::Importing
        {
            ui.add_space(5.0);
            ui.horizontal(|ui| {
                let state_text = match self.import_state {
                    ImportState::Loading => "Loading file...",
                    ImportState::Parsing => "Parsing hosts...",
                    ImportState::Importing => "Importing...",
                    _ => "",
                };
                ui.label(RichText::new(state_text).color(egui::Color32::from_rgb(100, 180, 255)));

                // Progress bar
                let progress_bar = egui::ProgressBar::new(self.progress)
                    .desired_width(200.0)
                    .animate(self.import_state == ImportState::Importing);
                ui.add(progress_bar);
            });
            ui.add_space(5.0);
        }

        // Error message
        if let Some(ref error) = self.error_message {
            ui.add_space(5.0);
            ui.colored_label(egui::Color32::RED, error);
            ui.add_space(5.0);
        }

        // Preview section (when parsed hosts are available)
        if self.import_state == ImportState::Previewing
            || self.import_state == ImportState::Completed
        {
            self.render_preview_section(ui, result, should_close);
        }

        // Import result section
        if self.import_state == ImportState::Completed && self.import_result.is_some() {
            self.render_result_section(ui, should_close);
        }

        // Bottom buttons (when idle or previewing)
        if self.import_state == ImportState::Idle || self.import_state == ImportState::Previewing {
            ui.separator();
            ui.horizontal(|ui| {
                // Cancel button
                if ui.button("Cancel").clicked() {
                    *should_close = true;
                    *result = ImportConfigDialogResult::Cancel;
                }

                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    // Import button
                    let can_import = self.import_state == ImportState::Previewing
                        && self.parsed_hosts.iter().any(|h| h.selected);

                    let import_button = Button::new("Import Selected")
                        .min_size([120.0, 36.0].into())
                        .fill(if can_import {
                            egui::Color32::from_rgb(64, 156, 255)
                        } else {
                            egui::Color32::from_rgb(80, 80, 80)
                        });

                    if ui.add_enabled(can_import, import_button).clicked() {
                        self.perform_import(view_model);
                    }
                });
            });
        }
    }

    /// Check if current path is the default ~/.ssh/config
    fn is_default_path(&self) -> bool {
        if let Some(ref path) = self.file_path {
            if let Some(home) = dirs::home_dir() {
                let default_path = home.join(".ssh").join("config");
                return *path == default_path;
            }
        }
        false
    }

    /// Load and parse the SSH config file
    fn load_and_parse_file(
        &mut self,
        view_model: &std::sync::Arc<std::sync::Mutex<crate::viewmodels::AppViewModel>>,
    ) {
        if let Some(ref path) = self.file_path {
            self.import_state = ImportState::Loading;
            self.progress = 0.0;

            // Read file content
            match std::fs::read_to_string(path) {
                Ok(content) => {
                    self.progress = 0.5;
                    self.import_state = ImportState::Parsing;

                    // Parse SSH config
                    self.parse_ssh_config(&content);

                    // Get existing servers to check for conflicts
                    let existing_servers = view_model.lock().unwrap().get_servers();
                    self.check_existing_servers(&existing_servers);

                    self.progress = 1.0;
                    self.import_state = ImportState::Previewing;

                    if self.parsed_hosts.is_empty() {
                        self.error_message = Some("No hosts found in SSH config file".to_string());
                        self.import_state = ImportState::Error;
                    }
                }
                Err(e) => {
                    self.error_message = Some(format!("Failed to read file: {}", e));
                    self.import_state = ImportState::Error;
                }
            }
        }
    }

    /// Render the preview section with hosts table
    fn render_preview_section(
        &mut self,
        ui: &mut Ui,
        result: &mut ImportConfigDialogResult,
        _should_close: &mut bool,
    ) {
        let (total, selected, new_count, existing_count) = self.get_stats();

        // Statistics header
        ui.group(|ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new("Found:").strong());
                ui.label(format!("{} hosts", total));
                ui.label(RichText::new("|").color(egui::Color32::GRAY));
                ui.label(RichText::new("New:").color(egui::Color32::GREEN));
                ui.label(format!("{}", new_count));
                ui.label(RichText::new("|").color(egui::Color32::GRAY));
                ui.label(RichText::new("Existing:").color(egui::Color32::YELLOW));
                ui.label(format!("{}", existing_count));
                ui.label(RichText::new("|").color(egui::Color32::GRAY));
                ui.label(RichText::new("Selected:").color(egui::Color32::from_rgb(100, 180, 255)));
                ui.label(format!("{}", selected));
            });

            ui.add_space(5.0);

            // Conflict resolution options
            ui.horizontal(|ui| {
                ui.label(RichText::new("Conflict Resolution:").strong());
                ui.radio_value(
                    &mut self.conflict_resolution,
                    easyssh_core::ConfigConflictResolution::Skip,
                    "Skip existing",
                );
                ui.radio_value(
                    &mut self.conflict_resolution,
                    easyssh_core::ConfigConflictResolution::Overwrite,
                    "Overwrite",
                );
                ui.radio_value(
                    &mut self.conflict_resolution,
                    easyssh_core::ConfigConflictResolution::Merge,
                    "Merge",
                );
            });
        });

        ui.add_space(5.0);

        // Filter controls
        ui.horizontal(|ui| {
            ui.label("Filter:");
            ui.add(
                TextEdit::singleline(&mut self.search_filter)
                    .desired_width(150.0)
                    .hint_text("Search..."),
            );

            ui.checkbox(&mut self.show_existing_only, "Existing only");
            ui.checkbox(&mut self.show_conflicts_only, "Conflicts only");

            ui.separator();

            // Bulk selection buttons
            if ui.button("Select All").clicked() {
                self.select_all();
            }
            if ui.button("Select None").clicked() {
                self.deselect_all();
            }
            if ui.button("Select New").clicked() {
                self.select_new_only();
            }
        });

        ui.add_space(5.0);

        // Hosts table
        egui::ScrollArea::vertical()
            .max_height(280.0)
            .show(ui, |ui| {
                use egui_extras::{Column, TableBuilder};

                TableBuilder::new(ui)
                    .striped(true)
                    .resizable(true)
                    .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
                    .column(Column::initial(40.0).at_least(30.0)) // Checkbox
                    .column(Column::initial(100.0).at_least(80.0)) // Name
                    .column(Column::initial(120.0).at_least(100.0)) // Hostname
                    .column(Column::initial(50.0).at_least(40.0)) // Port
                    .column(Column::initial(80.0).at_least(60.0)) // User
                    .column(Column::initial(80.0).at_least(60.0)) // Auth
                    .column(Column::remainder().at_least(100.0)) // Warnings
                    .header(25.0, |mut header| {
                        header.col(|ui| {
                            ui.strong("Sel");
                        });
                        header.col(|ui| {
                            ui.strong("Name");
                        });
                        header.col(|ui| {
                            ui.strong("Hostname");
                        });
                        header.col(|ui| {
                            ui.strong("Port");
                        });
                        header.col(|ui| {
                            ui.strong("User");
                        });
                        header.col(|ui| {
                            ui.strong("Auth");
                        });
                        header.col(|ui| {
                            ui.strong("Status");
                        });
                    })
                    .body(|mut body| {
                        // Pre-compute filtered indices to avoid borrow conflict
                        let filtered_indices: Vec<usize> = self
                            .parsed_hosts
                            .iter()
                            .enumerate()
                            .filter(|(_, host)| {
                                // Search filter
                                if !self.search_filter.is_empty() {
                                    let filter = self.search_filter.to_lowercase();
                                    if !host.name.to_lowercase().contains(&filter)
                                        && !host.host.to_lowercase().contains(&filter)
                                        && !host.username.to_lowercase().contains(&filter)
                                    {
                                        return false;
                                    }
                                }

                                // Show existing only filter
                                if self.show_existing_only && !host.exists {
                                    return false;
                                }

                                // Show conflicts only filter
                                if self.show_conflicts_only && !host.exists {
                                    return false;
                                }

                                true
                            })
                            .map(|(idx, _)| idx)
                            .collect();

                        for host_idx in filtered_indices {
                            body.row(20.0, |mut row| {
                                let host = &mut self.parsed_hosts[host_idx];

                                row.col(|ui| {
                                    // Checkbox for selection
                                    let checkbox_response = ui.checkbox(&mut host.selected, "");
                                    if checkbox_response.clicked() {
                                        *result = ImportConfigDialogResult::SelectionChanged;
                                    }
                                });

                                row.col(|ui| {
                                    // Name (with color based on status)
                                    let name_color = if host.exists {
                                        egui::Color32::YELLOW
                                    } else {
                                        egui::Color32::WHITE
                                    };
                                    ui.label(RichText::new(&host.name).color(name_color));
                                });

                                row.col(|ui| {
                                    ui.label(&host.host);
                                });

                                row.col(|ui| {
                                    ui.label(host.port.to_string());
                                });

                                row.col(|ui| {
                                    ui.label(&host.username);
                                });

                                row.col(|ui| {
                                    let auth_text = match host.auth_type.as_str() {
                                        "key" => "Key",
                                        "agent" => "Agent",
                                        _ => "Password",
                                    };
                                    ui.label(auth_text);
                                });

                                row.col(|ui| {
                                    // Status/warnings
                                    if host.exists {
                                        ui.colored_label(egui::Color32::YELLOW, "Exists");
                                    } else if !host.warnings.is_empty() {
                                        ui.colored_label(
                                            egui::Color32::from_rgb(255, 150, 0),
                                            host.warnings.join(", "),
                                        );
                                    } else {
                                        ui.colored_label(egui::Color32::GREEN, "OK");
                                    }
                                });
                            });
                        }
                    });
            });
    }

    /// Render the import result section
    fn render_result_section(&mut self, ui: &mut Ui, should_close: &mut bool) {
        ui.separator();
        ui.add_space(10.0);

        ui.group(|ui| {
            ui.heading(
                RichText::new("Import Complete!")
                    .color(egui::Color32::GREEN)
                    .strong(),
            );
            ui.add_space(10.0);

            if let Some(ref import_result) = self.import_result {
                ui.horizontal(|ui| {
                    ui.label(format!(
                        "Servers imported: {}",
                        import_result.servers_imported
                    ));
                });
                ui.horizontal(|ui| {
                    ui.label(format!(
                        "Servers skipped: {}",
                        import_result.servers_skipped
                    ));
                });
                if import_result.groups_imported > 0 {
                    ui.label(format!(
                        "Groups imported: {}",
                        import_result.groups_imported
                    ));
                }
                if import_result.identities_imported > 0 {
                    ui.label(format!(
                        "Identities imported: {}",
                        import_result.identities_imported
                    ));
                }

                if !import_result.errors.is_empty() {
                    ui.add_space(5.0);
                    ui.separator();
                    ui.label(RichText::new("Errors:").color(egui::Color32::RED).strong());
                    for error in &import_result.errors {
                        ui.colored_label(egui::Color32::RED, format!("• {}", error));
                    }
                }
            }
        });

        ui.add_space(10.0);
        ui.horizontal(|ui| {
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if ui.button("Close").clicked() {
                    *should_close = true;
                }
            });
        });
    }

    /// Perform the import
    fn perform_import(
        &mut self,
        view_model: &std::sync::Arc<std::sync::Mutex<crate::viewmodels::AppViewModel>>,
    ) {
        self.import_state = ImportState::Importing;
        self.progress = 0.0;

        // Build SSH config content from selected hosts
        let mut config_content = String::new();
        let selected_hosts: Vec<_> = self.parsed_hosts.iter().filter(|h| h.selected).collect();
        let total_hosts = selected_hosts.len();

        for (i, host) in selected_hosts.iter().enumerate() {
            config_content.push_str(&format!("Host {}\n", host.name));
            config_content.push_str(&format!("    HostName {}\n", host.host));
            config_content.push_str(&format!("    Port {}\n", host.port));
            config_content.push_str(&format!("    User {}\n", host.username));

            if let Some(ref identity_file) = host.identity_file {
                config_content.push_str(&format!("    IdentityFile {}\n", identity_file));
            }

            if host.auth_type == "agent" {
                config_content.push_str("    ForwardAgent yes\n");
            }

            config_content.push('\n');
            self.progress = (i + 1) as f32 / total_hosts as f32;
        }

        // Perform import using the view model
        let import_result = view_model.lock().unwrap().import_config(
            &config_content,
            easyssh_core::ImportFormat::SshConfig,
            self.conflict_resolution.clone(),
        );

        match import_result {
            Ok(result) => {
                self.import_result = Some(result);
                self.import_state = ImportState::Completed;
                self.progress = 1.0;
            }
            Err(e) => {
                self.error_message = Some(format!("Import failed: {}", e));
                self.import_state = ImportState::Error;
            }
        }
    }
}

/// Import config dialog result
#[derive(Debug, Clone)]
pub enum ImportConfigDialogResult {
    None,
    ImportCompleted {
        servers_imported: usize,
        servers_skipped: usize,
    },
    SelectionChanged,
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
            PasswordDialogResult::Ok {
                password,
                save_password,
            } => {
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

    #[test]
    fn test_import_config_dialog_creation() {
        let dialog = ImportConfigDialog::new();
        assert!(!dialog.open);
        assert!(dialog.file_path.is_none());
        assert!(dialog.parsed_hosts.is_empty());
        assert_eq!(dialog.import_state, ImportState::Idle);
    }

    #[test]
    fn test_import_config_dialog_open() {
        let mut dialog = ImportConfigDialog::new();
        dialog.open();
        assert!(dialog.open);
        assert_eq!(dialog.import_state, ImportState::Idle);
    }

    #[test]
    fn test_import_config_dialog_parse_ssh_config() {
        let mut dialog = ImportConfigDialog::new();
        let config_content = r#"
# My servers
Host myserver
    HostName 192.168.1.100
    Port 2222
    User admin
    IdentityFile ~/.ssh/id_rsa

Host production
    HostName prod.example.com
    User root
    ForwardAgent yes
"#;

        dialog.parse_ssh_config(config_content);

        assert_eq!(dialog.parsed_hosts.len(), 2);

        // First host
        assert_eq!(dialog.parsed_hosts[0].name, "myserver");
        assert_eq!(dialog.parsed_hosts[0].host, "192.168.1.100");
        assert_eq!(dialog.parsed_hosts[0].port, 2222);
        assert_eq!(dialog.parsed_hosts[0].username, "admin");
        assert_eq!(dialog.parsed_hosts[0].auth_type, "key");
        assert!(dialog.parsed_hosts[0].identity_file.is_some());
        assert!(dialog.parsed_hosts[0].selected);

        // Second host
        assert_eq!(dialog.parsed_hosts[1].name, "production");
        assert_eq!(dialog.parsed_hosts[1].host, "prod.example.com");
        assert_eq!(dialog.parsed_hosts[1].port, 22);
        assert_eq!(dialog.parsed_hosts[1].username, "root");
        assert_eq!(dialog.parsed_hosts[1].auth_type, "agent");
    }

    #[test]
    fn test_import_config_dialog_select_all() {
        let mut dialog = ImportConfigDialog::new();
        dialog.parsed_hosts = vec![
            ParsedSshHost {
                name: "server1".to_string(),
                host: "host1".to_string(),
                port: 22,
                username: "user".to_string(),
                auth_type: "password".to_string(),
                identity_file: None,
                group_name: None,
                selected: false,
                exists: false,
                warnings: vec![],
            },
            ParsedSshHost {
                name: "server2".to_string(),
                host: "host2".to_string(),
                port: 22,
                username: "user".to_string(),
                auth_type: "password".to_string(),
                identity_file: None,
                group_name: None,
                selected: false,
                exists: true,
                warnings: vec!["Already exists".to_string()],
            },
        ];

        dialog.select_all();
        assert!(dialog.parsed_hosts.iter().all(|h| h.selected));

        dialog.deselect_all();
        assert!(dialog.parsed_hosts.iter().all(|h| !h.selected));

        dialog.select_new_only();
        assert!(dialog.parsed_hosts[0].selected);
        assert!(!dialog.parsed_hosts[1].selected);
    }

    #[test]
    fn test_import_config_dialog_stats() {
        let mut dialog = ImportConfigDialog::new();
        dialog.parsed_hosts = vec![
            ParsedSshHost {
                name: "server1".to_string(),
                host: "host1".to_string(),
                port: 22,
                username: "user".to_string(),
                auth_type: "password".to_string(),
                identity_file: None,
                group_name: None,
                selected: true,
                exists: false,
                warnings: vec![],
            },
            ParsedSshHost {
                name: "server2".to_string(),
                host: "host2".to_string(),
                port: 22,
                username: "user".to_string(),
                auth_type: "password".to_string(),
                identity_file: None,
                group_name: None,
                selected: false,
                exists: true,
                warnings: vec!["Already exists".to_string()],
            },
            ParsedSshHost {
                name: "server3".to_string(),
                host: "host3".to_string(),
                port: 22,
                username: "user".to_string(),
                auth_type: "password".to_string(),
                identity_file: None,
                group_name: None,
                selected: true,
                exists: false,
                warnings: vec![],
            },
        ];

        let (total, selected, new_count, existing_count) = dialog.get_stats();
        assert_eq!(total, 3);
        assert_eq!(selected, 2);
        assert_eq!(new_count, 2);
        assert_eq!(existing_count, 1);
    }

    #[test]
    fn test_import_config_dialog_filter() {
        let mut dialog = ImportConfigDialog::new();
        dialog.parsed_hosts = vec![
            ParsedSshHost {
                name: "myserver".to_string(),
                host: "192.168.1.100".to_string(),
                port: 22,
                username: "admin".to_string(),
                auth_type: "password".to_string(),
                identity_file: None,
                group_name: None,
                selected: true,
                exists: false,
                warnings: vec![],
            },
            ParsedSshHost {
                name: "production".to_string(),
                host: "prod.example.com".to_string(),
                port: 22,
                username: "root".to_string(),
                auth_type: "password".to_string(),
                identity_file: None,
                group_name: None,
                selected: true,
                exists: true,
                warnings: vec!["Already exists".to_string()],
            },
        ];

        // Test search filter
        dialog.search_filter = "myserver".to_string();
        let filtered = dialog.get_filtered_hosts();
        assert_eq!(filtered.len(), 1);

        // Test existing only filter
        dialog.search_filter.clear();
        dialog.show_existing_only = true;
        let filtered = dialog.get_filtered_hosts();
        assert_eq!(filtered.len(), 1);

        // Test conflicts only filter
        dialog.show_existing_only = false;
        dialog.show_conflicts_only = true;
        let filtered = dialog.get_filtered_hosts();
        assert_eq!(filtered.len(), 1);
    }

    #[test]
    fn test_import_state() {
        assert_eq!(ImportState::Idle, ImportState::Idle);
        assert_ne!(ImportState::Idle, ImportState::Loading);
        assert_ne!(ImportState::Previewing, ImportState::Importing);
    }

    #[test]
    fn test_import_config_dialog_result() {
        let result = ImportConfigDialogResult::ImportCompleted {
            servers_imported: 5,
            servers_skipped: 2,
        };

        match result {
            ImportConfigDialogResult::ImportCompleted {
                servers_imported,
                servers_skipped,
            } => {
                assert_eq!(servers_imported, 5);
                assert_eq!(servers_skipped, 2);
            }
            _ => panic!("Expected ImportCompleted variant"),
        }
    }
}
