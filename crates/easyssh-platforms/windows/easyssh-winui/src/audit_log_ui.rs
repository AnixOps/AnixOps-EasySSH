#![allow(dead_code)]

use eframe::egui;
use egui::{Color32, Frame, RichText, ScrollArea, Ui, Vec2};
use std::sync::{Arc, Mutex};

use easyssh_core::audit::{
    AuditAction, AuditEntry, AuditFilter, AuditLogger, AuditResult, AuditTarget,
};

/// Audit Log UI Panel for viewing and filtering operation records
pub struct AuditLogPanel {
    /// Filter by action type
    filter_action: Option<AuditAction>,
    /// Filter by result
    filter_result: Option<AuditResult>,
    /// Search query for actor or target
    search_query: String,
    /// Date range filter
    date_from: Option<chrono::NaiveDate>,
    date_to: Option<chrono::NaiveDate>,
    /// Selected entry for detail view
    selected_entry: Option<AuditEntry>,
    /// Sort by field
    sort_by: SortBy,
    /// Sort descending
    sort_descending: bool,
    /// Current page for pagination
    current_page: usize,
    /// Items per page
    items_per_page: usize,
    /// Show detail panel
    show_details: bool,
    /// Export dialog open
    show_export_dialog: bool,
    /// Export format
    export_format: ExportFormat,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SortBy {
    Timestamp,
    Action,
    Actor,
    Result,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ExportFormat {
    Json,
    Csv,
}

impl AuditLogPanel {
    pub fn new() -> Self {
        Self {
            filter_action: None,
            filter_result: None,
            search_query: String::new(),
            date_from: None,
            date_to: None,
            selected_entry: None,
            sort_by: SortBy::Timestamp,
            sort_descending: true,
            current_page: 0,
            items_per_page: 50,
            show_details: true,
            show_export_dialog: false,
            export_format: ExportFormat::Json,
        }
    }

    pub fn ui(&mut self, ui: &mut Ui, audit_logger: &Arc<Mutex<AuditLogger>>) -> AuditLogResponse {
        let mut response = AuditLogResponse::default();

        // Header
        ui.horizontal(|ui| {
            ui.heading("📋 Audit Log");
            ui.label(
                RichText::new("Operation records and activity tracking")
                    .size(12.0)
                    .color(Color32::GRAY),
            );

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("📥 Export").clicked() {
                    self.show_export_dialog = true;
                }
                if ui.button("🔄 Refresh").clicked() {
                    response.refresh_requested = true;
                }
                if ui.button("Clear Filters").clicked() {
                    self.clear_filters();
                }
            });
        });

        ui.separator();

        // Filters
        self.render_filters(ui);

        ui.separator();

        // Main content: entry list + details
        let available_height = ui.available_height();
        let list_height = if self.show_details {
            available_height * 0.6
        } else {
            available_height
        };

        // Entry list
        ScrollArea::vertical()
            .max_height(list_height)
            .show(ui, |ui| {
                self.render_entry_list(ui, audit_logger);
            });

        // Details panel
        if self.show_details {
            ui.separator();
            self.render_details_panel(ui);
        }

        // Export dialog
        if self.show_export_dialog {
            self.render_export_dialog(ui, audit_logger, &mut response);
        }

        response
    }

    fn render_filters(&mut self, ui: &mut Ui) {
        ui.horizontal_wrapped(|ui| {
            // Action filter
            ui.label("Action:");
            let mut action_text = self
                .filter_action
                .map_or("All".to_string(), |a| format!("{:?}", a));
            egui::ComboBox::from_id_source("audit_action_filter")
                .selected_text(action_text)
                .width(120.0)
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.filter_action, None, "All");
                    ui.selectable_value(&mut self.filter_action, Some(AuditAction::Login), "Login");
                    ui.selectable_value(
                        &mut self.filter_action,
                        Some(AuditAction::Logout),
                        "Logout",
                    );
                    ui.selectable_value(
                        &mut self.filter_action,
                        Some(AuditAction::ServerConnect),
                        "Server Connect",
                    );
                    ui.selectable_value(
                        &mut self.filter_action,
                        Some(AuditAction::ServerCreate),
                        "Server Create",
                    );
                    ui.selectable_value(
                        &mut self.filter_action,
                        Some(AuditAction::ServerUpdate),
                        "Server Update",
                    );
                    ui.selectable_value(
                        &mut self.filter_action,
                        Some(AuditAction::ServerDelete),
                        "Server Delete",
                    );
                    ui.selectable_value(
                        &mut self.filter_action,
                        Some(AuditAction::KeyImport),
                        "Key Import",
                    );
                    ui.selectable_value(
                        &mut self.filter_action,
                        Some(AuditAction::WorkflowExecute),
                        "Workflow Execute",
                    );
                });

            ui.add_space(16.0);

            // Result filter
            ui.label("Result:");
            egui::ComboBox::from_id_source("audit_result_filter")
                .selected_text(
                    self.filter_result
                        .map_or("All".to_string(), |r| format!("{:?}", r)),
                )
                .width(100.0)
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.filter_result, None, "All");
                    ui.selectable_value(
                        &mut self.filter_result,
                        Some(AuditResult::Success),
                        "Success",
                    );
                    ui.selectable_value(
                        &mut self.filter_result,
                        Some(AuditResult::Failure),
                        "Failure",
                    );
                    ui.selectable_value(
                        &mut self.filter_result,
                        Some(AuditResult::Denied),
                        "Denied",
                    );
                });

            ui.add_space(16.0);

            // Search
            ui.label("Search:");
            ui.text_edit_singleline(&mut self.search_query);

            ui.add_space(16.0);

            // Sort
            ui.label("Sort by:");
            let sort_text = match self.sort_by {
                SortBy::Timestamp => "Time",
                SortBy::Action => "Action",
                SortBy::Actor => "Actor",
                SortBy::Result => "Result",
            };
            egui::ComboBox::from_id_source("audit_sort_by")
                .selected_text(sort_text)
                .width(80.0)
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.sort_by, SortBy::Timestamp, "Time");
                    ui.selectable_value(&mut self.sort_by, SortBy::Action, "Action");
                    ui.selectable_value(&mut self.sort_by, SortBy::Actor, "Actor");
                    ui.selectable_value(&mut self.sort_by, SortBy::Result, "Result");
                });

            if ui
                .button(if self.sort_descending { "↓" } else { "↑" })
                .clicked()
            {
                self.sort_descending = !self.sort_descending;
            }
        });
    }

    fn render_entry_list(&mut self, ui: &mut Ui, audit_logger: &Arc<Mutex<AuditLogger>>) {
        let logger = audit_logger.lock().unwrap();
        let entries: Vec<AuditEntry> = logger
            .get_all()
            .iter()
            .filter(|e| self.matches_filters(e))
            .cloned()
            .collect();
        drop(logger);

        if entries.is_empty() {
            ui.centered_and_justified(|ui| {
                ui.label(RichText::new("No audit entries found").color(Color32::GRAY));
            });
            return;
        }

        // Header row
        ui.horizontal(|ui| {
            ui.set_min_width(ui.available_width());
            ui.colored_label(Color32::GRAY, "Time");
            ui.add_space(120.0);
            ui.colored_label(Color32::GRAY, "Action");
            ui.add_space(150.0);
            ui.colored_label(Color32::GRAY, "Actor");
            ui.add_space(100.0);
            ui.colored_label(Color32::GRAY, "Target");
            ui.add_space(100.0);
            ui.colored_label(Color32::GRAY, "Result");
        });

        ui.separator();

        // Entry rows
        let total_entries = entries.len();
        let start_idx = self.current_page * self.items_per_page;
        let end_idx = ((start_idx + self.items_per_page).min(total_entries)) as usize;

        for (idx, entry) in entries
            .iter()
            .enumerate()
            .skip(start_idx)
            .take(end_idx - start_idx)
        {
            self.render_entry_row(ui, entry, idx);
        }

        // Pagination
        if total_entries > self.items_per_page {
            ui.separator();
            let total_pages = (total_entries + self.items_per_page - 1) / self.items_per_page;
            ui.horizontal(|ui| {
                ui.label(format!(
                    "Page {} of {} ({} entries)",
                    self.current_page + 1,
                    total_pages,
                    total_entries
                ));

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("→").clicked() && self.current_page < total_pages - 1 {
                        self.current_page += 1;
                    }
                    if ui.button("←").clicked() && self.current_page > 0 {
                        self.current_page -= 1;
                    }
                });
            });
        }
    }

    fn render_entry_row(&mut self, ui: &mut Ui, entry: &AuditEntry, _idx: usize) {
        let is_selected =
            self.selected_entry.as_ref().map(|e| e.timestamp) == Some(entry.timestamp);

        let bg_color = if is_selected {
            Color32::from_rgb(40, 60, 90)
        } else {
            Color32::from_gray(35)
        };

        let result_color = match entry.result {
            AuditResult::Success => Color32::GREEN,
            AuditResult::Failure => Color32::RED,
            AuditResult::Denied => Color32::YELLOW,
            _ => Color32::GRAY,
        };

        Frame::group(ui.style()).fill(bg_color).show(ui, |ui| {
            ui.set_min_width(ui.available_width());

            let row_response = ui.horizontal(|ui| {
                // Timestamp
                ui.label(
                    RichText::new(entry.timestamp.format("%Y-%m-%d %H:%M:%S").to_string())
                        .size(12.0)
                        .monospace(),
                );
                ui.add_space(20.0);

                // Action
                let action_text = format!("{:?}", entry.action);
                ui.label(RichText::new(&action_text).size(13.0).strong());
                ui.add_space(20.0);

                // Actor
                let actor_text = match &entry.actor {
                    easyssh_core::audit::Actor::User { user_id, .. } => user_id.clone(),
                    easyssh_core::audit::Actor::System { process_id, .. } => {
                        format!("system:{}", process_id)
                    }
                    easyssh_core::audit::Actor::Api { api_key_id, .. } => {
                        format!("api:{}", api_key_id)
                    }
                    easyssh_core::audit::Actor::Automation { workflow_id, .. } => {
                        format!("auto:{}", workflow_id)
                    }
                };
                ui.label(RichText::new(actor_text).size(12.0));
                ui.add_space(20.0);

                // Target
                let target_text = match &entry.target {
                    AuditTarget::Server(server_id) => format!("server:{}", server_id),
                    AuditTarget::User(user_id) => format!("user:{}", user_id),
                    AuditTarget::Team(team_id) => format!("team:{}", team_id),
                    AuditTarget::Workflow(workflow_id) => format!("workflow:{}", workflow_id),
                    AuditTarget::Resource(res_id) => format!("resource:{}", res_id),
                    AuditTarget::System => "system".to_string(),
                    _ => "unknown".to_string(),
                };
                ui.label(
                    RichText::new(target_text)
                        .size(12.0)
                        .color(Color32::LIGHT_GRAY),
                );
                ui.add_space(20.0);

                // Result indicator
                let result_icon = match entry.result {
                    AuditResult::Success => "✓",
                    AuditResult::Failure => "✗",
                    AuditResult::Denied => "⊘",
                    _ => "○",
                };
                ui.colored_label(result_color, RichText::new(result_icon).size(14.0).strong());
            });

            if row_response.response.clicked() {
                self.selected_entry = Some(entry.clone());
            }
        });

        ui.add_space(2.0);
    }

    fn render_details_panel(&mut self, ui: &mut Ui) {
        if let Some(ref entry) = self.selected_entry {
            Frame::group(ui.style()).show(ui, |ui| {
                ui.set_min_width(ui.available_width());

                ui.heading("Entry Details");
                ui.separator();

                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.label(RichText::new("Timestamp:").strong());
                        ui.label(entry.timestamp.format("%Y-%m-%d %H:%M:%S UTC").to_string());
                    });

                    ui.add_space(40.0);

                    ui.vertical(|ui| {
                        ui.label(RichText::new("Action:").strong());
                        ui.label(format!("{:?}", entry.action));
                    });

                    ui.add_space(40.0);

                    ui.vertical(|ui| {
                        ui.label(RichText::new("Result:").strong());
                        let (result_text, result_color) = match entry.result {
                            AuditResult::Success => ("Success", Color32::GREEN),
                            AuditResult::Failure => ("Failure", Color32::RED),
                            AuditResult::Denied => ("Denied", Color32::YELLOW),
                            _ => ("Unknown", Color32::GRAY),
                        };
                        ui.colored_label(result_color, result_text);
                    });
                });

                ui.add_space(10.0);

                // Actor details
                ui.label(RichText::new("Actor:").strong());
                match &entry.actor {
                    easyssh_core::audit::Actor::User {
                        user_id,
                        username,
                        ip_address,
                        ..
                    } => {
                        ui.label(format!("  Type: User"));
                        ui.label(format!("  ID: {}", user_id));
                        ui.label(format!(
                            "  Username: {}",
                            username.as_deref().unwrap_or("N/A")
                        ));
                        ui.label(format!("  IP: {}", ip_address.as_deref().unwrap_or("N/A")));
                    }
                    easyssh_core::audit::Actor::System { process_id, .. } => {
                        ui.label(format!("  Type: System"));
                        ui.label(format!("  Process ID: {}", process_id));
                    }
                    easyssh_core::audit::Actor::Api { api_key_id, .. } => {
                        ui.label(format!("  Type: API"));
                        ui.label(format!("  API Key ID: {}", api_key_id));
                    }
                    easyssh_core::audit::Actor::Automation { workflow_id, .. } => {
                        ui.label(format!("  Type: Automation"));
                        ui.label(format!("  Workflow ID: {}", workflow_id));
                    }
                }

                ui.add_space(10.0);

                // Target details
                ui.label(RichText::new("Target:").strong());
                match &entry.target {
                    AuditTarget::Server(id) => ui.label(format!("  Server: {}", id)),
                    AuditTarget::User(id) => ui.label(format!("  User: {}", id)),
                    AuditTarget::Team(id) => ui.label(format!("  Team: {}", id)),
                    AuditTarget::Workflow(id) => ui.label(format!("  Workflow: {}", id)),
                    AuditTarget::Resource(id) => ui.label(format!("  Resource: {}", id)),
                    AuditTarget::System => ui.label("  System"),
                    _ => ui.label("  Unknown"),
                };

                // Details
                if let Some(ref details) = entry.details {
                    ui.add_space(10.0);
                    ui.label(RichText::new("Details:").strong());
                    ui.monospace(details);
                }
            });
        } else {
            ui.centered_and_justified(|ui| {
                ui.label(RichText::new("Select an entry to view details").color(Color32::GRAY));
            });
        }
    }

    fn render_export_dialog(
        &mut self,
        ui: &mut Ui,
        audit_logger: &Arc<Mutex<AuditLogger>>,
        response: &mut AuditLogResponse,
    ) {
        let id = ui.make_persistent_id("audit_export_dialog");
        egui::Window::new("Export Audit Log")
            .id(id)
            .collapsible(false)
            .resizable(false)
            .default_size([400.0, 200.0])
            .show(ui.ctx(), |ui| {
                ui.label("Export Format:");
                ui.radio_value(&mut self.export_format, ExportFormat::Json, "JSON");
                ui.radio_value(&mut self.export_format, ExportFormat::Csv, "CSV");

                ui.separator();

                let logger = audit_logger.lock().unwrap();
                let entry_count = logger.get_all().len();
                drop(logger);

                ui.label(format!("Will export {} entries", entry_count));

                ui.separator();

                ui.horizontal(|ui| {
                    if ui.button("Export").clicked() {
                        response.export_requested = Some(self.export_format);
                        self.show_export_dialog = false;
                    }
                    if ui.button("Cancel").clicked() {
                        self.show_export_dialog = false;
                    }
                });
            });
    }

    fn matches_filters(&self, entry: &AuditEntry) -> bool {
        // Action filter
        if let Some(action) = self.filter_action {
            if entry.action != action {
                return false;
            }
        }

        // Result filter
        if let Some(result) = self.filter_result {
            if entry.result != result {
                return false;
            }
        }

        // Search filter
        if !self.search_query.is_empty() {
            let query = self.search_query.to_lowercase();
            let actor_text = match &entry.actor {
                easyssh_core::audit::Actor::User { user_id, .. } => user_id.to_lowercase(),
                easyssh_core::audit::Actor::System { process_id, .. } => process_id.to_lowercase(),
                easyssh_core::audit::Actor::Api { api_key_id, .. } => api_key_id.to_lowercase(),
                easyssh_core::audit::Actor::Automation { workflow_id, .. } => {
                    workflow_id.to_lowercase()
                }
            };

            let target_text = match &entry.target {
                AuditTarget::Server(id) => id.to_lowercase(),
                AuditTarget::User(id) => id.to_lowercase(),
                AuditTarget::Team(id) => id.to_lowercase(),
                AuditTarget::Workflow(id) => id.to_lowercase(),
                AuditTarget::Resource(id) => id.to_lowercase(),
                _ => String::new(),
            };

            if !actor_text.contains(&query) && !target_text.contains(&query) {
                return false;
            }
        }

        // Date filters would go here

        true
    }

    fn clear_filters(&mut self) {
        self.filter_action = None;
        self.filter_result = None;
        self.search_query.clear();
        self.date_from = None;
        self.date_to = None;
        self.current_page = 0;
    }

    pub fn set_selected_entry(&mut self, entry: Option<AuditEntry>) {
        self.selected_entry = entry;
    }

    pub fn refresh(&mut self) {
        self.current_page = 0;
    }
}

/// Response from audit log panel
#[derive(Debug, Default)]
pub struct AuditLogResponse {
    pub refresh_requested: bool,
    pub export_requested: Option<ExportFormat>,
}

/// Simple audit log viewer window
pub struct AuditLogWindow {
    pub open: bool,
    panel: AuditLogPanel,
}

impl AuditLogWindow {
    pub fn new() -> Self {
        Self {
            open: false,
            panel: AuditLogPanel::new(),
        }
    }

    pub fn render(&mut self, ctx: &egui::Context, audit_logger: &Arc<Mutex<AuditLogger>>) {
        if !self.open {
            return;
        }

        egui::Window::new("Audit Log")
            .open(&mut self.open)
            .collapsible(true)
            .resizable(true)
            .default_size([900.0, 600.0])
            .frame(egui::Frame {
                fill: egui::Color32::from_rgb(30, 30, 30),
                stroke: egui::Stroke::new(1.0, egui::Color32::from_rgb(60, 70, 85)),
                ..Default::default()
            })
            .show(ctx, |ui| {
                self.panel.ui(ui, audit_logger);
            });
    }
}
