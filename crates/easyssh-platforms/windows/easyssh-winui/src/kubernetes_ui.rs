//! Kubernetes Management UI Panel for EasySSH
//!
//! Provides Kubernetes pod, deployment, service, and namespace management UI.

use crate::design::DesignTheme;
use chrono::{DateTime, Utc};
use egui::{Color32, RichText, Ui};

/// Kubernetes pod status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PodStatus {
    #[default]
    Pending,
    Running,
    Succeeded,
    Failed,
    Unknown,
    Terminating,
}

impl PodStatus {
    pub fn display_name(&self) -> &'static str {
        match self {
            PodStatus::Pending => "等待中",
            PodStatus::Running => "运行中",
            PodStatus::Succeeded => "已完成",
            PodStatus::Failed => "失败",
            PodStatus::Unknown => "未知",
            PodStatus::Terminating => "终止中",
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            PodStatus::Running => "▶️",
            PodStatus::Pending => "⏳",
            PodStatus::Succeeded => "✅",
            PodStatus::Failed => "❌",
            PodStatus::Unknown => "❓",
            PodStatus::Terminating => "🗑️",
        }
    }

    pub fn color(&self) -> Color32 {
        match self {
            PodStatus::Running => Color32::from_rgb(100, 200, 100), // Green
            PodStatus::Pending => Color32::from_rgb(255, 193, 7),   // Yellow
            PodStatus::Succeeded => Color32::from_rgb(64, 156, 255), // Blue
            PodStatus::Failed => Color32::from_rgb(220, 53, 69),    // Red
            PodStatus::Unknown => Color32::from_rgb(150, 150, 150), // Gray
            PodStatus::Terminating => Color32::from_rgb(255, 165, 0), // Orange
        }
    }
}

/// Kubernetes view tabs
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum KubernetesTab {
    #[default]
    Pods,
    Deployments,
    Services,
    ConfigMaps,
    Secrets,
    Nodes,
    Namespaces,
    Ingresses,
}

impl KubernetesTab {
    pub fn display_name(&self) -> &'static str {
        match self {
            KubernetesTab::Pods => "Pods",
            KubernetesTab::Deployments => "部署",
            KubernetesTab::Services => "服务",
            KubernetesTab::ConfigMaps => "配置",
            KubernetesTab::Secrets => "密钥",
            KubernetesTab::Nodes => "节点",
            KubernetesTab::Namespaces => "命名空间",
            KubernetesTab::Ingresses => "入口",
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            KubernetesTab::Pods => "📦",
            KubernetesTab::Deployments => "🚀",
            KubernetesTab::Services => "🔌",
            KubernetesTab::ConfigMaps => "⚙️",
            KubernetesTab::Secrets => "🔐",
            KubernetesTab::Nodes => "🖥️",
            KubernetesTab::Namespaces => "📁",
            KubernetesTab::Ingresses => "🌐",
        }
    }
}

/// Kubernetes namespace
#[derive(Debug, Clone)]
pub struct NamespaceInfo {
    pub name: String,
    pub status: String,
    pub created: DateTime<Utc>,
}

/// Kubernetes node
#[derive(Debug, Clone)]
pub struct NodeInfo {
    pub name: String,
    pub status: String,
    pub role: String,
    pub version: String,
    pub cpu_capacity: String,
    pub memory_capacity: String,
    pub cpu_usage: Option<f64>,
    pub memory_usage: Option<f64>,
}

/// Kubernetes pod information
#[derive(Debug, Clone)]
pub struct PodInfo {
    pub name: String,
    pub namespace: String,
    pub status: PodStatus,
    pub ready: String,
    pub restarts: i32,
    pub age: DateTime<Utc>,
    pub ip: String,
    pub node: String,
    pub containers: Vec<ContainerInfo>,
}

/// Container information within a pod
#[derive(Debug, Clone)]
pub struct ContainerInfo {
    pub name: String,
    pub image: String,
    pub ready: bool,
    pub restart_count: i32,
    pub state: String,
}

/// Kubernetes deployment information
#[derive(Debug, Clone)]
pub struct DeploymentInfo {
    pub name: String,
    pub namespace: String,
    pub ready: String,
    pub up_to_date: i32,
    pub available: i32,
    pub age: DateTime<Utc>,
    pub strategy: String,
}

/// Kubernetes service information
#[derive(Debug, Clone)]
pub struct ServiceInfo {
    pub name: String,
    pub namespace: String,
    pub type_: String,
    pub cluster_ip: String,
    pub external_ip: String,
    pub ports: Vec<ServicePort>,
    pub age: DateTime<Utc>,
}

/// Service port
#[derive(Debug, Clone)]
pub struct ServicePort {
    pub name: String,
    pub port: i32,
    pub target_port: String,
    pub protocol: String,
}

/// Kubernetes manager UI state
#[derive(Default)]
pub struct KubernetesManagerUI {
    pub namespaces: Vec<NamespaceInfo>,
    pub nodes: Vec<NodeInfo>,
    pub pods: Vec<PodInfo>,
    pub deployments: Vec<DeploymentInfo>,
    pub services: Vec<ServiceInfo>,
    pub active_tab: KubernetesTab,
    pub selected_namespace: String,
    pub selected_pod: Option<String>,
    pub selected_deployment: Option<String>,
    pub search_query: String,
    pub is_loading: bool,
    pub error_message: Option<String>,
    pub success_message: Option<(String, std::time::Instant)>,
    pub show_create_dialog: bool,
    pub new_resource_form: NewResourceForm,
    pub show_logs_panel: bool,
    pub log_content: String,
    pub auto_refresh: bool,
    pub last_refresh: Option<std::time::Instant>,
    pub connected: bool,
    pub current_context: String,
}

#[derive(Default)]
pub struct NewResourceForm {
    pub resource_type: String,
    pub name: String,
    pub namespace: String,
    pub image: String,
    pub replicas: i32,
    pub port: i32,
    pub yaml_content: String,
}

impl KubernetesManagerUI {
    pub fn new() -> Self {
        let mut manager = Self::default();
        manager.selected_namespace = "default".to_string();
        manager.current_context = "minikube".to_string();
        manager.load_mock_data();
        manager
    }

    fn load_mock_data(&mut self) {
        self.connected = true;

        // Mock namespaces
        self.namespaces = vec![
            NamespaceInfo {
                name: "default".to_string(),
                status: "Active".to_string(),
                created: Utc::now() - chrono::Duration::days(30),
            },
            NamespaceInfo {
                name: "kube-system".to_string(),
                status: "Active".to_string(),
                created: Utc::now() - chrono::Duration::days(30),
            },
            NamespaceInfo {
                name: "dev".to_string(),
                status: "Active".to_string(),
                created: Utc::now() - chrono::Duration::days(7),
            },
            NamespaceInfo {
                name: "prod".to_string(),
                status: "Active".to_string(),
                created: Utc::now() - chrono::Duration::days(14),
            },
        ];

        // Mock nodes
        self.nodes = vec![NodeInfo {
            name: "minikube".to_string(),
            status: "Ready".to_string(),
            role: "control-plane".to_string(),
            version: "v1.28.3".to_string(),
            cpu_capacity: "4".to_string(),
            memory_capacity: "8Gi".to_string(),
            cpu_usage: Some(35.5),
            memory_usage: Some(42.3),
        }];

        // Mock pods
        self.pods = vec![
            PodInfo {
                name: "nginx-deployment-7c5c9c4d4f-abc12".to_string(),
                namespace: "default".to_string(),
                status: PodStatus::Running,
                ready: "1/1".to_string(),
                restarts: 0,
                age: Utc::now() - chrono::Duration::hours(2),
                ip: "10.244.0.15".to_string(),
                node: "minikube".to_string(),
                containers: vec![ContainerInfo {
                    name: "nginx".to_string(),
                    image: "nginx:1.25".to_string(),
                    ready: true,
                    restart_count: 0,
                    state: "Running".to_string(),
                }],
            },
            PodInfo {
                name: "postgres-0".to_string(),
                namespace: "default".to_string(),
                status: PodStatus::Running,
                ready: "1/1".to_string(),
                restarts: 1,
                age: Utc::now() - chrono::Duration::days(1),
                ip: "10.244.0.20".to_string(),
                node: "minikube".to_string(),
                containers: vec![ContainerInfo {
                    name: "postgres".to_string(),
                    image: "postgres:15".to_string(),
                    ready: true,
                    restart_count: 1,
                    state: "Running".to_string(),
                }],
            },
            PodInfo {
                name: "redis-cache-5d4f8b7c9-x2y3z".to_string(),
                namespace: "dev".to_string(),
                status: PodStatus::Pending,
                ready: "0/1".to_string(),
                restarts: 0,
                age: Utc::now() - chrono::Duration::minutes(5),
                ip: "<none>".to_string(),
                node: "minikube".to_string(),
                containers: vec![ContainerInfo {
                    name: "redis".to_string(),
                    image: "redis:7".to_string(),
                    ready: false,
                    restart_count: 0,
                    state: "Pending".to_string(),
                }],
            },
            PodInfo {
                name: "failed-job-xxxxx".to_string(),
                namespace: "default".to_string(),
                status: PodStatus::Failed,
                ready: "0/1".to_string(),
                restarts: 3,
                age: Utc::now() - chrono::Duration::hours(3),
                ip: "10.244.0.25".to_string(),
                node: "minikube".to_string(),
                containers: vec![ContainerInfo {
                    name: "job-runner".to_string(),
                    image: "busybox:latest".to_string(),
                    ready: false,
                    restart_count: 3,
                    state: "Error".to_string(),
                }],
            },
        ];

        // Mock deployments
        self.deployments = vec![
            DeploymentInfo {
                name: "nginx-deployment".to_string(),
                namespace: "default".to_string(),
                ready: "3/3".to_string(),
                up_to_date: 3,
                available: 3,
                age: Utc::now() - chrono::Duration::days(2),
                strategy: "RollingUpdate".to_string(),
            },
            DeploymentInfo {
                name: "api-service".to_string(),
                namespace: "dev".to_string(),
                ready: "2/2".to_string(),
                up_to_date: 2,
                available: 2,
                age: Utc::now() - chrono::Duration::days(1),
                strategy: "RollingUpdate".to_string(),
            },
            DeploymentInfo {
                name: "web-frontend".to_string(),
                namespace: "prod".to_string(),
                ready: "5/5".to_string(),
                up_to_date: 5,
                available: 5,
                age: Utc::now() - chrono::Duration::days(7),
                strategy: "RollingUpdate".to_string(),
            },
        ];

        // Mock services
        self.services = vec![
            ServiceInfo {
                name: "kubernetes".to_string(),
                namespace: "default".to_string(),
                type_: "ClusterIP".to_string(),
                cluster_ip: "10.96.0.1".to_string(),
                external_ip: "<none>".to_string(),
                ports: vec![ServicePort {
                    name: "https".to_string(),
                    port: 443,
                    target_port: "6443".to_string(),
                    protocol: "TCP".to_string(),
                }],
                age: Utc::now() - chrono::Duration::days(30),
            },
            ServiceInfo {
                name: "nginx-service".to_string(),
                namespace: "default".to_string(),
                type_: "NodePort".to_string(),
                cluster_ip: "10.99.123.45".to_string(),
                external_ip: "<none>".to_string(),
                ports: vec![ServicePort {
                    name: "http".to_string(),
                    port: 80,
                    target_port: "80".to_string(),
                    protocol: "TCP".to_string(),
                }],
                age: Utc::now() - chrono::Duration::days(2),
            },
            ServiceInfo {
                name: "postgres".to_string(),
                namespace: "default".to_string(),
                type_: "ClusterIP".to_string(),
                cluster_ip: "10.99.200.1".to_string(),
                external_ip: "<none>".to_string(),
                ports: vec![ServicePort {
                    name: "postgresql".to_string(),
                    port: 5432,
                    target_port: "5432".to_string(),
                    protocol: "TCP".to_string(),
                }],
                age: Utc::now() - chrono::Duration::days(1),
            },
        ];
    }

    /// Render the Kubernetes panel
    pub fn render(&mut self, ctx: &egui::Context, show_panel: &mut bool) {
        if !*show_panel {
            return;
        }

        let theme = DesignTheme::dark();

        egui::SidePanel::left("kubernetes_panel")
            .width_range(500.0..=750.0)
            .default_width(600.0)
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
                    KubernetesTab::Pods => self.render_pods(ui, &theme),
                    KubernetesTab::Deployments => self.render_deployments(ui, &theme),
                    KubernetesTab::Services => self.render_services(ui, &theme),
                    KubernetesTab::ConfigMaps => self.render_configmaps(ui, &theme),
                    KubernetesTab::Secrets => self.render_secrets(ui, &theme),
                    KubernetesTab::Nodes => self.render_nodes(ui, &theme),
                    KubernetesTab::Namespaces => self.render_namespaces(ui, &theme),
                    KubernetesTab::Ingresses => self.render_ingresses(ui, &theme),
                }
            });

        // Render dialogs
        if self.show_create_dialog {
            self.render_create_dialog(ctx);
        }
    }

    fn render_header(&mut self, ui: &mut Ui, theme: &DesignTheme) {
        ui.horizontal(|ui| {
            ui.label(RichText::new("☸️").size(20.0));
            ui.heading(
                RichText::new("Kubernetes 管理")
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

        // Context and namespace selector
        ui.horizontal(|ui| {
            ui.label(RichText::new(format!("上下文: {}", self.current_context)).size(12.0));
            ui.add_space(20.0);
            ui.label("命名空间:");
            egui::ComboBox::from_id_source("namespace_selector")
                .selected_text(&self.selected_namespace)
                .width(120.0)
                .show_ui(ui, |ui| {
                    for ns in &self.namespaces {
                        ui.selectable_value(
                            &mut self.selected_namespace,
                            ns.name.clone(),
                            &ns.name,
                        );
                    }
                });
        });
    }

    fn render_tabs(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            let tabs = [
                KubernetesTab::Pods,
                KubernetesTab::Deployments,
                KubernetesTab::Services,
                KubernetesTab::ConfigMaps,
                KubernetesTab::Secrets,
                KubernetesTab::Nodes,
                KubernetesTab::Namespaces,
            ];

            for tab in tabs {
                let is_active = self.active_tab == tab;
                let text = format!("{} {}", tab.icon(), tab.display_name());

                let btn = egui::Button::new(RichText::new(text).size(11.0)).fill(if is_active {
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

    fn render_pods(&mut self, ui: &mut Ui, theme: &DesignTheme) {
        ui.horizontal(|ui| {
            ui.add(
                egui::TextEdit::singleline(&mut self.search_query)
                    .hint_text("🔍 搜索 Pod...")
                    .desired_width(200.0),
            );

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("➕ 创建 Pod").clicked() {
                    self.show_create_dialog = true;
                    self.new_resource_form = NewResourceForm::default();
                    self.new_resource_form.resource_type = "Pod".to_string();
                }
            });
        });

        ui.add_space(8.0);

        if self.is_loading {
            ui.label("加载中...");
            return;
        }

        if let Some(ref error) = self.error_message {
            ui.colored_label(egui::Color32::RED, format!("错误: {}", error));
        }

        egui::ScrollArea::vertical().show(ui, |ui| {
            for pod in &self.pods.clone() {
                if !self.search_query.is_empty() {
                    let search_lower = self.search_query.to_lowercase();
                    if !pod.name.to_lowercase().contains(&search_lower) {
                        continue;
                    }
                }
                if pod.namespace == self.selected_namespace || self.selected_namespace == "all" {
                    self.render_pod_item(ui, pod, theme);
                }
            }
        });
    }

    fn render_pod_item(&mut self, ui: &mut Ui, pod: &PodInfo, theme: &DesignTheme) {
        let is_selected = self.selected_pod.as_ref() == Some(&pod.name);

        ui.group(|ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new(pod.status.icon()).size(16.0));

                ui.vertical(|ui| {
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new(&pod.name)
                                .strong()
                                .color(theme.text_primary)
                                .size(13.0),
                        );
                        // Status badge
                        let status_text = format!(" {} ", pod.status.display_name());
                        ui.label(
                            RichText::new(status_text)
                                .size(9.0)
                                .background_color(pod.status.color())
                                .color(egui::Color32::WHITE),
                        );
                    });

                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new(format!(
                                "命名空间: {} | 节点: {}",
                                pod.namespace, pod.node
                            ))
                            .size(10.0)
                            .color(theme.text_secondary),
                        );
                    });

                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new(format!(
                                "就绪: {} | 重启: {} | IP: {}",
                                pod.ready, pod.restarts, pod.ip
                            ))
                            .size(10.0)
                            .color(theme.text_secondary),
                        );
                    });

                    // Containers
                    if !pod.containers.is_empty() {
                        ui.horizontal(|ui| {
                            ui.label(RichText::new("容器:").size(10.0));
                            for container in &pod.containers {
                                let ready_icon = if container.ready { "✅" } else { "❌" };
                                ui.label(
                                    RichText::new(format!(
                                        "{} {} ({})",
                                        ready_icon, container.name, container.image
                                    ))
                                    .size(10.0)
                                    .color(theme.text_secondary),
                                );
                            }
                        });
                    }
                });

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.horizontal(|ui| {
                        if ui.button("🗑️").on_hover_text("删除").clicked() {
                            self.delete_pod(&pod.name, &pod.namespace);
                        }
                        if ui.button("📜 日志").clicked() {
                            self.show_pod_logs(&pod.name, &pod.namespace);
                        }
                        if ui.button("🔍 详情").clicked() {
                            self.selected_pod = Some(pod.name.clone());
                        }
                        if pod.status == PodStatus::Running {
                            if ui.button("🐚 终端").clicked() {
                                self.exec_pod(&pod.name, &pod.namespace);
                            }
                        }
                    });
                });
            });
        });

        ui.add_space(4.0);
    }

    fn render_deployments(&mut self, ui: &mut Ui, theme: &DesignTheme) {
        ui.horizontal(|ui| {
            ui.add(
                egui::TextEdit::singleline(&mut self.search_query)
                    .hint_text("🔍 搜索 Deployment...")
                    .desired_width(200.0),
            );

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("➕ 创建 Deployment").clicked() {
                    self.show_create_dialog = true;
                    self.new_resource_form = NewResourceForm::default();
                    self.new_resource_form.resource_type = "Deployment".to_string();
                }
            });
        });

        ui.add_space(8.0);

        egui::ScrollArea::vertical().show(ui, |ui| {
            for deployment in &self.deployments.clone() {
                if !self.search_query.is_empty() {
                    let search_lower = self.search_query.to_lowercase();
                    if !deployment.name.to_lowercase().contains(&search_lower) {
                        continue;
                    }
                }
                if deployment.namespace == self.selected_namespace
                    || self.selected_namespace == "all"
                {
                    self.render_deployment_item(ui, deployment, theme);
                }
            }
        });
    }

    fn render_deployment_item(
        &mut self,
        ui: &mut Ui,
        deployment: &DeploymentInfo,
        theme: &DesignTheme,
    ) {
        ui.group(|ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new("🚀").size(16.0));

                ui.vertical(|ui| {
                    ui.label(
                        RichText::new(&deployment.name)
                            .strong()
                            .color(theme.text_primary),
                    );
                    ui.label(
                        RichText::new(format!(
                            "命名空间: {} | 策略: {}",
                            deployment.namespace, deployment.strategy
                        ))
                        .size(10.0)
                        .color(theme.text_secondary),
                    );
                    ui.label(
                        RichText::new(format!(
                            "就绪: {} | 最新: {} | 可用: {}",
                            deployment.ready, deployment.up_to_date, deployment.available
                        ))
                        .size(10.0)
                        .color(theme.text_secondary),
                    );
                });

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.horizontal(|ui| {
                        if ui.button("🗑️").on_hover_text("删除").clicked() {
                            self.delete_deployment(&deployment.name, &deployment.namespace);
                        }
                        if ui.button("⬆️ 扩容").clicked() {
                            // TODO: Scale deployment
                        }
                        if ui.button("🔄 重启").clicked() {
                            self.restart_deployment(&deployment.name, &deployment.namespace);
                        }
                    });
                });
            });
        });

        ui.add_space(4.0);
    }

    fn render_services(&mut self, ui: &mut Ui, theme: &DesignTheme) {
        ui.horizontal(|ui| {
            ui.add(
                egui::TextEdit::singleline(&mut self.search_query)
                    .hint_text("🔍 搜索 Service...")
                    .desired_width(200.0),
            );

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("➕ 创建 Service").clicked() {
                    self.show_create_dialog = true;
                    self.new_resource_form = NewResourceForm::default();
                    self.new_resource_form.resource_type = "Service".to_string();
                }
            });
        });

        ui.add_space(8.0);

        egui::ScrollArea::vertical().show(ui, |ui| {
            for service in &self.services.clone() {
                if !self.search_query.is_empty() {
                    let search_lower = self.search_query.to_lowercase();
                    if !service.name.to_lowercase().contains(&search_lower) {
                        continue;
                    }
                }
                if service.namespace == self.selected_namespace || self.selected_namespace == "all"
                {
                    self.render_service_item(ui, service, theme);
                }
            }
        });
    }

    fn render_service_item(&mut self, ui: &mut Ui, service: &ServiceInfo, theme: &DesignTheme) {
        ui.group(|ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new("🔌").size(16.0));

                ui.vertical(|ui| {
                    ui.label(
                        RichText::new(&service.name)
                            .strong()
                            .color(theme.text_primary),
                    );
                    ui.label(
                        RichText::new(format!(
                            "类型: {} | ClusterIP: {} | 外部IP: {}",
                            service.type_, service.cluster_ip, service.external_ip
                        ))
                        .size(10.0)
                        .color(theme.text_secondary),
                    );
                    let ports: Vec<String> = service
                        .ports
                        .iter()
                        .map(|p| {
                            format!("{}:{}/{} ({})", p.port, p.target_port, p.protocol, p.name)
                        })
                        .collect();
                    ui.label(
                        RichText::new(format!("端口: {}", ports.join(", ")))
                            .size(10.0)
                            .color(theme.text_secondary),
                    );
                });

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("🗑️").on_hover_text("删除").clicked() {
                        self.delete_service(&service.name, &service.namespace);
                    }
                    if ui.button("🔍 详情").clicked() {
                        // TODO: Show service details
                    }
                });
            });
        });

        ui.add_space(4.0);
    }

    fn render_configmaps(&mut self, ui: &mut Ui, theme: &DesignTheme) {
        ui.horizontal(|ui| {
            ui.heading("ConfigMaps");
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("➕ 创建").clicked() {
                    self.show_create_dialog = true;
                    self.new_resource_form = NewResourceForm::default();
                    self.new_resource_form.resource_type = "ConfigMap".to_string();
                }
            });
        });

        ui.add_space(16.0);

        // Placeholder
        ui.centered_and_justified(|ui| {
            ui.label("⚙️\n暂无 ConfigMap\n(Pro 功能)");
        });
    }

    fn render_secrets(&mut self, ui: &mut Ui, theme: &DesignTheme) {
        ui.horizontal(|ui| {
            ui.heading("Secrets");
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("➕ 创建").clicked() {
                    self.show_create_dialog = true;
                    self.new_resource_form = NewResourceForm::default();
                    self.new_resource_form.resource_type = "Secret".to_string();
                }
            });
        });

        ui.add_space(16.0);

        // Placeholder
        ui.centered_and_justified(|ui| {
            ui.label("🔐\n暂无 Secret\n(Pro 功能)");
        });
    }

    fn render_nodes(&mut self, ui: &mut Ui, theme: &DesignTheme) {
        ui.heading("集群节点");
        ui.add_space(8.0);

        egui::ScrollArea::vertical().show(ui, |ui| {
            for node in &self.nodes.clone() {
                self.render_node_item(ui, node, theme);
            }
        });
    }

    fn render_node_item(&mut self, ui: &mut Ui, node: &NodeInfo, theme: &DesignTheme) {
        ui.group(|ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new("🖥️").size(18.0));

                ui.vertical(|ui| {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(&node.name).strong().color(theme.text_primary));
                        let status_color = if node.status == "Ready" {
                            Color32::from_rgb(100, 200, 100)
                        } else {
                            Color32::from_rgb(220, 53, 69)
                        };
                        ui.label(
                            RichText::new(format!(" {} ", node.status))
                                .size(10.0)
                                .background_color(status_color)
                                .color(egui::Color32::WHITE),
                        );
                    });

                    ui.label(
                        RichText::new(format!("角色: {} | 版本: {}", node.role, node.version))
                            .size(11.0)
                            .color(theme.text_secondary),
                    );

                    ui.label(
                        RichText::new(format!(
                            "CPU: {} | 内存: {}",
                            node.cpu_capacity, node.memory_capacity
                        ))
                        .size(11.0)
                        .color(theme.text_secondary),
                    );

                    if let (Some(cpu), Some(mem)) = (node.cpu_usage, node.memory_usage) {
                        ui.label(
                            RichText::new(format!("使用率: CPU {:.1}% | 内存 {:.1}%", cpu, mem))
                                .size(10.0)
                                .color(Color32::from_rgb(100, 200, 100)),
                        );
                    }
                });

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("🔍 详情").clicked() {
                        // TODO: Show node details
                    }
                });
            });
        });

        ui.add_space(4.0);
    }

    fn render_namespaces(&mut self, ui: &mut Ui, theme: &DesignTheme) {
        ui.horizontal(|ui| {
            ui.heading("命名空间");
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("➕ 创建").clicked() {
                    self.show_create_dialog = true;
                    self.new_resource_form = NewResourceForm::default();
                    self.new_resource_form.resource_type = "Namespace".to_string();
                }
            });
        });

        ui.add_space(8.0);

        egui::ScrollArea::vertical().show(ui, |ui| {
            for ns in &self.namespaces.clone() {
                ui.group(|ui| {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("📁").size(16.0));

                        ui.vertical(|ui| {
                            ui.label(RichText::new(&ns.name).strong().color(theme.text_primary));
                            ui.label(
                                RichText::new(format!("状态: {}", ns.status))
                                    .size(11.0)
                                    .color(theme.text_secondary),
                            );
                        });

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ns.name != "default" && ns.name != "kube-system" {
                                if ui.button("🗑️").clicked() {
                                    self.delete_namespace(&ns.name);
                                }
                            }
                            if ui.button("选择").clicked() {
                                self.selected_namespace = ns.name.clone();
                                self.show_success(format!("切换到命名空间 {}", ns.name).as_str());
                            }
                        });
                    });
                });
                ui.add_space(4.0);
            }
        });
    }

    fn render_ingresses(&mut self, ui: &mut Ui, _theme: &DesignTheme) {
        ui.heading("Ingress 入口");
        ui.add_space(16.0);

        // Placeholder
        ui.centered_and_justified(|ui| {
            ui.label("🌐\n暂无 Ingress\n(Pro 功能)");
        });
    }

    fn render_create_dialog(&mut self, ctx: &egui::Context) {
        let mut open = true;
        egui::Window::new(format!("创建 {}", self.new_resource_form.resource_type))
            .collapsible(false)
            .resizable(false)
            .default_size([450.0, 500.0])
            .open(&mut open)
            .show(ctx, |ui| {
                ui.label("名称:");
                ui.text_edit_singleline(&mut self.new_resource_form.name);

                if self.new_resource_form.resource_type != "Namespace" {
                    ui.label("命名空间:");
                    ui.text_edit_singleline(&mut self.new_resource_form.namespace);
                }

                match self.new_resource_form.resource_type.as_str() {
                    "Deployment" | "Pod" => {
                        ui.label("镜像:");
                        ui.text_edit_singleline(&mut self.new_resource_form.image);

                        ui.label("副本数:");
                        ui.add(egui::DragValue::new(&mut self.new_resource_form.replicas).speed(1));

                        if self.new_resource_form.resource_type == "Pod" {
                            ui.label("端口:");
                            ui.add(egui::DragValue::new(&mut self.new_resource_form.port).speed(1));
                        }
                    }
                    "Service" => {
                        ui.label("端口:");
                        ui.add(egui::DragValue::new(&mut self.new_resource_form.port).speed(1));
                    }
                    _ => {}
                }

                ui.label("YAML 配置 (可选):");
                ui.add(
                    egui::TextEdit::multiline(&mut self.new_resource_form.yaml_content)
                        .code_editor()
                        .desired_rows(10)
                        .desired_width(f32::INFINITY),
                );

                ui.add_space(16.0);

                ui.horizontal(|ui| {
                    if ui.button("✅ 创建").clicked() {
                        self.create_resource();
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

    // Actions
    fn refresh_data(&mut self) {
        self.is_loading = true;
        self.last_refresh = Some(std::time::Instant::now());
        // TODO: Call backend API
        self.is_loading = false;
    }

    fn delete_pod(&mut self, name: &str, namespace: &str) {
        self.show_success(format!("删除 Pod {}/{}", namespace, name).as_str());
        self.pods
            .retain(|p| !(p.name == name && p.namespace == namespace));
    }

    fn show_pod_logs(&mut self, name: &str, namespace: &str) {
        self.selected_pod = Some(name.to_string());
        self.show_logs_panel = true;
        self.log_content = format!("显示 Pod {}/{} 的日志...\n", namespace, name);
        // TODO: Stream logs from backend
    }

    fn exec_pod(&mut self, name: &str, namespace: &str) {
        self.show_success(format!("在 Pod {}/{} 中启动终端", namespace, name).as_str());
        // TODO: Open terminal exec
    }

    fn delete_deployment(&mut self, name: &str, namespace: &str) {
        self.show_success(format!("删除 Deployment {}/{}", namespace, name).as_str());
        self.deployments
            .retain(|d| !(d.name == name && d.namespace == namespace));
    }

    fn restart_deployment(&mut self, name: &str, namespace: &str) {
        self.show_success(format!("重启 Deployment {}/{}", namespace, name).as_str());
        // TODO: Call backend API to restart
    }

    fn delete_service(&mut self, name: &str, namespace: &str) {
        self.show_success(format!("删除 Service {}/{}", namespace, name).as_str());
        self.services
            .retain(|s| !(s.name == name && s.namespace == namespace));
    }

    fn delete_namespace(&mut self, name: &str) {
        self.show_success(format!("删除命名空间 {}", name).as_str());
        self.namespaces.retain(|n| n.name != name);
    }

    fn create_resource(&mut self) {
        let resource_type = self.new_resource_form.resource_type.clone();
        self.show_success(format!("创建 {} 成功", resource_type).as_str());

        // Add mock resource
        match resource_type.as_str() {
            "Pod" => {
                let pod = PodInfo {
                    name: self.new_resource_form.name.clone(),
                    namespace: self.new_resource_form.namespace.clone(),
                    status: PodStatus::Pending,
                    ready: "0/1".to_string(),
                    restarts: 0,
                    age: Utc::now(),
                    ip: "<none>".to_string(),
                    node: "minikube".to_string(),
                    containers: vec![ContainerInfo {
                        name: "main".to_string(),
                        image: self.new_resource_form.image.clone(),
                        ready: false,
                        restart_count: 0,
                        state: "Pending".to_string(),
                    }],
                };
                self.pods.push(pod);
            }
            "Deployment" => {
                let deployment = DeploymentInfo {
                    name: self.new_resource_form.name.clone(),
                    namespace: self.new_resource_form.namespace.clone(),
                    ready: format!("0/{}", self.new_resource_form.replicas),
                    up_to_date: 0,
                    available: 0,
                    age: Utc::now(),
                    strategy: "RollingUpdate".to_string(),
                };
                self.deployments.push(deployment);
            }
            "Service" => {
                let service = ServiceInfo {
                    name: self.new_resource_form.name.clone(),
                    namespace: self.new_resource_form.namespace.clone(),
                    type_: "ClusterIP".to_string(),
                    cluster_ip: "10.99.x.x".to_string(),
                    external_ip: "<none>".to_string(),
                    ports: vec![ServicePort {
                        name: "http".to_string(),
                        port: self.new_resource_form.port,
                        target_port: self.new_resource_form.port.to_string(),
                        protocol: "TCP".to_string(),
                    }],
                    age: Utc::now(),
                };
                self.services.push(service);
            }
            "Namespace" => {
                let ns = NamespaceInfo {
                    name: self.new_resource_form.name.clone(),
                    status: "Active".to_string(),
                    created: Utc::now(),
                };
                self.namespaces.push(ns);
            }
            _ => {}
        }
    }

    fn show_success(&mut self, message: &str) {
        self.success_message = Some((message.to_string(), std::time::Instant::now()));
    }

    /// Update and clear expired messages
    pub fn update(&mut self) {
        if let Some((_, time)) = &self.success_message {
            if time.elapsed().as_secs() > 3 {
                self.success_message = None;
            }
        }

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

/// Render Kubernetes panel helper function
pub fn render_kubernetes_panel(
    ctx: &egui::Context,
    show_panel: &mut bool,
    manager: &mut KubernetesManagerUI,
) {
    manager.update();
    manager.render(ctx, show_panel);

    // Show success message as notification
    if let Some((message, _)) = &manager.success_message {
        egui::TopBottomPanel::top("kubernetes_notification")
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
