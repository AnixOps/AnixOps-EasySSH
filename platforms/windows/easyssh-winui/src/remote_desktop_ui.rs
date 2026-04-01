#![allow(dead_code)]

use eframe::egui;
use std::collections::HashMap;

/// Render the remote desktop panel
pub fn render_remote_desktop_panel(ctx: &egui::Context, manager: &mut RemoteDesktopManagerUI) {
    egui::SidePanel::right("remote_desktop_panel")
        .resizable(true)
        .default_width(350.0)
        .max_width(500.0)
        .min_width(250.0)
        .frame(egui::Frame {
            fill: egui::Color32::from_rgb(30, 34, 42),
            stroke: egui::Stroke::new(1.0, egui::Color32::from_rgb(45, 50, 60)),
            ..Default::default()
        })
        .show(ctx, |ui| {
            ui.vertical(|ui| {
                // Header
                ui.horizontal(|ui| {
                    ui.heading(
                        egui::RichText::new("Remote Desktop")
                            .color(egui::Color32::from_rgb(220, 225, 235)),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("+").clicked() {
                            manager.show_add_dialog = true;
                            manager.new_connection = RemoteDesktopConnectionUI::default();
                            manager.new_connection.id = uuid::Uuid::new_v4().to_string();
                        }
                    });
                });
                ui.separator();

                // Connections list
                egui::ScrollArea::vertical().show(ui, |ui| {
                    render_connections_list(ui, manager);
                });

                ui.separator();

                // Active sessions
                render_active_sessions(ui, manager);
            });
        });

    // Render connection dialog if open
    if manager.show_add_dialog || manager.editing_connection.is_some() {
        render_connection_dialog(ctx, manager);
    }
}

use crate::embedded_rdp::{ConnectionSettings, RemoteDesktopType, RemoteDesktopViewer};

/// Remote desktop connection UI state
#[derive(Debug, Clone)]
pub struct RemoteDesktopConnectionUI {
    pub id: String,
    pub name: String,
    pub protocol: RemoteDesktopProtocol,
    pub host: String,
    pub port: u16,
    pub username: String,
    pub domain: String,
    pub password: String,
    pub use_ssh_tunnel: bool,
    pub ssh_host: String,
    pub ssh_port: u16,
    pub ssh_username: String,
    pub ssh_auth_type: SshAuthType,
    pub ssh_password: String,
    pub display: DisplaySettingsUI,
    pub performance: PerformanceSettingsUI,
    pub local_resources: LocalResourcesUI,
    pub experience: ExperienceSettingsUI,
    pub recording: RecordingSettingsUI,
    pub expanded_section: Option<SettingsSection>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RemoteDesktopProtocol {
    Rdp,
    Vnc,
    SshTunnelRdp,
    SshTunnelVnc,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SshAuthType {
    Agent,
    Password,
    Key,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingsSection {
    Display,
    Performance,
    LocalResources,
    Experience,
    Gateway,
    Recording,
}

#[derive(Debug, Clone)]
pub struct DisplaySettingsUI {
    pub width: u32,
    pub height: u32,
    pub bpp: u8,
    pub fullscreen: bool,
    pub multi_monitor: bool,
    pub smart_sizing: bool,
    pub dynamic_resolution: bool,
    pub fit_session_to_window: bool,
    pub desktop_scale_factor: u32,
}

impl Default for DisplaySettingsUI {
    fn default() -> Self {
        Self {
            width: 1920,
            height: 1080,
            bpp: 32,
            fullscreen: false,
            multi_monitor: false,
            smart_sizing: true,
            dynamic_resolution: true,
            fit_session_to_window: true,
            desktop_scale_factor: 100,
        }
    }
}

#[derive(Debug, Clone)]
pub struct PerformanceSettingsUI {
    pub connection_type: ConnectionType,
    pub disable_wallpaper: bool,
    pub disable_themes: bool,
    pub disable_menu_animations: bool,
    pub disable_full_window_drag: bool,
    pub disable_font_smoothing: bool,
    pub persistent_bitmap_caching: bool,
    pub compression: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionType {
    Modem,
    LowSpeedBroadband,
    Satellite,
    HighSpeedBroadband,
    Wan,
    Lan,
}

impl Default for PerformanceSettingsUI {
    fn default() -> Self {
        Self {
            connection_type: ConnectionType::Lan,
            disable_wallpaper: false,
            disable_themes: false,
            disable_menu_animations: false,
            disable_full_window_drag: false,
            disable_font_smoothing: false,
            persistent_bitmap_caching: true,
            compression: true,
        }
    }
}

#[derive(Debug, Clone)]
pub struct LocalResourcesUI {
    pub clipboard: bool,
    pub printer: bool,
    pub smart_cards: bool,
    pub ports: bool,
    pub drives: DriveRedirectionMode,
    pub audio: AudioRedirectionMode,
    pub microphone: bool,
    pub video_capture: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DriveRedirectionMode {
    Disabled,
    LocalDrives,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AudioRedirectionMode {
    Server,
    Client,
    DoNotPlay,
}

impl Default for LocalResourcesUI {
    fn default() -> Self {
        Self {
            clipboard: true,
            printer: false,
            smart_cards: false,
            ports: false,
            drives: DriveRedirectionMode::Disabled,
            audio: AudioRedirectionMode::Client,
            microphone: false,
            video_capture: false,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ExperienceSettingsUI {
    pub desktop_background: bool,
    pub font_smoothing: bool,
    pub desktop_composition: bool,
    pub show_window_contents: bool,
    pub menu_window_animation: bool,
    pub visual_styles: bool,
    pub reconnect_on_disconnect: bool,
    pub auto_reconnect: bool,
    pub auto_reconnect_max_attempts: u32,
}

impl Default for ExperienceSettingsUI {
    fn default() -> Self {
        Self {
            desktop_background: true,
            font_smoothing: true,
            desktop_composition: true,
            show_window_contents: true,
            menu_window_animation: true,
            visual_styles: true,
            reconnect_on_disconnect: true,
            auto_reconnect: true,
            auto_reconnect_max_attempts: 20,
        }
    }
}

#[derive(Debug, Clone)]
pub struct RecordingSettingsUI {
    pub enabled: bool,
    pub auto_start: bool,
    pub format: RecordingFormat,
    pub quality: RecordingQuality,
    pub include_audio: bool,
    pub output_path: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecordingFormat {
    Mkv,
    Mp4,
    Avi,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecordingQuality {
    Low,
    Medium,
    High,
    Lossless,
}

impl Default for RecordingSettingsUI {
    fn default() -> Self {
        Self {
            enabled: false,
            auto_start: false,
            format: RecordingFormat::Mkv,
            quality: RecordingQuality::High,
            include_audio: true,
            output_path: String::new(),
        }
    }
}

impl Default for RemoteDesktopConnectionUI {
    fn default() -> Self {
        Self {
            id: String::new(),
            name: String::new(),
            protocol: RemoteDesktopProtocol::Rdp,
            host: String::new(),
            port: 3389,
            username: String::new(),
            domain: String::new(),
            password: String::new(),
            use_ssh_tunnel: false,
            ssh_host: String::new(),
            ssh_port: 22,
            ssh_username: String::new(),
            ssh_auth_type: SshAuthType::Agent,
            ssh_password: String::new(),
            display: DisplaySettingsUI::default(),
            performance: PerformanceSettingsUI::default(),
            local_resources: LocalResourcesUI::default(),
            experience: ExperienceSettingsUI::default(),
            recording: RecordingSettingsUI::default(),
            expanded_section: None,
        }
    }
}

/// Remote desktop manager UI
pub struct RemoteDesktopManagerUI {
    pub connections: Vec<RemoteDesktopConnectionUI>,
    pub active_sessions: HashMap<String, RemoteDesktopSessionUI>,
    pub selected_connection: Option<String>,
    pub show_add_dialog: bool,
    pub editing_connection: Option<RemoteDesktopConnectionUI>,
    pub new_connection: RemoteDesktopConnectionUI,
    pub drag_drop_state: DragDropState,
    pub clipboard_sync_enabled: bool,
}

#[derive(Debug, Clone)]
pub struct RemoteDesktopSessionUI {
    pub id: String,
    pub connection_id: String,
    pub connection_name: String,
    pub protocol: RemoteDesktopProtocol,
    pub status: SessionStatus,
    pub started_at: std::time::Instant,
    pub recording_active: bool,
    pub recording_path: Option<String>,
    pub recording_duration: Option<std::time::Duration>,
    pub view_mode: ViewMode,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionStatus {
    Connecting,
    Connected,
    Disconnected,
    Error,
    Recording,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewMode {
    Embedded,
    ExternalWindow,
    Fullscreen,
}

#[derive(Debug, Clone, Default)]
pub struct DragDropState {
    pub is_dragging: bool,
    pub drag_source: Option<String>,
    pub drop_target: Option<String>,
    pub files_being_dragged: Vec<String>,
}

impl RemoteDesktopManagerUI {
    pub fn new() -> Self {
        Self {
            connections: Vec::new(),
            active_sessions: HashMap::new(),
            selected_connection: None,
            show_add_dialog: false,
            editing_connection: None,
            new_connection: RemoteDesktopConnectionUI::default(),
            drag_drop_state: DragDropState::default(),
            clipboard_sync_enabled: true,
        }
    }

    pub fn add_connection(&mut self, connection: RemoteDesktopConnectionUI) {
        self.connections.push(connection);
    }

    pub fn remove_connection(&mut self, id: &str) {
        self.connections.retain(|c| c.id != id);
        self.active_sessions.remove(id);
    }

    pub fn get_connection(&self, id: &str) -> Option<&RemoteDesktopConnectionUI> {
        self.connections.iter().find(|c| c.id == id)
    }

    pub fn get_connection_mut(&mut self, id: &str) -> Option<&mut RemoteDesktopConnectionUI> {
        self.connections.iter_mut().find(|c| c.id == id)
    }

    pub fn start_session(&mut self, connection_id: &str) -> Option<String> {
        if let Some(connection) = self.get_connection(connection_id) {
            let session_id = format!("session_{}", uuid::Uuid::new_v4());

            // Launch external remote desktop client
            let viewer_type = match connection.protocol {
                RemoteDesktopProtocol::Rdp | RemoteDesktopProtocol::SshTunnelRdp => RemoteDesktopType::Rdp,
                RemoteDesktopProtocol::Vnc | RemoteDesktopProtocol::SshTunnelVnc => RemoteDesktopType::Vnc,
            };

            let settings = ConnectionSettings {
                host: connection.host.clone(),
                port: connection.port,
                username: connection.username.clone(),
                password: connection.password.clone(),
                domain: if connection.domain.is_empty() { None } else { Some(connection.domain.clone()) },
            };

            // Try to create viewer and connect
            let dummy_rect = egui::Rect::ZERO;
            match RemoteDesktopViewer::new(dummy_rect, settings, viewer_type) {
                Ok(mut viewer) => {
                    if let Err(e) = viewer.connect() {
                        eprintln!("Failed to connect remote desktop: {}", e);
                        // Create session with error status
                        let session = RemoteDesktopSessionUI {
                            id: session_id.clone(),
                            connection_id: connection_id.to_string(),
                            connection_name: connection.name.clone(),
                            protocol: connection.protocol,
                            status: SessionStatus::Error,
                            started_at: std::time::Instant::now(),
                            recording_active: false,
                            recording_path: None,
                            recording_duration: None,
                            view_mode: ViewMode::ExternalWindow,
                        };
                        self.active_sessions.insert(session_id.clone(), session);
                        return Some(session_id);
                    }

                    // Successfully launched external client
                    let session = RemoteDesktopSessionUI {
                        id: session_id.clone(),
                        connection_id: connection_id.to_string(),
                        connection_name: connection.name.clone(),
                        protocol: connection.protocol,
                        status: SessionStatus::Connected,
                        started_at: std::time::Instant::now(),
                        recording_active: false,
                        recording_path: None,
                        recording_duration: None,
                        view_mode: ViewMode::ExternalWindow,
                    };
                    self.active_sessions.insert(session_id.clone(), session);
                    println!("Launched external remote desktop client for session {}", session_id);
                    return Some(session_id);
                }
                Err(e) => {
                    eprintln!("Failed to create remote desktop viewer: {}", e);
                    return None;
                }
            }
        }
        None
    }

    pub fn stop_session(&mut self, session_id: &str) {
        if let Some(session) = self.active_sessions.get_mut(session_id) {
            session.status = SessionStatus::Disconnected;
        }
    }

    pub fn start_recording(&mut self, session_id: &str, path: String) {
        if let Some(session) = self.active_sessions.get_mut(session_id) {
            session.recording_active = true;
            session.recording_path = Some(path);
            session.status = SessionStatus::Recording;
        }
    }

    pub fn stop_recording(&mut self, session_id: &str) {
        if let Some(session) = self.active_sessions.get_mut(session_id) {
            session.recording_active = false;
            session.status = SessionStatus::Connected;
        }
    }
}

impl Default for RemoteDesktopManagerUI {
    fn default() -> Self {
        Self::new()
    }
}

/// Render the remote desktop connections list
pub fn render_connections_list(ui: &mut egui::Ui, manager: &mut RemoteDesktopManagerUI) {
    ui.horizontal(|ui| {
        ui.heading("Remote Desktop");
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.button("+ Add").clicked() {
                manager.show_add_dialog = true;
                manager.new_connection = RemoteDesktopConnectionUI::default();
                manager.new_connection.id = uuid::Uuid::new_v4().to_string();
            }
        });
    });

    ui.separator();

    // Collect connection info first to avoid borrow issues
    let connection_info: Vec<(String, String, String)> = manager
        .connections
        .iter()
        .map(|c| (c.id.clone(), c.name.clone(), c.host.clone()))
        .collect();

    // Track deferred actions
    let mut connect_id: Option<String> = None;
    let mut edit_id: Option<String> = None;
    let mut delete_id: Option<String> = None;

    // Connection list
    egui::ScrollArea::vertical().show(ui, |ui| {
        for (id, name, host) in &connection_info {
            let is_selected = manager.selected_connection.as_ref() == Some(id);
            let response = ui.selectable_label(is_selected, format!("{} ({})", name, host));

            if response.clicked() {
                manager.selected_connection = Some(id.clone());
            }

            // Context menu - capture actions to execute later
            response.context_menu(|ui| {
                if ui.button("Connect").clicked() {
                    connect_id = Some(id.clone());
                    ui.close_menu();
                }
                if ui.button("Edit").clicked() {
                    edit_id = Some(id.clone());
                    ui.close_menu();
                }
                if ui.button("Delete").clicked() {
                    delete_id = Some(id.clone());
                    ui.close_menu();
                }
            });
        }
    });

    // Execute deferred actions after iteration completes
    if let Some(id) = connect_id {
        manager.start_session(&id);
    }
    if let Some(id) = edit_id {
        if let Some(conn) = manager.get_connection(&id) {
            manager.editing_connection = Some(conn.clone());
        }
    }
    if let Some(id) = delete_id {
        manager.remove_connection(&id);
    }
}

/// Render active sessions panel
pub fn render_active_sessions(ui: &mut egui::Ui, manager: &mut RemoteDesktopManagerUI) {
    ui.heading("Active Sessions");
    ui.separator();

    // Collect session data first to avoid borrow issues
    let session_data: Vec<(
        String,
        String,
        SessionStatus,
        RemoteDesktopProtocol,
        std::time::Instant,
        bool,
    )> = manager
        .active_sessions
        .iter()
        .map(|(id, s)| {
            (
                id.clone(),
                s.connection_name.clone(),
                s.status,
                s.protocol,
                s.started_at,
                s.recording_active,
            )
        })
        .collect();

    // Track actions to execute after iteration
    let mut to_stop: Vec<String> = Vec::new();
    let mut to_stop_recording: Vec<String> = Vec::new();
    let mut to_start_recording: Vec<String> = Vec::new();
    let mut view_mode_changes: Vec<(String, ViewMode)> = Vec::new();

    for (session_id, connection_name, status, _protocol, started_at, recording_active) in
        session_data
    {
        ui.group(|ui| {
            ui.horizontal(|ui| {
                ui.label(&connection_name);

                // Status indicator
                let status_color = match status {
                    SessionStatus::Connecting => egui::Color32::YELLOW,
                    SessionStatus::Connected => egui::Color32::GREEN,
                    SessionStatus::Recording => egui::Color32::RED,
                    SessionStatus::Disconnected => egui::Color32::GRAY,
                    SessionStatus::Error => egui::Color32::DARK_RED,
                };
                ui.colored_label(status_color, format!("{:?}", status));

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if recording_active {
                        ui.label(format!("REC {}", format_duration(started_at.elapsed())));
                    } else {
                        ui.label(format_duration(started_at.elapsed()));
                    }

                    let session_id_clone = session_id.clone();
                    if ui.button("Stop").clicked() {
                        to_stop.push(session_id_clone);
                    }

                    if recording_active {
                        if ui.button("Stop Recording").clicked() {
                            to_stop_recording.push(session_id.clone());
                        }
                    } else if ui.button("Record").clicked() {
                        to_start_recording.push(session_id.clone());
                    }

                    // View mode toggle - capture current session to modify
                    if ui.button("Switch View").clicked() {
                        if let Some(session) = manager.active_sessions.get(&session_id) {
                            let new_mode = match session.view_mode {
                                ViewMode::Embedded => ViewMode::ExternalWindow,
                                ViewMode::ExternalWindow => ViewMode::Fullscreen,
                                ViewMode::Fullscreen => ViewMode::Embedded,
                            };
                            view_mode_changes.push((session_id, new_mode));
                        }
                    }
                });
            });
        });
    }

    // Execute deferred actions
    for id in to_stop {
        manager.stop_session(&id);
        manager.active_sessions.remove(&id);
    }

    for id in to_stop_recording {
        manager.stop_recording(&id);
    }

    for id in to_start_recording {
        let path = format!(
            "{}/recording_{}.mkv",
            dirs::video_dir().unwrap_or_default().display(),
            chrono::Local::now().format("%Y%m%d_%H%M%S")
        );
        manager.start_recording(&id, path);
    }

    for (session_id, new_mode) in view_mode_changes {
        if let Some(session) = manager.active_sessions.get_mut(&session_id) {
            session.view_mode = new_mode;
        }
    }
}

fn format_duration(duration: std::time::Duration) -> String {
    let secs = duration.as_secs();
    let hours = secs / 3600;
    let minutes = (secs % 3600) / 60;
    let seconds = secs % 60;
    format!("{:02}:{:02}:{:02}", hours, minutes, seconds)
}

/// Render the add/edit connection dialog
pub fn render_connection_dialog(ctx: &egui::Context, manager: &mut RemoteDesktopManagerUI) -> bool {
    let mut should_close = false;

    // Determine dialog state before any borrows
    let is_editing = manager.editing_connection.is_some();
    let title = if is_editing {
        "Edit Connection"
    } else {
        "Add Connection"
    };

    // Extract the connection to edit or use the new connection
    // Work with cloned data to avoid borrow issues in the closure
    let mut connection = if is_editing {
        manager.editing_connection.clone().unwrap_or_default()
    } else {
        manager.new_connection.clone()
    };

    // Track if we need to save
    let mut save_clicked = false;
    let mut cancel_clicked = false;
    let mut add_clicked = false;

    egui::Window::new(title)
        .collapsible(false)
        .resizable(true)
        .default_size([500.0, 600.0])
        .show(ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                // Basic settings
                ui.group(|ui| {
                    ui.label("Basic Settings");
                    ui.horizontal(|ui| {
                        ui.label("Name:");
                        ui.text_edit_singleline(&mut connection.name);
                    });

                    ui.horizontal(|ui| {
                        ui.label("Protocol:");
                        egui::ComboBox::from_id_source("protocol")
                            .selected_text(format!("{:?}", connection.protocol))
                            .show_ui(ui, |ui| {
                                ui.selectable_value(
                                    &mut connection.protocol,
                                    RemoteDesktopProtocol::Rdp,
                                    "RDP",
                                );
                                ui.selectable_value(
                                    &mut connection.protocol,
                                    RemoteDesktopProtocol::Vnc,
                                    "VNC",
                                );
                                ui.selectable_value(
                                    &mut connection.protocol,
                                    RemoteDesktopProtocol::SshTunnelRdp,
                                    "SSH Tunnel RDP",
                                );
                                ui.selectable_value(
                                    &mut connection.protocol,
                                    RemoteDesktopProtocol::SshTunnelVnc,
                                    "SSH Tunnel VNC",
                                );
                            });
                    });

                    ui.horizontal(|ui| {
                        ui.label("Host:");
                        ui.text_edit_singleline(&mut connection.host);
                    });

                    ui.horizontal(|ui| {
                        ui.label("Port:");
                        ui.add(egui::DragValue::new(&mut connection.port));
                    });

                    ui.horizontal(|ui| {
                        ui.label("Username:");
                        ui.text_edit_singleline(&mut connection.username);
                    });

                    ui.horizontal(|ui| {
                        ui.label("Domain:");
                        ui.text_edit_singleline(&mut connection.domain);
                    });

                    ui.horizontal(|ui| {
                        ui.label("Password:");
                        ui.add(egui::TextEdit::singleline(&mut connection.password).password(true));
                    });
                });

                // SSH Tunnel settings (if applicable)
                if matches!(
                    connection.protocol,
                    RemoteDesktopProtocol::SshTunnelRdp | RemoteDesktopProtocol::SshTunnelVnc
                ) {
                    ui.group(|ui| {
                        ui.label("SSH Tunnel Settings");
                        ui.checkbox(&mut connection.use_ssh_tunnel, "Use SSH Tunnel");

                        if connection.use_ssh_tunnel {
                            ui.horizontal(|ui| {
                                ui.label("SSH Host:");
                                ui.text_edit_singleline(&mut connection.ssh_host);
                            });

                            ui.horizontal(|ui| {
                                ui.label("SSH Port:");
                                ui.add(egui::DragValue::new(&mut connection.ssh_port));
                            });

                            ui.horizontal(|ui| {
                                ui.label("SSH Username:");
                                ui.text_edit_singleline(&mut connection.ssh_username);
                            });

                            ui.horizontal(|ui| {
                                ui.label("Auth Type:");
                                egui::ComboBox::from_id_source("ssh_auth")
                                    .selected_text(format!("{:?}", connection.ssh_auth_type))
                                    .show_ui(ui, |ui| {
                                        ui.selectable_value(
                                            &mut connection.ssh_auth_type,
                                            SshAuthType::Agent,
                                            "SSH Agent",
                                        );
                                        ui.selectable_value(
                                            &mut connection.ssh_auth_type,
                                            SshAuthType::Password,
                                            "Password",
                                        );
                                        ui.selectable_value(
                                            &mut connection.ssh_auth_type,
                                            SshAuthType::Key,
                                            "Private Key",
                                        );
                                    });
                            });

                            if matches!(connection.ssh_auth_type, SshAuthType::Password) {
                                ui.horizontal(|ui| {
                                    ui.label("SSH Password:");
                                    ui.add(
                                        egui::TextEdit::singleline(&mut connection.ssh_password)
                                            .password(true),
                                    );
                                });
                            }
                        }
                    });
                }

                ui.separator();

                // Collapsible settings sections
                render_settings_sections(ui, &mut connection);
            });

            ui.separator();

            // Buttons
            ui.horizontal(|ui| {
                if ui.button("Cancel").clicked() {
                    cancel_clicked = true;
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button(if is_editing { "Save" } else { "Add" }).clicked() {
                        if is_editing {
                            save_clicked = true;
                        } else {
                            add_clicked = true;
                        }
                    }

                    if ui.button("Test Connection").clicked() {
                        // Test connection logic
                    }
                });
            });
        });

    // Apply changes outside the closure to avoid borrow issues
    if cancel_clicked {
        should_close = true;
    } else if save_clicked {
        // Update existing connection
        if let Some(idx) = manager
            .connections
            .iter()
            .position(|c| c.id == connection.id)
        {
            manager.connections[idx] = connection;
        }
        manager.editing_connection = None;
        should_close = true;
    } else if add_clicked {
        // Add new connection
        manager.add_connection(connection);
        manager.show_add_dialog = false;
        should_close = true;
    }

    if should_close {
        manager.show_add_dialog = false;
        manager.editing_connection = None;
    }

    manager.show_add_dialog || manager.editing_connection.is_some()
}

fn render_settings_sections(ui: &mut egui::Ui, connection: &mut RemoteDesktopConnectionUI) {
    // Display Settings
    let mut display_open = connection.expanded_section == Some(SettingsSection::Display);
    egui::CollapsingHeader::new("Display Settings").show(ui, |ui| {
        display_open = true;
        ui.horizontal(|ui| {
            ui.label("Resolution:");
            ui.add(egui::DragValue::new(&mut connection.display.width));
            ui.label("x");
            ui.add(egui::DragValue::new(&mut connection.display.height));
        });

        ui.horizontal(|ui| {
            ui.label("Color Depth:");
            egui::ComboBox::from_id_source("bpp")
                .selected_text(format!("{}-bit", connection.display.bpp))
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut connection.display.bpp, 8, "8-bit");
                    ui.selectable_value(&mut connection.display.bpp, 15, "15-bit");
                    ui.selectable_value(&mut connection.display.bpp, 16, "16-bit");
                    ui.selectable_value(&mut connection.display.bpp, 24, "24-bit");
                    ui.selectable_value(&mut connection.display.bpp, 32, "32-bit");
                });
        });

        ui.checkbox(&mut connection.display.fullscreen, "Fullscreen");
        ui.checkbox(&mut connection.display.multi_monitor, "Use all monitors");
        ui.checkbox(&mut connection.display.smart_sizing, "Smart sizing");
        ui.checkbox(
            &mut connection.display.dynamic_resolution,
            "Dynamic resolution update",
        );
        ui.checkbox(
            &mut connection.display.fit_session_to_window,
            "Fit session to window",
        );
    });
    if display_open {
        connection.expanded_section = Some(SettingsSection::Display);
    }

    // Performance Settings
    let mut perf_open = connection.expanded_section == Some(SettingsSection::Performance);
    egui::CollapsingHeader::new("Performance Settings").show(ui, |ui| {
        perf_open = true;
        ui.horizontal(|ui| {
            ui.label("Connection Type:");
            egui::ComboBox::from_id_source("conn_type")
                .selected_text(format!("{:?}", connection.performance.connection_type))
                .show_ui(ui, |ui| {
                    ui.selectable_value(
                        &mut connection.performance.connection_type,
                        ConnectionType::Lan,
                        "LAN",
                    );
                    ui.selectable_value(
                        &mut connection.performance.connection_type,
                        ConnectionType::Wan,
                        "WAN",
                    );
                    ui.selectable_value(
                        &mut connection.performance.connection_type,
                        ConnectionType::HighSpeedBroadband,
                        "High Speed Broadband",
                    );
                    ui.selectable_value(
                        &mut connection.performance.connection_type,
                        ConnectionType::Satellite,
                        "Satellite",
                    );
                    ui.selectable_value(
                        &mut connection.performance.connection_type,
                        ConnectionType::LowSpeedBroadband,
                        "Low Speed Broadband",
                    );
                    ui.selectable_value(
                        &mut connection.performance.connection_type,
                        ConnectionType::Modem,
                        "Modem",
                    );
                });
        });

        ui.checkbox(
            &mut connection.performance.disable_wallpaper,
            "Disable wallpaper",
        );
        ui.checkbox(&mut connection.performance.disable_themes, "Disable themes");
        ui.checkbox(
            &mut connection.performance.disable_menu_animations,
            "Disable menu animations",
        );
        ui.checkbox(
            &mut connection.performance.disable_full_window_drag,
            "Disable full window drag",
        );
        ui.checkbox(
            &mut connection.performance.disable_font_smoothing,
            "Disable font smoothing",
        );
        ui.checkbox(
            &mut connection.performance.persistent_bitmap_caching,
            "Persistent bitmap caching",
        );
        ui.checkbox(
            &mut connection.performance.compression,
            "Enable compression",
        );
    });
    if perf_open {
        connection.expanded_section = Some(SettingsSection::Performance);
    }

    // Local Resources
    let mut resources_open = connection.expanded_section == Some(SettingsSection::LocalResources);
    egui::CollapsingHeader::new("Local Resources").show(ui, |ui| {
        resources_open = true;
        ui.checkbox(&mut connection.local_resources.clipboard, "Clipboard");
        ui.checkbox(&mut connection.local_resources.printer, "Printers");
        ui.checkbox(&mut connection.local_resources.smart_cards, "Smart cards");
        ui.checkbox(&mut connection.local_resources.ports, "COM ports");
        ui.checkbox(&mut connection.local_resources.microphone, "Microphone");
        ui.checkbox(
            &mut connection.local_resources.video_capture,
            "Video capture device",
        );

        ui.horizontal(|ui| {
            ui.label("Drives:");
            egui::ComboBox::from_id_source("drives")
                .selected_text(format!("{:?}", connection.local_resources.drives))
                .show_ui(ui, |ui| {
                    ui.selectable_value(
                        &mut connection.local_resources.drives,
                        DriveRedirectionMode::Disabled,
                        "Disabled",
                    );
                    ui.selectable_value(
                        &mut connection.local_resources.drives,
                        DriveRedirectionMode::LocalDrives,
                        "Local drives",
                    );
                });
        });

        ui.horizontal(|ui| {
            ui.label("Audio:");
            egui::ComboBox::from_id_source("audio")
                .selected_text(format!("{:?}", connection.local_resources.audio))
                .show_ui(ui, |ui| {
                    ui.selectable_value(
                        &mut connection.local_resources.audio,
                        AudioRedirectionMode::Server,
                        "Play on remote computer",
                    );
                    ui.selectable_value(
                        &mut connection.local_resources.audio,
                        AudioRedirectionMode::Client,
                        "Play on this computer",
                    );
                    ui.selectable_value(
                        &mut connection.local_resources.audio,
                        AudioRedirectionMode::DoNotPlay,
                        "Do not play",
                    );
                });
        });
    });
    if resources_open {
        connection.expanded_section = Some(SettingsSection::LocalResources);
    }

    // Experience Settings
    let mut exp_open = connection.expanded_section == Some(SettingsSection::Experience);
    egui::CollapsingHeader::new("Experience Settings").show(ui, |ui| {
        exp_open = true;
        ui.checkbox(
            &mut connection.experience.desktop_background,
            "Desktop background",
        );
        ui.checkbox(&mut connection.experience.font_smoothing, "Font smoothing");
        ui.checkbox(
            &mut connection.experience.desktop_composition,
            "Desktop composition",
        );
        ui.checkbox(
            &mut connection.experience.show_window_contents,
            "Show window contents while dragging",
        );
        ui.checkbox(
            &mut connection.experience.menu_window_animation,
            "Menu and window animation",
        );
        ui.checkbox(&mut connection.experience.visual_styles, "Visual styles");

        ui.separator();
        ui.checkbox(
            &mut connection.experience.reconnect_on_disconnect,
            "Reconnect if connection is dropped",
        );
        ui.checkbox(&mut connection.experience.auto_reconnect, "Auto reconnect");
        if connection.experience.auto_reconnect {
            ui.horizontal(|ui| {
                ui.label("Max attempts:");
                ui.add(egui::DragValue::new(
                    &mut connection.experience.auto_reconnect_max_attempts,
                ));
            });
        }
    });
    if exp_open {
        connection.expanded_section = Some(SettingsSection::Experience);
    }

    // Recording Settings
    let mut rec_open = connection.expanded_section == Some(SettingsSection::Recording);
    egui::CollapsingHeader::new("Recording Settings").show(ui, |ui| {
        rec_open = true;
        ui.checkbox(
            &mut connection.recording.enabled,
            "Enable session recording",
        );
        ui.checkbox(
            &mut connection.recording.auto_start,
            "Auto-start recording on connect",
        );

        ui.horizontal(|ui| {
            ui.label("Format:");
            egui::ComboBox::from_id_source("format")
                .selected_text(format!("{:?}", connection.recording.format))
                .show_ui(ui, |ui| {
                    ui.selectable_value(
                        &mut connection.recording.format,
                        RecordingFormat::Mkv,
                        "MKV",
                    );
                    ui.selectable_value(
                        &mut connection.recording.format,
                        RecordingFormat::Mp4,
                        "MP4",
                    );
                    ui.selectable_value(
                        &mut connection.recording.format,
                        RecordingFormat::Avi,
                        "AVI",
                    );
                });
        });

        ui.horizontal(|ui| {
            ui.label("Quality:");
            egui::ComboBox::from_id_source("quality")
                .selected_text(format!("{:?}", connection.recording.quality))
                .show_ui(ui, |ui| {
                    ui.selectable_value(
                        &mut connection.recording.quality,
                        RecordingQuality::Low,
                        "Low",
                    );
                    ui.selectable_value(
                        &mut connection.recording.quality,
                        RecordingQuality::Medium,
                        "Medium",
                    );
                    ui.selectable_value(
                        &mut connection.recording.quality,
                        RecordingQuality::High,
                        "High",
                    );
                    ui.selectable_value(
                        &mut connection.recording.quality,
                        RecordingQuality::Lossless,
                        "Lossless",
                    );
                });
        });

        ui.checkbox(&mut connection.recording.include_audio, "Include audio");

        ui.horizontal(|ui| {
            ui.label("Output path:");
            ui.text_edit_singleline(&mut connection.recording.output_path);
            if ui.button("Browse").clicked() {
                // File browser dialog
            }
        });
    });
    if rec_open {
        connection.expanded_section = Some(SettingsSection::Recording);
    }
}
