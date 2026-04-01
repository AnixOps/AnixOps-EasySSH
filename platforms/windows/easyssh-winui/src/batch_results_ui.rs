#![allow(dead_code)]

use eframe::egui;
use egui::{Color32, Frame, ProgressBar, RichText, Ui};
use std::collections::HashMap;

use easyssh_core::workflow_executor::{BatchExecutionSummary, ServerExecutionResult};
use easyssh_core::workflow_engine::StepStatus;

/// Batch execution results viewer
pub struct BatchExecutionResultsPanel {
    /// Current execution summary
    current_summary: Option<BatchExecutionSummary>,
    /// Selected server for detail view
    selected_server: Option<String>,
    /// Selected step for detail view
    selected_step: Option<String>,
    /// Show only failed results
    show_only_failed: bool,
    /// Auto-refresh interval
    auto_refresh: bool,
    /// View mode: summary, details, or log
    view_mode: ResultsViewMode,
    /// Expanded servers
    expanded_servers: HashMap<String, bool>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ResultsViewMode {
    Summary,
    Details,
    Logs,
}

impl BatchExecutionResultsPanel {
    pub fn new() -> Self {
        Self {
            current_summary: None,
            selected_server: None,
            selected_step: None,
            show_only_failed: false,
            auto_refresh: false,
            view_mode: ResultsViewMode::Summary,
            expanded_servers: HashMap::new(),
        }
    }

    pub fn with_summary(summary: BatchExecutionSummary) -> Self {
        let mut panel = Self::new();
        panel.current_summary = Some(summary);
        panel
    }

    pub fn ui(&mut self, ui: &mut Ui) -> BatchResultsResponse {
        let mut response = BatchResultsResponse::default();

        if let Some(ref summary) = self.current_summary.clone() {
            // Header with overall status
            self.render_header(ui, summary, &mut response);

            ui.separator();

            // View mode tabs
            ui.horizontal(|ui| {
                ui.selectable_value(&mut self.view_mode, ResultsViewMode::Summary, "Summary");
                ui.selectable_value(&mut self.view_mode, ResultsViewMode::Details, "Details");
                ui.selectable_value(&mut self.view_mode, ResultsViewMode::Logs, "Logs");

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.checkbox(&mut self.show_only_failed, "Show failed only");
                    ui.checkbox(&mut self.auto_refresh, "Auto-refresh");
                });
            });

            ui.separator();

            match self.view_mode {
                ResultsViewMode::Summary => {
                    self.render_summary_view(ui, summary);
                }
                ResultsViewMode::Details => {
                    self.render_details_view(ui, summary);
                }
                ResultsViewMode::Logs => {
                    self.render_logs_view(ui, summary);
                }
            }
        } else {
            ui.centered_and_justified(|ui| {
                ui.label("No execution results available");
            });
        }

        response
    }

    fn render_header(&mut self, ui: &mut Ui, summary: &BatchExecutionSummary, response: &mut BatchResultsResponse) {
        ui.horizontal(|ui| {
            // Overall status indicator
            let (icon, color) = if summary.failed == 0 && summary.cancelled == 0 {
                ("✓", Color32::GREEN)
            } else if summary.successful == 0 {
                ("✗", Color32::RED)
            } else {
                ("⚠", Color32::YELLOW)
            };

            ui.colored_label(color, RichText::new(icon).size(24.0));
            ui.heading("Batch Execution Results");

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("Export").clicked() {
                    response.export_requested = true;
                }
                if ui.button("Close").clicked() {
                    response.close_requested = true;
                }
            });
        });

        // Stats row
        ui.horizontal(|ui| {
            let total = summary.total_servers;
            let success_pct = if total > 0 { (summary.successful as f32 / total as f32) * 100.0 } else { 0.0 };

            ui.label(format!(
                "{}/{} successful ({:.1}%)",
                summary.successful, total, success_pct
            ));

            if summary.failed > 0 {
                ui.colored_label(Color32::RED, format!("{} failed", summary.failed));
            }
            if summary.cancelled > 0 {
                ui.colored_label(Color32::YELLOW, format!("{} cancelled", summary.cancelled));
            }

            let duration = summary.completed_at - summary.started_at;
            ui.label(format!("| Duration: {}s", duration.num_seconds()));
        });

        // Progress bar
        let progress = if summary.total_servers > 0 {
            (summary.successful + summary.failed + summary.cancelled) as f32 / summary.total_servers as f32
        } else {
            1.0
        };
        ui.add(ProgressBar::new(progress).text(format!("{}/{} complete", summary.successful + summary.failed + summary.cancelled, summary.total_servers)));
    }

    fn render_summary_view(&mut self, ui: &mut Ui, summary: &BatchExecutionSummary) {
        // Server results grid
        ui.label("Server Results:");

        egui::ScrollArea::vertical()
            .show(ui, |ui| {
                for server_result in &summary.results {
                    if self.show_only_failed && server_result.success {
                        continue;
                    }
                    self.render_server_summary_card(ui, server_result);
                }
            });
    }

    fn render_server_summary_card(&mut self, ui: &mut Ui, result: &ServerExecutionResult) {
        let (status_icon, status_color) = if result.success {
            ("✓", Color32::GREEN)
        } else {
            ("✗", Color32::RED)
        };

        let bg_color = if result.success {
            Color32::from_rgb(30, 50, 30)
        } else {
            Color32::from_rgb(50, 30, 30)
        };

        Frame::group(ui.style())
            .fill(bg_color)
            .show(ui, |ui| {
                ui.set_min_width(ui.available_width());

                ui.horizontal(|ui| {
                    ui.colored_label(status_color, RichText::new(status_icon).size(20.0));

                    ui.vertical(|ui| {
                        ui.label(RichText::new(&result.server_name).strong());
                        ui.label(RichText::new(&result.server_id).size(11.0).color(Color32::GRAY));
                    });

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(format!("{}ms", result.execution_time_ms));

                        if let Some(ref _error) = result.error_message {
                            ui.colored_label(Color32::RED, "Error");
                        }
                    });
                });

                if !result.step_results.is_empty() {
                    ui.horizontal_wrapped(|ui| {
                        for step in &result.step_results {
                            let step_icon = match step.status {
                                StepStatus::Completed => "✓",
                                StepStatus::Failed => "✗",
                                StepStatus::Skipped => "⊘",
                                _ => "•",
                            };
                            let step_color = match step.status {
                                StepStatus::Completed => Color32::GREEN,
                                StepStatus::Failed => Color32::RED,
                                StepStatus::Skipped => Color32::GRAY,
                                _ => Color32::YELLOW,
                            };
                            ui.colored_label(step_color, step_icon);
                        }
                    });
                }

                if let Some(ref error) = result.error_message {
                    ui.colored_label(Color32::RED, RichText::new(error).size(12.0));
                }
            });

        ui.add_space(4.0);
    }

    fn render_details_view(&mut self, ui: &mut Ui, summary: &BatchExecutionSummary) {
        // Two-column layout: server list on left, details on right
        let available_width = ui.available_width();
        let left_width = available_width * 0.3;

        ui.horizontal(|ui| {
            // Server list
            ui.vertical(|ui| {
                ui.set_width(left_width);
                ui.label("Servers:");

                egui::ScrollArea::vertical()
                    .max_height(400.0)
                    .show(ui, |ui| {
                        for result in &summary.results {
                            if self.show_only_failed && result.success {
                                continue;
                            }

                            let is_selected = self.selected_server.as_ref() == Some(&result.server_id);
                            let (icon, color) = if result.success {
                                ("✓", Color32::GREEN)
                            } else {
                                ("✗", Color32::RED)
                            };

                            let text = format!("{} {}", icon, result.server_name);
                            let label = if is_selected {
                                RichText::new(text).strong().background_color(Color32::from_gray(50))
                            } else {
                                RichText::new(text).color(color)
                            };

                            if ui.selectable_label(is_selected, label).clicked() {
                                self.selected_server = Some(result.server_id.clone());
                            }
                        }
                    });
            });

            ui.separator();

            // Details panel
            ui.vertical(|ui| {
                if let Some(ref server_id) = self.selected_server {
                    if let Some(result) = summary.results.iter().find(|r| &r.server_id == server_id) {
                        self.render_server_details(ui, result);
                    }
                } else {
                    ui.centered_and_justified(|ui| {
                        ui.label("Select a server to view details");
                    });
                }
            });
        });
    }

    fn render_server_details(&mut self, ui: &mut Ui, result: &ServerExecutionResult) {
        ui.heading(&result.server_name);

        ui.horizontal(|ui| {
            let (status_text, status_color) = if result.success {
                ("Success", Color32::GREEN)
            } else {
                ("Failed", Color32::RED)
            };
            ui.colored_label(status_color, status_text);
            ui.label(format!("| Execution time: {}ms", result.execution_time_ms));
        });

        if let Some(ref error) = result.error_message {
            ui.colored_label(Color32::RED, "Error:");
            ui.label(error);
        }

        ui.separator();

        ui.label("Step Results:");
        for step in &result.step_results {
            let (icon, color) = match step.status {
                StepStatus::Completed => ("✓", Color32::GREEN),
                StepStatus::Failed => ("✗", Color32::RED),
                StepStatus::Skipped => ("⊘", Color32::GRAY),
                _ => ("•", Color32::YELLOW),
            };

            ui.horizontal(|ui| {
                ui.colored_label(color, icon);
                ui.label(&step.step_name);

                if let Some(ref output) = step.output_preview {
                    ui.label(RichText::new(output).size(11.0).color(Color32::GRAY).monospace());
                }
            });
        }
    }

    fn render_logs_view(&mut self, ui: &mut Ui, summary: &BatchExecutionSummary) {
        ui.label("Execution Logs:");

        // Server filter
        ui.horizontal(|ui| {
            ui.label("Filter by server:");
            if ui.button("All").clicked() {
                self.selected_server = None;
            }
            for result in &summary.results {
                let is_selected = self.selected_server.as_ref() == Some(&result.server_id);
                if ui.selectable_label(is_selected, &result.server_name).clicked() {
                    self.selected_server = Some(result.server_id.clone());
                }
            }
        });

        ui.separator();

        // Log output
        egui::ScrollArea::vertical()
            .max_height(400.0)
            .show(ui, |ui| {
                let logs = self.generate_combined_logs(summary);
                ui.monospace(&logs);
            });
    }

    fn generate_combined_logs(&self, summary: &BatchExecutionSummary) -> String {
        let mut logs = String::new();

        logs.push_str(&format!(
            "=== Batch Execution: {} ===\n",
            summary.execution_id
        ));
        logs.push_str(&format!(
            "Started: {}\n",
            summary.started_at.format("%Y-%m-%d %H:%M:%S")
        ));
        logs.push_str(&format!(
            "Total servers: {}\n",
            summary.total_servers
        ));
        logs.push('\n');

        for result in &summary.results {
            // Filter if server selected
            if let Some(ref selected) = self.selected_server {
                if &result.server_id != selected {
                    continue;
                }
            }

            logs.push_str(&format!(
                "--- {} ({}) ---\n",
                result.server_name, result.server_id
            ));
            logs.push_str(&format!(
                "Status: {}\n",
                if result.success { "SUCCESS" } else { "FAILED" }
            ));

            if let Some(ref error) = result.error_message {
                logs.push_str(&format!("Error: {}\n", error));
            }

            for step in &result.step_results {
                logs.push_str(&format!(
                    "[{}] {}: {:?}\n",
                    step.step_name,
                    match step.status {
                        StepStatus::Completed => "OK",
                        StepStatus::Failed => "FAIL",
                        StepStatus::Skipped => "SKIP",
                        _ => "???",
                    },
                    step.status
                ));

                if let Some(ref output) = step.output_preview {
                    logs.push_str(&format!("  Output: {}\n", output));
                }
            }

            logs.push('\n');
        }

        logs.push_str(&format!(
            "=== Completed: {} ===\n",
            summary.completed_at.format("%Y-%m-%d %H:%M:%S")
        ));

        logs
    }

    pub fn set_summary(&mut self, summary: BatchExecutionSummary) {
        self.current_summary = Some(summary);
    }

    pub fn update_summary(&mut self, summary: BatchExecutionSummary) {
        self.current_summary = Some(summary);
    }
}

/// Response from batch results panel
#[derive(Debug, Default)]
pub struct BatchResultsResponse {
    pub export_requested: bool,
    pub close_requested: bool,
    pub retry_failed_requested: bool,
}
