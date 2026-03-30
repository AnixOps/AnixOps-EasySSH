use eframe::egui;
use std::sync::Arc;
use std::sync::Mutex;
use tracing::info;

mod bridge;
mod viewmodels;

use bridge::BridgeHandle;
use viewmodels::{AppViewModel, ServerViewModel};

fn main() -> eframe::Result {
    // Initialize tracing
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
        }
    }
}

impl eframe::App for EasySSHApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Top panel - title bar
        egui::TopBottomPanel::top("top_bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading("EasySSH");
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("+ Add Server").clicked() {
                        // TODO: Add server dialog
                    }
                });
            });
        });

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
                for server in &self.servers {
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
                        // TODO: Initiate connection
                        info!("Connecting to {}", server.name);
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
