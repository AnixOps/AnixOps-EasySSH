//! Detail Panel Module for EasySSH Lite
//!
//! Provides the right panel containing:
//! - Server information display
//! - Edit form for server details
//! - Connection actions

use crate::terminal_launcher::{SshConnection, TerminalDiagnostics, TerminalPreference};
use crate::viewmodels::{GroupViewModel, ServerViewModel};
use egui::{Align, Button, Color32, Layout, RichText, TextEdit, Ui, Vec2};

/// Server info display component
pub struct ServerInfo {
    pub server: ServerViewModel,
    pub group_name: Option<String>,
    pub is_connected: bool,
    pub terminal_diagnostics: TerminalDiagnostics,
}

impl ServerInfo {
    pub fn new(server: ServerViewModel, groups: &[GroupViewModel], is_connected: bool) -> Self {
        let group_name = server
            .group_id
            .as_ref()
            .and_then(|id| groups.iter().find(|g| &g.id == id))
            .map(|g| g.name.clone());

        Self {
            server,
            group_name,
            is_connected,
            terminal_diagnostics: crate::terminal_launcher::get_terminal_diagnostics(),
        }
    }

    pub fn show(&self, ui: &mut Ui) -> ServerInfoResponse {
        let mut response = ServerInfoResponse::default();

        ui.vertical(|ui| {
            ui.spacing_mut().item_spacing.y = 16.0;

            // Header with server name
            ui.horizontal(|ui| {
                ui.heading(&self.server.name);

                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    // Status indicator
                    let status_text = if self.is_connected {
                        RichText::new("● Connected").color(Color32::GREEN)
                    } else {
                        RichText::new("● Disconnected").color(Color32::GRAY)
                    };
                    ui.label(status_text);
                });
            });

            ui.separator();

            // Connection details section
            ui.heading("Connection Details");

            egui::Grid::new("server_details_grid")
                .num_columns(2)
                .spacing([12.0, 8.0])
                .show(ui, |ui| {
                    ui.label("Host:");
                    ui.label(&self.server.host);
                    ui.end_row();

                    ui.label("Port:");
                    ui.label(self.server.port.to_string());
                    ui.end_row();

                    ui.label("Username:");
                    ui.label(&self.server.username);
                    ui.end_row();

                    ui.label("Authentication:");
                    let auth_display = match self.server.auth_type.as_str() {
                        "agent" => "SSH Agent Forwarding",
                        "key" => "Private Key",
                        "password" => "Password",
                        _ => &self.server.auth_type,
                    };
                    ui.label(auth_display);
                    ui.end_row();

                    ui.label("Group:");
                    ui.label(self.group_name.as_deref().unwrap_or("None"));
                    ui.end_row();
                });

            ui.separator();

            // Actions section
            ui.heading("Actions");

            ui.horizontal(|ui| {
                // Connect/Disconnect button
                if self.is_connected {
                    if ui
                        .button(RichText::new("🔌 Disconnect").size(14.0))
                        .clicked()
                    {
                        response.disconnect_clicked = true;
                    }
                } else {
                    let connect_btn = ui.button(RichText::new("▶ Connect").size(14.0));
                    if connect_btn.clicked() {
                        response.connect_clicked = true;
                    }

                    // Show terminal availability info
                    if !self.terminal_diagnostics.ssh_available {
                        ui.colored_label(Color32::YELLOW, "⚠ SSH command not found in PATH");
                    }
                }
            });

            ui.add_space(8.0);

            // Terminal diagnostics
            if !self.is_connected {
                ui.collapsing("Terminal Information", |ui| {
                    ui.label(format!(
                        "Windows Terminal: {}",
                        if self.terminal_diagnostics.windows_terminal_available {
                            "✓ Available"
                        } else {
                            "✗ Not found"
                        }
                    ));
                    ui.label(format!(
                        "PowerShell: {}",
                        if self.terminal_diagnostics.powershell_available {
                            "✓ Available"
                        } else {
                            "✗ Not found"
                        }
                    ));
                    ui.label(format!(
                        "CMD: {}",
                        if self.terminal_diagnostics.cmd_available {
                            "✓ Available"
                        } else {
                            "✗ Not found"
                        }
                    ));
                    ui.label(format!(
                        "SSH: {}",
                        if self.terminal_diagnostics.ssh_available {
                            "✓ Available"
                        } else {
                            "✗ Not found"
                        }
                    ));

                    if self.terminal_diagnostics.in_windows_terminal {
                        ui.colored_label(Color32::GREEN, "Running inside Windows Terminal");
                    }
                });
            }
        });

        response
    }

    pub fn build_ssh_connection(&self) -> SshConnection {
        SshConnection::new(
            self.server.host.clone(),
            self.server.port as u16,
            self.server.username.clone(),
            self.server.auth_type.clone(),
            None, // identity file would come from server record
        )
    }
}

/// Server info response
#[derive(Debug, Default)]
pub struct ServerInfoResponse {
    pub connect_clicked: bool,
    pub disconnect_clicked: bool,
}

/// Edit form component for server details
pub struct EditForm {
    pub server: ServerViewModel,
    pub groups: Vec<GroupViewModel>,
    pub edited_name: String,
    pub edited_host: String,
    pub edited_port: String,
    pub edited_username: String,
    pub edited_auth_type: String,
    pub edited_group_id: Option<String>,
    pub has_changes: bool,
    pub error_message: Option<String>,
}

impl EditForm {
    pub fn new(server: ServerViewModel, groups: Vec<GroupViewModel>) -> Self {
        Self {
            edited_name: server.name.clone(),
            edited_host: server.host.clone(),
            edited_port: server.port.to_string(),
            edited_username: server.username.clone(),
            edited_auth_type: server.auth_type.clone(),
            edited_group_id: server.group_id.clone(),
            server,
            groups,
            has_changes: false,
            error_message: None,
        }
    }

    pub fn show(&mut self, ui: &mut Ui) -> EditFormResponse {
        let mut response = EditFormResponse::default();

        ui.vertical(|ui| {
            ui.spacing_mut().item_spacing.y = 12.0;

            ui.heading("Edit Server");
            ui.separator();

            // Form fields
            egui::Grid::new("edit_form_grid")
                .num_columns(2)
                .spacing([12.0, 12.0])
                .show(ui, |ui| {
                    // Name
                    ui.label("Name:*");
                    let name_response = ui.text_edit_singleline(&mut self.edited_name);
                    if name_response.changed() {
                        self.check_for_changes();
                    }
                    ui.end_row();

                    // Host
                    ui.label("Host:*");
                    let host_response = ui.text_edit_singleline(&mut self.edited_host);
                    if host_response.changed() {
                        self.check_for_changes();
                    }
                    ui.end_row();

                    // Port
                    ui.label("Port:*");
                    let port_response =
                        ui.add(TextEdit::singleline(&mut self.edited_port).desired_width(60.0));
                    if port_response.changed() {
                        self.check_for_changes();
                    }
                    ui.end_row();

                    // Username
                    ui.label("Username:*");
                    let user_response = ui.text_edit_singleline(&mut self.edited_username);
                    if user_response.changed() {
                        self.check_for_changes();
                    }
                    ui.end_row();

                    // Auth Type
                    ui.label("Auth Type:");
                    let auth_response = egui::ComboBox::from_id_source("edit_form_auth")
                        .width(150.0)
                        .selected_text(match self.edited_auth_type.as_str() {
                            "agent" => "SSH Agent",
                            "key" => "Private Key",
                            "password" => "Password",
                            _ => "SSH Agent",
                        })
                        .show_ui(ui, |ui| {
                            let mut changed = false;
                            if ui
                                .selectable_value(
                                    &mut self.edited_auth_type,
                                    "agent".to_string(),
                                    "SSH Agent",
                                )
                                .clicked()
                            {
                                changed = true;
                            }
                            if ui
                                .selectable_value(
                                    &mut self.edited_auth_type,
                                    "key".to_string(),
                                    "Private Key",
                                )
                                .clicked()
                            {
                                changed = true;
                            }
                            if ui
                                .selectable_value(
                                    &mut self.edited_auth_type,
                                    "password".to_string(),
                                    "Password",
                                )
                                .clicked()
                            {
                                changed = true;
                            }
                            changed
                        })
                        .response;

                    if auth_response.changed() {
                        self.check_for_changes();
                    }
                    ui.end_row();

                    // Group
                    ui.label("Group:");
                    let group_response = egui::ComboBox::from_id_source("edit_form_group")
                        .width(150.0)
                        .selected_text(
                            self.edited_group_id
                                .as_ref()
                                .and_then(|id| self.groups.iter().find(|g| &g.id == id))
                                .map(|g| g.name.clone())
                                .unwrap_or_else(|| "None".to_string()),
                        )
                        .show_ui(ui, |ui| {
                            let mut changed = false;
                            if ui
                                .selectable_value(&mut self.edited_group_id, None, "None")
                                .clicked()
                            {
                                changed = true;
                            }
                            for group in &self.groups {
                                if ui
                                    .selectable_value(
                                        &mut self.edited_group_id,
                                        Some(group.id.clone()),
                                        &group.name,
                                    )
                                    .clicked()
                                {
                                    changed = true;
                                }
                            }
                            changed
                        })
                        .response;

                    if group_response.changed() {
                        self.check_for_changes();
                    }
                    ui.end_row();
                });

            // Error message
            if let Some(ref error) = self.error_message {
                ui.colored_label(Color32::RED, error);
            }

            ui.separator();

            // Action buttons
            ui.horizontal(|ui| {
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if ui.button("Cancel").clicked() {
                        response.cancelled = true;
                        self.reset();
                    }

                    let save_btn = ui.add_enabled(self.has_changes, Button::new("Save Changes"));
                    if save_btn.clicked() {
                        if self.validate() {
                            response.save_clicked = true;
                            response.server_data = Some(self.get_server_update());
                        }
                    }
                });
            });
        });

        response
    }

    fn check_for_changes(&mut self) {
        self.has_changes = self.edited_name != self.server.name
            || self.edited_host != self.server.host
            || self.edited_port != self.server.port.to_string()
            || self.edited_username != self.server.username
            || self.edited_auth_type != self.server.auth_type
            || self.edited_group_id != self.server.group_id;
    }

    fn validate(&mut self) -> bool {
        if self.edited_name.trim().is_empty() {
            self.error_message = Some("Name is required".to_string());
            return false;
        }
        if self.edited_host.trim().is_empty() {
            self.error_message = Some("Host is required".to_string());
            return false;
        }
        if self.edited_username.trim().is_empty() {
            self.error_message = Some("Username is required".to_string());
            return false;
        }
        if self.edited_port.parse::<i64>().is_err() {
            self.error_message = Some("Invalid port number".to_string());
            return false;
        }

        self.error_message = None;
        true
    }

    fn get_server_update(&self) -> ServerUpdateData {
        ServerUpdateData {
            id: self.server.id.clone(),
            name: self.edited_name.trim().to_string(),
            host: self.edited_host.trim().to_string(),
            port: self.edited_port.parse::<i64>().unwrap_or(22),
            username: self.edited_username.trim().to_string(),
            auth_type: self.edited_auth_type.clone(),
            group_id: self.edited_group_id.clone(),
        }
    }

    fn reset(&mut self) {
        self.edited_name = self.server.name.clone();
        self.edited_host = self.server.host.clone();
        self.edited_port = self.server.port.to_string();
        self.edited_username = self.server.username.clone();
        self.edited_auth_type = self.server.auth_type.clone();
        self.edited_group_id = self.server.group_id.clone();
        self.has_changes = false;
        self.error_message = None;
    }

    pub fn update_server(&mut self, server: ServerViewModel) {
        self.server = server;
        self.reset();
    }
}

/// Server update data
#[derive(Debug, Clone)]
pub struct ServerUpdateData {
    pub id: String,
    pub name: String,
    pub host: String,
    pub port: i64,
    pub username: String,
    pub auth_type: String,
    pub group_id: Option<String>,
}

/// Edit form response
#[derive(Debug, Default)]
pub struct EditFormResponse {
    pub save_clicked: bool,
    pub cancelled: bool,
    pub server_data: Option<ServerUpdateData>,
}

/// Detail panel showing server info or edit form
pub enum DetailPanel {
    Info(ServerInfo),
    Edit(EditForm),
    Empty,
}

impl DetailPanel {
    pub fn show(&mut self, ui: &mut Ui) -> DetailPanelResponse {
        match self {
            DetailPanel::Info(info) => {
                let response = info.show(ui);
                DetailPanelResponse::Info(response)
            }
            DetailPanel::Edit(form) => {
                let response = form.show(ui);
                DetailPanelResponse::Edit(response)
            }
            DetailPanel::Empty => {
                Self::show_empty_state(ui);
                DetailPanelResponse::None
            }
        }
    }

    fn show_empty_state(ui: &mut Ui) {
        ui.centered_and_justified(|ui| {
            ui.vertical_centered(|ui| {
                ui.label(RichText::new("📡").size(48.0));
                ui.add_space(16.0);
                ui.heading("No Server Selected");
                ui.add_space(8.0);
                ui.label("Select a server from the list to view details");
                ui.label("or add a new server to get started.");
            });
        });
    }

    pub fn set_info(
        &mut self,
        server: ServerViewModel,
        groups: &[GroupViewModel],
        is_connected: bool,
    ) {
        *self = DetailPanel::Info(ServerInfo::new(server, groups, is_connected));
    }

    pub fn set_edit(&mut self, server: ServerViewModel, groups: Vec<GroupViewModel>) {
        *self = DetailPanel::Edit(EditForm::new(server, groups));
    }

    pub fn set_empty(&mut self) {
        *self = DetailPanel::Empty;
    }

    pub fn is_editing(&self) -> bool {
        matches!(self, DetailPanel::Edit(_))
    }

    pub fn is_info(&self) -> bool {
        matches!(self, DetailPanel::Info(_))
    }

    pub fn switch_to_edit(&mut self, groups: Vec<GroupViewModel>) {
        if let DetailPanel::Info(info) = self {
            let server = info.server.clone();
            *self = DetailPanel::Edit(EditForm::new(server, groups));
        }
    }

    pub fn switch_to_info(&mut self, groups: &[GroupViewModel], is_connected: bool) {
        if let DetailPanel::Edit(form) = self {
            let server = form.server.clone();
            *self = DetailPanel::Info(ServerInfo::new(server, groups, is_connected));
        }
    }
}

/// Detail panel response
#[derive(Debug)]
pub enum DetailPanelResponse {
    None,
    Info(ServerInfoResponse),
    Edit(EditFormResponse),
}

/// Detail panel container with navigation
pub struct DetailPanelContainer {
    pub panel: DetailPanel,
    pub show_edit_button: bool,
}

impl Default for DetailPanelContainer {
    fn default() -> Self {
        Self {
            panel: DetailPanel::Empty,
            show_edit_button: true,
        }
    }
}

impl DetailPanelContainer {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn show(&mut self, ui: &mut Ui) -> DetailPanelContainerResponse {
        let mut response = DetailPanelContainerResponse::default();

        ui.vertical(|ui| {
            // Toolbar
            if self.show_edit_button && self.panel.is_info() {
                ui.horizontal(|ui| {
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if ui.button("✏ Edit").clicked() {
                            response.edit_requested = true;
                        }
                    });
                });
                ui.separator();
            }

            // Panel content
            match self.panel.show(ui) {
                DetailPanelResponse::Info(info_response) => {
                    if info_response.connect_clicked {
                        response.connect_clicked = true;
                    }
                    if info_response.disconnect_clicked {
                        response.disconnect_clicked = true;
                    }
                }
                DetailPanelResponse::Edit(edit_response) => {
                    if edit_response.save_clicked {
                        response.save_requested = true;
                        response.server_update = edit_response.server_data;
                    }
                    if edit_response.cancelled {
                        response.cancel_edit = true;
                    }
                }
                DetailPanelResponse::None => {}
            }
        });

        response
    }

    pub fn show_server(
        &mut self,
        server: ServerViewModel,
        groups: &[GroupViewModel],
        is_connected: bool,
    ) {
        self.panel.set_info(server, groups, is_connected);
        self.show_edit_button = true;
    }

    pub fn edit_server(&mut self, server: ServerViewModel, groups: Vec<GroupViewModel>) {
        self.panel.set_edit(server, groups);
        self.show_edit_button = false;
    }

    pub fn clear(&mut self) {
        self.panel.set_empty();
        self.show_edit_button = true;
    }

    pub fn get_ssh_connection(&self) -> Option<SshConnection> {
        match &self.panel {
            DetailPanel::Info(info) => Some(info.build_ssh_connection()),
            _ => None,
        }
    }
}

/// Detail panel container response
#[derive(Debug, Default)]
pub struct DetailPanelContainerResponse {
    pub edit_requested: bool,
    pub save_requested: bool,
    pub cancel_edit: bool,
    pub connect_clicked: bool,
    pub disconnect_clicked: bool,
    pub server_update: Option<ServerUpdateData>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_server() -> ServerViewModel {
        ServerViewModel {
            id: "srv-1".to_string(),
            name: "Test Server".to_string(),
            host: "example.com".to_string(),
            port: 22,
            username: "user".to_string(),
            group_id: Some("group-1".to_string()),
            auth_type: "agent".to_string(),
        }
    }

    fn create_test_groups() -> Vec<GroupViewModel> {
        vec![
            GroupViewModel {
                id: "group-1".to_string(),
                name: "Production".to_string(),
            },
            GroupViewModel {
                id: "group-2".to_string(),
                name: "Development".to_string(),
            },
        ]
    }

    #[test]
    fn test_server_info_response_default() {
        let response = ServerInfoResponse::default();
        assert!(!response.connect_clicked);
        assert!(!response.disconnect_clicked);
    }

    #[test]
    fn test_edit_form_response_default() {
        let response = EditFormResponse::default();
        assert!(!response.save_clicked);
        assert!(!response.cancelled);
        assert!(response.server_data.is_none());
    }

    #[test]
    fn test_edit_form_detects_changes() {
        let server = create_test_server();
        let groups = create_test_groups();
        let mut form = EditForm::new(server.clone(), groups);

        assert!(!form.has_changes);

        form.edited_name = "Changed Name".to_string();
        form.check_for_changes();
        assert!(form.has_changes);
    }

    #[test]
    fn test_edit_form_validate() {
        let server = create_test_server();
        let groups = create_test_groups();
        let mut form = EditForm::new(server, groups);

        // Valid data
        assert!(form.validate());
        assert!(form.error_message.is_none());

        // Empty name
        form.edited_name = "".to_string();
        assert!(!form.validate());
        assert!(form.error_message.is_some());

        // Invalid port
        form.edited_name = "Test".to_string();
        form.edited_port = "invalid".to_string();
        assert!(!form.validate());
    }

    #[test]
    fn test_edit_form_reset() {
        let server = create_test_server();
        let groups = create_test_groups();
        let mut form = EditForm::new(server.clone(), groups);

        form.edited_name = "Changed".to_string();
        form.edited_host = "changed.com".to_string();
        form.check_for_changes();
        assert!(form.has_changes);

        form.reset();
        assert_eq!(form.edited_name, server.name);
        assert_eq!(form.edited_host, server.host);
        assert!(!form.has_changes);
    }

    #[test]
    fn test_detail_panel_state() {
        let mut panel = DetailPanel::Empty;
        assert!(!panel.is_editing());
        assert!(!panel.is_info());

        let server = create_test_server();
        let groups = create_test_groups();

        panel.set_info(server.clone(), &groups, false);
        assert!(panel.is_info());
        assert!(!panel.is_editing());

        panel.set_edit(server, groups);
        assert!(panel.is_editing());
        assert!(!panel.is_info());
    }

    #[test]
    fn test_server_update_data() {
        let data = ServerUpdateData {
            id: "srv-1".to_string(),
            name: "Updated".to_string(),
            host: "new.com".to_string(),
            port: 2222,
            username: "newuser".to_string(),
            auth_type: "key".to_string(),
            group_id: Some("group-2".to_string()),
        };

        assert_eq!(data.id, "srv-1");
        assert_eq!(data.port, 2222);
    }

    #[test]
    fn test_terminal_diagnostics() {
        let diag = TerminalDiagnostics {
            windows_terminal_available: true,
            powershell_available: true,
            cmd_available: true,
            ssh_available: true,
            in_windows_terminal: false,
        };

        assert!(diag.any_terminal_available());
        assert_eq!(
            diag.get_best_terminal(),
            TerminalPreference::WindowsTerminal
        );

        let diag2 = TerminalDiagnostics {
            windows_terminal_available: false,
            powershell_available: true,
            cmd_available: false,
            ssh_available: false,
            in_windows_terminal: false,
        };

        assert!(diag2.any_terminal_available());
        assert_eq!(diag2.get_best_terminal(), TerminalPreference::PowerShell);

        let diag3 = TerminalDiagnostics {
            windows_terminal_available: false,
            powershell_available: false,
            cmd_available: false,
            ssh_available: false,
            in_windows_terminal: false,
        };

        assert!(!diag3.any_terminal_available());
    }
}
