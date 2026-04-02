#![allow(dead_code)]

use eframe::egui;
use easyssh_core::{
    docker::{
        ContainerInfo, ContainerStatus, ImageInfo, NetworkInfo, VolumeInfo,
        ComposeProject, ContainerStats, PortMapping, DockerConnection, DockerHostType,
    },
    AppState, docker_list_containers, docker_start_container, docker_stop_container,
    docker_restart_container, docker_remove_container, docker_list_images,
    docker_pull_image, docker_list_networks, docker_list_volumes,
    docker_stream_logs, docker_exec, docker_get_stats,
    docker_list_compose_projects, docker_compose_up, docker_compose_down,
};
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};
use std::collections::HashMap;

/// Docker Management Panel for EasySSH
pub struct DockerPanel {
    /// Current view mode
    view_mode: DockerViewMode,
    /// Selected SSH session ID
    selected_session: Option<String>,
    /// Container list
    containers: Vec<ContainerInfo>,
    /// Image list
    images: Vec<ImageInfo>,
    /// Network list
    networks: Vec<NetworkInfo>,
    /// Volume list
    volumes: Vec<VolumeInfo>,
    /// Compose projects
    compose_projects: Vec<ComposeProject>,
    /// Selected container ID
    selected_container: Option<String>,
    /// Selected image ID
    selected_image: Option<String>,
    /// Log receiver channel
    log_receiver: Option<mpsc::UnboundedReceiver<String>>,
    /// Log buffer
    log_buffer: String,
    /// Container stats
    container_stats: HashMap<String, ContainerStats>,
    /// Search query
    search_query: String,
    /// Show all containers (including stopped)
    show_all_containers: bool,
    /// Loading state
    is_loading: bool,
    /// Error message
    error_message: Option<String>,
    /// New container dialog
    show_create_dialog: bool,
    /// Pull image dialog
    show_pull_dialog: bool,
    /// Create container form
    create_form: CreateContainerForm,
    /// Pull image form
    pull_form: PullImageForm,
    /// Docker connections
    connections: Vec<DockerConnection>,
    /// Selected connection
    selected_connection: Option<String>,
    /// Auto-refresh interval
    auto_refresh: bool,
    /// Last refresh time
    last_refresh: Option<std::time::Instant>,
}

#[derive(Debug, Clone, PartialEq)]
enum DockerViewMode {
    Containers,
    Images,
    Networks,
    Volumes,
    Compose,
    Logs,
    Stats,
}

#[derive(Default)]
struct CreateContainerForm {
    name: String,
    image: String,
    command: String,
    ports: Vec<(u16, u16, String)>, // host, container, protocol
    volumes: Vec<(String, String)>, // host, container
    env: Vec<(String, String)>,    // key, value
    network: String,
    restart_policy: String,
}

#[derive(Default)]
struct PullImageForm {
    image: String,
    tag: String,
    registry: String,
}

impl DockerPanel {
    pub fn new() -> Self {
        Self {
            view_mode: DockerViewMode::Containers,
            selected_session: None,
            containers: Vec::new(),
            images: Vec::new(),
            networks: Vec::new(),
            volumes: Vec::new(),
            compose_projects: Vec::new(),
            selected_container: None,
            selected_image: None,
            log_receiver: None,
            log_buffer: String::new(),
            container_stats: HashMap::new(),
            search_query: String::new(),
            show_all_containers: false,
            is_loading: false,
            error_message: None,
            show_create_dialog: false,
            show_pull_dialog: false,
            create_form: CreateContainerForm::default(),
            pull_form: PullImageForm::default(),
            connections: Vec::new(),
            selected_connection: None,
            auto_refresh: true,
            last_refresh: None,
        }
    }

    pub fn set_session(&mut self, session_id: String) {
        self.selected_session = Some(session_id);
        self.refresh_data();
    }

    pub fn update(&mut self, ctx: &egui::Context) {
        // Handle auto-refresh
        if self.auto_refresh {
            if let Some(last) = self.last_refresh {
                if last.elapsed().as_secs() > 5 {
                    self.refresh_data();
                }
            } else {
                self.refresh_data();
            }
        }

        // Receive logs
        if let Some(ref mut rx) = self.log_receiver {
            while let Ok(log) = rx.try_recv() {
                self.log_buffer.push_str(&log);
                self.log_buffer.push('\n');
                // Keep buffer size limited
                if self.log_buffer.len() > 100_000 {
                    let split = self.log_buffer.len() - 50_000;
                    self.log_buffer = self.log_buffer.split_off(split);
                }
            }
        }
    }

    pub fn render(&mut self, ui: &mut egui::Ui) {
        // Toolbar
        ui.horizontal(|ui| {
            ui.heading("Docker Management");
            ui.add_space(20.0);

            // View selector
            ui.label("View:");
            ui.selectable_value(&mut self.view_mode, DockerViewMode::Containers, "Containers");
            ui.selectable_value(&mut self.view_mode, DockerViewMode::Images, "Images");
            ui.selectable_value(&mut self.view_mode, DockerViewMode::Networks, "Networks");
            ui.selectable_value(&mut self.view_mode, DockerViewMode::Volumes, "Volumes");
            ui.selectable_value(&mut self.view_mode, DockerViewMode::Compose, "Compose");

            ui.add_space(20.0);

            // Refresh button
            if ui.button("Refresh").clicked() {
                self.refresh_data();
            }

            // Auto-refresh toggle
            ui.checkbox(&mut self.auto_refresh, "Auto-refresh");

            ui.add_space(20.0);

            // Search
            ui.label("Search:");
            ui.text_edit_singleline(&mut self.search_query);
        });

        ui.separator();

        // Error message
        if let Some(ref error) = self.error_message {
            ui.colored_label(egui::Color32::RED, format!("Error: {}", error));
        }

        // Loading indicator
        if self.is_loading {
            ui.label("Loading...");
        }

        // Main content based on view mode
        match self.view_mode {
            DockerViewMode::Containers => self.render_containers(ui),
            DockerViewMode::Images => self.render_images(ui),
            DockerViewMode::Networks => self.render_networks(ui),
            DockerViewMode::Volumes => self.render_volumes(ui),
            DockerViewMode::Compose => self.render_compose(ui),
            DockerViewMode::Logs => self.render_logs(ui),
            DockerViewMode::Stats => self.render_stats(ui),
        }

        // Dialogs
        if self.show_create_dialog {
            self.render_create_dialog(ui.ctx());
        }
        if self.show_pull_dialog {
            self.render_pull_dialog(ui.ctx());
        }
    }

    fn render_containers(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.checkbox(&mut self.show_all_containers, "Show all containers");
            ui.add_space(20.0);
            if ui.button("Create Container").clicked() {
                self.show_create_dialog = true;
            }
            ui.add_space(10.0);
            if ui.button("Prune").clicked() {
                // TODO: Implement prune
            }
        });

        ui.separator();

        // Container table
        let text_height = egui::TextStyle::Body.resolve(ui.style()).size;
        let table = egui_extras::TableBuilder::new(ui)
            .striped(true)
            .resizable(true)
            .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
            .column(egui_extras::Column::auto())
            .column(egui_extras::Column::auto())
            .column(egui_extras::Column::auto())
            .column(egui_extras::Column::auto())
            .column(egui_extras::Column::auto())
            .column(egui_extras::Column::auto())
            .column(egui_extras::Column::remainder());

        table.header(text_height, |mut header| {
            header.col(|ui| { ui.heading("ID"); });
            header.col(|ui| { ui.heading("Name"); });
            header.col(|ui| { ui.heading("Image"); });
            header.col(|ui| { ui.heading("Status"); });
            header.col(|ui| { ui.heading("Ports"); });
            header.col(|ui| { ui.heading("Created"); });
            header.col(|ui| { ui.heading("Actions"); });
        })
        .body(|mut body| {
            for container in &self.containers {
                if !self.search_query.is_empty() {
                    let search_lower = self.search_query.to_lowercase();
                    let name_match = container.names.iter().any(|n| n.to_lowercase().contains(&search_lower));
                    let image_match = container.image.to_lowercase().contains(&search_lower);
                    let id_match = container.id.to_lowercase().contains(&search_lower);
                    if !name_match && !image_match && !id_match {
                        continue;
                    }
                }

                body.row(text_height, |mut row| {
                    row.col(|ui| {
                        ui.monospace(&container.id[..12]);
                    });
                    row.col(|ui| {
                        ui.label(container.names.first().unwrap_or(&"<unnamed>".to_string()));
                    });
                    row.col(|ui| {
                        ui.label(&container.image);
                    });
                    row.col(|ui| {
                        let color = match container.status {
                            ContainerStatus::Running => egui::Color32::GREEN,
                            ContainerStatus::Exited | ContainerStatus::Dead => egui::Color32::RED,
                            ContainerStatus::Paused => egui::Color32::YELLOW,
                            _ => egui::Color32::GRAY,
                        };
                        ui.colored_label(color, format!("{:?}", container.status));
                    });
                    row.col(|ui| {
                        let ports: Vec<String> = container.ports.iter()
                            .map(|p| format!("{}:{}/{}", p.public_port, p.private_port, p.protocol))
                            .collect();
                        ui.label(ports.join(", "));
                    });
                    row.col(|ui| {
                        let created = chrono::DateTime::from_timestamp(container.created, 0)
                            .map(|dt| dt.format("%Y-%m-%d %H:%M").to_string())
                            .unwrap_or_else(|| "Unknown".to_string());
                        ui.label(created);
                    });
                    row.col(|ui| {
                        ui.horizontal(|ui| {
                            if container.status.can_start() {
                                if ui.button("Start").clicked() {
                                    self.start_container(&container.id);
                                }
                            }
                            if container.status.can_stop() {
                                if ui.button("Stop").clicked() {
                                    self.stop_container(&container.id);
                                }
                            }
                            if container.status.can_restart() {
                                if ui.button("Restart").clicked() {
                                    self.restart_container(&container.id);
                                }
                            }
                            if ui.button("Logs").clicked() {
                                self.selected_container = Some(container.id.clone());
                                self.view_mode = DockerViewMode::Logs;
                                self.start_log_stream(&container.id);
                            }
                            if ui.button("Stats").clicked() {
                                self.selected_container = Some(container.id.clone());
                                self.view_mode = DockerViewMode::Stats;
                            }
                            if ui.button("Exec").clicked() {
                                self.show_exec_dialog(ui, &container.id);
                            }
                            if ui.button("Remove").clicked() {
                                self.remove_container(&container.id);
                            }
                        });
                    });
                });
            }
        });
    }

    fn render_images(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            if ui.button("Pull Image").clicked() {
                self.show_pull_dialog = true;
            }
            ui.add_space(10.0);
            if ui.button("Prune").clicked() {
                // TODO: Implement prune
            }
        });

        ui.separator();

        // Image table
        let text_height = egui::TextStyle::Body.resolve(ui.style()).size;
        let table = egui_extras::TableBuilder::new(ui)
            .striped(true)
            .resizable(true)
            .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
            .column(egui_extras::Column::auto())
            .column(egui_extras::Column::auto())
            .column(egui_extras::Column::auto())
            .column(egui_extras::Column::remainder());

        table.header(text_height, |mut header| {
            header.col(|ui| { ui.heading("ID"); });
            header.col(|ui| { ui.heading("Repository"); });
            header.col(|ui| { ui.heading("Size"); });
            header.col(|ui| { ui.heading("Actions"); });
        })
        .body(|mut body| {
            for image in &self.images {
                if !self.search_query.is_empty() {
                    let search_lower = self.search_query.to_lowercase();
                    let tag_match = image.repo_tags.iter().any(|t| t.to_lowercase().contains(&search_lower));
                    if !tag_match {
                        continue;
                    }
                }

                body.row(text_height, |mut row| {
                    row.col(|ui| {
                        ui.monospace(&image.id[..12]);
                    });
                    row.col(|ui| {
                        ui.label(image.repo_tags.first().unwrap_or(&"<none>".to_string()));
                    });
                    row.col(|ui| {
                        let size_mb = image.size as f64 / (1024.0 * 1024.0);
                        ui.label(format!("{:.2} MB", size_mb));
                    });
                    row.col(|ui| {
                        ui.horizontal(|ui| {
                            if ui.button("Remove").clicked() {
                                self.remove_image(&image.id);
                            }
                        });
                    });
                });
            }
        });
    }

    fn render_networks(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            if ui.button("Create Network").clicked() {
                // TODO: Show create network dialog
            }
        });

        ui.separator();

        // Network table
        let text_height = egui::TextStyle::Body.resolve(ui.style()).size;
        let table = egui_extras::TableBuilder::new(ui)
            .striped(true)
            .resizable(true)
            .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
            .column(egui_extras::Column::auto())
            .column(egui_extras::Column::auto())
            .column(egui_extras::Column::auto())
            .column(egui_extras::Column::remainder());

        table.header(text_height, |mut header| {
            header.col(|ui| { ui.heading("ID"); });
            header.col(|ui| { ui.heading("Name"); });
            header.col(|ui| { ui.heading("Driver"); });
            header.col(|ui| { ui.heading("Actions"); });
        })
        .body(|mut body| {
            for network in &self.networks {
                body.row(text_height, |mut row| {
                    row.col(|ui| {
                        ui.monospace(&network.id[..12]);
                    });
                    row.col(|ui| {
                        ui.label(&network.name);
                    });
                    row.col(|ui| {
                        ui.label(&network.driver);
                    });
                    row.col(|ui| {
                        if ui.button("Remove").clicked() {
                            self.remove_network(&network.id);
                        }
                    });
                });
            }
        });
    }

    fn render_volumes(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            if ui.button("Create Volume").clicked() {
                // TODO: Show create volume dialog
            }
            ui.add_space(10.0);
            if ui.button("Prune").clicked() {
                // TODO: Implement prune
            }
        });

        ui.separator();

        // Volume table
        let text_height = egui::TextStyle::Body.resolve(ui.style()).size;
        let table = egui_extras::TableBuilder::new(ui)
            .striped(true)
            .resizable(true)
            .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
            .column(egui_extras::Column::auto())
            .column(egui_extras::Column::auto())
            .column(egui_extras::Column::auto())
            .column(egui_extras::Column::remainder());

        table.header(text_height, |mut header| {
            header.col(|ui| { ui.heading("Name"); });
            header.col(|ui| { ui.heading("Driver"); });
            header.col(|ui| { ui.heading("Mountpoint"); });
            header.col(|ui| { ui.heading("Actions"); });
        })
        .body(|mut body| {
            for volume in &self.volumes {
                body.row(text_height, |mut row| {
                    row.col(|ui| {
                        ui.label(&volume.name);
                    });
                    row.col(|ui| {
                        ui.label(&volume.driver);
                    });
                    row.col(|ui| {
                        ui.monospace(&volume.mountpoint);
                    });
                    row.col(|ui| {
                        if ui.button("Remove").clicked() {
                            self.remove_volume(&volume.name);
                        }
                    });
                });
            }
        });
    }

    fn render_compose(&mut self, ui: &mut egui::Ui) {
        // Compose projects list
        egui::ScrollArea::vertical().show(ui, |ui| {
            for project in &self.compose_projects {
                ui.group(|ui| {
                    ui.horizontal(|ui| {
                        ui.heading(&project.name);
                        ui.label(format!("Status: {}", project.status));
                    });

                    ui.label(format!("Config files: {}", project.config_files.join(", ")));

                    // Services
                    ui.collapsing("Services", |ui| {
                        for service in &project.services {
                            ui.horizontal(|ui| {
                                ui.label(&service.name);
                                let color = match service.state.as_str() {
                                    "running" => egui::Color32::GREEN,
                                    _ => egui::Color32::GRAY,
                                };
                                ui.colored_label(color, &service.state);
                                if let Some(ref health) = service.health {
                                    ui.label(format!("Health: {}", health));
                                }
                            });
                        }
                    });

                    // Actions
                    ui.horizontal(|ui| {
                        if ui.button("Up").clicked() {
                            if let Some(ref dir) = project.config_files.first() {
                                let project_dir = std::path::Path::new(dir)
                                    .parent()
                                    .map(|p| p.to_string_lossy().to_string())
                                    .unwrap_or_else(|| ".".to_string());
                                self.compose_up(&project_dir);
                            }
                        }
                        if ui.button("Down").clicked() {
                            if let Some(ref dir) = project.config_files.first() {
                                let project_dir = std::path::Path::new(dir)
                                    .parent()
                                    .map(|p| p.to_string_lossy().to_string())
                                    .unwrap_or_else(|| ".".to_string());
                                self.compose_down(&project_dir);
                            }
                        }
                    });
                });
            }
        });
    }

    fn render_logs(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            if let Some(ref id) = self.selected_container {
                ui.label(format!("Logs for container: {}", &id[..12]));
            }
            ui.add_space(20.0);
            if ui.button("Clear").clicked() {
                self.log_buffer.clear();
            }
            if ui.button("Stop").clicked() {
                self.stop_log_stream();
            }
            if ui.button("Back").clicked() {
                self.view_mode = DockerViewMode::Containers;
                self.stop_log_stream();
            }
        });

        ui.separator();

        egui::ScrollArea::vertical()
            .stick_to_bottom(true)
            .show(ui, |ui| {
                ui.monospace(&self.log_buffer);
            });
    }

    fn render_stats(&mut self, ui: &mut egui::Ui) {
        if let Some(ref id) = self.selected_container {
            ui.label(format!("Stats for container: {}", &id[..12]));

            if let Some(stats) = self.container_stats.get(id) {
                ui.group(|ui| {
                    ui.heading("CPU");
                    ui.label(format!("Total Usage: {}", stats.cpu_stats.total_usage));
                    ui.label(format!("Online CPUs: {}", stats.cpu_stats.online_cpus));
                });

                ui.group(|ui| {
                    ui.heading("Memory");
                    let usage_mb = stats.memory_stats.usage as f64 / (1024.0 * 1024.0);
                    let limit_mb = stats.memory_stats.limit as f64 / (1024.0 * 1024.0);
                    ui.label(format!("Usage: {:.2} MB", usage_mb));
                    ui.label(format!("Limit: {:.2} MB", limit_mb));
                    if limit_mb > 0.0 {
                        let percentage = (usage_mb / limit_mb) * 100.0;
                        ui.label(format!("Percentage: {:.1}%", percentage));
                    }
                });

                ui.group(|ui| {
                    ui.heading("Network");
                    let rx_mb = stats.network_stats.rx_bytes as f64 / (1024.0 * 1024.0);
                    let tx_mb = stats.network_stats.tx_bytes as f64 / (1024.0 * 1024.0);
                    ui.label(format!("RX: {:.2} MB", rx_mb));
                    ui.label(format!("TX: {:.2} MB", tx_mb));
                });
            } else {
                ui.label("Loading stats...");
            }
        }

        if ui.button("Back").clicked() {
            self.view_mode = DockerViewMode::Containers;
        }
    }

    fn render_create_dialog(&mut self, ctx: &egui::Context) {
        let mut open = true;
        egui::Window::new("Create Container")
            .open(&mut open)
            .show(ctx, |ui| {
                ui.label("Name:");
                ui.text_edit_singleline(&mut self.create_form.name);

                ui.label("Image:");
                ui.text_edit_singleline(&mut self.create_form.image);

                ui.label("Command:");
                ui.text_edit_singleline(&mut self.create_form.command);

                ui.label("Network:");
                ui.text_edit_singleline(&mut self.create_form.network);

                ui.label("Restart Policy:");
                egui::ComboBox::from_label("Policy")
                    .selected_text(&self.create_form.restart_policy)
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut self.create_form.restart_policy, "".to_string(), "None");
                        ui.selectable_value(&mut self.create_form.restart_policy, "always".to_string(), "Always");
                        ui.selectable_value(&mut self.create_form.restart_policy, "unless-stopped".to_string(), "Unless Stopped");
                        ui.selectable_value(&mut self.create_form.restart_policy, "on-failure".to_string(), "On Failure");
                    });

                ui.horizontal(|ui| {
                    if ui.button("Create").clicked() {
                        self.create_container();
                        self.show_create_dialog = false;
                    }
                    if ui.button("Cancel").clicked() {
                        self.show_create_dialog = false;
                    }
                });
            });

        if !open {
            self.show_create_dialog = false;
        }
    }

    fn render_pull_dialog(&mut self, ctx: &egui::Context) {
        let mut open = true;
        egui::Window::new("Pull Image")
            .open(&mut open)
            .show(ctx, |ui| {
                ui.label("Image:");
                ui.text_edit_singleline(&mut self.pull_form.image);

                ui.label("Tag:");
                ui.text_edit_singleline(&mut self.pull_form.tag);

                ui.label("Registry (optional):");
                ui.text_edit_singleline(&mut self.pull_form.registry);

                ui.horizontal(|ui| {
                    if ui.button("Pull").clicked() {
                        self.pull_image();
                        self.show_pull_dialog = false;
                    }
                    if ui.button("Cancel").clicked() {
                        self.show_pull_dialog = false;
                    }
                });
            });

        if !open {
            self.show_pull_dialog = false;
        }
    }

    fn show_exec_dialog(&self, ui: &mut egui::Ui, container_id: &str) {
        // TODO: Implement exec dialog
    }

    // Async actions

    fn refresh_data(&mut self) {
        if let Some(ref session_id) = self.selected_session {
            let session_id = session_id.clone();

            match self.view_mode {
                DockerViewMode::Containers => {
                    // TODO: Spawn async task to fetch containers
                }
                DockerViewMode::Images => {
                    // TODO: Spawn async task to fetch images
                }
                DockerViewMode::Networks => {
                    // TODO: Spawn async task to fetch networks
                }
                DockerViewMode::Volumes => {
                    // TODO: Spawn async task to fetch volumes
                }
                DockerViewMode::Compose => {
                    // TODO: Spawn async task to fetch compose projects
                }
                _ => {}
            }

            self.last_refresh = Some(std::time::Instant::now());
        }
    }

    fn start_container(&mut self, container_id: &str) {
        // TODO: Spawn async task
    }

    fn stop_container(&mut self, container_id: &str) {
        // TODO: Spawn async task
    }

    fn restart_container(&mut self, container_id: &str) {
        // TODO: Spawn async task
    }

    fn remove_container(&mut self, container_id: &str) {
        // TODO: Spawn async task
    }

    fn create_container(&mut self) {
        // TODO: Spawn async task
    }

    fn pull_image(&mut self) {
        // TODO: Spawn async task
    }

    fn remove_image(&mut self, image_id: &str) {
        // TODO: Spawn async task
    }

    fn remove_network(&mut self, network_id: &str) {
        // TODO: Spawn async task
    }

    fn remove_volume(&mut self, volume_name: &str) {
        // TODO: Spawn async task
    }

    fn compose_up(&mut self, project_dir: &str) {
        // TODO: Spawn async task
    }

    fn compose_down(&mut self, project_dir: &str) {
        // TODO: Spawn async task
    }

    fn start_log_stream(&mut self, container_id: &str) {
        // TODO: Spawn async task to get log receiver
    }

    fn stop_log_stream(&mut self) {
        self.log_receiver = None;
    }
}

impl Default for DockerPanel {
    fn default() -> Self {
        Self::new()
    }
}
