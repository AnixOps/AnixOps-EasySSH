#![allow(dead_code)]

use eframe::egui;
use egui::{Color32, Frame, ProgressBar, RichText, ScrollArea, TextEdit, Ui};
use std::collections::HashMap;

use easyssh_core::workflow_engine::StepStatus;
use easyssh_core::workflow_executor::{BatchExecutionSummary, ServerExecutionResult};

/// Batch execution results viewer with advanced features
pub struct BatchExecutionResultsPanel {
    /// Current execution summary
    current_summary: Option<BatchExecutionSummary>,
    /// Selected server for detail view
    selected_server: Option<String>,
    /// Selected step for detail view
    selected_step: Option<String>,
    /// Show only failed results
    show_only_failed: bool,
    /// Show only succeeded results
    show_only_succeeded: bool,
    /// Filter by server name
    server_filter: String,
    /// Auto-refresh interval
    auto_refresh: bool,
    /// View mode: summary, details, or log
    view_mode: ResultsViewMode,
    /// Expanded servers
    expanded_servers: HashMap<String, bool>,
    /// Group results by status
    group_by_status: bool,
    /// Sort order
    sort_order: SortOrder,
    /// Export format
    export_format: ExportFormat,
    /// Show retry dialog
    show_retry_dialog: bool,
    /// Retry strategy
    retry_strategy: RetryStrategy,
    /// Detailed log view
    show_log_viewer: bool,
    /// Selected log content
    log_content: String,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ResultsViewMode {
    Summary,
    Details,
    Logs,
    Analytics,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SortOrder {
    NameAsc,
    NameDesc,
    DurationAsc,
    DurationDesc,
    Status,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ExportFormat {
    Json,
    Csv,
    Markdown,
    Html,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RetryStrategy {
    AllFailed,
    FailedAndCancelled,
    SelectedOnly,
}

/// Response from batch results panel
#[derive(Debug, Default)]
pub struct BatchResultsResponse {
    pub export_requested: bool,
    pub export_format: Option<ExportFormat>,
    pub close_requested: bool,
    pub retry_failed_requested: bool,
    pub retry_strategy: RetryStrategy,
    pub server_ids_to_retry: Vec<String>,
    pub view_server_logs: Option<String>,
    pub compare_servers: Option<(String, String)>,
}

impl BatchExecutionResultsPanel {
    pub fn new() -> Self {
        Self {
            current_summary: None,
            selected_server: None,
            selected_step: None,
            show_only_failed: false,
            show_only_succeeded: false,
            server_filter: String::new(),
            auto_refresh: false,
            view_mode: ResultsViewMode::Summary,
            expanded_servers: HashMap::new(),
            group_by_status: false,
            sort_order: SortOrder::NameAsc,
            export_format: ExportFormat::Json,
            show_retry_dialog: false,
            retry_strategy: RetryStrategy::AllFailed,
            show_log_viewer: false,
            log_content: String::new(),
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

            // Toolbar with filters and view options
            self.render_toolbar(ui, summary);

            ui.separator();

            // View content based on mode
            match self.view_mode {
                ResultsViewMode::Summary => {
                    self.render_summary_view(ui, summary, &mut response);
                }
                ResultsViewMode::Details => {
                    self.render_details_view(ui, summary, &mut response);
                }
                ResultsViewMode::Logs => {
                    self.render_logs_view(ui, summary, &mut response);
                }
                ResultsViewMode::Analytics => {
                    self.render_analytics_view(ui, summary);
                }
            }

            // Retry dialog
            if self.show_retry_dialog {
                self.render_retry_dialog(ui, summary, &mut response);
            }

            // Log viewer
            if self.show_log_viewer {
                self.render_log_viewer(ui);
            }
        } else {
            ui.centered_and_justified(|ui| {
                ui.label(RichText::new("No execution results available").size(16.0));
                ui.label("Run a workflow to see results here");
            });
        }

        response
    }

    fn render_header(
        &mut self,
        ui: &mut Ui,
        summary: &BatchExecutionSummary,
        response: &mut BatchResultsResponse,
    ) {
        ui.horizontal(|ui| {
            // Overall status indicator
            let (icon, color, status_text) = if summary.failed == 0 && summary.cancelled == 0 {
                ("✓", Color32::GREEN, "All Succeeded")
            } else if summary.successful == 0 {
                ("✗", Color32::RED, "All Failed")
            } else {
                ("⚠", Color32::YELLOW, "Partial Success")
            };

            ui.colored_label(color, RichText::new(icon).size(28.0).strong());
            ui.heading("Batch Execution Results");
            ui.colored_label(color, RichText::new(status_text).size(14.0));

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                // Export dropdown
                ui.menu_button("Export ▼", |ui| {
                    ui.set_min_width(120.0);
                    if ui.button("📄 Export as JSON").clicked() {
                        response.export_requested = true;
                        response.export_format = Some(ExportFormat::Json);
                        ui.close_menu();
                    }
                    if ui.button("📊 Export as CSV").clicked() {
                        response.export_requested = true;
                        response.export_format = Some(ExportFormat::Csv);
                        ui.close_menu();
                    }
                    if ui.button("📝 Export as Markdown").clicked() {
                        response.export_requested = true;
                        response.export_format = Some(ExportFormat::Markdown);
                        ui.close_menu();
                    }
                    if ui.button("🌐 Export as HTML").clicked() {
                        response.export_requested = true;
                        response.export_format = Some(ExportFormat::Html);
                        ui.close_menu();
                    }
                });

                if ui.button("Close").clicked() {
                    response.close_requested = true;
                }
            });
        });

        // Stats row with detailed metrics
        ui.horizontal(|ui| {
            let total = summary.total_servers;
            let success_pct = if total > 0 {
                (summary.successful as f32 / total as f32) * 100.0
            } else {
                0.0
            };

            ui.label(format!(
                "✓ {} succeeded ({:.1}%)",
                summary.successful, success_pct
            ));

            if summary.failed > 0 {
                ui.colored_label(Color32::RED, format!("✗ {} failed", summary.failed));
            }
            if summary.cancelled > 0 {
                ui.colored_label(Color32::YELLOW, format!("⊘ {} cancelled", summary.cancelled));
            }

            ui.separator();

            let duration = summary.completed_at - summary.started_at;
            ui.label(format!("⏱ Duration: {}s", duration.num_seconds()));

            // Average execution time
            if !summary.results.is_empty() {
                let avg_time: f64 = summary
                    .results
                    .iter()
                    .map(|r| r.execution_time_ms as f64)
                    .sum::<f64>()
                    / summary.results.len() as f64;
                ui.label(format!("avg {:.0}ms", avg_time));
            }
        });

        // Progress bar
        let progress = if summary.total_servers > 0 {
            (summary.successful + summary.failed + summary.cancelled) as f32
                / summary.total_servers as f32
        } else {
            1.0
        };
        let progress_text = format!(
            "{}/{} complete",
            summary.successful + summary.failed + summary.cancelled,
            summary.total_servers
        );
        ui.add(ProgressBar::new(progress).text(progress_text));
    }

    fn render_toolbar(&mut self, ui: &mut Ui, _summary: &BatchExecutionSummary) {
        ui.horizontal(|ui| {
            // View mode tabs
            ui.selectable_value(&mut self.view_mode, ResultsViewMode::Summary, "Summary");
            ui.selectable_value(&mut self.view_mode, ResultsViewMode::Details, "Details");
            ui.selectable_value(&mut self.view_mode, ResultsViewMode::Logs, "Logs");
            ui.selectable_value(&mut self.view_mode, ResultsViewMode::Analytics, "Analytics");

            ui.separator();

            // Filters
            ui.checkbox(&mut self.show_only_failed, "Failed only");
            ui.checkbox(&mut self.show_only_succeeded, "Succeeded only");

            // Server filter input
            ui.add(TextEdit::singleline(&mut self.server_filter).hint_text("Filter servers..."));

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                // Sort order dropdown
                egui::ComboBox::from_label("Sort")
                    .selected_text(format!("{:?}", self.sort_order))
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut self.sort_order, SortOrder::NameAsc, "Name ↑");
                        ui.selectable_value(&mut self.sort_order, SortOrder::NameDesc, "Name ↓");
                        ui.selectable_value(&mut self.sort_order, SortOrder::DurationAsc, "Duration ↑");
                        ui.selectable_value(&mut self.sort_order, SortOrder::DurationDesc, "Duration ↓");
                        ui.selectable_value(&mut self.sort_order, SortOrder::Status, "Status");
                    });

                ui.checkbox(&mut self.group_by_status, "Group by status");
                ui.checkbox(&mut self.auto_refresh, "Auto-refresh");
            });
        });
    }

    fn get_filtered_and_sorted_results<'a>(
        &'a self,
        summary: &'a BatchExecutionSummary,
    ) -> Vec<&'a ServerExecutionResult> {
        let mut results: Vec<_> = summary.results.iter().collect();

        // Apply filters
        results.retain(|r| {
            if self.show_only_failed && r.success {
                return false;
            }
            if self.show_only_succeeded && !r.success {
                return false;
            }
            if !self.server_filter.is_empty()
                && !r.server_name
                    .to_lowercase()
                    .contains(&self.server_filter.to_lowercase())
            {
                return false;
            }
            true
        });

        // Apply sorting
        results.sort_by(|a, b| match self.sort_order {
            SortOrder::NameAsc => a.server_name.cmp(&b.server_name),
            SortOrder::NameDesc => b.server_name.cmp(&a.server_name),
            SortOrder::DurationAsc => a.execution_time_ms.cmp(&b.execution_time_ms),
            SortOrder::DurationDesc => b.execution_time_ms.cmp(&a.execution_time_ms),
            SortOrder::Status => a.success.cmp(&b.success).reverse(),
        });

        results
    }

    fn render_summary_view(
        &mut self,
        ui: &mut Ui,
        summary: &BatchExecutionSummary,
        response: &mut BatchResultsResponse,
    ) {
        // Collect filtered and sorted results to avoid borrow issues
        let mut results: Vec<_> = summary.results.iter().filter(|r| {
            if self.show_only_failed && r.success {
                return false;
            }
            if self.show_only_succeeded && !r.success {
                return false;
            }
            if !self.server_filter.is_empty()
                && !r.server_name
                    .to_lowercase()
                    .contains(&self.server_filter.to_lowercase())
            {
                return false;
            }
            true
        }).cloned().collect();

        // Apply sorting
        results.sort_by(|a, b| match self.sort_order {
            SortOrder::NameAsc => a.server_name.cmp(&b.server_name),
            SortOrder::NameDesc => b.server_name.cmp(&a.server_name),
            SortOrder::DurationAsc => a.execution_time_ms.cmp(&b.execution_time_ms),
            SortOrder::DurationDesc => b.execution_time_ms.cmp(&a.execution_time_ms),
            SortOrder::Status => a.success.cmp(&b.success).reverse(),
        });

        if results.is_empty() {
            ui.centered_and_justified(|ui| {
                ui.label("No results match the current filters");
            });
            return;
        }

        if self.group_by_status {
            // Group by success/failure
            let (successful, failed): (Vec<_>, Vec<_>) =
                results.iter().partition(|r| r.success);

            if !successful.is_empty() {
                ui.colored_label(Color32::GREEN, format!("✓ Succeeded ({})", successful.len()));
                ScrollArea::vertical()
                    .id_salt("successful_group")
                    .max_height(200.0)
                    .show(ui, |ui| {
                        for result in successful {
                            self.render_server_summary_card(ui, result, response);
                        }
                    });
            }

            if !failed.is_empty() {
                ui.colored_label(Color32::RED, format!("✗ Failed ({})", failed.len()));
                ScrollArea::vertical()
                    .id_salt("failed_group")
                    .max_height(200.0)
                    .show(ui, |ui| {
                        for result in failed {
                            self.render_server_summary_card(ui, result, response);
                        }
                    });
            }
        } else {
            // Server results grid
            ui.label("Server Results:");

            ScrollArea::vertical().show(ui, |ui| {
                for result in &results {
                    self.render_server_summary_card(ui, result, response);
                }
            });
        }
    }

    fn render_server_summary_card(
        &mut self,
        ui: &mut Ui,
        result: &ServerExecutionResult,
        response: &mut BatchResultsResponse,
    ) {
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

        let is_selected = self.selected_server.as_ref() == Some(&result.server_id);
        let frame_color = if is_selected {
            Color32::from_rgb(60, 80, 100)
        } else {
            bg_color
        };

        Frame::group(ui.style()).fill(frame_color).show(ui, |ui| {
            ui.set_min_width(ui.available_width());

            ui.horizontal(|ui| {
                ui.colored_label(status_color, RichText::new(status_icon).size(20.0));

                ui.vertical(|ui| {
                    ui.label(RichText::new(&result.server_name).strong());
                    ui.label(
                        RichText::new(&result.server_id)
                            .size(11.0)
                            .color(Color32::GRAY),
                    );
                });

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    // Actions
                    if !result.success {
                        if ui.button("Retry").clicked() {
                            response.retry_failed_requested = true;
                            response.retry_strategy = RetryStrategy::SelectedOnly;
                            response.server_ids_to_retry.push(result.server_id.clone());
                        }
                    }
                    if ui.button("Logs").clicked() {
                        response.view_server_logs = Some(result.server_id.clone());
                        self.log_content = self.generate_server_log(result);
                        self.show_log_viewer = true;
                    }
                    if ui.button("View").clicked() {
                        self.selected_server = Some(result.server_id.clone());
                        self.view_mode = ResultsViewMode::Details;
                    }

                    ui.label(format!("{}ms", result.execution_time_ms));
                });
            });

            // Step results mini view
            if !result.step_results.is_empty() {
                ui.horizontal_wrapped(|ui| {
                    ui.label("Steps:");
                    for step in &result.step_results {
                        let step_icon = match step.status {
                            StepStatus::Completed => ("✓", Color32::GREEN),
                            StepStatus::Failed => ("✗", Color32::RED),
                            StepStatus::Skipped => ("⊘", Color32::GRAY),
                            _ => ("•", Color32::YELLOW),
                        };
                        ui.colored_label(
                            step_icon.1,
                            format!("{} {}", step_icon.0, &step.step_name),
                        );
                    }
                });
            }

            if let Some(ref error) = result.error_message {
                ui.colored_label(Color32::RED, RichText::new(error).size(12.0));
            }
        });

        ui.add_space(4.0);
    }

    fn render_details_view(
        &mut self,
        ui: &mut Ui,
        summary: &BatchExecutionSummary,
        response: &mut BatchResultsResponse,
    ) {
        // Collect filtered and sorted results first to avoid borrow issues
        let results: Vec<_> = summary
            .results
            .iter()
            .filter(|r| {
                if self.show_only_failed && r.success {
                    return false;
                }
                if self.show_only_succeeded && !r.success {
                    return false;
                }
                if !self.server_filter.is_empty()
                    && !r.server_name
                        .to_lowercase()
                        .contains(&self.server_filter.to_lowercase())
                {
                    return false;
                }
                true
            })
            .cloned()
            .collect();

        // Apply sorting
        let mut results = results;
        results.sort_by(|a, b| match self.sort_order {
            SortOrder::NameAsc => a.server_name.cmp(&b.server_name),
            SortOrder::NameDesc => b.server_name.cmp(&a.server_name),
            SortOrder::DurationAsc => a.execution_time_ms.cmp(&b.execution_time_ms),
            SortOrder::DurationDesc => b.execution_time_ms.cmp(&a.execution_time_ms),
            SortOrder::Status => a.success.cmp(&b.success).reverse(),
        });

        // Two-column layout: server list on left, details on right
        let available_width = ui.available_width();
        let left_width = (available_width * 0.35).min(300.0);

        ui.horizontal(|ui| {
            // Server list
            ui.vertical(|ui| {
                ui.set_width(left_width);
                ui.label(RichText::new("Servers").strong().size(14.0));

                ScrollArea::vertical()
                    .max_height(450.0)
                    .show(ui, |ui| {
                        for result in &results {
                            let is_selected =
                                self.selected_server.as_ref() == Some(&result.server_id);
                            let (icon, color) = if result.success {
                                ("✓", Color32::GREEN)
                            } else {
                                ("✗", Color32::RED)
                            };

                            let text = format!("{} {}", icon, result.server_name);
                            let label = if is_selected {
                                RichText::new(text)
                                    .strong()
                                    .background_color(Color32::from_gray(50))
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
                ui.set_width(ui.available_width());

                if let Some(ref server_id) = self.selected_server {
                    if let Some(result) = summary.results.iter().find(|r| &r.server_id == server_id)
                    {
                        self.render_server_details(ui, result, response);
                    }
                } else {
                    ui.centered_and_justified(|ui| {
                        ui.label(RichText::new("Select a server to view details").size(14.0));
                    });
                }
            });
        });
    }

    fn render_server_details(
        &mut self,
        ui: &mut Ui,
        result: &ServerExecutionResult,
        response: &mut BatchResultsResponse,
    ) {
        ui.heading(&result.server_name);

        ui.horizontal(|ui| {
            let (status_text, status_color) = if result.success {
                ("✓ Success", Color32::GREEN)
            } else {
                ("✗ Failed", Color32::RED)
            };
            ui.colored_label(status_color, RichText::new(status_text).strong());

            ui.separator();
            ui.label(format!("⏱ {}ms", result.execution_time_ms));

            if !result.success {
                ui.separator();
                if ui.button("🔁 Retry").clicked() {
                    response.retry_failed_requested = true;
                    response.retry_strategy = RetryStrategy::SelectedOnly;
                    response.server_ids_to_retry.push(result.server_id.clone());
                }
            }
        });

        if let Some(ref error) = result.error_message {
            ui.separator();
            ui.colored_label(Color32::RED, "Error:");
            Frame::group(ui.style())
                .fill(Color32::from_rgb(40, 20, 20))
                .show(ui, |ui| {
                    ui.set_min_width(ui.available_width());
                    ui.label(RichText::new(error).monospace());
                });
        }

        ui.separator();

        // Step results
        ui.label(RichText::new("Step Results:").strong().size(14.0));
        for step in &result.step_results {
            let (icon, color, status_text) = match step.status {
                StepStatus::Completed => ("✓", Color32::GREEN, "Completed"),
                StepStatus::Failed => ("✗", Color32::RED, "Failed"),
                StepStatus::Skipped => ("⊘", Color32::GRAY, "Skipped"),
                StepStatus::Pending => ("○", Color32::YELLOW, "Pending"),
                StepStatus::Running => ("▶", Color32::BLUE, "Running"),
            };

            Frame::group(ui.style()).show(ui, |ui| {
                ui.set_min_width(ui.available_width());

                ui.horizontal(|ui| {
                    ui.colored_label(color, RichText::new(icon).size(16.0));
                    ui.label(RichText::new(&step.step_name).strong());
                    ui.colored_label(color, status_text);
                });

                if let Some(ref output) = step.output_preview {
                    ui.separator();
                    ui.label(
                        RichText::new("Output:")
                            .size(11.0)
                            .color(Color32::LIGHT_GRAY),
                    );
                    Frame::group(ui.style())
                        .fill(Color32::from_gray(25))
                        .show(ui, |ui| {
                            ui.set_min_width(ui.available_width());
                            ui.add(ScrollArea::horizontal().show(ui, |ui| {
                                ui.monospace(output);
                            }).inner);
                        });
                }

                if step.status == StepStatus::Failed {
                    ui.separator();
                    ui.colored_label(Color32::RED, "Error:");
                    if let Some(ref output) = step.output_preview {
                        ui.label(RichText::new(output).monospace().size(11.0));
                    }
                }
            });

            ui.add_space(4.0);
        }
    }

    fn render_logs_view(
        &mut self,
        ui: &mut Ui,
        summary: &BatchExecutionSummary,
        response: &mut BatchResultsResponse,
    ) {
        // Server filter
        ui.horizontal(|ui| {
            ui.label(RichText::new("Filter by server:").strong());
            if ui.button("All").clicked() {
                self.selected_server = None;
            }
            for result in &summary.results {
                let is_selected = self.selected_server.as_ref() == Some(&result.server_id);
                let label = if result.success {
                    RichText::new(&result.server_name).color(Color32::GREEN)
                } else {
                    RichText::new(&result.server_name).color(Color32::RED)
                };
                if ui.selectable_label(is_selected, label).clicked() {
                    self.selected_server = Some(result.server_id.clone());
                }
            }
        });

        ui.separator();

        // Action buttons
        ui.horizontal(|ui| {
            if ui.button("📋 Copy All").clicked() {
                ui.output_mut(|o| {
                    o.copied_text = self.generate_combined_logs(summary);
                });
            }
            if ui.button("💾 Save to File").clicked() {
                response.export_requested = true;
                response.export_format = Some(ExportFormat::Markdown);
            }
        });

        ui.separator();

        // Log output
        ScrollArea::vertical()
            .max_height(400.0)
            .show(ui, |ui| {
                let logs = self.generate_combined_logs(summary);
                Frame::group(ui.style())
                    .fill(Color32::from_gray(20))
                    .show(ui, |ui| {
                        ui.set_min_width(ui.available_width());
                        ui.add(ScrollArea::horizontal().show(ui, |ui| {
                            ui.monospace(&logs);
                        }).inner);
                    });
            });
    }

    fn render_analytics_view(&mut self, ui: &mut Ui, summary: &BatchExecutionSummary) {
        let total = summary.total_servers;
        let success_rate = if total > 0 {
            (summary.successful as f32 / total as f32) * 100.0
        } else {
            0.0
        };

        // Statistics cards
        ui.horizontal(|ui| {
            // Success rate card
            Frame::group(ui.style())
                .fill(Color32::from_gray(35))
                .show(ui, |ui| {
                    ui.set_min_width(150.0);
                    ui.vertical_centered(|ui| {
                        ui.label(RichText::new("Success Rate").size(12.0).color(Color32::GRAY));
                        ui.label(
                            RichText::new(format!("{:.1}%", success_rate))
                                .size(24.0)
                                .strong()
                                .color(if success_rate >= 90.0 {
                                    Color32::GREEN
                                } else if success_rate >= 50.0 {
                                    Color32::YELLOW
                                } else {
                                    Color32::RED
                                }),
                        );
                    });
                });

            // Duration card
            let duration = summary.completed_at - summary.started_at;
            Frame::group(ui.style())
                .fill(Color32::from_gray(35))
                .show(ui, |ui| {
                    ui.set_min_width(150.0);
                    ui.vertical_centered(|ui| {
                        ui.label(RichText::new("Duration").size(12.0).color(Color32::GRAY));
                        ui.label(
                            RichText::new(format!("{}s", duration.num_seconds()))
                                .size(24.0)
                                .strong(),
                        );
                    });
                });

            // Average execution time
            if !summary.results.is_empty() {
                let avg_time = summary
                    .results
                    .iter()
                    .map(|r| r.execution_time_ms as f64)
                    .sum::<f64>()
                    / summary.results.len() as f64;
                Frame::group(ui.style())
                    .fill(Color32::from_gray(35))
                    .show(ui, |ui| {
                        ui.set_min_width(150.0);
                        ui.vertical_centered(|ui| {
                            ui.label(
                                RichText::new("Avg Execution").size(12.0).color(Color32::GRAY),
                            );
                            ui.label(
                                RichText::new(format!("{:.0}ms", avg_time))
                                    .size(24.0)
                                    .strong(),
                            );
                        });
                    });
            }
        });

        ui.add_space(16.0);

        // Execution time distribution
        ui.label(RichText::new("Execution Time Distribution:").strong());

        let max_time = summary
            .results
            .iter()
            .map(|r| r.execution_time_ms)
            .max()
            .unwrap_or(0);

        ScrollArea::vertical().show(ui, |ui| {
            for result in &summary.results {
                let bar_width = if max_time > 0 {
                    (result.execution_time_ms as f32 / max_time as f32) * 200.0
                } else {
                    0.0
                };

                ui.horizontal(|ui| {
                    ui.label(&result.server_name);
                    ui.add_space(4.0);
                    ui.add(
                        ProgressBar::new(result.execution_time_ms as f32 / max_time.max(1) as f32)
                            .desired_width(200.0),
                    );
                    ui.label(format!("{}ms", result.execution_time_ms));
                });
            }
        });
    }

    fn render_retry_dialog(
        &mut self,
        ui: &mut Ui,
        summary: &BatchExecutionSummary,
        response: &mut BatchResultsResponse,
    ) {
        let id = ui.make_persistent_id("retry_dialog");

        egui::Window::new("Retry Failed Servers")
            .id(id)
            .collapsible(false)
            .resizable(false)
            .show(ui.ctx(), |ui| {
                ui.label("Select retry strategy:");

                ui.radio_value(
                    &mut self.retry_strategy,
                    RetryStrategy::AllFailed,
                    format!(
                        "Retry all failed servers ({})",
                        summary.results.iter().filter(|r| !r.success).count()
                    ),
                );

                ui.radio_value(
                    &mut self.retry_strategy,
                    RetryStrategy::FailedAndCancelled,
                    format!(
                        "Retry failed and cancelled ({})",
                        summary
                            .results
                            .iter()
                            .filter(|r| !r.success || r.error_message.is_some())
                            .count()
                    ),
                );

                ui.radio_value(
                    &mut self.retry_strategy,
                    RetryStrategy::SelectedOnly,
                    "Retry selected servers only",
                );

                ui.separator();

                ui.horizontal(|ui| {
                    if ui.button("Retry").clicked() {
                        response.retry_failed_requested = true;
                        response.retry_strategy = self.retry_strategy;
                        self.show_retry_dialog = false;
                    }
                    if ui.button("Cancel").clicked() {
                        self.show_retry_dialog = false;
                    }
                });
            });
    }

    fn render_log_viewer(&mut self, ui: &mut Ui) {
        let id = ui.make_persistent_id("log_viewer");

        egui::Window::new("Server Log")
            .id(id)
            .collapsible(false)
            .resizable(true)
            .default_size([600.0, 400.0])
            .show(ui.ctx(), |ui| {
                ui.horizontal(|ui| {
                    if ui.button("📋 Copy").clicked() {
                        ui.output_mut(|o| {
                            o.copied_text = self.log_content.clone();
                        });
                    }
                    if ui.button("💾 Save").clicked() {
                        // Would trigger save dialog
                    }
                    if ui.button("✕ Close").clicked() {
                        self.show_log_viewer = false;
                    }
                });

                ui.separator();

                ScrollArea::both()
                    .max_height(350.0)
                    .show(ui, |ui| {
                        Frame::group(ui.style())
                            .fill(Color32::from_gray(20))
                            .show(ui, |ui| {
                                ui.set_min_width(ui.available_width());
                                ui.monospace(&self.log_content);
                            });
                    });
            });
    }

    fn generate_combined_logs(&self, summary: &BatchExecutionSummary) -> String {
        let mut logs = String::new();

        logs.push_str(&format!(
            "=== Batch Execution: {} ===\n",
            summary.execution_id
        ));
        logs.push_str(&format!(
            "Workflow: {}\n",
            summary.workflow_id
        ));
        logs.push_str(&format!(
            "Started: {}\n",
            summary.started_at.format("%Y-%m-%d %H:%M:%S")
        ));
        logs.push_str(&format!(
            "Completed: {}\n",
            summary.completed_at.format("%Y-%m-%d %H:%M:%S")
        ));
        let duration = summary.completed_at - summary.started_at;
        logs.push_str(&format!("Duration: {}s\n", duration.num_seconds()));
        logs.push_str(&format!("Total servers: {}\n", summary.total_servers));
        logs.push_str(&format!("Successful: {}\n", summary.successful));
        logs.push_str(&format!("Failed: {}\n", summary.failed));
        logs.push_str(&format!("Cancelled: {}\n", summary.cancelled));
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
            logs.push_str(&format!("Execution time: {}ms\n", result.execution_time_ms));

            if let Some(ref error) = result.error_message {
                logs.push_str(&format!("Error: {}\n", error));
            }

            if !result.step_results.is_empty() {
                logs.push_str("Steps:\n");
                for step in &result.step_results {
                    let status_str = match step.status {
                        StepStatus::Completed => "OK",
                        StepStatus::Failed => "FAIL",
                        StepStatus::Skipped => "SKIP",
                        StepStatus::Pending => "PENDING",
                        StepStatus::Running => "RUNNING",
                        StepStatus::Cancelled => "CANCELLED",
                    };
                    logs.push_str(&format!("  [{}]: {}\n", step.step_name, status_str));

                    if let Some(ref output) = step.output_preview {
                        for line in output.lines() {
                            logs.push_str(&format!("    > {}\n", line));
                        }
                    }

                    if step.status == StepStatus::Failed {
                        if let Some(ref output) = step.output_preview {
                            logs.push_str(&format!("    ! ERROR: {}\n", output));
                        }
                    }
                }
            }

            logs.push('\n');
        }

        logs.push_str("=== End of Execution Log ===\n");

        logs
    }

    fn generate_server_log(&self, result: &ServerExecutionResult) -> String {
        let mut logs = String::new();

        logs.push_str(&format!("=== Server: {} ===\n", result.server_name));
        logs.push_str(&format!("ID: {}\n", result.server_id));
        logs.push_str(&format!(
            "Status: {}\n",
            if result.success { "SUCCESS" } else { "FAILED" }
        ));
        logs.push_str(&format!("Execution time: {}ms\n\n", result.execution_time_ms));

        if let Some(ref error) = result.error_message {
            logs.push_str(&format!("ERROR: {}\n\n", error));
        }

        if !result.step_results.is_empty() {
            logs.push_str("STEP RESULTS:\n");
            for step in &result.step_results {
                let status_str = match step.status {
                    StepStatus::Completed => "OK",
                    StepStatus::Failed => "FAIL",
                    StepStatus::Skipped => "SKIP",
                    StepStatus::Pending => "PENDING",
                    StepStatus::Running => "RUNNING",
                    StepStatus::Cancelled => "CANCELLED",
                };
                logs.push_str(&format!("\n[{}] - {}\n", step.step_name, status_str));

                if let Some(ref output) = step.output_preview {
                    logs.push_str("Output:\n");
                    for line in output.lines() {
                        logs.push_str(&format!("  {}\n", line));
                    }
                }

                if step.status == StepStatus::Failed {
                    if let Some(ref output) = step.output_preview {
                        logs.push_str(&format!("Error: {}\n", output));
                    }
                }
            }
        }

        logs.push_str("\n=== End Log ===\n");
        logs
    }

    /// Generate export content based on format
    pub fn generate_export_content(
        &self,
        summary: &BatchExecutionSummary,
        format: ExportFormat,
    ) -> String {
        match format {
            ExportFormat::Json => self.generate_json_export(summary),
            ExportFormat::Csv => self.generate_csv_export(summary),
            ExportFormat::Markdown => self.generate_markdown_export(summary),
            ExportFormat::Html => self.generate_html_export(summary),
        }
    }

    fn generate_json_export(&self, summary: &BatchExecutionSummary) -> String {
        // Simplified JSON generation
        format!(
            r#"{{
  "execution_id": "{}",
  "workflow_id": "{}",
  "started_at": "{}",
  "completed_at": "{}",
  "total_servers": {},
  "successful": {},
  "failed": {},
  "cancelled": {},
  "results": [
{}
  ]
}}"#,
            summary.execution_id,
            summary.workflow_id,
            summary.started_at.to_rfc3339(),
            summary.completed_at.to_rfc3339(),
            summary.total_servers,
            summary.successful,
            summary.failed,
            summary.cancelled,
            summary
                .results
                .iter()
                .map(|r| format!(
                    r#"    {{
      "server_id": "{}",
      "server_name": "{}",
      "success": {},
      "execution_time_ms": {}
    }}"#,
                    r.server_id, r.server_name, r.success, r.execution_time_ms
                ))
                .collect::<Vec<_>>()
                .join(",\n")
        )
    }

    fn generate_csv_export(&self, summary: &BatchExecutionSummary) -> String {
        let mut csv = String::new();
        csv.push_str("server_id,server_name,success,execution_time_ms,error\n");

        for result in &summary.results {
            let error_escaped = result
                .error_message
                .as_ref()
                .map(|e| e.replace('"', "\"").replace('\n', " "))
                .unwrap_or_default();
            csv.push_str(&format!(
                "{},{},{},{},\"{}\"\n",
                result.server_id,
                result.server_name,
                result.success,
                result.execution_time_ms,
                error_escaped
            ));
        }

        csv
    }

    fn generate_markdown_export(&self, summary: &BatchExecutionSummary) -> String {
        let duration = summary.completed_at - summary.started_at;

        let mut md = String::new();
        md.push_str("# Batch Execution Report\n\n");
        md.push_str(&format!("**Execution ID:** {}\n\n", summary.execution_id));
        md.push_str(&format!("**Workflow ID:** {}\n\n", summary.workflow_id));
        md.push_str(&format!(
            "**Started:** {}\n\n",
            summary.started_at.format("%Y-%m-%d %H:%M:%S")
        ));
        md.push_str(&format!(
            "**Completed:** {}\n\n",
            summary.completed_at.format("%Y-%m-%d %H:%M:%S")
        ));
        md.push_str(&format!("**Duration:** {} seconds\n\n", duration.num_seconds()));
        md.push_str("## Summary\n\n");
        md.push_str(&format!("- **Total Servers:** {}\n", summary.total_servers));
        md.push_str(&format!("- **Successful:** {}\n", summary.successful));
        md.push_str(&format!("- **Failed:** {}\n", summary.failed));
        md.push_str(&format!("- **Cancelled:** {}\n\n", summary.cancelled));

        md.push_str("## Results\n\n");
        md.push_str("| Server | Status | Duration | Error |\n");
        md.push_str("|--------|--------|----------|-------|\n");

        for result in &summary.results {
            let status = if result.success { "✓" } else { "✗" };
            let error = result.error_message.as_deref().unwrap_or("-");
            md.push_str(&format!(
                "| {} | {} | {}ms | {} |\n",
                result.server_name, status, result.execution_time_ms, error
            ));
        }

        md
    }

    fn generate_html_export(&self, summary: &BatchExecutionSummary) -> String {
        let duration = summary.completed_at - summary.started_at;
        let success_rate = if summary.total_servers > 0 {
            (summary.successful as f32 / summary.total_servers as f32) * 100.0
        } else {
            0.0
        };

        let rows: String = summary
            .results
            .iter()
            .map(|r| {
                let status_color = if r.success { "#4CAF50" } else { "#f44336" };
                let status = if r.success { "Success" } else { "Failed" };
                let error = r
                    .error_message
                    .as_ref()
                    .map(|e| format!("<td>{}</td>", e))
                    .unwrap_or_else(|| "<td>-</td>".to_string());
                format!(
                    "<tr><td>{}</td><td style='color: {}'>{}</td><td>{}ms</td>{}</tr>",
                    r.server_name, status_color, status, r.execution_time_ms, error
                )
            })
            .collect();

        format!(
            r#"<!DOCTYPE html>
<html>
<head><title>Batch Execution Report</title></head>
<body style="font-family: sans-serif; padding: 20px;">
<h1>Batch Execution Report</h1>
<p><strong>Execution ID:</strong> {}</p>
<p><strong>Workflow:</strong> {}</p>
<p><strong>Started:</strong> {}</p>
<p><strong>Completed:</strong> {}</p>
<p><strong>Duration:</strong> {}s</p>
<h2>Summary</h2>
<p>Success Rate: {:.1}%</p>
<p>Total: {}, Successful: {}, Failed: {}, Cancelled: {}</p>
<h2>Results</h2>
<table border="1" cellpadding="5" style="border-collapse: collapse;">
<tr><th>Server</th><th>Status</th><th>Duration</th><th>Error</th></tr>
{}
</table>
</body>
</html>"#,
            summary.execution_id,
            summary.workflow_id,
            summary.started_at.format("%Y-%m-%d %H:%M:%S"),
            summary.completed_at.format("%Y-%m-%d %H:%M:%S"),
            duration.num_seconds(),
            success_rate,
            summary.total_servers,
            summary.successful,
            summary.failed,
            summary.cancelled,
            rows
        )
    }

    pub fn set_summary(&mut self, summary: BatchExecutionSummary) {
        self.current_summary = Some(summary);
    }

    pub fn update_summary(&mut self, summary: BatchExecutionSummary) {
        self.current_summary = Some(summary);
    }

    pub fn show_retry_dialog(&mut self) {
        self.show_retry_dialog = true;
    }
}

impl Default for BatchExecutionResultsPanel {
    fn default() -> Self {
        Self::new()
    }
}
