#![allow(dead_code)]

use eframe::egui;
use egui::{RichText, Ui};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use easyssh_core::workflow_engine::*;
use easyssh_core::workflow_executor::*;
use easyssh_core::workflow_scheduler::*;
use easyssh_core::workflow_variables::ServerContext;
use easyssh_core::script_library::{ScriptLibrary, ScriptType};

use crate::workflow_editor::{WorkflowEditor, ScriptLibraryBrowser};
use crate::macro_recorder_ui::MacroRecorderPanel;
use crate::scheduled_tasks_ui::ScheduledTasksPanel;
use crate::batch_results_ui::BatchExecutionResultsPanel;

/// Main workflow automation panel integrating all features
pub struct WorkflowPanel {
    /// Active tab
    active_tab: WorkflowTab,
    /// Script library
    script_library: Arc<Mutex<ScriptLibrary>>,
    /// Task scheduler
    scheduler: Arc<Mutex<TaskScheduler>>,
    /// Workflow executor
    executor: Arc<Mutex<WorkflowExecutor>>,
    /// Workflow editor
    workflow_editor: Option<WorkflowEditor>,
    /// Script library browser
    library_browser: ScriptLibraryBrowser,
    /// Macro recorder panel
    macro_recorder: MacroRecorderPanel,
    /// Scheduled tasks panel
    scheduled_tasks: ScheduledTasksPanel,
    /// Batch results panel
    batch_results: Option<BatchExecutionResultsPanel>,
    /// Selected workflow for execution
    selected_workflow: Option<Workflow>,
    /// Target servers for execution
    target_servers: Vec<ServerContext>,
    /// Execution in progress
    execution_in_progress: bool,
    /// Last execution summary
    last_execution_summary: Option<BatchExecutionSummary>,
    /// Show execution dialog
    show_execution_dialog: bool,
    /// New workflow name dialog
    show_new_workflow_dialog: bool,
    new_workflow_name: String,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum WorkflowTab {
    Library,
    Editor,
    Recorder,
    Scheduler,
    Results,
}

impl WorkflowPanel {
    pub fn new() -> Self {
        // Initialize script library with storage path
        let storage_path = dirs::data_local_dir()
            .map(|d| d.join("EasySSH").join("scripts"))
            .unwrap_or_else(|| PathBuf::from("./scripts"));

        let script_library = Arc::new(Mutex::new(ScriptLibrary::new(storage_path)));

        // Add sample workflows if library is empty
        {
            let mut lib = script_library.lock().unwrap();
            if lib.list_workflows().is_empty() {
                // Add template workflows
                let deployment = WorkflowTemplates::deployment_workflow();
                lib.add_workflow(deployment.clone(), None);

                let backup = WorkflowTemplates::backup_workflow();
                lib.add_workflow(backup, None);

                let update = WorkflowTemplates::system_update_workflow();
                lib.add_workflow(update, None);
            }
        }

        Self {
            active_tab: WorkflowTab::Library,
            script_library: script_library.clone(),
            scheduler: Arc::new(Mutex::new(TaskScheduler::new())),
            executor: Arc::new(Mutex::new(WorkflowExecutor::new())),
            workflow_editor: None,
            library_browser: ScriptLibraryBrowser::new(),
            macro_recorder: MacroRecorderPanel::new(),
            scheduled_tasks: ScheduledTasksPanel::new(),
            batch_results: None,
            selected_workflow: None,
            target_servers: Vec::new(),
            execution_in_progress: false,
            last_execution_summary: None,
            show_execution_dialog: false,
            show_new_workflow_dialog: false,
            new_workflow_name: String::new(),
        }
    }

    pub fn ui(&mut self, ui: &mut Ui, view_model: &crate::viewmodels::AppViewModel) -> WorkflowPanelResponse {
        let mut response = WorkflowPanelResponse::default();

        // Tab bar
        ui.horizontal(|ui| {
            ui.selectable_value(&mut self.active_tab, WorkflowTab::Library, "📚 Library");
            ui.selectable_value(&mut self.active_tab, WorkflowTab::Editor, "✏ Editor");
            ui.selectable_value(&mut self.active_tab, WorkflowTab::Recorder, "⏺ Recorder");
            ui.selectable_value(&mut self.active_tab, WorkflowTab::Scheduler, "⏰ Scheduler");
            ui.selectable_value(&mut self.active_tab, WorkflowTab::Results, "📊 Results");

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("+ New Workflow").clicked() {
                    self.show_new_workflow_dialog = true;
                }
            });
        });

        ui.separator();

        // Tab content
        match self.active_tab {
            WorkflowTab::Library => {
                self.render_library_tab(ui, &mut response);
            }
            WorkflowTab::Editor => {
                self.render_editor_tab(ui, &mut response);
            }
            WorkflowTab::Recorder => {
                self.render_recorder_tab(ui, view_model, &mut response);
            }
            WorkflowTab::Scheduler => {
                self.render_scheduler_tab(ui, &mut response);
            }
            WorkflowTab::Results => {
                self.render_results_tab(ui, &mut response);
            }
        }

        // Dialogs
        if self.show_new_workflow_dialog {
            self.render_new_workflow_dialog(ui, &mut response);
        }

        if self.show_execution_dialog {
            self.render_execution_dialog(ui, &mut response);
        }

        response
    }

    fn render_library_tab(&mut self, ui: &mut Ui, _response: &mut WorkflowPanelResponse) {
        // Split view: browser on left, details on right
        let available_width = ui.available_width();
        let browser_width = available_width * 0.6;

        // Collect any navigation actions to perform after releasing locks
        let mut navigate_to_editor = false;
        let mut navigate_to_recorder = false;
        let mut new_workflow_to_select: Option<Workflow> = None;

        ui.horizontal(|ui| {
            // Browser
            ui.vertical(|ui| {
                ui.set_width(browser_width);

                let lib = self.script_library.lock().unwrap();
                let selection = self.library_browser.ui(ui, &lib);

                // Handle selection - clone data while holding lock, then process after
                if let Some(selection) = selection {
                    // Load selected script
                    match selection.script_type {
                        ScriptType::Workflow => {
                            if let Some(workflow) = lib.get_workflow(&selection.id) {
                                new_workflow_to_select = Some(workflow.clone());
                                navigate_to_editor = true;
                            }
                        }
                        ScriptType::Macro => {
                            if let Some(macr) = lib.get_macro(&selection.id) {
                                // Clone macro data for later use
                                let macro_clone = macr.clone();
                                drop(lib); // Drop lock before mutating self
                                self.macro_recorder.load_macro(macro_clone);
                                navigate_to_recorder = true;
                                return; // Exit early since we've dropped lib
                            }
                        }
                        _ => {}
                    }
                }
                drop(lib); // Ensure lock is dropped
            });

            ui.separator();

            // Apply any deferred state changes
            if let Some(workflow) = new_workflow_to_select {
                self.selected_workflow = Some(workflow.clone());
                self.workflow_editor = Some(WorkflowEditor::new(workflow));
            }
            if navigate_to_editor {
                self.active_tab = WorkflowTab::Editor;
            }
            if navigate_to_recorder {
                self.active_tab = WorkflowTab::Recorder;
            }

            // Details panel
            ui.vertical(|ui| {
                ui.heading("Details");

                if let Some(ref workflow) = self.selected_workflow {
                    ui.label(RichText::new(&workflow.name).strong().size(18.0));
                    if let Some(ref desc) = workflow.description {
                        ui.label(desc);
                    }

                    ui.separator();

                    ui.label(format!("Steps: {}", workflow.steps.len()));
                    ui.label(format!("Version: {}", workflow.version));

                    if let Some(ref cat) = workflow.category {
                        ui.label(format!("Category: {}", cat));
                    }

                    ui.separator();

                    if ui.button("▶ Execute").clicked() {
                        self.show_execution_dialog = true;
                    }
                    if ui.button("✏ Edit").clicked() {
                        // Clone workflow for editor
                        let workflow = workflow.clone();
                        self.workflow_editor = Some(WorkflowEditor::new(workflow));
                        self.active_tab = WorkflowTab::Editor;
                    }
                    if ui.button("📋 Duplicate").clicked() {
                        let mut new_workflow = workflow.clone();
                        new_workflow.id = uuid::Uuid::new_v4().to_string();
                        new_workflow.name = format!("{} (Copy)", new_workflow.name);
                        new_workflow.updated_at = chrono::Utc::now();

                        // Lock, add workflow, then immediately drop lock
                        {
                            let mut lib = self.script_library.lock().unwrap();
                            lib.add_workflow(new_workflow.clone(), None);
                        } // lock dropped here

                        self.selected_workflow = Some(new_workflow);
                    }
                } else {
                    ui.label("Select a workflow from the library to view details");
                }
            });
        });
    }

    fn render_editor_tab(&mut self, ui: &mut Ui, _response: &mut WorkflowPanelResponse) {
        if let Some(ref mut editor) = self.workflow_editor {
            let editor_response = editor.ui(ui);
            editor.process_response(&editor_response);

            if editor_response.save_requested {
                let workflow = editor.workflow().clone();
                // Scope the lock tightly
                {
                    let mut lib = self.script_library.lock().unwrap();
                    lib.update_workflow(&workflow.id, workflow.clone()).ok();
                } // lock dropped here
                self.selected_workflow = Some(workflow);
            }

            if editor_response.run_requested {
                self.show_execution_dialog = true;
            }
        } else {
            ui.centered_and_justified(|ui| {
                ui.label("No workflow selected");
                if ui.button("Create New Workflow").clicked() {
                    self.show_new_workflow_dialog = true;
                }
                if ui.button("Browse Library").clicked() {
                    self.active_tab = WorkflowTab::Library;
                }
            });
        }
    }

    fn render_recorder_tab(&mut self, ui: &mut Ui, _view_model: &crate::viewmodels::AppViewModel, response: &mut WorkflowPanelResponse) {
        let recorder_response = self.macro_recorder.ui(ui);

        if let Some(macr) = recorder_response.save_macro {
            let mut lib = self.script_library.lock().unwrap();
            let id = lib.add_macro(macr, None);
            drop(lib);

            // Convert macro to workflow option
            response.macro_saved = Some(id);
        }

        if recorder_response.playback_requested {
            response.playback_started = true;
        }
    }

    fn render_scheduler_tab(&mut self, ui: &mut Ui, response: &mut WorkflowPanelResponse) {
        // Collect responses from the scheduler UI without holding locks across self calls
        let scheduler_response = {
            let mut scheduler = self.scheduler.lock().unwrap();
            let lib = self.script_library.lock().unwrap();
            self.scheduled_tasks.ui(ui, &mut scheduler, &lib)
        }; // both locks dropped here

        // Now process responses without any locks held
        if let Some(task) = scheduler_response.create_task {
            let mut scheduler = self.scheduler.lock().unwrap();
            let id = scheduler.add_task(task).unwrap();
            response.task_created = Some(id);
        }

        if let Some(task) = scheduler_response.update_task {
            let mut scheduler = self.scheduler.lock().unwrap();
            let task_id = task.id.clone();
            scheduler.update_task(&task_id, task).ok();
            response.task_updated = Some(task_id);
        }

        if let Some(task_id) = scheduler_response.delete_task {
            let mut scheduler = self.scheduler.lock().unwrap();
            scheduler.remove_task(&task_id);
            response.task_deleted = Some(task_id);
        }

        if let Some((task_id, enabled)) = scheduler_response.toggle_enabled {
            let mut scheduler = self.scheduler.lock().unwrap();
            scheduler.set_task_enabled(&task_id, enabled).ok();
        }
    }

    fn render_results_tab(&mut self, ui: &mut Ui, response: &mut WorkflowPanelResponse) {
        if let Some(ref mut results_panel) = self.batch_results {
            let results_response = results_panel.ui(ui);

            if results_response.export_requested {
                response.export_results_requested = true;
            }

            if results_response.close_requested {
                self.batch_results = None;
            }
        } else if let Some(ref summary) = self.last_execution_summary {
            // Create results panel on demand
            self.batch_results = Some(BatchExecutionResultsPanel::with_summary(summary.clone()));
        } else {
            ui.centered_and_justified(|ui| {
                ui.label("No execution results available");
                ui.label("Run a workflow to see results here");
            });
        }
    }

    fn render_new_workflow_dialog(&mut self, ui: &mut Ui, response: &mut WorkflowPanelResponse) {
        let id = ui.make_persistent_id("new_workflow_dialog");

        // Collect dialog interaction results first
        let mut create_clicked = false;
        let mut cancel_clicked = false;
        let mut name_to_use = String::new();

        egui::Window::new("Create New Workflow")
            .id(id)
            .collapsible(false)
            .resizable(false)
            .show(ui.ctx(), |ui| {
                ui.label("Workflow Name:");
                ui.text_edit_singleline(&mut self.new_workflow_name);

                ui.horizontal(|ui| {
                    create_clicked = ui.button("Create").clicked() && !self.new_workflow_name.is_empty();
                    cancel_clicked = ui.button("Cancel").clicked();
                });
            });

        // Process results after UI interaction
        if create_clicked {
            name_to_use = self.new_workflow_name.clone();
        }

        if create_clicked && !name_to_use.is_empty() {
            let workflow = Workflow::new(&name_to_use);

            // Scope the lock tightly
            let workflow_id = {
                let mut lib = self.script_library.lock().unwrap();
                lib.add_workflow(workflow.clone(), None)
            }; // lock dropped here

            self.selected_workflow = Some(workflow.clone());
            self.workflow_editor = Some(WorkflowEditor::new(workflow));
            self.active_tab = WorkflowTab::Editor;

            self.new_workflow_name.clear();
            self.show_new_workflow_dialog = false;

            response.workflow_created = Some(workflow_id);
        }

        if cancel_clicked {
            self.show_new_workflow_dialog = false;
        }
    }

    fn render_execution_dialog(&mut self, ui: &mut Ui, response: &mut WorkflowPanelResponse) {
        let id = ui.make_persistent_id("execution_dialog");
        egui::Window::new("Execute Workflow")
            .id(id)
            .collapsible(false)
            .resizable(true)
            .default_size([400.0, 300.0])
            .show(ui.ctx(), |ui| {
                if let Some(ref workflow) = self.selected_workflow {
                    ui.label(format!("Workflow: {}", workflow.name));
                    ui.separator();

                    ui.label("Target Servers:");
                    ui.label("(Server selection would be implemented here)");

                    ui.separator();

                    ui.label("Execution Mode:");
                    ui.radio_value(&mut response.execution_parallel, true, "Parallel (fast)");
                    ui.radio_value(&mut response.execution_parallel, false, "Sequential (safe)");

                    if response.execution_parallel {
                        ui.label("Max parallel executions:");
                        ui.add(egui::Slider::new(&mut response.max_parallel, 1..=10));
                    }

                    ui.separator();

                    ui.horizontal(|ui| {
                        if ui.button("Execute").clicked() {
                            response.execution_requested = true;
                            self.show_execution_dialog = false;
                            self.execution_in_progress = true;
                        }
                        if ui.button("Cancel").clicked() {
                            self.show_execution_dialog = false;
                        }
                    });
                }
            });
    }

    /// Get available workflows for execution
    pub fn get_available_workflows(&self) -> Vec<(String, String)> {
        let lib = self.script_library.lock().unwrap();
        lib.list_workflows()
            .into_iter()
            .map(|e| (e.id.clone(), e.workflow.name.clone()))
            .collect()
    }

    /// Execute workflow on selected servers
    pub async fn execute_workflow(
        &mut self,
        workflow: &Workflow,
        servers: Vec<ServerContext>,
        parallel: bool,
        max_parallel: usize,
    ) -> BatchExecutionSummary {
        let executor = self.executor.lock().unwrap();

        let results = if parallel {
            executor.execute_parallel(workflow, servers, max_parallel).await
        } else {
            executor.execute_sequential(workflow, servers).await
        };

        let summary = BatchExecutionSummary::from_results(
            uuid::Uuid::new_v4().to_string(),
            workflow.id.clone(),
            results,
        );

        self.last_execution_summary = Some(summary.clone());
        self.batch_results = Some(BatchExecutionResultsPanel::with_summary(summary.clone()));
        self.execution_in_progress = false;
        self.active_tab = WorkflowTab::Results;

        summary
    }

    pub fn is_execution_in_progress(&self) -> bool {
        self.execution_in_progress
    }

    pub fn set_target_servers(&mut self, servers: Vec<ServerContext>) {
        self.target_servers = servers;
    }
}

/// Response from workflow panel
#[derive(Debug, Default)]
pub struct WorkflowPanelResponse {
    pub workflow_created: Option<String>,
    pub workflow_updated: Option<String>,
    pub macro_saved: Option<String>,
    pub task_created: Option<String>,
    pub task_updated: Option<String>,
    pub task_deleted: Option<String>,
    pub execution_requested: bool,
    pub execution_parallel: bool,
    pub max_parallel: usize,
    pub playback_started: bool,
    pub export_results_requested: bool,
}
