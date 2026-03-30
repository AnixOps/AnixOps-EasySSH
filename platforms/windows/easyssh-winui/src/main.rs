use eframe::egui;
use std::sync::Arc;
use std::sync::Mutex;
use tracing::{info, error};
use uuid::Uuid;

mod bridge;
mod viewmodels;

use viewmodels::{AppViewModel, ServerViewModel};

fn main() -> eframe::Result {
    tracing_subscriber::fmt::init();
    info!("Starting EasySSH for Windows");

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1000.0, 700.0])
            .with_min_inner_size([600.0, 400.0]),
        ..Default::default()
    };

    eframe::run_native(
        "EasySSH",
        options,
        Box::new(|cc| Ok(Box::new(EasySSHApp::new(cc)))),
    )
}

struct EasySSHApp {
    view_model: Arc<Mutex<AppViewModel>>,
    servers: Vec<ServerViewModel>,
    selected_server: Option<String>,
    search_query: String,
    // Add Server Dialog
    show_add_dialog: bool,
    new_server: NewServerForm,
    add_error: Option<String>,
    // Connect Dialog
    show_connect_dialog: bool,
    connect_server: Option<ServerViewModel>,
    password: String,
    connect_status: ConnectStatus,
    connect_error: Option<String>,
    // Session
    current_session_id: Option<String>,
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
    Connected,
    Error,
}

impl EasySSHApp {
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
            connect_status: ConnectStatus::Idle,
            connect_error: None,
            current_session_id: None,
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
        match vm.add_server(&self.new_server.name, &self.new_server.host, port, &self.new_server.username, auth) {
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
        if let Some(server_id) = &self.selected_server {
            if let Some(server) = self.servers.iter().find(|s| &s.id == server_id).cloned() {
                self.show_connect_dialog = true;
                self.connect_server = Some(server);
                self.password = String::new();
                self.connect_status = ConnectStatus::Idle;
                self.connect_error = None;
                self.current_session_id = None;
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
            match vm.connect(
                &session_id,
                &server.host,
                server.port,
                &server.username,
                password.as_deref()
            ) {
                Ok(_) => {
                    info!("Connected to {}", server.name);
                    self.connect_status = ConnectStatus::Connected;
                    self.current_session_id = Some(session_id);
                }
                Err(e) => {
                    error!("Failed to connect to {}: {}", server.name, e);
                    self.connect_status = ConnectStatus::Error;
                    self.connect_error = Some(e.to_string());
                }
            }
        }
    }
}

impl eframe::App for EasySSHApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Top panel
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

        // Add Server Dialog
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
                                ui.radio_value(&mut self.new_server.auth_type, AuthType::Password, "Password");
                                ui.radio_value(&mut self.new_server.auth_type, AuthType::Key, "SSH Key");
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

        // Connect Dialog
        if self.show_connect_dialog {
            if let Some(ref server) = self.connect_server {
                let is_connected = self.connect_status == ConnectStatus::Connected;
                let server_name = server.name.clone();
                let server_info = format!("{}@{}:{}", server.username, server.host, server.port);

                egui::Window::new(format!("Connect to {}", server_name))
                    .collapsible(false)
                    .resizable(false)
                    .default_size([400.0, 250.0])
                    .show(ctx, |ui| {
                        ui.heading(&server_info);
                        ui.separator();

                        // Error message
                        if let Some(ref err) = self.connect_error {
                            ui.colored_label(egui::Color32::RED, err);
                            ui.separator();
                        }

                        // Status
                        match self.connect_status {
                            ConnectStatus::Idle => {
                                ui.label("Enter password to connect:");
                                ui.add_space(10.0);

                                ui.horizontal(|ui| {
                                    ui.label("Password:");
                                    ui.add(egui::TextEdit::singleline(&mut self.password).password(true));
                                });

                                ui.separator();

                                ui.horizontal(|ui| {
                                    if ui.button("Cancel").clicked() {
                                        self.show_connect_dialog = false;
                                    }
                                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                        if ui.button("Connect").clicked() {
                                            self.do_connect();
                                        }
                                    });
                                });
                            }
                            ConnectStatus::Connecting => {
                                ui.label("Connecting...");
                                ui.spinner();
                            }
                            ConnectStatus::Connected => {
                                ui.colored_label(egui::Color32::GREEN, "✓ Connected successfully!");
                                ui.separator();
                                ui.label("Session active");
                                ui.separator();
                                if ui.button("Disconnect").clicked() {
                                    self.show_connect_dialog = false;
                                    self.connect_status = ConnectStatus::Idle;
                                    self.current_session_id = None;
                                }
                            }
                            ConnectStatus::Error => {
                                ui.colored_label(egui::Color32::RED, "✗ Connection failed");
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

        // Left panel - server list
        egui::SidePanel::left("server_list").width_range(200.0..=400.0).show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label("Servers");
            });
            ui.separator();

            // Search box
            ui.horizontal(|ui| {
                ui.label("Search:");
                ui.text_edit_singleline(&mut self.search_query);
            });
            ui.separator();

            // Server list
            egui::ScrollArea::vertical().show(ui, |ui| {
                let filtered: Vec<&ServerViewModel> = if self.search_query.is_empty() {
                    self.servers.iter().collect()
                } else {
                    let query = self.search_query.to_lowercase();
                    self.servers.iter()
                        .filter(|s| s.name.to_lowercase().contains(&query)
                            || s.host.to_lowercase().contains(&query))
                        .collect()
                };

                for server in filtered {
                    let is_selected = self.selected_server.as_ref() == Some(&server.id);
                    let label = format!("{}\n{}@{}:{}",
                        server.name,
                        server.username,
                        server.host,
                        server.port
                    );

                    let response = ui.selectable_label(is_selected, label);
                    if response.clicked() {
                        self.selected_server = Some(server.id.clone());
                    }
                }
            });
        });

        // Central panel - connection details
        egui::CentralPanel::default().show(ctx, |ui| {
            if let Some(server_id) = &self.selected_server {
                if let Some(server) = self.servers.iter().find(|s| &s.id == server_id) {
                    ui.heading(&server.name);
                    ui.separator();

                    ui.group(|ui| {
                        ui.label(format!("Host: {}", server.host));
                        ui.label(format!("Port: {}", server.port));
                        ui.label(format!("Username: {}", server.username));
                    });

                    ui.separator();

                    if ui.button("Connect").clicked() {
                        self.start_connect();
                    }
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
