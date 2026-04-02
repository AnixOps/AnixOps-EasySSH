#![allow(dead_code)]

use eframe::egui;
use egui::{Color32, Frame, RichText, ScrollArea, TextEdit, Ui};

use easyssh_core::script_library::ScriptLibrary;
use easyssh_core::workflow_scheduler::*;

/// Scheduled tasks management UI with execution history
pub struct ScheduledTasksPanel {
    /// Currently selected task
    selected_task: Option<String>,
    /// Show create dialog
    show_create_dialog: bool,
    /// Show edit dialog
    show_edit_dialog: bool,
    /// New task form
    new_task_form: NewTaskForm,
    /// Edit task form
    edit_task_form: Option<ScheduledTask>,
    /// Filter by status
    filter_status: Option<TaskStatus>,
    /// Search query
    search_query: String,
    /// Show only enabled tasks
    show_enabled_only: bool,
    /// Execution history panel
    show_history: bool,
    /// Selected history task
    history_task_id: Option<String>,
    /// Show task details panel
    show_task_details: bool,
    /// History view mode
    history_view_mode: HistoryViewMode,
    /// Bulk selection mode
    bulk_selection: Vec<String>,
    /// Show bulk actions
    show_bulk_actions: bool,
}

#[derive(Debug, Default)]
struct NewTaskForm {
    name: String,
    description: String,
    workflow_id: String,
    cron_expression: String,
    server_ids: Vec<String>,
    timeout_minutes: u64,
    email_notifications: bool,
    notification_email: String,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum HistoryViewMode {
    List,
    Statistics,
}

/// Response from scheduled tasks panel
#[derive(Debug, Default)]
pub struct ScheduledTasksResponse {
    pub create_task: Option<ScheduledTask>,
    pub update_task: Option<ScheduledTask>,
    pub delete_task: Option<String>,
    pub toggle_enabled: Option<(String, bool)>,
    pub run_now: Option<String>,
    pub bulk_delete: Vec<String>,
    pub bulk_toggle: Vec<(String, bool)>,
}

impl ScheduledTasksPanel {
    pub fn new() -> Self {
        Self {
            selected_task: None,
            show_create_dialog: false,
            show_edit_dialog: false,
            new_task_form: NewTaskForm {
                timeout_minutes: 30,
                email_notifications: false,
                ..Default::default()
            },
            edit_task_form: None,
            filter_status: None,
            search_query: String::new(),
            show_enabled_only: false,
            show_history: false,
            history_task_id: None,
            show_task_details: false,
            history_view_mode: HistoryViewMode::List,
            bulk_selection: Vec::new(),
            show_bulk_actions: false,
        }
    }

    pub fn ui(
        &mut self,
        ui: &mut Ui,
        scheduler: &mut TaskScheduler,
        script_library: &ScriptLibrary,
    ) -> ScheduledTasksResponse {
        let mut response = ScheduledTasksResponse::default();

        // Header
        ui.horizontal(|ui| {
            ui.heading("Scheduled Tasks");

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("+ New Task").clicked() {
                    self.show_create_dialog = true;
                    self.new_task_form = NewTaskForm::default();
                }

                if ui.button("📊 Stats").clicked() {
                    self.show_history = true;
                }

                ui.checkbox(&mut self.show_enabled_only, "Enabled only");
                ui.add(TextEdit::singleline(&mut self.search_query).hint_text("Search tasks..."));
            });
        });

        ui.separator();

        // Status filter tabs
        ui.horizontal(|ui| {
            let filters = vec![
                (None, "All"),
                (Some(TaskStatus::Pending), "Pending"),
                (Some(TaskStatus::Running), "Running"),
                (Some(TaskStatus::Completed), "Completed"),
                (Some(TaskStatus::Failed), "Failed"),
            ];

            for (status, label) in filters {
                let selected = self.filter_status == status;
                if ui.selectable_label(selected, label).clicked() {
                    self.filter_status = status;
                }
            }

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if self.show_bulk_actions && !self.bulk_selection.is_empty() {
                    ui.menu_button("Bulk Actions ▼", |ui| {
                        if ui.button("Delete Selected").clicked() {
                            response.bulk_delete = self.bulk_selection.clone();
                            self.bulk_selection.clear();
                            self.show_bulk_actions = false;
                            ui.close_menu();
                        }
                        if ui.button("Enable Selected").clicked() {
                            response.bulk_toggle = self
                                .bulk_selection
                                .iter()
                                .map(|id| (id.clone(), true))
                                .collect();
                            self.bulk_selection.clear();
                            self.show_bulk_actions = false;
                            ui.close_menu();
                        }
                        if ui.button("Disable Selected").clicked() {
                            response.bulk_toggle = self
                                .bulk_selection
                                .iter()
                                .map(|id| (id.clone(), false))
                                .collect();
                            self.bulk_selection.clear();
                            self.show_bulk_actions = false;
                            ui.close_menu();
                        }
                    });
                }
                ui.checkbox(&mut self.show_bulk_actions, "Bulk select");
            });
        });

        ui.separator();

        // Running tasks banner
        let running = scheduler.get_running_tasks();
        if !running.is_empty() {
            Frame::group(ui.style())
                .fill(Color32::from_rgb(20, 60, 20))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.colored_label(Color32::GREEN, "▶ Running Tasks:");
                        for handle in &running {
                            if let Some(task) = scheduler.get_task(&handle.task_id) {
                                ui.colored_label(
                                    Color32::LIGHT_GREEN,
                                    format!("{} ({}s)", task.name, handle.elapsed_seconds()),
                                );
                            }
                        }
                    });
                });
            ui.separator();
        }

        // Task list
        self.render_task_list(ui, scheduler, &mut response);

        // Create dialog
        if self.show_create_dialog {
            self.render_create_dialog(ui, scheduler, script_library, &mut response);
        }

        // Edit dialog
        if self.show_edit_dialog && self.edit_task_form.is_some() {
            self.render_edit_dialog(ui, scheduler, &mut response);
        }

        // History/Stats panel
        if self.show_history {
            self.render_statistics_panel(ui, scheduler);
        }

        response
    }

    fn render_task_list(
        &mut self,
        ui: &mut Ui,
        scheduler: &mut TaskScheduler,
        response: &mut ScheduledTasksResponse,
    ) {
        let tasks: Vec<_> = scheduler
            .get_all_tasks()
            .into_iter()
            .filter(|t| {
                // Filter by status
                if let Some(ref status) = self.filter_status {
                    if t.last_status.as_ref() != Some(status) && *status != TaskStatus::Pending {
                        return false;
                    }
                }

                // Filter by enabled
                if self.show_enabled_only && !t.enabled {
                    return false;
                }

                // Filter by search
                if !self.search_query.is_empty()
                    && !t.name.to_lowercase().contains(&self.search_query.to_lowercase())
                    && !t.description
                        .as_ref()
                        .map(|d| d.to_lowercase())
                        .unwrap_or_default()
                        .contains(&self.search_query.to_lowercase())
                {
                    return false;
                }

                true
            })
            .cloned()
            .collect();

        if tasks.is_empty() {
            ui.centered_and_justified(|ui| {
                ui.label(RichText::new("No scheduled tasks found").size(14.0));
                ui.label("Create a new task to get started");
            });
            return;
        }

        // Task table
        ScrollArea::vertical().show(ui, |ui| {
            for task in tasks {
                self.render_task_row(ui, &task, scheduler, response);
            }
        });
    }

    fn render_task_row(
        &mut self,
        ui: &mut Ui,
        task: &ScheduledTask,
        _scheduler: &TaskScheduler,
        response: &mut ScheduledTasksResponse,
    ) {
        let is_selected = self.selected_task.as_ref() == Some(&task.id);
        let is_bulk_selected = self.bulk_selection.contains(&task.id);

        let bg_color = if is_selected {
            Color32::from_rgb(40, 50, 70)
        } else if is_bulk_selected {
            Color32::from_rgb(50, 50, 40)
        } else {
            Color32::from_gray(35)
        };

        Frame::group(ui.style()).fill(bg_color).show(ui, |ui| {
            ui.set_min_width(ui.available_width());

            ui.horizontal(|ui| {
                // Bulk selection checkbox
                if self.show_bulk_actions {
                    let mut checked = is_bulk_selected;
                    if ui.checkbox(&mut checked, "").changed() {
                        if checked {
                            if !self.bulk_selection.contains(&task.id) {
                                self.bulk_selection.push(task.id.clone());
                            }
                        } else {
                            self.bulk_selection.retain(|id| id != &task.id);
                        }
                    }
                    ui.add_space(4.0);
                }

                // Status indicator
                let status_color = match task.last_status {
                    Some(TaskStatus::Running) => Color32::GREEN,
                    Some(TaskStatus::Completed) => Color32::BLUE,
                    Some(TaskStatus::Failed) => Color32::RED,
                    Some(TaskStatus::TimedOut) => Color32::from_rgb(255, 165, 0),
                    Some(TaskStatus::Cancelled) => Color32::GRAY,
                    _ => Color32::GRAY,
                };
                ui.colored_label(status_color, "●");

                // Enable toggle
                let mut enabled = task.enabled;
                if ui.checkbox(&mut enabled, "").changed() {
                    response.toggle_enabled = Some((task.id.clone(), enabled));
                }

                // Task name
                let name_color = if task.enabled {
                    Color32::WHITE
                } else {
                    Color32::GRAY
                };
                if ui
                    .selectable_label(is_selected, RichText::new(&task.name).color(name_color))
                    .clicked()
                {
                    self.selected_task = Some(task.id.clone());
                    self.show_task_details = !self.show_task_details;
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    // Quick actions
                    if ui.button("▶ Run Now").clicked() {
                        response.run_now = Some(task.id.clone());
                    }
                    if ui.button("✏ Edit").clicked() {
                        self.edit_task_form = Some(task.clone());
                        self.show_edit_dialog = true;
                    }
                    if ui.button("🗑").clicked() {
                        response.delete_task = Some(task.id.clone());
                    }
                });
            });

            // Task details
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(&task.schedule_description)
                        .size(12.0)
                        .color(Color32::LIGHT_GRAY),
                );
                ui.label("|");
                ui.label(
                    RichText::new(format!("Workflow: {}", task.workflow_id)).size(12.0),
                );
                if !task.target_servers.is_empty() {
                    ui.label("|");
                    ui.label(
                        RichText::new(format!("{} servers", task.target_servers.len())).size(12.0),
                    );
                }
            });

            // Next run / Last run
            ui.horizontal(|ui| {
                if let Some(next) = task.next_run {
                    ui.label(
                        RichText::new(format!("⏰ Next: {}", next.format("%Y-%m-%d %H:%M")))
                            .size(11.0)
                            .color(Color32::LIGHT_GREEN),
                    );
                }
                if let Some(last) = task.last_run {
                    ui.label("|");
                    let status_icon = match task.last_status {
                        Some(TaskStatus::Completed) => "✓",
                        Some(TaskStatus::Failed) => "✗",
                        _ => "○",
                    };
                    ui.label(
                        RichText::new(format!("{} Last: {}", status_icon, last.format("%Y-%m-%d %H:%M")))
                            .size(11.0),
                    );
                }
                if task.total_runs > 0 {
                    ui.label("|");
                    let success_rate = (task.successful_runs as f32 / task.total_runs as f32) * 100.0;
                    ui.label(
                        RichText::new(format!(
                            "📊 {} runs ({:.0}% success)",
                            task.total_runs, success_rate
                        ))
                        .size(11.0)
                        .color(if success_rate >= 80.0 {
                            Color32::GREEN
                        } else if success_rate >= 50.0 {
                            Color32::YELLOW
                        } else {
                            Color32::RED
                        }),
                    );
                }
            });

            // Description if present
            if let Some(ref desc) = task.description {
                if !desc.is_empty() {
                    ui.label(
                        RichText::new(desc)
                            .size(11.0)
                            .color(Color32::from_gray(180))
                            .italics(),
                    );
                }
            }
        });

        ui.add_space(4.0);
    }

    fn render_create_dialog(
        &mut self,
        ui: &mut Ui,
        _scheduler: &mut TaskScheduler,
        script_library: &ScriptLibrary,
        response: &mut ScheduledTasksResponse,
    ) {
        let id = ui.make_persistent_id("create_task_dialog");
        egui::Window::new("Create Scheduled Task")
            .id(id)
            .collapsible(false)
            .resizable(true)
            .default_size([500.0, 650.0])
            .show(ui.ctx(), |ui| {
                ScrollArea::vertical().show(ui, |ui| {
                    ui.label("Task Name:");
                    ui.text_edit_singleline(&mut self.new_task_form.name);

                    ui.label("Description:");
                    ui.text_edit_multiline(&mut self.new_task_form.description);

                    ui.separator();

                    // Workflow selection
                    ui.label(RichText::new("Select Workflow:").strong());
                    let workflows = script_library.list_workflows();
                    for workflow_entry in workflows {
                        let is_selected = self.new_task_form.workflow_id == workflow_entry.id;
                        if ui
                            .selectable_label(
                                is_selected,
                                format!("{} ({})", workflow_entry.workflow.name, workflow_entry.id),
                            )
                            .clicked()
                        {
                            self.new_task_form.workflow_id = workflow_entry.id.clone();
                        }
                    }

                    ui.separator();

                    // Schedule configuration
                    ui.label(RichText::new("Schedule Configuration").strong());
                    ui.label("Cron Expression:");
                    ui.text_edit_singleline(&mut self.new_task_form.cron_expression);

                    // Quick presets
                    ui.horizontal_wrapped(|ui| {
                        ui.label("Presets:");
                        let presets = vec![
                            ("0 * * * *", "Hourly"),
                            ("0 0 * * *", "Daily"),
                            ("0 0 * * 0", "Weekly (Sun)"),
                            ("0 0 * * 1", "Weekly (Mon)"),
                            ("0 0 1 * *", "Monthly"),
                            ("*/15 * * * *", "Every 15 min"),
                            ("*/30 * * * *", "Every 30 min"),
                        ];
                        for (cron, label) in presets {
                            if ui.button(label).clicked() {
                                self.new_task_form.cron_expression = cron.to_string();
                            }
                        }
                    });

                    // Validate cron
                    if !self.new_task_form.cron_expression.is_empty() {
                        match CronSchedule::parse(&self.new_task_form.cron_expression) {
                            Ok(schedule) => {
                                ui.colored_label(
                                    Color32::GREEN,
                                    format!("✓ {}", schedule.describe()),
                                );
                            }
                            Err(e) => {
                                ui.colored_label(Color32::RED, format!("✗ {}", e));
                            }
                        }
                    }

                    ui.separator();

                    // Target servers
                    ui.label(RichText::new("Target Servers:").strong());
                    ui.label("(Select servers to run this task on)");
                    ui.label("• Server selection from server list");

                    ui.separator();

                    // Timeout
                    ui.label("Timeout (minutes):");
                    ui.add(egui::Slider::new(
                        &mut self.new_task_form.timeout_minutes,
                        1..=180,
                    ));

                    ui.separator();

                    // Notifications
                    ui.label(RichText::new("Notifications:").strong());
                    ui.checkbox(
                        &mut self.new_task_form.email_notifications,
                        "Enable email notifications",
                    );
                    if self.new_task_form.email_notifications {
                        ui.label("Email address:");
                        ui.text_edit_singleline(&mut self.new_task_form.notification_email);
                    }

                    ui.separator();

                    // Actions
                    ui.horizontal(|ui| {
                        if ui.button("Create Task").clicked() {
                            if let Ok(mut task) = ScheduledTask::new(
                                &self.new_task_form.name,
                                &self.new_task_form.workflow_id,
                                &self.new_task_form.cron_expression,
                            ) {
                                // Set additional properties
                                if !self.new_task_form.description.is_empty() {
                                    task.description = Some(self.new_task_form.description.clone());
                                }
                                task.timeout_minutes = self.new_task_form.timeout_minutes;
                                response.create_task = Some(task);
                                self.show_create_dialog = false;
                            }
                        }
                        if ui.button("Cancel").clicked() {
                            self.show_create_dialog = false;
                        }
                    });
                });
            });
    }

    fn render_edit_dialog(
        &mut self,
        ui: &mut Ui,
        _scheduler: &mut TaskScheduler,
        response: &mut ScheduledTasksResponse,
    ) {
        let id = ui.make_persistent_id("edit_task_dialog");
        if let Some(ref mut task) = self.edit_task_form {
            egui::Window::new("Edit Scheduled Task")
                .id(id)
                .collapsible(false)
                .resizable(true)
                .default_size([450.0, 400.0])
                .show(ui.ctx(), |ui| {
                    ScrollArea::vertical().show(ui, |ui| {
                        ui.label("Task Name:");
                        ui.text_edit_singleline(&mut task.name);

                        ui.label("Description:");
                        let mut desc = task.description.clone().unwrap_or_default();
                        if ui.text_edit_multiline(&mut desc).changed() {
                            task.description = if desc.is_empty() { None } else { Some(desc) };
                        }

                        ui.label("Cron Expression:");
                        ui.text_edit_singleline(&mut task.cron_expression);

                        // Validate and update description
                        match CronSchedule::parse(&task.cron_expression) {
                            Ok(schedule) => {
                                ui.colored_label(
                                    Color32::GREEN,
                                    format!("✓ {}", schedule.describe()),
                                );
                                task.schedule_description = schedule.describe();
                            }
                            Err(e) => {
                                ui.colored_label(Color32::RED, format!("✗ {}", e));
                            }
                        }

                        ui.separator();

                        ui.label("Timeout (minutes):");
                        ui.add(egui::Slider::new(&mut task.timeout_minutes, 1..=180));

                        ui.separator();

                        ui.horizontal(|ui| {
                            if ui.button("💾 Save Changes").clicked() {
                                response.update_task = Some(task.clone());
                                self.show_edit_dialog = false;
                            }
                            if ui.button("Cancel").clicked() {
                                self.show_edit_dialog = false;
                            }
                        });
                    });
                });
        }
    }

    fn render_statistics_panel(&mut self, ui: &mut Ui, scheduler: &TaskScheduler) {
        let id = ui.make_persistent_id("statistics_panel");

        egui::Window::new("Task Statistics")
            .id(id)
            .collapsible(true)
            .resizable(true)
            .default_size([500.0, 400.0])
            .show(ui.ctx(), |ui| {
                ui.horizontal(|ui| {
                    ui.heading("Scheduled Tasks Overview");
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("✕").clicked() {
                            self.show_history = false;
                        }
                    });
                });

                ui.separator();

                let all_tasks = scheduler.get_all_tasks();

                // Statistics cards
                ui.horizontal(|ui| {
                    let total = all_tasks.len();
                    let enabled = all_tasks.iter().filter(|t| t.enabled).count();
                    let running = scheduler.get_running_tasks().len();
                    let with_failures = all_tasks.iter().filter(|t| t.failed_runs > 0).count();

                    // Total tasks
                    Frame::group(ui.style())
                        .fill(Color32::from_gray(35))
                        .show(ui, |ui| {
                            ui.set_min_width(100.0);
                            ui.vertical_centered(|ui| {
                                ui.label(RichText::new("Total").size(11.0).color(Color32::GRAY));
                                ui.label(RichText::new(total.to_string()).size(22.0).strong());
                            });
                        });

                    // Enabled
                    Frame::group(ui.style())
                        .fill(Color32::from_rgb(30, 50, 30))
                        .show(ui, |ui| {
                            ui.set_min_width(100.0);
                            ui.vertical_centered(|ui| {
                                ui.label(RichText::new("Enabled").size(11.0).color(Color32::GRAY));
                                ui.label(
                                    RichText::new(enabled.to_string())
                                        .size(22.0)
                                        .strong()
                                        .color(Color32::GREEN),
                                );
                            });
                        });

                    // Running
                    Frame::group(ui.style())
                        .fill(Color32::from_rgb(20, 40, 60))
                        .show(ui, |ui| {
                            ui.set_min_width(100.0);
                            ui.vertical_centered(|ui| {
                                ui.label(RichText::new("Running").size(11.0).color(Color32::GRAY));
                                ui.label(
                                    RichText::new(running.to_string())
                                        .size(22.0)
                                        .strong()
                                        .color(Color32::BLUE),
                                );
                        });
                    });

                    // With failures
                    Frame::group(ui.style())
                        .fill(Color32::from_rgb(50, 30, 30))
                        .show(ui, |ui| {
                            ui.set_min_width(100.0);
                            ui.vertical_centered(|ui| {
                                ui.label(RichText::new("Failed").size(11.0).color(Color32::GRAY));
                                ui.label(
                                    RichText::new(with_failures.to_string())
                                        .size(22.0)
                                        .strong()
                                        .color(Color32::RED),
                                );
                            });
                        });
                });

                ui.add_space(16.0);

                // Tasks by status
                ui.label(RichText::new("Tasks by Status:").strong());

                let pending = all_tasks.iter().filter(|t| t.last_status == Some(TaskStatus::Pending)).count();
                let completed = all_tasks.iter().filter(|t| t.last_status == Some(TaskStatus::Completed)).count();
                let failed = all_tasks.iter().filter(|t| t.last_status == Some(TaskStatus::Failed)).count();
                let timed_out = all_tasks.iter().filter(|t| t.last_status == Some(TaskStatus::TimedOut)).count();

                let statuses = vec![
                    ("Pending", pending, Color32::YELLOW),
                    ("Completed", completed, Color32::BLUE),
                    ("Failed", failed, Color32::RED),
                    ("Timed Out", timed_out, Color32::from_rgb(255, 165, 0)),
                ];

                for (name, count, color) in statuses {
                    if count > 0 {
                        ui.horizontal(|ui| {
                            ui.label(format!("{}:", name));
                            ui.colored_label(color, format!("{}", count));
                        });
                    }
                }

                ui.add_space(16.0);

                // Total execution stats
                let total_runs: u64 = all_tasks.iter().map(|t| t.total_runs).sum();
                let total_successful: u64 = all_tasks.iter().map(|t| t.successful_runs).sum();
                let total_failed: u64 = all_tasks.iter().map(|t| t.failed_runs).sum();

                if total_runs > 0 {
                    ui.label(RichText::new("Total Execution Statistics:").strong());
                    ui.label(format!("Total runs: {}", total_runs));
                    ui.label(format!("Successful: {}", total_successful));
                    ui.label(format!("Failed: {}", total_failed));

                    let overall_success_rate = (total_successful as f32 / total_runs as f32) * 100.0;
                    ui.label(
                        RichText::new(format!("Overall success rate: {:.1}%", overall_success_rate))
                            .color(if overall_success_rate >= 80.0 {
                                Color32::GREEN
                            } else if overall_success_rate >= 50.0 {
                                Color32::YELLOW
                            } else {
                                Color32::RED
                            }),
                    );
                }
            });
    }

    /// Show the statistics panel
    pub fn show_statistics(&mut self) {
        self.show_history = true;
    }
}

impl Default for ScheduledTasksPanel {
    fn default() -> Self {
        Self::new()
    }
}
