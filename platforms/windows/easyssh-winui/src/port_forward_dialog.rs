#![allow(dead_code)]

//! Port Forwarding Dialog for Windows UI
//!
//! Visual port forwarding management with topology visualization,
//! traffic monitoring, and rule templates.

use crate::apple_design::{AppleButton, AppleCard, ButtonSize, ButtonStyle};
use crate::viewmodels::port_forward::{
    CreateForwardRuleRequest, ForwardRuleDto, ForwardRuleTemplateDto, ForwardTopologyDto,
    PortForwardViewModel, TrafficStatsDto,
};
use eframe::egui;
use std::collections::HashMap;

/// Port forwarding dialog state
#[derive(Default)]
pub struct PortForwardDialog {
    pub show: bool,
    pub server_id: Option<String>,
    pub server_name: Option<String>,

    // Tabs
    pub active_tab: PortForwardTab,

    // Rules list
    pub rules: Vec<ForwardRuleDto>,
    pub selected_rule: Option<String>,

    // Templates
    pub templates: Vec<ForwardRuleTemplateDto>,
    pub selected_template: Option<String>,

    // Create rule form
    pub show_create_form: bool,
    pub new_rule_name: String,
    pub new_rule_type: ForwardType,
    pub new_rule_local_addr: String,
    pub new_rule_remote_host: String,
    pub new_rule_remote_port: String,
    pub new_rule_auto_reconnect: bool,
    pub new_rule_browser_url: String,
    pub new_rule_notes: String,

    // Topology
    pub topology: Option<ForwardTopologyDto>,
    pub show_topology: bool,

    // Traffic stats refresh
    pub last_stats_refresh: Option<std::time::Instant>,
    pub stats_cache: HashMap<String, TrafficStatsDto>,

    // Status message
    pub status_message: Option<(String, egui::Color32, std::time::Instant)>,
}

#[derive(Default, Clone, Copy, PartialEq)]
pub enum PortForwardTab {
    #[default]
    Rules,
    Templates,
    Topology,
    Traffic,
}

#[derive(Default, Clone, Copy, PartialEq)]
pub enum ForwardType {
    #[default]
    Local,
    Remote,
    Dynamic,
}

impl PortForwardDialog {
    /// Create new port forward dialog
    pub fn new() -> Self {
        Self {
            show: false,
            server_id: None,
            server_name: None,
            active_tab: PortForwardTab::Rules,
            rules: Vec::new(),
            selected_rule: None,
            templates: Vec::new(),
            selected_template: None,
            show_create_form: false,
            new_rule_name: String::new(),
            new_rule_type: ForwardType::Local,
            new_rule_local_addr: "127.0.0.1:8080".to_string(),
            new_rule_remote_host: "localhost".to_string(),
            new_rule_remote_port: "80".to_string(),
            new_rule_auto_reconnect: true,
            new_rule_browser_url: String::new(),
            new_rule_notes: String::new(),
            topology: None,
            show_topology: false,
            last_stats_refresh: None,
            stats_cache: HashMap::new(),
            status_message: None,
        }
    }

    /// Open dialog for a specific server
    pub fn open(&mut self, server_id: String, server_name: String) {
        self.show = true;
        self.server_id = Some(server_id);
        self.server_name = Some(server_name);
        self.active_tab = PortForwardTab::Rules;
        self.refresh_rules();
    }

    /// Refresh rules list
    pub fn refresh_rules(&mut self) {
        // In a real implementation, this would fetch from the viewmodel
        // For now, we'll leave it empty and populate when connected
    }

    /// Load templates
    pub fn load_templates(&mut self, vm: &PortForwardViewModel) {
        self.templates = vm.get_templates();
    }

    /// Show status message
    pub fn show_status(&mut self, message: &str, color: egui::Color32) {
        self.status_message = Some((message.to_string(), color, std::time::Instant::now()));
    }

    /// Render the dialog
    pub fn render(&mut self, ctx: &egui::Context, vm: &PortForwardViewModel) {
        if !self.show {
            return;
        }

        let title = format!(
            "Port Forwarding - {}",
            self.server_name.as_deref().unwrap_or("Unknown")
        );

        let window = egui::Window::new(&title)
            .id(egui::Id::new("port_forward_dialog"))
            .resizable(true)
            .default_size([800.0, 600.0])
            .collapsible(false);

        window.show(ctx, |ui| {
            self.render_content(ui, vm);
        });

        // Clear status message after 3 seconds
        if let Some((_, _, time)) = &self.status_message {
            if time.elapsed().as_secs() > 3 {
                self.status_message = None;
            }
        }
    }

    /// Render dialog content
    fn render_content(&mut self, ui: &mut egui::Ui, vm: &PortForwardViewModel) {
        // Status bar
        if let Some((msg, color, _)) = &self.status_message {
            ui.horizontal(|ui| {
                ui.colored_label(*color, msg);
            });
            ui.separator();
        }

        // Tab bar
        ui.horizontal(|ui| {
            let tabs = [
                (PortForwardTab::Rules, "Rules", "list"),
                (PortForwardTab::Templates, "Templates", "template"),
                (PortForwardTab::Topology, "Topology", "network"),
                (PortForwardTab::Traffic, "Traffic", "chart"),
            ];

            for (tab, label, _icon) in &tabs {
                let selected = self.active_tab == *tab;
                let button = if selected {
                    egui::Button::new(*label).fill(egui::Color32::from_rgb(0, 122, 255))
                } else {
                    egui::Button::new(*label)
                };

                if ui.add(button).clicked() {
                    self.active_tab = *tab;
                    if *tab == PortForwardTab::Templates {
                        self.load_templates(vm);
                    } else if *tab == PortForwardTab::Topology {
                        self.topology = Some(vm.get_topology());
                    }
                }
            }

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("Close").clicked() {
                    self.show = false;
                }
            });
        });

        ui.separator();

        // Tab content
        match self.active_tab {
            PortForwardTab::Rules => self.render_rules_tab(ui, vm),
            PortForwardTab::Templates => self.render_templates_tab(ui, vm),
            PortForwardTab::Topology => self.render_topology_tab(ui),
            PortForwardTab::Traffic => self.render_traffic_tab(ui, vm),
        }
    }

    /// Render rules tab
    fn render_rules_tab(&mut self, ui: &mut egui::Ui, vm: &PortForwardViewModel) {
        // Toolbar
        let theme = crate::design::DesignTheme::light();
        ui.horizontal(|ui| {
            if AppleButton::new(&theme, "+ New Rule")
                .style(ButtonStyle::Primary)
                .size(ButtonSize::Small)
                .show(ui)
                .clicked()
            {
                self.show_create_form = true;
                self.reset_form();
            }

            ui.add_space(16.0);

            if AppleButton::new(&theme, "Refresh")
                .style(ButtonStyle::Secondary)
                .size(ButtonSize::Small)
                .show(ui)
                .clicked()
            {
                self.refresh_rules();
                self.show_status("Rules refreshed", egui::Color32::GREEN);
            }

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let active_count = self.rules.iter().filter(|r| r.status == "Active").count();
                ui.label(format!(
                    "{} active / {} total",
                    active_count,
                    self.rules.len()
                ));
            });
        });

        ui.separator();

        // Create form
        if self.show_create_form {
            self.render_create_form(ui, vm);
            ui.separator();
        }

        // Rules list
        if self.rules.is_empty() {
            ui.vertical_centered(|ui| {
                ui.add_space(50.0);
                ui.label(
                    egui::RichText::new("No forwarding rules configured")
                        .size(16.0)
                        .color(ui.visuals().weak_text_color()),
                );
                ui.label("Click 'New Rule' to create one or select a template");
            });
        } else {
            egui::ScrollArea::vertical().show(ui, |ui| {
                for rule in &self.rules.clone() {
                    self.render_rule_card(ui, vm, rule);
                }
            });
        }
    }

    /// Render create form
    fn render_create_form(&mut self, _ui: &mut egui::Ui, _vm: &PortForwardViewModel) {
        let theme = crate::design::DesignTheme::light();
        AppleCard::new(&theme, |ui| {
            ui.heading("Create New Forwarding Rule");
            ui.add_space(16.0);

            // Rule name
            ui.horizontal(|ui| {
                ui.label("Name:");
                ui.text_edit_singleline(&mut self.new_rule_name);
            });

            ui.add_space(8.0);

            // Forward type
            ui.horizontal(|ui| {
                ui.label("Type:");
                ui.radio_value(&mut self.new_rule_type, ForwardType::Local, "Local (-L)");
                ui.radio_value(&mut self.new_rule_type, ForwardType::Remote, "Remote (-R)");
                ui.radio_value(
                    &mut self.new_rule_type,
                    ForwardType::Dynamic,
                    "Dynamic/SOCKS (-D)",
                );
            });

            ui.add_space(8.0);

            // Local address
            ui.horizontal(|ui| {
                ui.label("Local Address:");
                ui.text_edit_singleline(&mut self.new_rule_local_addr);
                ui.label("e.g., 127.0.0.1:8080");
            });

            ui.add_space(8.0);

            // Remote address (for local/remote types)
            if self.new_rule_type != ForwardType::Dynamic {
                ui.horizontal(|ui| {
                    ui.label("Remote Host:");
                    ui.text_edit_singleline(&mut self.new_rule_remote_host);
                    ui.label("Port:");
                    ui.text_edit_singleline(&mut self.new_rule_remote_port);
                });
                ui.add_space(8.0);
            }

            // Auto-reconnect
            ui.checkbox(
                &mut self.new_rule_auto_reconnect,
                "Auto-reconnect on failure",
            );

            ui.add_space(8.0);

            // Browser URL
            ui.horizontal(|ui| {
                ui.label("Auto-open URL:");
                ui.text_edit_singleline(&mut self.new_rule_browser_url);
            });

            ui.add_space(8.0);

            // Notes
            ui.horizontal(|ui| {
                ui.label("Notes:");
                ui.text_edit_singleline(&mut self.new_rule_notes);
            });

            ui.add_space(16.0);

            // Buttons
            let theme = crate::design::DesignTheme::light();
            ui.horizontal(|ui| {
                if AppleButton::new(&theme, "Create")
                    .style(ButtonStyle::Primary)
                    .size(ButtonSize::Small)
                    .show(ui)
                    .clicked()
                {
                    self.create_rule_from_form(_vm);
                }

                if AppleButton::new(&theme, "Cancel")
                    .style(ButtonStyle::Secondary)
                    .size(ButtonSize::Small)
                    .show(ui)
                    .clicked()
                {
                    self.show_create_form = false;
                }
            });
        });
    }

    /// Render a rule card
    fn render_rule_card(
        &mut self,
        ui: &mut egui::Ui,
        vm: &PortForwardViewModel,
        rule: &ForwardRuleDto,
    ) {
        let _is_selected = self.selected_rule.as_ref() == Some(&rule.id);
        let theme = crate::design::DesignTheme::light();

        // Build card content
        AppleCard::new(&theme, |ui| {
            ui.horizontal(|ui| {
                // Status indicator
                let (status_color, status_icon) = match rule.status.as_str() {
                    "Active" => (egui::Color32::GREEN, "●"),
                    "Starting" => (egui::Color32::YELLOW, "◐"),
                    "Reconnecting" => (egui::Color32::YELLOW, "↻"),
                    "Error" => (egui::Color32::RED, "✗"),
                    _ => (egui::Color32::GRAY, "○"),
                };

                ui.colored_label(status_color, status_icon);
                ui.add_space(8.0);

                // Rule info
                ui.vertical(|ui| {
                    ui.label(egui::RichText::new(&rule.name).strong());
                    ui.label(format!(
                        "{} → {}",
                        rule.forward_type_display, rule.local_addr
                    ));
                    if let Some(ref remote) = rule.remote_addr {
                        ui.label(format!("→ {}", remote));
                    }
                });

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    // Action buttons
                    if rule.status == "Active" {
                        if AppleButton::new(&theme, "Stop")
                            .style(ButtonStyle::Destructive)
                            .size(ButtonSize::Small)
                            .show(ui)
                            .clicked()
                        {
                            if let Err(e) = vm.stop_forward(&rule.id) {
                                self.show_status(
                                    &format!("Stop failed: {}", e),
                                    egui::Color32::RED,
                                );
                            } else {
                                self.show_status("Rule stopped", egui::Color32::GREEN);
                                self.refresh_rules();
                            }
                        }
                    } else if AppleButton::new(&theme, "Start")
                        .style(ButtonStyle::Primary)
                        .size(ButtonSize::Small)
                        .show(ui)
                        .clicked()
                    {
                        // Start the forward rule
                        // This would need the current session_id
                        self.show_status("Starting rule...", egui::Color32::YELLOW);
                    }

                    if AppleButton::new(&theme, "Delete")
                        .style(ButtonStyle::Secondary)
                        .size(ButtonSize::Small)
                        .show(ui)
                        .clicked()
                    {
                        // Remove from list (would delete from DB in real impl)
                        self.rules.retain(|r| r.id != rule.id);
                        self.show_status("Rule deleted", egui::Color32::GREEN);
                    }
                });
            });

            // Traffic stats if active
            if rule.status == "Active" {
                ui.separator();
                ui.horizontal(|ui| {
                    ui.label(format!(
                        "Sent: {} | Received: {} | Connections: {}",
                        rule.traffic.bytes_sent_formatted,
                        rule.traffic.bytes_received_formatted,
                        rule.traffic.connections_active
                    ));

                    if let Some(ref url) = rule.browser_url {
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.button("🌐 Open").clicked() {
                                let _ = vm.open_browser(url);
                            }
                        });
                    }
                });
            }
        })
        .show(ui);

        ui.add_space(8.0);
    }

    /// Render templates tab
    fn render_templates_tab(&mut self, ui: &mut egui::Ui, vm: &PortForwardViewModel) {
        ui.heading("Rule Templates");
        ui.label("Click a template to create a new forwarding rule:");
        ui.add_space(16.0);

        // Collect templates by category first
        let categories: Vec<(String, Vec<ForwardRuleTemplateDto>)>;
        {
            let mut by_category: HashMap<String, Vec<ForwardRuleTemplateDto>> = HashMap::new();
            for template in &self.templates {
                by_category
                    .entry(template.category.clone())
                    .or_default()
                    .push(template.clone());
            }
            categories = by_category.into_iter().collect();
        }

        egui::ScrollArea::vertical().show(ui, |ui| {
            for (category, templates) in categories {
                ui.collapsing(category, |ui| {
                    for template in templates {
                        self.render_template_card(ui, vm, &template);
                    }
                });
                ui.add_space(8.0);
            }
        });
    }

    /// Render a template card
    fn render_template_card(
        &mut self,
        ui: &mut egui::Ui,
        vm: &PortForwardViewModel,
        template: &ForwardRuleTemplateDto,
    ) {
        let theme = crate::design::DesignTheme::light();
        AppleCard::new(&theme, |ui| {
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.label(egui::RichText::new(&template.name).strong());
                    ui.label(&template.description);
                    ui.label(format!(
                        "Local: {} | Remote: {}",
                        template.local_addr_pattern,
                        template.remote_addr_pattern.as_deref().unwrap_or("N/A")
                    ));
                });

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if AppleButton::new(&theme, "Use")
                        .style(ButtonStyle::Primary)
                        .size(ButtonSize::Small)
                        .show(ui)
                        .clicked()
                    {
                        if let Some(server_id) = &self.server_id {
                            match vm.create_from_template(&template.id, server_id, None) {
                                Ok(rule) => {
                                    self.rules.push(rule);
                                    self.show_status(
                                        "Rule created from template",
                                        egui::Color32::GREEN,
                                    );
                                }
                                Err(e) => {
                                    self.show_status(
                                        &format!("Failed to create rule: {}", e),
                                        egui::Color32::RED,
                                    );
                                }
                            }
                        }
                    }
                });
            });
        })
        .show(ui);

        ui.add_space(8.0);
    }

    /// Render topology tab
    fn render_topology_tab(&mut self, ui: &mut egui::Ui) {
        ui.heading("Forwarding Topology");
        ui.label("Visual representation of active forwarding rules:");
        ui.add_space(16.0);

        if let Some(ref topology) = self.topology {
            // Simple visualization using cards
            egui::ScrollArea::vertical().show(ui, |ui| {
                // Draw nodes
                for node in &topology.nodes {
                    self.render_topology_node(ui, node);
                }

                ui.add_space(16.0);

                // Draw edges
                ui.heading("Connections");
                for edge in &topology.edges {
                    self.render_topology_edge(ui, edge);
                }
            });
        } else {
            ui.vertical_centered(|ui| {
                ui.add_space(50.0);
                ui.label("No active forwarding rules to display");
                ui.label("Start a forwarding rule to see the topology");
            });
        }
    }

    /// Render topology node
    fn render_topology_node(
        &self,
        ui: &mut egui::Ui,
        node: &crate::viewmodels::port_forward::TopologyNodeDto,
    ) {
        let color = match node.node_type.as_str() {
            "Local" => egui::Color32::from_rgb(0, 122, 255),
            "Server" => egui::Color32::from_rgb(52, 199, 89),
            "Target" => egui::Color32::from_rgb(255, 149, 0),
            _ => egui::Color32::GRAY,
        };

        ui.horizontal(|ui| {
            ui.colored_label(color, "●");
            ui.add_space(8.0);
            ui.label(egui::RichText::new(&node.label).strong());
            ui.label(format!("({})", node.address));
        });
    }

    /// Render topology edge
    fn render_topology_edge(
        &self,
        ui: &mut egui::Ui,
        edge: &crate::viewmodels::port_forward::TopologyEdgeDto,
    ) {
        let theme = crate::design::DesignTheme::light();
        AppleCard::new(&theme, |ui| {
            ui.horizontal(|ui| {
                ui.label(&edge.from);
                ui.label("→");
                ui.label(&edge.to);

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(&edge.label);
                });
            });

            if edge.bytes_sent > 0 || edge.bytes_received > 0 {
                ui.separator();
                ui.label(format!(
                    "Traffic: {} sent, {} received, {} active connections",
                    format_bytes(edge.bytes_sent),
                    format_bytes(edge.bytes_received),
                    edge.connections_active
                ));
            }
        })
        .show(ui);

        ui.add_space(4.0);
    }

    /// Render traffic tab
    fn render_traffic_tab(&mut self, ui: &mut egui::Ui, vm: &PortForwardViewModel) {
        ui.heading("Traffic Statistics");
        ui.label("Real-time traffic monitoring for active forwarding rules:");
        ui.add_space(16.0);

        let active_rules: Vec<_> = self.rules.iter().filter(|r| r.status == "Active").collect();

        if active_rules.is_empty() {
            ui.vertical_centered(|ui| {
                ui.add_space(50.0);
                ui.label("No active rules to monitor");
            });
        } else {
            let theme = crate::design::DesignTheme::light();
            egui::ScrollArea::vertical().show(ui, |ui| {
                for rule in active_rules {
                    // Get fresh stats
                    let stats = vm.get_rule_traffic(&rule.id);

                    AppleCard::new(&theme, |ui| {
                        ui.label(egui::RichText::new(&rule.name).strong());
                        ui.separator();

                        // Traffic bars
                        ui.label(format!(
                            "Sent: {} ({} bytes)",
                            stats.bytes_sent_formatted, stats.bytes_sent
                        ));
                        ui.label(format!(
                            "Received: {} ({} bytes)",
                            stats.bytes_received_formatted, stats.bytes_received
                        ));
                        ui.label(format!(
                            "Connections: {} active / {} total",
                            stats.connections_active, stats.connections_total
                        ));
                        ui.label(format!("Errors: {}", stats.errors_total));
                    })
                    .show(ui);

                    ui.add_space(8.0);
                }
            });
        }
    }

    /// Reset form fields
    fn reset_form(&mut self) {
        self.new_rule_name.clear();
        self.new_rule_type = ForwardType::Local;
        self.new_rule_local_addr = "127.0.0.1:8080".to_string();
        self.new_rule_remote_host = "localhost".to_string();
        self.new_rule_remote_port = "80".to_string();
        self.new_rule_auto_reconnect = true;
        self.new_rule_browser_url.clear();
        self.new_rule_notes.clear();
    }

    /// Create rule from form data
    fn create_rule_from_form(&mut self, vm: &PortForwardViewModel) {
        let server_id = match &self.server_id {
            Some(id) => id.clone(),
            None => {
                self.show_status("No server selected", egui::Color32::RED);
                return;
            }
        };

        let forward_type = match self.new_rule_type {
            ForwardType::Local => "local".to_string(),
            ForwardType::Remote => "remote".to_string(),
            ForwardType::Dynamic => "dynamic".to_string(),
        };

        let request = CreateForwardRuleRequest {
            name: self.new_rule_name.clone(),
            server_id,
            forward_type,
            local_addr: self.new_rule_local_addr.clone(),
            remote_host: if self.new_rule_type != ForwardType::Dynamic {
                Some(self.new_rule_remote_host.clone())
            } else {
                None
            },
            remote_port: if self.new_rule_type != ForwardType::Dynamic {
                self.new_rule_remote_port.parse().ok()
            } else {
                None
            },
            auto_reconnect: self.new_rule_auto_reconnect,
            browser_url: if self.new_rule_browser_url.is_empty() {
                None
            } else {
                Some(self.new_rule_browser_url.clone())
            },
            notes: if self.new_rule_notes.is_empty() {
                None
            } else {
                Some(self.new_rule_notes.clone())
            },
        };

        match vm.create_rule(request) {
            Ok(rule) => {
                self.rules.push(rule);
                self.show_create_form = false;
                self.show_status("Rule created successfully", egui::Color32::GREEN);
            }
            Err(e) => {
                self.show_status(&format!("Failed to create rule: {}", e), egui::Color32::RED);
            }
        }
    }
}

/// Format bytes to human-readable
fn format_bytes(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    let mut size = bytes as f64;
    let mut unit_index = 0;

    while size >= 1024.0 && unit_index < UNITS.len() - 1 {
        size /= 1024.0;
        unit_index += 1;
    }

    format!("{:.2} {}", size, UNITS[unit_index])
}
