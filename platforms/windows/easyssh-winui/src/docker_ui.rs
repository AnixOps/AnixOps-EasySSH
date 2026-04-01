//! Docker Management UI Panel for EasySSH
//!
//! Provides Docker container, image, network, and volume management UI.

use crate::design::DesignTheme;
use chrono::{DateTime, Utc};
use egui::{Color32, RichText, Ui};

/// Docker container status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ContainerStatus {
    #[default]
    Created,
    Running,
    Paused,
    Restarting,
    Removing,
    Exited,
    Dead,
}

impl ContainerStatus {
    pub fn display_name(&self) -> &'static str {
        match self {
            ContainerStatus::Created => "已创建",
            ContainerStatus::Running => "运行中",
            ContainerStatus::Paused => "已暂停",
            ContainerStatus::Restarting => "重启中",
            ContainerStatus::Removing => "删除中",
            ContainerStatus::Exited => "已退出",
            ContainerStatus::Dead => "已死亡",
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            ContainerStatus::Running => "▶️",
            ContainerStatus::Paused => "⏸️",
            ContainerStatus::Exited => "⏹️",
            ContainerStatus::Dead => "💀",
            ContainerStatus::Created => "📦",
            ContainerStatus::Restarting => "🔄",
            ContainerStatus::Removing => "🗑️",
        }
    }

    pub fn color(&self) -> Color32 {
        match self {
            ContainerStatus::Running => Color32::from_rgb(100, 200, 100),    // Green
            ContainerStatus::Paused => Color32::from_rgb(255, 193, 7),       // Yellow
            ContainerStatus::Exited => Color32::from_rgb(150, 150, 150),      // Gray
            ContainerStatus::Dead => Color32::from_rgb(220, 53, 69),        // Red
            ContainerStatus::Created => Color32::from_rgb(100, 149, 237),   // Cornflower
            ContainerStatus::Restarting => Color32::from_rgb(255, 165, 0),  // Orange
            ContainerStatus::Removing => Color32::from_rgb(220, 53, 69),    // Red
        }
    }

    pub fn can_start(&self) -> bool {
        matches!(self, ContainerStatus::Created | ContainerStatus::Exited | ContainerStatus::Dead)
    }

    pub fn can_stop(&self) -> bool {
        matches!(self, ContainerStatus::Running | ContainerStatus::Restarting | ContainerStatus::Paused)
    }

    pub fn can_restart(&self) -> bool {
        matches!(self, ContainerStatus::Running | ContainerStatus::Exited | ContainerStatus::Paused | ContainerStatus::Dead)
    }
}

/// Docker container information
#[derive(Debug, Clone)]
pub struct ContainerInfo {
    pub id: String,
    pub name: String,
    pub image: String,
    pub status: ContainerStatus,
    pub created: DateTime<Utc>,
    pub ports: Vec<PortMapping>,
    pub cpu_usage: f64,
    pub memory_usage: f64,
}

/// Port mapping
#[derive(Debug, Clone)]
pub struct PortMapping {
    pub host_port: u16,
    pub container_port: u16,
    pub protocol: String,
}

/// Docker image information
#[derive(Debug, Clone)]
pub struct ImageInfo {
    pub id: String,
    pub repo_tags: Vec<String>,
    pub size: i64,
    pub created: DateTime<Utc>,
}

/// Docker network information
#[derive(Debug, Clone)]
pub struct NetworkInfo {
    pub id: String,
    pub name: String,
    pub driver: String,
    pub scope: String,
}

/// Docker volume information
#[derive(Debug, Clone)]
pub struct VolumeInfo {
    pub name: String,
    pub driver: String,
    pub mountpoint: String,
    pub size: Option<i64>,
}

/// Docker view tabs
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DockerTab {
    #[default]
    Containers,
    Images,
    Networks,
    Volumes,
    Compose,
}

impl DockerTab {
    pub fn display_name(&self) -> &'static str {
        match self {
            DockerTab::Containers => "容器",
            DockerTab::Images => "镜像",
            DockerTab::Networks => "网络",
            DockerTab::Volumes => "卷",
            DockerTab::Compose => "Compose",
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            DockerTab::Containers => "📦",
            DockerTab::Images => "🖼️",
            DockerTab::Networks => "🌐",
            DockerTab::Volumes => "💾",
            DockerTab::Compose => "🐳",
        }
    }
}

/// Docker manager UI state
#[derive(Default)]
pub struct DockerManagerUI {
    pub containers: Vec<ContainerInfo>,
    pub images: Vec<ImageInfo>,
    pub networks: Vec<NetworkInfo>,
    pub volumes: Vec<VolumeInfo>,
    pub active_tab: DockerTab,
    pub selected_container: Option<String>,
    pub selected_image: Option<String>,
    pub search_query: String,
    pub show_all_containers: bool,
    pub is_loading: bool,
    pub error_message: Option<String>,
    pub success_message: Option<(String, std::time::Instant)>,
    pub show_create_dialog: bool,
    pub show_pull_dialog: bool,
    pub new_container_form: NewContainerForm,
    pub pull_image_form: PullImageForm,
    pub show_logs_panel: bool,
    pub log_content: String,
    pub auto_refresh: bool,
    pub last_refresh: Option<std::time::Instant>,
}

#[derive(Default)]
pub struct NewContainerForm {
    pub name: String,
    pub image: String,
    pub command: String,
    pub ports: String,
    pub volumes: String,
    pub env_vars: String,
    pub network: String,
    pub restart_policy: String,
}

#[derive(Default)]
pub struct PullImageForm {
    pub image_name: String,
    pub tag: String,
    pub registry: String,
}

impl DockerManagerUI {
    pub fn new() -> Self {
        let mut manager = Self::default();
        manager.load_mock_data();
        manager
    }

    fn load_mock_data(&mut self) {
        // Mock containers
        self.containers = vec![
            ContainerInfo {
                id: "abc123def456".to_string(),
                name: "nginx-web".to_string(),
                image: "nginx:latest".to_string(),
                status: ContainerStatus::Running,
                created: Utc::now() - chrono::Duration::hours(2),
                ports: vec![
                    PortMapping { host_port: 8080, container_port: 80, protocol: "tcp".to_string() },
                ],
                cpu_usage: 2.5,
                memory_usage: 45.2,
            },
            ContainerInfo {
                id: "def789ghi012".to_string(),
                name: "redis-cache".to_string(),
                image: "redis:7-alpine".to_string(),
                status: ContainerStatus::Running,
                created: Utc::now() - chrono::Duration::days(1),
                ports: vec![
                    PortMapping { host_port: 6379, container_port: 6379, protocol: "tcp".to_string() },
                ],
                cpu_usage: 0.8,
                memory_usage: 12.3,
            },
            ContainerInfo {
                id: "ghi345jkl678".to_string(),
                name: "postgres-db".to_string(),
                image: "postgres:15".to_string(),
                status: ContainerStatus::Exited,
                created: Utc::now() - chrono::Duration::days(3),
                ports: vec![
                    PortMapping { host_port: 5432, container_port: 5432, protocol: "tcp".to_string() },
                ],
                cpu_usage: 0.0,
                memory_usage: 0.0,
            },
        ];

        // Mock images
        self.images = vec![
            ImageInfo {
                id: "sha256:abc123".to_string(),
                repo_tags: vec!["nginx:latest".to_string()],
                size: 187_000_000,
                created: Utc::now() - chrono::Duration::days(7),
            },
            ImageInfo {
                id: "sha256:def456".to_string(),
                repo_tags: vec!["redis:7-alpine".to_string()],
                size: 42_000_000,
                created: Utc::now() - chrono::Duration::days(14),
            },
            ImageInfo {
                id: "sha256:ghi789".to_string(),
                repo_tags: vec!["postgres:15".to_string(), "postgres:latest".to_string()],
                size: 450_000_000,
                created: Utc::now() - chrono::Duration::days(30),
            },
        ];

        // Mock networks
        self.networks = vec![
            NetworkInfo {
                id: "net123".to_string(),
                name: "bridge".to_string(),
                driver: "bridge".to_string(),
                scope: "local".to_string(),
            },
            NetworkInfo {
                id: "net456".to_string(),
                name: "host".to_string(),
                driver: "host".to_string(),
                scope: "local".to_string(),
            },
            NetworkInfo {
                id: "net789".to_string(),
                name: "app-network".to_string(),
                driver: "bridge".to_string(),
                scope: "local".to_string(),
            },
        ];

        // Mock volumes
        self.volumes = vec![
            VolumeInfo {
                name: "app_data".to_string(),
                driver: "local".to_string(),
                mountpoint: "/var/lib/docker/volumes/app_data/_data".to_string(),
                size: Some(1_073_741_824),
            },
            VolumeInfo {
                name: "db_storage".to_string(),
                driver: "local".to_string(),
                mountpoint: "/var/lib/docker/volumes/db_storage/_data".to_string(),
                size: Some(2_147_483_648),
            },
        ];
    }

    /// Render the Docker panel
    pub fn render(&mut self, ctx: &egui::Context, show_panel: &mut bool) {
        if !*show_panel {
            return;
        }

        let theme = DesignTheme::dark();

        egui::SidePanel::left("docker_panel")
            .width_range(450.0..=650.0)
            .default_width(550.0)
            .frame(egui::Frame {
                fill: theme.bg_secondary,
                stroke: egui::Stroke::new(1.0, theme.border_default),
                ..Default::default()
            })
            .show(ctx, |ui| {
                self.render_header(ui, &theme);
                ui.separator();
                self.render_tabs(ui);
                ui.separator();

                match self.active_tab {
                    DockerTab::Containers => self.render_containers(ui, &theme),
                    DockerTab::Images => self.render_images(ui, &theme),
                    DockerTab::Networks => self.render_networks(ui, &theme),
                    DockerTab::Volumes => self.render_volumes(ui, &theme),
                    DockerTab::Compose => self.render_compose(ui, &theme),
                }
            });

        // Render dialogs
        if self.show_create_dialog {
            self.render_create_container_dialog(ctx);
        }
        if self.show_pull_dialog {
            self.render_pull_image_dialog(ctx);
        }
    }

    fn render_header(&mut self, ui: &mut Ui, theme: &DesignTheme) {
        ui.horizontal(|ui| {
            ui.label(RichText::new("🐳").size(20.0));
            ui.heading(
                RichText::new("Docker 管理")
                    .color(theme.text_primary)
                    .size(18.0),
            );

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                // Auto-refresh toggle
                ui.checkbox(&mut self.auto_refresh, "自动刷新");
                ui.add_space(10.0);
                if ui.button("🔄 刷新").clicked() {
                    self.refresh_data();
                }
            });
        });
    }

    fn render_tabs(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            let tabs = [
                DockerTab::Containers,
                DockerTab::Images,
                DockerTab::Networks,
                DockerTab::Volumes,
                DockerTab::Compose,
            ];

            for tab in tabs {
                let is_active = self.active_tab == tab;
                let text = format!("{} {}", tab.icon(), tab.display_name());

                let btn = egui::Button::new(RichText::new(text).size(12.0))
                    .fill(if is_active {
                        egui::Color32::from_rgb(64, 156, 255)
                    } else {
                        egui::Color32::TRANSPARENT
                    });

                if ui.add(btn).clicked() {
                    self.active_tab = tab;
                }
            }
        });
    }

    fn render_containers(&mut self, ui: &mut Ui, theme: &DesignTheme) {
        // Search and filter
        ui.horizontal(|ui| {
            ui.add(
                egui::TextEdit::singleline(&mut self.search_query)
                    .hint_text("🔍 搜索容器...")
                    .desired_width(200.0),
            );
            ui.checkbox(&mut self.show_all_containers, "显示全部");

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("➕ 创建容器").clicked() {
                    self.show_create_dialog = true;
                    self.new_container_form = NewContainerForm::default();
                }
            });
        });

        ui.add_space(8.0);

        // Loading indicator
        if self.is_loading {
            ui.label("加载中...");
            return;
        }

        // Error message
        if let Some(ref error) = self.error_message {
            ui.colored_label(egui::Color32::RED, format!("错误: {}", error));
        }

        // Containers list
        let containers_clone = self.containers.clone();
        let filtered_containers: Vec<_> = if self.show_all_containers {
            containers_clone
        } else {
            containers_clone.into_iter()
                .filter(|c| matches!(c.status, ContainerStatus::Running | ContainerStatus::Paused | ContainerStatus::Restarting))
                .collect()
        };

        egui::ScrollArea::vertical().show(ui, |ui| {
            for container in filtered_containers {
                if !self.search_query.is_empty() {
                    let search_lower = self.search_query.to_lowercase();
                    if !container.name.to_lowercase().contains(&search_lower)
                        && !container.image.to_lowercase().contains(&search_lower)
                        && !container.id.to_lowercase().contains(&search_lower) {
                        continue;
                    }
                }
                self.render_container_item(ui, &container, theme);
            }
        });
    }

    fn render_container_item(&mut self, ui: &mut Ui, container: &ContainerInfo, theme: &DesignTheme) {
        let is_selected = self.selected_container.as_ref() == Some(&container.id);

        ui.group(|ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new(container.status.icon()).size(18.0));

                ui.vertical(|ui| {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(&container.name).strong().color(theme.text_primary));
                        // Status badge
                        let status_text = format!(" {} ", container.status.display_name());
                        ui.label(
                            RichText::new(status_text)
                                .size(10.0)
                                .background_color(container.status.color())
                                .color(egui::Color32::WHITE),
                        );
                    });
                    ui.label(
                        RichText::new(format!("{} ({})", container.image, &container.id[..12]))
                            .size(11.0)
                            .color(theme.text_secondary),
                    );

                    // Resource usage
                    if container.status == ContainerStatus::Running {
                        ui.label(
                            RichText::new(format!("CPU: {:.1}% | 内存: {:.1} MB", container.cpu_usage, container.memory_usage))
                                .size(10.0)
                                .color(egui::Color32::from_rgb(100, 200, 100)),
                        );
                    }

                    // Ports
                    if !container.ports.is_empty() {
                        let ports_str: Vec<String> = container.ports.iter()
                            .map(|p| format!("{}:{}/{}", p.host_port, p.container_port, p.protocol))
                            .collect();
                        ui.label(
                            RichText::new(format!("端口: {}", ports_str.join(", ")))
                                .size(10.0)
                                .color(theme.text_secondary),
                        );
                    }
                });

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    // Action buttons
                    ui.horizontal(|ui| {
                        if container.status.can_stop() {
                            if ui.button("⏹️ 停止").clicked() {
                                self.stop_container(&container.id);
                            }
                        }
                        if container.status.can_start() {
                            if ui.button("▶️ 启动").clicked() {
                                self.start_container(&container.id);
                            }
                        }
                        if container.status.can_restart() {
                            if ui.button("🔄 重启").clicked() {
                                self.restart_container(&container.id);
                            }
                        }
                        if ui.button("📜 日志").clicked() {
                            self.show_logs(&container.id);
                        }
                        if ui.button("🗑️").on_hover_text("删除").clicked() {
                            self.remove_container(&container.id);
                        }
                    });
                });
            });
        });

        ui.add_space(4.0);
    }

    fn render_images(&mut self, ui: &mut Ui, theme: &DesignTheme) {
        ui.horizontal(|ui| {
            ui.add(
                egui::TextEdit::singleline(&mut self.search_query)
                    .hint_text("🔍 搜索镜像...")
                    .desired_width(200.0),
            );

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("⬇️ 拉取镜像").clicked() {
                    self.show_pull_dialog = true;
                    self.pull_image_form = PullImageForm::default();
                }
            });
        });

        ui.add_space(8.0);

        egui::ScrollArea::vertical().show(ui, |ui| {
            for image in &self.images.clone() {
                if !self.search_query.is_empty() {
                    let search_lower = self.search_query.to_lowercase();
                    let tag_match = image.repo_tags.iter().any(|t| t.to_lowercase().contains(&search_lower));
                    if !tag_match {
                        continue;
                    }
                }
                self.render_image_item(ui, image, theme);
            }
        });
    }

    fn render_image_item(&mut self, ui: &mut Ui, image: &ImageInfo, theme: &DesignTheme) {
        ui.group(|ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new("🖼️").size(18.0));

                ui.vertical(|ui| {
                    for tag in &image.repo_tags {
                        ui.label(RichText::new(tag).strong().color(theme.text_primary));
                    }
                    ui.label(
                        RichText::new(format!("ID: {} | 大小: {:.2} MB", &image.id[..12], image.size as f64 / 1_048_576.0))
                            .size(11.0)
                            .color(theme.text_secondary),
                    );
                });

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("🗑️ 删除").clicked() {
                        self.remove_image(&image.id);
                    }
                    if ui.button("▶️ 运行").clicked() {
                        self.run_image(&image.id);
                    }
                });
            });
        });

        ui.add_space(4.0);
    }

    fn render_networks(&mut self, ui: &mut Ui, theme: &DesignTheme) {
        ui.horizontal(|ui| {
            ui.heading("Docker 网络");
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("➕ 创建网络").clicked() {
                    // TODO: Show create network dialog
                }
            });
        });

        ui.add_space(8.0);

        egui::ScrollArea::vertical().show(ui, |ui| {
            for network in &self.networks.clone() {
                self.render_network_item(ui, network, theme);
            }
        });
    }

    fn render_network_item(&mut self, ui: &mut Ui, network: &NetworkInfo, theme: &DesignTheme) {
        ui.group(|ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new("🌐").size(18.0));

                ui.vertical(|ui| {
                    ui.label(RichText::new(&network.name).strong().color(theme.text_primary));
                    ui.label(
                        RichText::new(format!("驱动: {} | 范围: {}", network.driver, network.scope))
                            .size(11.0)
                            .color(theme.text_secondary),
                    );
                });

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if network.name != "bridge" && network.name != "host" && network.name != "none" {
                        if ui.button("🗑️ 删除").clicked() {
                            self.remove_network(&network.id);
                        }
                    }
                });
            });
        });

        ui.add_space(4.0);
    }

    fn render_volumes(&mut self, ui: &mut Ui, theme: &DesignTheme) {
        ui.horizontal(|ui| {
            ui.heading("Docker 卷");
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("➕ 创建卷").clicked() {
                    // TODO: Show create volume dialog
                }
            });
        });

        ui.add_space(8.0);

        egui::ScrollArea::vertical().show(ui, |ui| {
            for volume in &self.volumes.clone() {
                self.render_volume_item(ui, volume, theme);
            }
        });
    }

    fn render_volume_item(&mut self, ui: &mut Ui, volume: &VolumeInfo, theme: &DesignTheme) {
        ui.group(|ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new("💾").size(18.0));

                ui.vertical(|ui| {
                    ui.label(RichText::new(&volume.name).strong().color(theme.text_primary));
                    let size_str = volume.size.map(|s| format!("{:.2} MB", s as f64 / 1_048_576.0))
                        .unwrap_or_else(|| "大小未知".to_string());
                    ui.label(
                        RichText::new(format!("驱动: {} | {}", volume.driver, size_str))
                            .size(11.0)
                            .color(theme.text_secondary),
                    );
                });

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("🗑️ 删除").clicked() {
                        self.remove_volume(&volume.name);
                    }
                });
            });
        });

        ui.add_space(4.0);
    }

    fn render_compose(&mut self, ui: &mut Ui, _theme: &DesignTheme) {
        ui.horizontal(|ui| {
            ui.heading("Docker Compose 项目");
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("⬆️ 启动项目").clicked() {
                    // TODO: Show compose up dialog
                }
            });
        });

        ui.add_space(16.0);

        ui.centered_and_justified(|ui| {
            ui.label("🐳\n暂无 Compose 项目\n请选择一个包含 docker-compose.yml 文件的目录启动");
        });
    }

    fn render_create_container_dialog(&mut self, ctx: &egui::Context) {
        let mut open = true;
        egui::Window::new("创建新容器")
            .collapsible(false)
            .resizable(false)
            .default_size([400.0, 450.0])
            .open(&mut open)
            .show(ctx, |ui| {
                ui.label("容器名称:");
                ui.text_edit_singleline(&mut self.new_container_form.name);

                ui.label("镜像:");
                ui.text_edit_singleline(&mut self.new_container_form.image);

                ui.label("命令 (可选):");
                ui.text_edit_singleline(&mut self.new_container_form.command);

                ui.label("端口映射 (格式: 主机端口:容器端口, 如 8080:80):");
                ui.text_edit_singleline(&mut self.new_container_form.ports);

                ui.label("卷映射 (格式: 主机路径:容器路径):");
                ui.text_edit_singleline(&mut self.new_container_form.volumes);

                ui.label("环境变量 (格式: KEY=value, 多个用逗号分隔):");
                ui.text_edit_singleline(&mut self.new_container_form.env_vars);

                ui.label("网络:");
                ui.text_edit_singleline(&mut self.new_container_form.network);

                ui.label("重启策略:");
                egui::ComboBox::from_label("restart_policy")
                    .selected_text(&self.new_container_form.restart_policy)
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut self.new_container_form.restart_policy, "".to_string(), "无");
                        ui.selectable_value(&mut self.new_container_form.restart_policy, "always".to_string(), "总是");
                        ui.selectable_value(&mut self.new_container_form.restart_policy, "unless-stopped".to_string(), "除非手动停止");
                        ui.selectable_value(&mut self.new_container_form.restart_policy, "on-failure".to_string(), "失败时");
                    });

                ui.add_space(16.0);

                ui.horizontal(|ui| {
                    if ui.button("✅ 创建").clicked() {
                        self.create_container();
                        self.show_create_dialog = false;
                    }
                    if ui.button("❌ 取消").clicked() {
                        self.show_create_dialog = false;
                    }
                });
            });

        if !open {
            self.show_create_dialog = false;
        }
    }

    fn render_pull_image_dialog(&mut self, ctx: &egui::Context) {
        let mut open = true;
        egui::Window::new("拉取镜像")
            .collapsible(false)
            .resizable(false)
            .default_size([350.0, 250.0])
            .open(&mut open)
            .show(ctx, |ui| {
                ui.label("镜像名称:");
                ui.text_edit_singleline(&mut self.pull_image_form.image_name);

                ui.label("标签 (默认 latest):");
                ui.text_edit_singleline(&mut self.pull_image_form.tag);

                ui.label("镜像仓库 (可选，如 docker.io):");
                ui.text_edit_singleline(&mut self.pull_image_form.registry);

                ui.add_space(16.0);

                ui.horizontal(|ui| {
                    if ui.button("⬇️ 拉取").clicked() {
                        self.pull_image();
                        self.show_pull_dialog = false;
                    }
                    if ui.button("❌ 取消").clicked() {
                        self.show_pull_dialog = false;
                    }
                });
            });

        if !open {
            self.show_pull_dialog = false;
        }
    }

    // Actions
    fn refresh_data(&mut self) {
        self.is_loading = true;
        self.last_refresh = Some(std::time::Instant::now());
        // TODO: Spawn async task to fetch data from backend
        self.is_loading = false;
    }

    fn start_container(&mut self, container_id: &str) {
        self.show_success(format!("启动容器 {}", &container_id[..12]).as_str());
        // TODO: Call backend API
    }

    fn stop_container(&mut self, container_id: &str) {
        self.show_success(format!("停止容器 {}", &container_id[..12]).as_str());
        // TODO: Call backend API
    }

    fn restart_container(&mut self, container_id: &str) {
        self.show_success(format!("重启容器 {}", &container_id[..12]).as_str());
        // TODO: Call backend API
    }

    fn remove_container(&mut self, container_id: &str) {
        self.show_success(format!("删除容器 {}", &container_id[..12]).as_str());
        self.containers.retain(|c| c.id != container_id);
    }

    fn show_logs(&mut self, container_id: &str) {
        self.selected_container = Some(container_id.to_string());
        self.show_logs_panel = true;
        self.log_content = format!("显示容器 {} 的日志...\n", &container_id[..12]);
        // TODO: Stream logs from backend
    }

    fn create_container(&mut self) {
        let container = ContainerInfo {
            id: format!("new-{}", uuid::Uuid::new_v4().to_string()[..12].to_string()),
            name: self.new_container_form.name.clone(),
            image: self.new_container_form.image.clone(),
            status: ContainerStatus::Created,
            created: Utc::now(),
            ports: Vec::new(),
            cpu_usage: 0.0,
            memory_usage: 0.0,
        };
        self.containers.push(container);
        self.show_success("容器创建成功");
    }

    fn pull_image(&mut self) {
        let tag = if self.pull_image_form.tag.is_empty() {
            "latest"
        } else {
            &self.pull_image_form.tag
        };
        let full_name = format!("{}:{}", self.pull_image_form.image_name, tag);
        self.show_success(format!("开始拉取镜像 {}", full_name).as_str());
        // TODO: Call backend API
    }

    fn remove_image(&mut self, image_id: &str) {
        self.show_success(format!("删除镜像 {}", &image_id[..12]).as_str());
        self.images.retain(|i| i.id != image_id);
    }

    fn run_image(&mut self, image_id: &str) {
        if let Some(image) = self.images.iter().find(|i| i.id == image_id) {
            let tag = image.repo_tags.first().cloned().unwrap_or_else(|| "unnamed".to_string());
            self.new_container_form.image = tag.clone();
            self.show_create_dialog = true;
        }
    }

    fn remove_network(&mut self, network_id: &str) {
        self.show_success(format!("删除网络 {}", &network_id[..12]).as_str());
        self.networks.retain(|n| n.id != network_id);
    }

    fn remove_volume(&mut self, volume_name: &str) {
        self.show_success(format!("删除卷 {}", volume_name).as_str());
        self.volumes.retain(|v| v.name != volume_name);
    }

    fn show_success(&mut self, message: &str) {
        self.success_message = Some((
            message.to_string(),
            std::time::Instant::now(),
        ));
    }

    /// Update and clear expired messages
    pub fn update(&mut self) {
        if let Some((_, time)) = &self.success_message {
            if time.elapsed().as_secs() > 3 {
                self.success_message = None;
            }
        }

        // Auto-refresh
        if self.auto_refresh {
            if let Some(last) = self.last_refresh {
                if last.elapsed().as_secs() > 30 {
                    self.refresh_data();
                }
            } else {
                self.refresh_data();
            }
        }
    }
}

/// Render Docker panel helper function
pub fn render_docker_panel(
    ctx: &egui::Context,
    show_panel: &mut bool,
    manager: &mut DockerManagerUI,
) {
    manager.update();
    manager.render(ctx, show_panel);

    // Show success message as notification
    if let Some((message, _)) = &manager.success_message {
        egui::TopBottomPanel::top("docker_notification")
            .exact_height(40.0)
            .show(ctx, |ui| {
                ui.horizontal_centered(|ui| {
                    ui.label(
                        RichText::new(format!("✅ {}", message))
                            .color(Color32::from_rgb(100, 200, 100))
                            .strong(),
                    );
                });
            });
    }
}
