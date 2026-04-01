#![allow(dead_code)]

use eframe::egui;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::sync::Mutex;
use std::thread;
use tokio::sync::mpsc;
use tracing::{error, info};
use uuid::Uuid;

mod bridge;
mod viewmodels;
mod ws_server;

use viewmodels::{AppViewModel, ServerViewModel};
use ws_server::{update_ui_debug, WsControlServer};

const DEV_TOOLS_HTML: &str = include_str!("../dev-tools.html");

fn main() -> eframe::Result {
    tracing_subscriber::fmt::init();
    info!("Starting EasySSH Debug for Windows");

    let rt = tokio::runtime::Runtime::new().expect("Failed to create Tokio runtime");

    // Start WebSocket debug API server
    let ws_view_model = Arc::new(Mutex::new(AppViewModel::new().expect("Failed to init")));
    let ws_vm_clone = ws_view_model.clone();
    rt.spawn(async move {
        let ws_server = WsControlServer::new(8765, ws_vm_clone);
        if let Err(e) = ws_server.start().await {
            error!("WebSocket server error: {}", e);
        }
    });

    // Start embedded dev-tools HTTP server
    thread::spawn(|| {
        if let Ok(server) = tiny_http::Server::http("0.0.0.0:8766") {
            for request in server.incoming_requests() {
                let url = request.url().to_string();
                let response = match url.as_str() {
                    "/" | "/dev-tools.html" | "/index.html" => {
                        tiny_http::Response::from_string(DEV_TOOLS_HTML).with_header(
                            tiny_http::Header::from_bytes(
                                &b"Content-Type"[..],
                                &b"text/html; charset=utf-8"[..],
                            )
                            .unwrap(),
                        )
                    }
                    _ => tiny_http::Response::from_string("404 Not Found").with_status_code(404),
                };
                let _ = request.respond(response);
            }
        }
    });

    let _ = webbrowser::open("http://localhost:8766/dev-tools.html");
    info!("WebSocket API: ws://localhost:8765 | DevTools: http://localhost:8766/dev-tools.html");

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1000.0, 700.0])
            .with_min_inner_size([600.0, 400.0]),
        ..Default::default()
    };

    eframe::run_native(
        "EasySSH-Debug",
        options,
        Box::new(|cc| Ok(Box::new(EasySSHApp::new(cc)))),
    )
}

#[derive(Clone)]
struct SessionTab {
    session_id: String,
    #[allow(dead_code)]
    server_id: String,
    title: String,
    output: String,
    #[allow(dead_code)]
    input: String,
    #[allow(dead_code)]
    connected: bool,
}

struct EasySSHApp {
    view_model: Arc<Mutex<AppViewModel>>,
    servers: Vec<ServerViewModel>,
    selected_server: Option<String>,
    search_query: String,
    show_add_dialog: bool,
    new_server: NewServerForm,
    add_error: Option<String>,
    show_connect_dialog: bool,
    connect_server: Option<ServerViewModel>,
    password: String,
    save_password: bool,
    use_saved_password: bool,
    connect_status: ConnectStatus,
    connect_error: Option<String>,
    current_session_id: Option<String>,
    terminal_output: String,
    command_input: String,
    is_terminal_active: bool,
    command_receiver: Option<mpsc::UnboundedReceiver<String>>,
    command_running: bool,

    // Termius-like features
    favorites: HashSet<String>,
    tags: HashMap<String, Vec<String>>,
    tag_input: String,
    session_tabs: Vec<SessionTab>,
    active_tab: Option<String>,

    // Command history
    command_history: Vec<String>,
    history_index: Option<usize>,
}

#[derive(Default)]
struct NewServerForm {
    name: String,
    host: String,
    port: String,
    username: String,
    auth_type: AuthType,
}

#[derive(Default, PartialEq)]
enum AuthType {
    #[default]
    Password,
    Key,
}

#[derive(Default, PartialEq)]
enum ConnectStatus {
    #[default]
    Idle,
    Connecting,
    #[allow(dead_code)]
    Connected,
    Error,
}

impl EasySSHApp {
    /// Maximum terminal output size to prevent memory issues
    const MAX_TERMINAL_CHARS: usize = 100_000;

    fn truncate_terminal(&mut self) {
        if self.terminal_output.len() > Self::MAX_TERMINAL_CHARS {
            let truncate_pos = self.terminal_output.len() - Self::MAX_TERMINAL_CHARS;
            if let Some(pos) = self.terminal_output[..truncate_pos].find('\n') {
                self.terminal_output = format!("[...truncated {} bytes...]\n{}",
                    truncate_pos - pos - 1,
                    &self.terminal_output[pos + 1..]);
            } else {
                self.terminal_output = self.terminal_output[truncate_pos..].to_string();
            }
        }
    }

    fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let view_model = Arc::new(Mutex::new(AppViewModel::new().expect("Failed to init")));
        let servers = view_model.lock().unwrap().get_servers();

        Self {
            view_model,
            servers,
            selected_server: None,
            search_query: String::new(),
            show_add_dialog: false,
            new_server: NewServerForm::default(),
            add_error: None,
            show_connect_dialog: false,
            connect_server: None,
            password: String::new(),
            save_password: false,
            use_saved_password: false,
            connect_status: ConnectStatus::Idle,
            connect_error: None,
            current_session_id: None,
            terminal_output: String::new(),
            command_input: String::new(),
            is_terminal_active: false,
            command_receiver: None,
            command_running: false,
            favorites: HashSet::new(),
            tags: HashMap::new(),
            tag_input: String::new(),
            session_tabs: Vec::new(),
            active_tab: None,
            command_history: Vec::new(),
            history_index: None,
        }
    }

    fn refresh_servers(&mut self) {
        let vm = self.view_model.lock().unwrap();
        self.servers = vm.get_servers();
    }

    fn add_server(&mut self) {
        if self.new_server.name.is_empty() {
            self.add_error = Some("Name is required".to_string());
            return;
        }
        if self.new_server.host.is_empty() {
            self.add_error = Some("Host is required".to_string());
            return;
        }
        if self.new_server.username.is_empty() {
            self.add_error = Some("Username is required".to_string());
            return;
        }

        let port: i64 = self.new_server.port.parse().unwrap_or(22);
        let auth = match self.new_server.auth_type {
            AuthType::Password => "password",
            AuthType::Key => "key",
        };

        let vm = self.view_model.lock().unwrap();
        match vm.add_server(
            &self.new_server.name,
            &self.new_server.host,
            port,
            &self.new_server.username,
            auth,
            None, // group_id
        ) {
            Ok(_) => {
                info!("Server added successfully: {}", self.new_server.name);
                self.show_add_dialog = false;
                self.new_server = NewServerForm::default();
                self.add_error = None;
                drop(vm);
                self.refresh_servers();
            }
            Err(e) => {
                error!("Failed to add server: {}", e);
                self.add_error = Some(format!("Failed to add server: {}", e));
            }
        }
    }

    fn start_connect(&mut self) {
        if let Some(server_id) = self.selected_server.as_ref() {
            if let Some(server) = self.servers.iter().find(|s| s.id == *server_id).cloned() {
                self.show_connect_dialog = true;
                self.connect_server = Some(server.clone());

                let vm = self.view_model.lock().unwrap();
                if let Some(saved_pwd) = vm.get_saved_password(&server.id) {
                    self.password = saved_pwd;
                    self.use_saved_password = true;
                    self.save_password = true;
                } else {
                    self.password.clear();
                    self.use_saved_password = false;
                    self.save_password = true;
                }

                self.connect_status = ConnectStatus::Idle;
                self.connect_error = None;
            }
        }
    }

    fn do_connect(&mut self) {
        if let Some(ref server) = self.connect_server {
            self.connect_status = ConnectStatus::Connecting;
            self.connect_error = None;

            let session_id = Uuid::new_v4().to_string();
            let password = if self.password.is_empty() {
                None
            } else {
                Some(self.password.clone())
            };

            let vm = self.view_model.lock().unwrap();

            if self.save_password {
                if let Some(ref pwd) = password {
                    if let Err(e) = vm.save_password(&server.id, pwd) {
                        error!("Failed to save password: {}", e);
                        self.connect_error = Some(format!("Save password failed: {}", e));
                    }
                }
            }

            match vm.connect(
                &session_id,
                &server.host,
                server.port,
                &server.username,
                password.as_deref(),
            ) {
                Ok(_) => {
                    info!("Connected to {}", server.name);
                    self.show_connect_dialog = false;
                    self.connect_status = ConnectStatus::Idle;
                    self.current_session_id = Some(session_id.clone());
                    self.is_terminal_active = true;
                    self.command_running = false;
                    self.terminal_output = format!(
                        "Connected to {} ({}@{}:{})\n\n",
                        server.name, server.username, server.host, server.port
                    );

                    // Create session tab (Termius-like multi-session)
                    let tab = SessionTab {
                        session_id: session_id.clone(),
                        server_id: server.id.clone(),
                        title: format!("{}@{}", server.username, server.host),
                        output: self.terminal_output.clone(),
                        input: String::new(),
                        connected: true,
                    };
                    self.session_tabs.push(tab);
                    self.active_tab = Some(session_id.clone());

                    // Start persistent shell stream once
                    match vm.execute_stream(&session_id, "") {
                        Ok(receiver) => {
                            self.command_receiver = Some(receiver);
                        }
                        Err(e) => {
                            self.terminal_output
                                .push_str(&format!("[stream init failed] {}\n", e));
                        }
                    }
                }
                Err(e) => {
                    error!("Failed to connect to {}: {}", server.name, e);
                    self.connect_status = ConnectStatus::Error;
                    self.connect_error = Some(e.to_string());
                }
            }
        }
    }

    fn poll_command_output(&mut self) {
        if let Some(receiver) = self.command_receiver.as_mut() {
            while let Ok(chunk) = receiver.try_recv() {
                self.terminal_output.push_str(&chunk);
                if let Some(active) = self.active_tab.clone() {
                    if let Some(tab) = self.session_tabs.iter_mut().find(|t| t.session_id == active) {
                        tab.output.push_str(&chunk);
                    }
                }
            }
        }
        // Prevent memory bloat from large terminal outputs
        self.truncate_terminal();
    }

    fn execute_command(&mut self) {
        if let Some(ref session_id) = self.current_session_id {
            let cmd = self.command_input.trim().to_string();
            if cmd.is_empty() {
                return;
            }

            // Add to history (avoid duplicates)
            if !self.command_history.contains(&cmd) {
                self.command_history.push(cmd.clone());
                if self.command_history.len() > 100 {
                    self.command_history.remove(0);
                }
            }
            self.history_index = None;

            let vm = self.view_model.lock().unwrap();
            self.terminal_output.push_str(&format!("$ {}\n", cmd));

            // True bidirectional shell: write command to existing persistent shell stdin
            let line = format!("{}\n", cmd);
            if let Err(e) = vm.write_shell_input(session_id, line.as_bytes()) {
                self.terminal_output
                    .push_str(&format!("Error writing to shell: {}\n", e));
                error!("Shell write failed: {}", e);
            }

            self.command_input.clear();
        }
    }

    fn navigate_history(&mut self, up: bool) {
        if self.command_history.is_empty() {
            return;
        }

        if up {
            if self.history_index.is_none() {
                self.history_index = Some(self.command_history.len().saturating_sub(1));
            } else {
                self.history_index = Some(self.history_index.unwrap().saturating_sub(1));
            }
        } else {
            if let Some(idx) = self.history_index {
                if idx >= self.command_history.len() - 1 {
                    self.history_index = None;
                    self.command_input.clear();
                    return;
                }
                self.history_index = Some(idx + 1);
            } else {
                return;
            }
        }

        if let Some(idx) = self.history_index {
            if let Some(cmd) = self.command_history.get(idx) {
                self.command_input = cmd.clone();
            }
        }
    }

    fn stop_current_command(&mut self) {
        if let Some(session_id) = self.current_session_id.clone() {
            let vm = self.view_model.lock().unwrap();
            let _ = vm.interrupt_command(&session_id);
            let _ = vm.write_shell_input(&session_id, &[0x03]);
            let _ = vm.write_shell_input(&session_id, b"\n");
            self.terminal_output.push_str("\n^C\n");
        }
    }

    fn disconnect(&mut self) {
        let session_id = self.current_session_id.clone();

        // Remove tab from session_tabs
        if let Some(ref sid) = session_id {
            self.session_tabs.retain(|t| t.session_id != *sid);
        }

        // Switch to another tab if available
        if let Some(tab) = self.session_tabs.last() {
            self.active_tab = Some(tab.session_id.clone());
            self.current_session_id = Some(tab.session_id.clone());
            self.terminal_output = tab.output.clone();
            self.is_terminal_active = true;
        } else {
            self.current_session_id = None;
            self.command_receiver = None;
            self.command_running = false;
            self.is_terminal_active = false;
            self.active_tab = None;
        }

        self.connect_status = ConnectStatus::Idle;
        self.show_connect_dialog = false;
        self.terminal_output.push_str("\nDisconnected.\n");

        if let Some(session_id) = session_id {
            let vm = self.view_model.lock().unwrap();
            let _ = vm.interrupt_command(&session_id);
            if let Err(e) = vm.disconnect(&session_id) {
                error!("Disconnect error: {}", e);
            }
        }
    }
}

impl eframe::App for EasySSHApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.poll_command_output();
        if self.command_receiver.is_some() {
            ctx.request_repaint_after(std::time::Duration::from_millis(16));
        }

        // Ctrl+number session tab switching (Termius-style quick switch)
        if ctx.input(|i| i.modifiers.ctrl) {
            for idx in 0..9 {
                let key = match idx {
                    0 => egui::Key::Num1,
                    1 => egui::Key::Num2,
                    2 => egui::Key::Num3,
                    3 => egui::Key::Num4,
                    4 => egui::Key::Num5,
                    5 => egui::Key::Num6,
                    6 => egui::Key::Num7,
                    7 => egui::Key::Num8,
                    _ => egui::Key::Num9,
                };
                if ctx.input(|i| i.key_pressed(key)) {
                    if let Some(tab) = self.session_tabs.get(idx).cloned() {
                        self.active_tab = Some(tab.session_id.clone());
                        self.current_session_id = Some(tab.session_id.clone());
                        self.terminal_output = tab.output.clone();
                    }
                }
            }
        }

        // Ctrl+C passthrough to persistent shell
        if self.current_session_id.is_some() {
            let ctrl_c = ctx.input(|i| i.modifiers.ctrl && i.key_pressed(egui::Key::C));
            if ctrl_c {
                self.stop_current_command();
            }
        }

        // Ctrl+L to clear terminal
        if self.current_session_id.is_some() {
            let ctrl_l = ctx.input(|i| i.modifiers.ctrl && i.key_pressed(egui::Key::L));
            if ctrl_l {
                self.terminal_output.clear();
            }
        }

        // Push UI heartbeat/debug state for websocket diagnostics
        update_ui_debug(
            self.current_session_id.is_some(),
            self.current_session_id.clone(),
            self.terminal_output.len(),
            self.command_input.len(),
        );

        egui::TopBottomPanel::top("top_bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading("EasySSH");
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("+ Add Server").clicked() {
                        self.show_add_dialog = true;
                        self.new_server = NewServerForm::default();
                        self.add_error = None;
                    }
                });
            });
        });

        if self.show_add_dialog {
            egui::Window::new("Add New Server")
                .collapsible(false)
                .resizable(false)
                .default_size([400.0, 350.0])
                .show(ctx, |ui| {
                    ui.vertical_centered(|ui| {
                        ui.heading("Add New Server");
                    });
                    ui.separator();

                    if let Some(ref err) = self.add_error {
                        ui.colored_label(egui::Color32::RED, err);
                        ui.separator();
                    }

                    egui::Grid::new("add_server_grid")
                        .num_columns(2)
                        .spacing([10.0, 10.0])
                        .show(ui, |ui| {
                            ui.label("Name:");
                            ui.text_edit_singleline(&mut self.new_server.name);
                            ui.end_row();

                            ui.label("Host:");
                            ui.text_edit_singleline(&mut self.new_server.host);
                            ui.end_row();

                            ui.label("Port:");
                            ui.text_edit_singleline(&mut self.new_server.port);
                            ui.end_row();

                            ui.label("Username:");
                            ui.text_edit_singleline(&mut self.new_server.username);
                            ui.end_row();

                            ui.label("Auth Type:");
                            ui.horizontal(|ui| {
                                ui.radio_value(
                                    &mut self.new_server.auth_type,
                                    AuthType::Password,
                                    "Password",
                                );
                                ui.radio_value(
                                    &mut self.new_server.auth_type,
                                    AuthType::Key,
                                    "SSH Key",
                                );
                            });
                            ui.end_row();
                        });

                    ui.separator();

                    if self.new_server.port.is_empty() {
                        ui.label("Default port: 22");
                    }

                    ui.separator();

                    ui.horizontal(|ui| {
                        if ui.button("Cancel").clicked() {
                            self.show_add_dialog = false;
                        }
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.button("Add Server").clicked() {
                                self.add_server();
                            }
                        });
                    });
                });
        }

        if self.show_connect_dialog {
            let server_info = self
                .connect_server
                .as_ref()
                .map(|s| (s.name.clone(), s.username.clone(), s.host.clone(), s.port));

            if let Some((name, username, host, port)) = server_info {
                egui::Window::new(format!("Connect to {}", name))
                    .collapsible(false)
                    .resizable(false)
                    .default_size([400.0, 300.0])
                    .show(ctx, |ui| {
                        ui.heading(format!("{}@{}:{}", username, host, port));
                        ui.separator();

                        if let Some(ref err) = self.connect_error {
                            ui.colored_label(egui::Color32::RED, err);
                            ui.separator();
                        }

                        match self.connect_status {
                            ConnectStatus::Idle => {
                                if self.use_saved_password && !self.password.is_empty() {
                                    ui.horizontal(|ui| {
                                        ui.colored_label(
                                            egui::Color32::GREEN,
                                            "Saved password loaded from keychain",
                                        );
                                        if ui.button("Clear").clicked() {
                                            self.password.clear();
                                            self.use_saved_password = false;
                                            self.save_password = false;
                                        }
                                    });
                                    ui.add_space(5.0);
                                }

                                ui.horizontal(|ui| {
                                    ui.label("Password:");
                                    ui.add(
                                        egui::TextEdit::singleline(&mut self.password)
                                            .password(true),
                                    );
                                });

                                ui.checkbox(&mut self.save_password, "Save password to keychain");

                                ui.separator();

                                ui.horizontal(|ui| {
                                    if ui.button("Cancel").clicked() {
                                        self.show_connect_dialog = false;
                                    }
                                    ui.with_layout(
                                        egui::Layout::right_to_left(egui::Align::Center),
                                        |ui| {
                                            if ui.button("Connect").clicked() {
                                                self.do_connect();
                                            }
                                        },
                                    );
                                });
                            }
                            ConnectStatus::Connecting => {
                                ui.label("Connecting...");
                                ui.spinner();
                            }
                            ConnectStatus::Connected => {
                                ui.colored_label(
                                    egui::Color32::GREEN,
                                    "Connected successfully!",
                                );
                            }
                            ConnectStatus::Error => {
                                ui.colored_label(egui::Color32::RED, "Connection failed");
                                ui.separator();
                                if ui.button("Close").clicked() {
                                    self.show_connect_dialog = false;
                                    self.connect_status = ConnectStatus::Idle;
                                }
                                if ui.button("Retry").clicked() {
                                    self.connect_status = ConnectStatus::Idle;
                                    self.connect_error = None;
                                }
                            }
                        }
                    });
            }
        }

        egui::SidePanel::left("server_list")
            .width_range(200.0..=400.0)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label("Servers");
                });
                ui.separator();

                ui.horizontal(|ui| {
                    ui.label("Search:");
                    ui.text_edit_singleline(&mut self.search_query);
                });
                ui.separator();

                egui::ScrollArea::vertical().show(ui, |ui| {
                    let filtered: Vec<&ServerViewModel> = if self.search_query.is_empty() {
                        self.servers.iter().collect()
                    } else {
                        let query = self.search_query.to_lowercase();
                        self.servers
                            .iter()
                            .filter(|s| {
                                s.name.to_lowercase().contains(&query)
                                    || s.host.to_lowercase().contains(&query)
                            })
                            .collect()
                    };

                    for server in filtered {
                        let is_selected = self.selected_server.as_ref() == Some(&server.id);
                        let has_session = self.current_session_id.is_some() && is_selected;

                        let is_favorite = self.favorites.contains(&server.id);
                        let favorite_prefix = if is_favorite { "★ " } else { "" };
                        let tags_text = self
                            .tags
                            .get(&server.id)
                            .map(|t| {
                                if t.is_empty() {
                                    "".to_string()
                                } else {
                                    format!("\n#{}", t.join(" #"))
                                }
                            })
                            .unwrap_or_default();

                        let label = if has_session {
                            format!(
                                "● {}{}\n{}@{}:{}{}",
                                favorite_prefix,
                                server.name,
                                server.username,
                                server.host,
                                server.port,
                                tags_text
                            )
                        } else {
                            format!(
                                "{}{}\n{}@{}:{}{}",
                                favorite_prefix,
                                server.name,
                                server.username,
                                server.host,
                                server.port,
                                tags_text
                            )
                        };

                        let response = ui.selectable_label(is_selected, label);
                        if response.clicked() {
                            self.selected_server = Some(server.id.clone());
                        }
                    }
                });
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            if self.is_terminal_active && self.current_session_id.is_some() {
                // Session tab bar (click to switch, Ctrl+1-9 shortcuts)
                if !self.session_tabs.is_empty() {
                    ui.horizontal_wrapped(|ui| {
                        for (idx, tab) in self.session_tabs.iter().enumerate() {
                            let is_active = self.active_tab.as_ref() == Some(&tab.session_id);
                            let shortcut = if idx < 9 { format!("{}{}", idx + 1, if is_active { " ●" } else { "" }) } else { String::new() };
                            let label = format!("{}{}", tab.title, shortcut);

                            let btn = egui::Button::new(
                                egui::RichText::new(label)
                                    .color(if is_active { egui::Color32::WHITE } else { egui::Color32::GRAY })
                            )
                            .fill(if is_active { egui::Color32::from_rgb(0, 80, 60) } else { egui::Color32::TRANSPARENT })
                            .rounding(4.0);

                            if ui.add(btn).clicked() {
                                self.active_tab = Some(tab.session_id.clone());
                                self.current_session_id = Some(tab.session_id.clone());
                                self.terminal_output = tab.output.clone();
                            }
                            ui.add_space(4.0);
                        }
                    });
                    ui.separator();
                }

                ui.horizontal(|ui| {
                    ui.heading("SSH Terminal");
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.colored_label(egui::Color32::GREEN, "● Connected");
                        if ui.button("Disconnect").clicked() {
                            self.disconnect();
                        }
                        if ui.button("Ctrl+C").clicked() {
                            self.stop_current_command();
                        }
                        if ui.button("Clear").clicked() {
                            self.terminal_output.clear();
                        }
                    });
                });
                ui.separator();

                let available_height = ui.available_height() - 60.0;
                egui::Frame::group(ui.style()).show(ui, |ui| {
                    ui.set_min_height(available_height);
                    ui.set_max_height(available_height);

                    egui::ScrollArea::vertical()
                        .auto_shrink([false; 2])
                        .stick_to_bottom(true)
                        .show(ui, |ui| {
                            ui.label(egui::RichText::new(&self.terminal_output).monospace().size(14.0));
                        });
                });

                ui.separator();

                ui.horizontal(|ui| {
                    ui.label("$");

                    let input_id = ui.make_persistent_id("cmd_input");
                    let enter_pressed = ui.input(|i| i.key_pressed(egui::Key::Enter));

                    // Command history navigation
                    if ui.input(|i| i.key_pressed(egui::Key::ArrowUp)) {
                        self.navigate_history(true);
                    }
                    if ui.input(|i| i.key_pressed(egui::Key::ArrowDown)) {
                        self.navigate_history(false);
                    }

                    let response = ui.add(
                        egui::TextEdit::singleline(&mut self.command_input)
                            .font(egui::TextStyle::Monospace)
                            .desired_width(ui.available_width() - 140.0)
                            .id(input_id),
                    );

                    if enter_pressed && !self.command_input.is_empty() {
                        self.execute_command();
                    }

                    ui.memory_mut(|m| m.request_focus(response.id));

                    if ui.button("Execute").clicked() {
                        self.execute_command();
                    }
                });
            } else if let Some(server_id) = &self.selected_server {
                if let Some(server) = self.servers.iter().find(|s| &s.id == server_id) {
                    let server_id_for_actions = server.id.clone();
                    ui.heading(&server.name);
                    ui.separator();

                    ui.group(|ui| {
                        ui.label(format!("Host: {}", server.host));
                        ui.label(format!("Port: {}", server.port));
                        ui.label(format!("Username: {}", server.username));
                    });

                    ui.separator();

                    ui.horizontal(|ui| {
                        let is_favorite = self.favorites.contains(&server.id);
                        if ui.button(if is_favorite { "★ Unfavorite" } else { "☆ Favorite" }).clicked() {
                            if is_favorite {
                                self.favorites.remove(&server.id);
                            } else {
                                self.favorites.insert(server.id.clone());
                            }
                        }

                        // Custom tag input + add button
                        ui.add(
                            egui::TextEdit::singleline(&mut self.tag_input)
                                .hint_text("tag")
                                .desired_width(100.0),
                        );

                        if ui.button("+Tag").clicked() {
                            let tag = self.tag_input.trim().to_lowercase();
                            if !tag.is_empty() {
                                let tags = self.tags.entry(server.id.clone()).or_default();
                                if !tags.iter().any(|t| t == &tag) {
                                    tags.push(tag);
                                }
                                self.tag_input.clear();
                            }
                        }

                        if ui.button("+prod").clicked() {
                            let tags = self.tags.entry(server.id.clone()).or_default();
                            if !tags.iter().any(|t| t == "prod") {
                                tags.push("prod".to_string());
                            }
                        }

                        if ui.button("+staging").clicked() {
                            let tags = self.tags.entry(server.id.clone()).or_default();
                            if !tags.iter().any(|t| t == "staging") {
                                tags.push("staging".to_string());
                            }
                        }

                        if ui.button("Clear Tags").clicked() {
                            self.tags.remove(&server.id);
                        }
                    });

                    // Tag chips with remove actions
                    if let Some(tags) = self.tags.get(&server.id).cloned() {
                        if !tags.is_empty() {
                            ui.horizontal_wrapped(|ui| {
                                ui.label("Tags:");
                                for tag in tags {
                                    if ui.button(format!("#{} ×", tag)).clicked() {
                                        if let Some(v) = self.tags.get_mut(&server.id) {
                                            v.retain(|t| t != &tag);
                                            if v.is_empty() {
                                                self.tags.remove(&server.id);
                                            }
                                        }
                                    }
                                }
                            });
                        }
                    }

                    ui.separator();

                    ui.horizontal(|ui| {
                        if ui.button("Connect").clicked() {
                            self.start_connect();
                        }

                        if ui.button("Delete Server").clicked() {
                            let delete_result = {
                                let vm = self.view_model.lock().unwrap();
                                vm.delete_server(&server_id_for_actions)
                            };

                            match delete_result {
                                Ok(_) => {
                                    self.selected_server = None;
                                    self.current_session_id = None;
                                    self.is_terminal_active = false;
                                    self.refresh_servers();
                                    self.terminal_output.push_str("\n[Server deleted]\n");
                                }
                                Err(e) => {
                                    self.terminal_output.push_str(&format!("\n[Delete failed: {}]\n", e));
                                }
                            }
                        }
                    });
                } else {
                    ui.label("Server not found");
                }
            } else {
                ui.centered_and_justified(|ui| {
                    ui.label("Select a server to connect");
                });
            }
        });
    }
}
