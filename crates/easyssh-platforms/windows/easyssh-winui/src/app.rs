//! Main Application Module for EasySSH Lite Windows UI
//!
//! Provides the core application state and main UI logic.
//!
//! Features:
//! - Keyboard shortcut hints and help panel
//! - Group color picker integration
//! - Terminal launch feedback with progress indication
//! - Smooth animations and transitions

use crate::design::{BrandColors, DesignTheme, Motion, Radius, Spacing, Typography};
use crate::detail_panel::{DetailPanelContainer, DetailPanelContainerResponse, ServerUpdateData};
use crate::dialogs::{
    AddServerDialog, DeleteConfirmDialog, DialogResult, EditServerDialog, ErrorDialog, GroupColor,
    GroupColorPicker, GroupDialogAction, GroupManagerDialog, PasswordDialogResult,
    PasswordPromptDialog,
};
use crate::sidebar::{QuickActionsBar, QuickActionsResponse, Sidebar, SidebarResponse};
use crate::terminal_launcher::{
    get_terminal_diagnostics, launch_ssh_terminal, SshConnection, TerminalError, TerminalPreference,
};
use crate::viewmodels::{AppViewModel, GroupViewModel, ServerViewModel};
use egui::{Align, Color32, Context, Frame, Layout, Margin, RichText, Stroke, Style, Visuals};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tracing::{error, info, warn};

/// Application state
pub struct EasySshApp {
    /// View model for data access
    view_model: Arc<Mutex<AppViewModel>>,

    /// Sidebar component
    sidebar: Sidebar,

    /// Detail panel component
    detail_panel: DetailPanelContainer,

    /// Dialogs
    add_server_dialog: AddServerDialog,
    edit_server_dialog: EditServerDialog,
    group_manager_dialog: GroupManagerDialog,
    password_prompt_dialog: PasswordPromptDialog,
    delete_confirm_dialog: DeleteConfirmDialog,
    error_dialog: ErrorDialog,
    group_color_picker: GroupColorPicker,

    /// Connected servers (server_id -> session_id)
    connected_servers: std::collections::HashMap<String, String>,

    /// Server pending password prompt
    pending_password_server: Option<(String, String, String)>, // (server_id, server_name, username)

    /// Toast notifications
    toasts: Vec<Toast>,

    /// Theme
    dark_mode: bool,

    /// Show terminal diagnostics
    show_diagnostics: bool,

    /// Show keyboard shortcuts help
    show_shortcuts_help: bool,

    /// Terminal launch progress
    terminal_launch_progress: Option<TerminalLaunchProgress>,
}

/// Terminal launch progress indicator
struct TerminalLaunchProgress {
    server_name: String,
    started_at: Instant,
    stage: LaunchStage,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LaunchStage {
    Preparing,
    Launching,
    Connecting,
    Connected,
    Failed,
}

impl LaunchStage {
    fn as_str(&self) -> &'static str {
        match self {
            LaunchStage::Preparing => "Preparing...",
            LaunchStage::Launching => "Launching terminal...",
            LaunchStage::Connecting => "Connecting...",
            LaunchStage::Connected => "Connected!",
            LaunchStage::Failed => "Failed",
        }
    }

    fn color(&self) -> Color32 {
        match self {
            LaunchStage::Preparing => BrandColors::C400,
            LaunchStage::Launching => BrandColors::C400,
            LaunchStage::Connecting => BrandColors::C500,
            LaunchStage::Connected => Color32::GREEN,
            LaunchStage::Failed => Color32::RED,
        }
    }
}

/// Toast notification
#[derive(Clone)]
struct Toast {
    message: String,
    level: ToastLevel,
    created_at: std::time::Instant,
    duration: std::time::Duration,
}

#[derive(Clone, Copy)]
enum ToastLevel {
    Info,
    Success,
    Warning,
    Error,
}

impl ToastLevel {
    fn color(&self) -> egui::Color32 {
        match self {
            ToastLevel::Info => egui::Color32::LIGHT_BLUE,
            ToastLevel::Success => egui::Color32::GREEN,
            ToastLevel::Warning => egui::Color32::YELLOW,
            ToastLevel::Error => egui::Color32::RED,
        }
    }
}

impl EasySshApp {
    /// Create a new application instance
    pub fn new(view_model: Arc<Mutex<AppViewModel>>) -> anyhow::Result<Self> {
        let mut app = Self {
            view_model,
            sidebar: Sidebar::new(),
            detail_panel: DetailPanelContainer::new(),
            add_server_dialog: AddServerDialog::new(),
            edit_server_dialog: EditServerDialog::new(),
            group_manager_dialog: GroupManagerDialog::new(),
            password_prompt_dialog: PasswordPromptDialog::new(),
            delete_confirm_dialog: DeleteConfirmDialog::new(),
            error_dialog: ErrorDialog::new(),
            group_color_picker: GroupColorPicker::new(),
            connected_servers: std::collections::HashMap::new(),
            pending_password_server: None,
            toasts: Vec::new(),
            dark_mode: true,
            show_diagnostics: false,
            show_shortcuts_help: false,
            terminal_launch_progress: None,
        };

        // Load initial data
        app.refresh_data();

        Ok(app)
    }

    /// Refresh data from view model
    fn refresh_data(&mut self) {
        let vm = self.view_model.lock().unwrap();
        let servers = vm.get_servers();
        let groups = vm.get_groups();
        let connected: Vec<String> = self.connected_servers.keys().cloned().collect();

        self.sidebar.update_data(servers, groups, connected);
    }

    /// Show toast notification
    fn show_toast(&mut self, message: impl Into<String>, level: ToastLevel) {
        self.toasts.push(Toast {
            message: message.into(),
            level,
            created_at: std::time::Instant::now(),
            duration: std::time::Duration::from_secs(3),
        });
    }
}

impl eframe::App for EasySshApp {
    fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        // Set theme with accessibility support
        let mut theme = DesignTheme::from_theme(if self.dark_mode {
            crate::design::Theme::Dark
        } else {
            crate::design::Theme::Light
        });
        theme.apply_accessibility_settings();
        theme.apply_to_ctx(ctx);

        // Main layout
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.horizontal(|ui| {
                // Left sidebar
                ui.vertical(|ui| {
                    ui.set_width(ui.available_width() * 0.35);
                    ui.set_height(ui.available_height());

                    // Quick actions bar
                    let actions_response = QuickActionsBar::show(ui);
                    self.handle_quick_actions(actions_response);

                    ui.separator();

                    // Sidebar
                    let sidebar_response = self.sidebar.show(ui);
                    self.handle_sidebar_response(sidebar_response, ctx);
                });

                ui.separator();

                // Right detail panel
                ui.vertical(|ui| {
                    ui.set_width(ui.available_width());
                    ui.set_height(ui.available_height());

                    let panel_response = self.detail_panel.show(ui);
                    self.handle_panel_response(panel_response);
                });
            });
        });

        // Show dialogs
        self.show_dialogs(ctx);

        // Show toasts
        self.show_toasts(ctx);

        // Show diagnostics window if enabled
        if self.show_diagnostics {
            self.show_diagnostics_window(ctx);
        }

        // Show keyboard shortcuts help if enabled
        if self.show_shortcuts_help {
            self.show_shortcuts_help_window(ctx);
        }

        // Show terminal launch progress
        if let Some(progress) = &self.terminal_launch_progress {
            self.show_terminal_progress(ctx, progress);
        }

        // Update terminal launch progress
        self.update_terminal_progress(ctx);

        // Handle keyboard shortcuts
        self.handle_shortcuts(ctx);
    }
}

impl EasySshApp {
    fn handle_quick_actions(&mut self, response: QuickActionsResponse) {
        if response.add_server {
            self.add_server_dialog.open();
        }

        if response.manage_groups {
            self.group_manager_dialog.open();
        }

        if response.open_settings {
            self.show_shortcuts_help = !self.show_shortcuts_help;
        }
    }

    fn handle_sidebar_response(&mut self, response: SidebarResponse, ctx: &Context) {
        // Server selection changed
        if let Some(server_id) = response.server_selected {
            let vm = self.view_model.lock().unwrap();
            let servers = vm.get_servers();
            let groups = vm.get_groups();

            if let Some(server) = servers.iter().find(|s| s.id == server_id).cloned() {
                let is_connected = self.connected_servers.contains_key(&server_id);
                self.detail_panel.show_server(server, &groups, is_connected);
            }
        }

        // Connect button clicked on server card
        if let Some(server_id) = response.connect_clicked {
            self.connect_to_server(&server_id);
        }

        // Group filter changed
        if response.group_changed.is_some() {
            // Data already updated in sidebar
        }
    }

    fn handle_panel_response(&mut self, response: DetailPanelContainerResponse) {
        // Edit button clicked
        if response.edit_requested {
            if let Some(server) = self.sidebar.get_selected_server().cloned() {
                let vm = self.view_model.lock().unwrap();
                let groups = vm.get_groups();
                self.detail_panel.edit_server(server, groups);
            }
        }

        // Save button clicked in edit form
        if response.save_requested {
            if let Some(data) = response.server_update {
                self.update_server(data);
            }
        }

        // Cancel edit
        if response.cancel_edit {
            if let Some(server) = self.sidebar.get_selected_server().cloned() {
                let vm = self.view_model.lock().unwrap();
                let groups = vm.get_groups();
                let is_connected = self.connected_servers.contains_key(&server.id);
                self.detail_panel.show_server(server, &groups, is_connected);
            }
        }

        // Connect button clicked in detail panel
        if response.connect_clicked {
            if let Some(server) = self.sidebar.get_selected_server() {
                let server_id = server.id.clone();
                self.connect_to_server(&server_id);
            }
        }

        // Disconnect button clicked
        if response.disconnect_clicked {
            if let Some(server) = self.sidebar.get_selected_server() {
                let server_id = server.id.clone();
                self.disconnect_from_server(&server_id);
            }
        }
    }

    fn show_dialogs(&mut self, ctx: &Context) {
        // Add Server Dialog
        let add_result = self.add_server_dialog.show(ctx, &self.sidebar.groups);
        match add_result {
            DialogResult::Ok => {
                if let Some((name, host, port, username, auth_type, group_id)) =
                    self.add_server_dialog.get_server_data()
                {
                    self.add_server(name, host, port, username, auth_type, group_id);
                }
            }
            DialogResult::Cancel => {
                // Dialog closed
            }
            DialogResult::None => {}
        }

        // Edit Server Dialog
        let edit_result = self.edit_server_dialog.show(ctx, &self.sidebar.groups);
        match edit_result {
            DialogResult::Ok => {
                if let Some((id, name, host, port, username, auth_type, group_id)) =
                    self.edit_server_dialog.get_server_data()
                {
                    self.update_server(ServerUpdateData {
                        id,
                        name,
                        host,
                        port,
                        username,
                        auth_type,
                        group_id,
                    });
                }
            }
            DialogResult::Cancel => {
                // Refresh data to show original values
                self.refresh_data();
            }
            DialogResult::None => {}
        }

        // Group Manager Dialog
        let group_action = self.group_manager_dialog.show(ctx, &self.sidebar.groups);
        match group_action {
            GroupDialogAction::Add(name) => {
                self.add_group(name);
                self.group_manager_dialog.clear_error();
            }
            GroupDialogAction::Update(id, name) => {
                self.update_group(id, name);
                self.group_manager_dialog.clear_error();
            }
            GroupDialogAction::Delete(id) => {
                self.delete_group(id);
                self.group_manager_dialog.clear_error();
            }
            GroupDialogAction::None => {}
        }

        // Password Prompt Dialog
        if self.pending_password_server.is_some() {
            let pwd_result = self.password_prompt_dialog.show(ctx);
            match pwd_result {
                PasswordDialogResult::Ok {
                    password,
                    save_password,
                } => {
                    if let Some((server_id, _, _)) = self.pending_password_server.take() {
                        // Connect with password
                        self.connect_with_password(&server_id, &password, save_password);
                    }
                }
                PasswordDialogResult::Cancel => {
                    self.pending_password_server = None;
                    self.show_toast("Connection cancelled", ToastLevel::Info);
                }
                PasswordDialogResult::None => {}
            }
        }

        // Delete Confirm Dialog
        let delete_result = self.delete_confirm_dialog.show(ctx);
        match delete_result {
            DialogResult::Ok => {
                // Handle deletion (would be set up when opening dialog)
            }
            DialogResult::Cancel => {}
            DialogResult::None => {}
        }

        // Error Dialog
        let _error_result = self.error_dialog.show(ctx);

        // Group Color Picker
        if let Some(selected_color) = self.group_color_picker.show(ctx) {
            // Apply selected color to active group (would need to track which group is being edited)
            info!("Selected group color: {:?}", selected_color);
        }
    }

    fn show_toasts(&mut self, ctx: &Context) {
        // Remove expired toasts
        let now = std::time::Instant::now();
        self.toasts
            .retain(|t| now.duration_since(t.created_at) < t.duration);

        // Show toasts in top-right corner
        if !self.toasts.is_empty() {
            let screen_rect = ctx.screen_rect();
            let toast_area = egui::Area::new(egui::Id::new("toasts")).fixed_pos(egui::pos2(
                screen_rect.max.x - 10.0,
                screen_rect.min.y + 10.0,
            ));

            toast_area.show(ctx, |ui| {
                ui.with_layout(egui::Layout::top_down(egui::Align::RIGHT), |ui| {
                    for toast in &self.toasts {
                        egui::Frame::none()
                            .fill(ui.visuals().panel_fill)
                            .stroke(egui::Stroke::new(1.0, toast.level.color()))
                            .rounding(egui::Rounding::same(4.0))
                            .inner_margin(egui::Margin::same(12.0))
                            .show(ui, |ui| {
                                ui.colored_label(toast.level.color(), &toast.message);
                            });
                        ui.add_space(8.0);
                    }
                });
            });
        }
    }

    fn show_diagnostics_window(&mut self, ctx: &Context) {
        let theme = DesignTheme::from_theme(if ctx.style().visuals.dark_mode {
            crate::design::Theme::Dark
        } else {
            crate::design::Theme::Light
        });

        egui::Window::new("Terminal Diagnostics")
            .collapsible(false)
            .resizable(false)
            .frame(Frame::window(&ctx.style()).shadow(if theme.reduced_motion {
                egui::Shadow::NONE
            } else {
                egui::Shadow::small_dark()
            }))
            .show(ctx, |ui| {
                let diag = get_terminal_diagnostics();

                ui.label(RichText::new("Terminal Availability:").strong());
                ui.add_space(4.0);

                let check = |available: bool| -> &str {
                    if available {
                        "✓"
                    } else {
                        "✗"
                    }
                };

                ui.horizontal(|ui| {
                    ui.label("  Windows Terminal:");
                    let color = if diag.windows_terminal_available {
                        Color32::GREEN
                    } else {
                        Color32::RED
                    };
                    ui.colored_label(color, check(diag.windows_terminal_available));
                });

                ui.horizontal(|ui| {
                    ui.label("  PowerShell:");
                    let color = if diag.powershell_available {
                        Color32::GREEN
                    } else {
                        Color32::RED
                    };
                    ui.colored_label(color, check(diag.powershell_available));
                });

                ui.horizontal(|ui| {
                    ui.label("  CMD:");
                    let color = if diag.cmd_available {
                        Color32::GREEN
                    } else {
                        Color32::RED
                    };
                    ui.colored_label(color, check(diag.cmd_available));
                });

                ui.horizontal(|ui| {
                    ui.label("  SSH:");
                    let color = if diag.ssh_available {
                        Color32::GREEN
                    } else {
                        Color32::RED
                    };
                    ui.colored_label(color, check(diag.ssh_available));
                });

                ui.separator();

                if diag.in_windows_terminal {
                    Frame::none()
                        .fill(Color32::GREEN.linear_multiply(0.1))
                        .rounding(Radius::SM)
                        .inner_margin(Margin::same(Spacing::_3))
                        .show(ui, |ui| {
                            ui.colored_label(Color32::GREEN, "✓ Running inside Windows Terminal");
                        });
                } else {
                    ui.colored_label(theme.text_secondary, "Not running inside Windows Terminal");
                }

                ui.add_space(8.0);

                ui.horizontal(|ui| {
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if ui.button("Close").clicked() {
                            self.show_diagnostics = false;
                        }
                    });
                });
            });
    }

    fn show_shortcuts_help_window(&mut self, ctx: &Context) {
        let theme = DesignTheme::from_theme(if ctx.style().visuals.dark_mode {
            crate::design::Theme::Dark
        } else {
            crate::design::Theme::Light
        });

        let shortcuts = vec![
            ("Ctrl + K", "Focus search box"),
            ("Ctrl + N", "Add new server"),
            ("Ctrl + G", "Manage groups"),
            ("Ctrl + D", "Show diagnostics"),
            ("Ctrl + H", "Toggle shortcuts help"),
            ("Delete", "Delete selected server"),
            ("Enter", "Connect to selected server"),
            ("Esc", "Close dialogs / Cancel"),
        ];

        egui::Window::new("Keyboard Shortcuts")
            .collapsible(false)
            .resizable(false)
            .frame(Frame::window(&ctx.style()).shadow(if theme.reduced_motion {
                egui::Shadow::NONE
            } else {
                egui::Shadow::small_dark()
            }))
            .show(ctx, |ui| {
                ui.label(RichText::new("Quick Actions:").strong());
                ui.add_space(8.0);

                for (shortcut, description) in shortcuts {
                    ui.horizontal(|ui| {
                        Frame::none()
                            .fill(theme.bg_quaternary)
                            .rounding(Radius::SM)
                            .inner_margin(Margin::symmetric(6.0, 4.0))
                            .show(ui, |ui| {
                                ui.monospace(
                                    RichText::new(shortcut)
                                        .size(12.0)
                                        .color(theme.text_secondary),
                                );
                            });
                        ui.add_space(12.0);
                        ui.label(
                            RichText::new(description)
                                .size(13.0)
                                .color(theme.text_primary),
                        );
                    });
                    ui.add_space(4.0);
                }

                ui.separator();
                ui.add_space(4.0);

                ui.horizontal(|ui| {
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if ui.button("Close").clicked() {
                            self.show_shortcuts_help = false;
                        }
                    });
                });
            });
    }

    fn show_terminal_progress(&self, ctx: &Context, progress: &TerminalLaunchProgress) {
        let theme = DesignTheme::from_theme(if ctx.style().visuals.dark_mode {
            crate::design::Theme::Dark
        } else {
            crate::design::Theme::Light
        });

        let elapsed = progress.started_at.elapsed().as_secs_f32();
        let progress_value = match progress.stage {
            LaunchStage::Preparing => (elapsed / 0.3).min(0.25),
            LaunchStage::Launching => 0.25 + (elapsed / 0.5).min(0.25),
            LaunchStage::Connecting => 0.5 + (elapsed / 1.0).min(0.4),
            LaunchStage::Connected => 1.0,
            LaunchStage::Failed => 0.0,
        };

        let screen_rect = ctx.screen_rect();
        let window_size = egui::vec2(320.0, 100.0);
        let window_pos = screen_rect.center() - window_size * 0.5;

        egui::Area::new(egui::Id::new("terminal_progress"))
            .fixed_pos(window_pos)
            .show(ctx, |ui| {
                Frame::window(&ctx.style())
                    .inner_margin(Margin::same(Spacing::_4))
                    .show(ui, |ui| {
                        ui.set_min_width(window_size.x);

                        ui.label(
                            RichText::new(format!("Connecting to {}", progress.server_name))
                                .strong(),
                        );
                        ui.add_space(4.0);

                        ui.horizontal(|ui| {
                            let color = progress.stage.color();
                            let spinner = if progress.stage != LaunchStage::Connected
                                && progress.stage != LaunchStage::Failed
                            {
                                let anim = (elapsed * 8.0).sin() * 0.5 + 0.5;
                                let bars = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
                                let idx = ((elapsed * 10.0) as usize) % bars.len();
                                format!("{} ", bars[idx])
                            } else {
                                String::new()
                            };
                            ui.colored_label(
                                color,
                                format!("{}{}", spinner, progress.stage.as_str()),
                            );
                        });

                        ui.add_space(4.0);

                        // Progress bar
                        ui.add(
                            egui::ProgressBar::new(progress_value)
                                .fill(progress.stage.color())
                                .desired_height(4.0),
                        );
                    });
            });
    }

    fn update_terminal_progress(&mut self, ctx: &Context) {
        if let Some(progress) = &self.terminal_launch_progress {
            let elapsed = progress.started_at.elapsed();

            // Auto-hide after completion or failure
            let should_hide = match progress.stage {
                LaunchStage::Connected if elapsed > Duration::from_secs(2) => true,
                LaunchStage::Failed if elapsed > Duration::from_secs(3) => true,
                _ => false,
            };

            if should_hide {
                self.terminal_launch_progress = None;
                ctx.request_repaint();
            } else {
                // Continuous updates during animation
                if progress.stage != LaunchStage::Connected && progress.stage != LaunchStage::Failed
                {
                    ctx.request_repaint_after(std::time::Duration::from_millis(50));
                }
            }
        }
    }

    fn start_terminal_launch(&mut self, server_name: String) {
        self.terminal_launch_progress = Some(TerminalLaunchProgress {
            server_name,
            started_at: Instant::now(),
            stage: LaunchStage::Preparing,
        });
    }

    fn update_launch_stage(&mut self, stage: LaunchStage) {
        if let Some(progress) = &mut self.terminal_launch_progress {
            progress.stage = stage;
        }
    }

    fn finish_terminal_launch(&mut self, success: bool) {
        if let Some(progress) = &mut self.terminal_launch_progress {
            progress.stage = if success {
                LaunchStage::Connected
            } else {
                LaunchStage::Failed
            };
        }
    }

    fn handle_shortcuts(&mut self, ctx: &Context) {
        // Ctrl+K: Focus search
        if ctx.input(|i| i.key_pressed(egui::Key::K) && i.modifiers.ctrl) {
            // Focus search box
            self.sidebar.search_box.focus(ctx);
            info!("Ctrl+K pressed - Focus search");
        }

        // Ctrl+N: Add new server
        if ctx.input(|i| i.key_pressed(egui::Key::N) && i.modifiers.ctrl) {
            self.add_server_dialog.open();
            info!("Ctrl+N pressed - Add server");
        }

        // Ctrl+G: Manage groups
        if ctx.input(|i| i.key_pressed(egui::Key::G) && i.modifiers.ctrl) {
            self.group_manager_dialog.open();
            info!("Ctrl+G pressed - Manage groups");
        }

        // Ctrl+D: Show diagnostics
        if ctx.input(|i| i.key_pressed(egui::Key::D) && i.modifiers.ctrl) {
            self.show_diagnostics = !self.show_diagnostics;
            info!("Ctrl+D pressed - Toggle diagnostics");
        }

        // Ctrl+H: Show shortcuts help
        if ctx.input(|i| i.key_pressed(egui::Key::H) && i.modifiers.ctrl) {
            self.show_shortcuts_help = !self.show_shortcuts_help;
            info!("Ctrl+H pressed - Toggle shortcuts help");
        }

        // Enter: Connect to selected server
        if ctx.input(|i| i.key_pressed(egui::Key::Enter)) {
            if let Some(server) = self.sidebar.get_selected_server() {
                let server_id = server.id.clone();
                self.connect_to_server(&server_id);
            }
        }

        // Escape: Close dialogs
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            self.show_shortcuts_help = false;
            self.show_diagnostics = false;
            // Could also close other dialogs here
        }

        // Delete: Delete selected server
        if ctx.input(|i| i.key_pressed(egui::Key::Delete)) {
            if let Some(server) = self.sidebar.get_selected_server() {
                let name = server.name.clone();
                let _id = server.id.clone();
                self.delete_confirm_dialog.open_for(&name, "server");
            }
        }
    }
}

// Data operations
impl EasySshApp {
    fn add_server(
        &mut self,
        name: String,
        host: String,
        port: i64,
        username: String,
        auth_type: String,
        group_id: Option<String>,
    ) {
        let result = {
            let vm = self.view_model.lock().unwrap();
            vm.add_server(&name, &host, port, &username, &auth_type, group_id)
        };
        match result {
            Ok(_) => {
                info!("Added server: {}", name);
                self.show_toast(format!("Server '{}' added", name), ToastLevel::Success);
                self.refresh_data();
            }
            Err(e) => {
                error!("Failed to add server: {}", e);
                self.error_dialog
                    .open_with_message("Error", &format!("Failed to add server: {}", e));
            }
        }
    }

    fn update_server(&mut self, data: ServerUpdateData) {
        let (update_result, group_result) = {
            let vm = self.view_model.lock().unwrap();
            let update_result = vm.update_server(
                &data.id,
                &data.name,
                &data.host,
                data.port,
                &data.username,
                &data.auth_type,
            );
            let group_result = if update_result.is_ok() {
                vm.update_server_group(&data.id, data.group_id)
            } else {
                Ok(())
            };
            (update_result, group_result)
        };

        match update_result {
            Ok(_) => {
                if let Err(e) = group_result {
                    warn!("Failed to update server group: {}", e);
                }

                info!("Updated server: {}", data.name);
                self.show_toast(
                    format!("Server '{}' updated", data.name),
                    ToastLevel::Success,
                );
                self.refresh_data();

                // Update detail panel
                let vm = self.view_model.lock().unwrap();
                let servers = vm.get_servers();
                let groups = vm.get_groups();
                if let Some(server) = servers.iter().find(|s| s.id == data.id).cloned() {
                    let is_connected = self.connected_servers.contains_key(&data.id);
                    self.detail_panel.show_server(server, &groups, is_connected);
                }
            }
            Err(e) => {
                error!("Failed to update server: {}", e);
                self.error_dialog
                    .open_with_message("Error", &format!("Failed to update server: {}", e));
            }
        }
    }

    fn add_group(&mut self, name: String) {
        let result = {
            let vm = self.view_model.lock().unwrap();
            vm.add_group(&name)
        };
        match result {
            Ok(id) => {
                info!("Added group: {} ({})", name, id);
                self.show_toast(format!("Group '{}' added", name), ToastLevel::Success);
                self.refresh_data();
            }
            Err(e) => {
                error!("Failed to add group: {}", e);
                self.group_manager_dialog
                    .set_error(format!("Failed to add group: {}", e));
            }
        }
    }

    fn update_group(&mut self, id: String, name: String) {
        let result = {
            let vm = self.view_model.lock().unwrap();
            vm.update_group(&id, &name)
        };
        match result {
            Ok(_) => {
                info!("Updated group {}: {}", id, name);
                self.show_toast(format!("Group '{}' updated", name), ToastLevel::Success);
                self.refresh_data();
            }
            Err(e) => {
                error!("Failed to update group: {}", e);
                self.group_manager_dialog
                    .set_error(format!("Failed to update group: {}", e));
            }
        }
    }

    fn delete_group(&mut self, id: String) {
        let result = {
            let vm = self.view_model.lock().unwrap();
            vm.delete_group(&id)
        };
        match result {
            Ok(_) => {
                info!("Deleted group: {}", id);
                self.show_toast("Group deleted", ToastLevel::Success);
                self.refresh_data();
            }
            Err(e) => {
                error!("Failed to delete group: {}", e);
                self.group_manager_dialog
                    .set_error(format!("Failed to delete group: {}", e));
            }
        }
    }

    fn connect_to_server(&mut self, server_id: &str) {
        // Check if already connected first (no need for vm lock)
        if self.connected_servers.contains_key(server_id) {
            self.show_toast("Already connected", ToastLevel::Warning);
            return;
        }

        // Get server data from view model
        let server = {
            let vm = self.view_model.lock().unwrap();
            let servers = vm.get_servers();
            servers.iter().find(|s| &s.id == server_id).cloned()
        };

        if let Some(server) = server {
            // Start progress indicator
            self.start_terminal_launch(server.name.clone());

            // Build SSH connection
            let connection = SshConnection::new(
                server.host.clone(),
                server.port as u16,
                server.username.clone(),
                server.auth_type.clone(),
                None,
            );

            // Update progress stage
            self.update_launch_stage(LaunchStage::Launching);

            // Launch terminal
            match launch_ssh_terminal(&connection, TerminalPreference::Auto) {
                Ok(_) => {
                    self.update_launch_stage(LaunchStage::Connected);
                    info!("Launched terminal for server: {}", server.name);
                    self.connected_servers
                        .insert(server_id.to_string(), format!("session-{}", server_id));
                    self.show_toast(
                        format!("Connected to '{}'", server.name),
                        ToastLevel::Success,
                    );

                    // Update UI - need vm lock again
                    let vm = self.view_model.lock().unwrap();
                    let groups = vm.get_groups();
                    let is_connected = self.connected_servers.contains_key(server_id);
                    self.detail_panel
                        .show_server(server.clone(), &groups, is_connected);

                    // Mark as finished (will auto-hide after delay)
                    self.finish_terminal_launch(true);
                }
                Err(e) => {
                    error!("Failed to launch terminal: {}", e);
                    self.finish_terminal_launch(false);
                    self.show_toast(format!("Failed to connect: {}", e), ToastLevel::Error);
                    self.error_dialog
                        .open_with_message("Connection Error", &format!("{}", e));
                }
            }
        }
    }

    fn connect_with_password(&mut self, server_id: &str, password: &str, save_password: bool) {
        // Get server data and optionally save password
        let server = {
            let vm = self.view_model.lock().unwrap();
            let servers = vm.get_servers();

            if let Some(server) = servers.iter().find(|s| &s.id == server_id) {
                // Save password if requested
                if save_password {
                    if let Err(e) = vm.save_password(server_id, password) {
                        warn!("Failed to save password: {}", e);
                    }
                }
                Some(server.clone())
            } else {
                None
            }
        };

        if let Some(server) = server {
            // Build SSH connection
            let connection = SshConnection::new(
                server.host.clone(),
                server.port as u16,
                server.username.clone(),
                "password".to_string(),
                None,
            );

            // Launch terminal
            match launch_ssh_terminal(&connection, TerminalPreference::Auto) {
                Ok(_) => {
                    info!("Launched terminal for server: {}", server.name);
                    self.connected_servers
                        .insert(server_id.to_string(), format!("session-{}", server_id));
                    self.show_toast(
                        format!("Connected to '{}'", server.name),
                        ToastLevel::Success,
                    );
                }
                Err(e) => {
                    error!("Failed to launch terminal: {}", e);
                    self.error_dialog
                        .open_with_message("Connection Error", &format!("{}", e));
                }
            }
        }
    }

    fn disconnect_from_server(&mut self, server_id: &str) {
        if self.connected_servers.remove(server_id).is_some() {
            info!("Disconnected from server: {}", server_id);
            self.show_toast("Disconnected", ToastLevel::Info);

            // Update UI
            let (groups, server) = {
                let vm = self.view_model.lock().unwrap();
                let groups = vm.get_groups();
                let servers = vm.get_servers();
                let server = servers.iter().find(|s| &s.id == server_id).cloned();
                (groups, server)
            };
            if let Some(server) = server {
                self.detail_panel.show_server(server, &groups, false);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_toast_level_colors() {
        assert_eq!(ToastLevel::Info.color(), egui::Color32::LIGHT_BLUE);
        assert_eq!(ToastLevel::Success.color(), egui::Color32::GREEN);
        assert_eq!(ToastLevel::Warning.color(), egui::Color32::YELLOW);
        assert_eq!(ToastLevel::Error.color(), egui::Color32::RED);
    }
}
