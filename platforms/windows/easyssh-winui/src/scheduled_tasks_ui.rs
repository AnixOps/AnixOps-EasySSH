#![allow(dead_code)]

use eframe::egui;
use egui::{Color32, Frame, RichText, Ui};

use easyssh_core::script_library::ScriptLibrary;
use easyssh_core::workflow_scheduler::*;

/// Scheduled tasks management UI
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
    /// Execution history
    show_history: bool,
    /// Selected history task
    history_task_id: Option<String>,
}

#[derive(Debug, Default)]
struct NewTaskForm {
    name: String,
    description: String,
    workflow_id: String,
    cron_expression: String,
    server_ids: Vec<String>,
    timeout_minutes: u64,
}

impl ScheduledTasksPanel {
    pub fn new() -> Self {
        Self {
            selected_task: None,
            show_create_dialog: false,
            show_edit_dialog: false,
            new_task_form: NewTaskForm {
                timeout_minutes: 30,
                ..Default::default()
            },
            edit_task_form: None,
            filter_status: None,
            search_query: String::new(),
            show_enabled_only: false,
            show_history: false,
            history_task_id: None,
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

                ui.checkbox(&mut self.show_enabled_only, "Enabled only");
                ui.text_edit_singleline(&mut self.search_query);
                ui.label("Search:");
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
        });

        ui.separator();

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
                    && !t
                        .name
                        .to_lowercase()
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
                ui.label("No scheduled tasks found");
            });
            return;
        }

        // Running tasks section
        let running = scheduler.get_running_tasks();
        if !running.is_empty() {
            ui.colored_label(
                Color32::GREEN,
                format!("▶ {} task(s) running", running.len()),
            );

            for handle in running {
                ui.horizontal(|ui| {
                    ui.label("●");
                    if let Some(task) = scheduler.get_task(&handle.task_id) {
                        ui.label(&task.name);
                    }
                    ui.label(format!(
                        "(started {})",
                        handle.started_at.format("%H:%M:%S")
                    ));
                });
            }
            ui.separator();
        }

        // Task table
        egui::ScrollArea::vertical().show(ui, |ui| {
            for task in tasks {
                self.render_task_row(ui, &task, response);
            }
        });
    }

    fn render_task_row(
        &mut self,
        ui: &mut Ui,
        task: &ScheduledTask,
        response: &mut ScheduledTasksResponse,
    ) {
        let is_selected = self.selected_task.as_ref() == Some(&task.id);

        let bg_color = if is_selected {
            Color32::from_rgb(40, 50, 70)
        } else {
            Color32::from_gray(35)
        };

        Frame::group(ui.style()).fill(bg_color).show(ui, |ui| {
            ui.set_min_width(ui.available_width());

            ui.horizontal(|ui| {
                // Status indicator
                let status_color = match task.last_status {
                    Some(TaskStatus::Running) => Color32::GREEN,
                    Some(TaskStatus::Completed) => Color32::BLUE,
                    Some(TaskStatus::Failed) => Color32::RED,
                    Some(TaskStatus::TimedOut) => Color32::from_rgb(255, 165, 0),
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
                ui.colored_label(name_color, &task.name);

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    // Actions
                    if ui.button("Run Now").clicked() {
                        response.run_now = Some(task.id.clone());
                    }
                    if ui.button("Edit").clicked() {
                        self.edit_task_form = Some(task.clone());
                        self.show_edit_dialog = true;
                    }
                    if ui.button("🗑").clicked() {
                        response.delete_task = Some(task.id.clone());
                    }
                    if ui.button("History").clicked() {
                        self.history_task_id = Some(task.id.clone());
                        self.show_history = true;
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
                ui.label(RichText::new(format!("Workflow: {}", task.workflow_id)).size(12.0));
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
                        RichText::new(format!("Next: {}", next.format("%Y-%m-%d %H:%M")))
                            .size(11.0),
                    );
                }
                if let Some(last) = task.last_run {
                    ui.label("|");
                    ui.label(
                        RichText::new(format!("Last: {}", last.format("%Y-%m-%d %H:%M")))
                            .size(11.0),
                    );
                }
                if task.total_runs > 0 {
                    ui.label("|");
                    ui.label(
                        RichText::new(format!(
                            "Runs: {} successful, {} failed",
                            task.successful_runs, task.failed_runs
                        ))
                        .size(11.0),
                    );
                }
            });
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
            .default_size([500.0, 600.0])
            .show(ui.ctx(), |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    ui.label("Task Name:");
                    ui.text_edit_singleline(&mut self.new_task_form.name);

                    ui.label("Description:");
                    ui.text_edit_multiline(&mut self.new_task_form.description);

                    ui.separator();

                    // Workflow selection
                    ui.label("Select Workflow:");
                    let workflows = script_library.list_workflows();
                    for workflow_entry in workflows {
                        if ui
                            .radio(
                                self.new_task_form.workflow_id == workflow_entry.id,
                                &workflow_entry.workflow.name,
                            )
                            .clicked()
                        {
                            self.new_task_form.workflow_id = workflow_entry.id.clone();
                        }
                    }

                    ui.separator();

                    // Schedule configuration
                    ui.label("Schedule (Cron Expression):");
                    ui.text_edit_singleline(&mut self.new_task_form.cron_expression);

                    // Quick presets
                    ui.horizontal_wrapped(|ui| {
                        ui.label("Presets:");
                        let presets = vec![
                            ("0 * * * *", "Hourly"),
                            ("0 0 * * *", "Daily"),
                            ("0 0 * * 0", "Weekly"),
                            ("0 0 1 * *", "Monthly"),
                            ("*/15 * * * *", "Every 15 min"),
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

                    // Server selection
                    ui.label("Target Servers:");
                    ui.label("(Server selection UI would go here)");

                    ui.separator();

                    // Timeout
                    ui.label("Timeout (minutes):");
                    ui.add(egui::Slider::new(
                        &mut self.new_task_form.timeout_minutes,
                        1..=120,
                    ));

                    ui.separator();

                    // Actions
                    ui.horizontal(|ui| {
                        if ui.button("Create").clicked() {
                            if let Ok(task) = ScheduledTask::new(
                                &self.new_task_form.name,
                                &self.new_task_form.workflow_id,
                                &self.new_task_form.cron_expression,
                            ) {
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
                .show(ui.ctx(), |ui| {
                    ui.label("Task Name:");
                    ui.text_edit_singleline(&mut task.name);

                    ui.label("Description:");
                    let mut desc = task.description.clone().unwrap_or_default();
                    if ui.text_edit_multiline(&mut desc).changed() {
                        task.description = if desc.is_empty() { None } else { Some(desc) };
                    }

                    ui.label("Cron Expression:");
                    ui.text_edit_singleline(&mut task.cron_expression);

                    if let Ok(schedule) = CronSchedule::parse(&task.cron_expression) {
                        ui.colored_label(Color32::GREEN, format!("✓ {}", schedule.describe()));
                        task.schedule_description = schedule.describe();
                    }

                    ui.separator();

                    ui.horizontal(|ui| {
                        if ui.button("Save").clicked() {
                            response.update_task = Some(task.clone());
                            self.show_edit_dialog = false;
                        }
                        if ui.button("Cancel").clicked() {
                            self.show_edit_dialog = false;
                        }
                    });
                });
        }
    }
}

/// Response from scheduled tasks panel
#[derive(Debug, Default)]
pub struct ScheduledTasksResponse {
    pub create_task: Option<ScheduledTask>,
    pub update_task: Option<ScheduledTask>,
    pub delete_task: Option<String>,
    pub toggle_enabled: Option<(String, bool)>,
    pub run_now: Option<String>,
}
