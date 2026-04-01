#![allow(dead_code)]

use eframe::egui;
use egui::{Color32, Frame, ProgressBar, RichText, Ui};

use easyssh_core::macro_recorder::*;
use easyssh_core::workflow_variables::ServerContext;

/// Macro recorder UI component
pub struct MacroRecorderPanel {
    recorder: MacroRecorder,
    /// Recording name
    recording_name: String,
    /// Recording description
    recording_description: String,
    /// Selected server for recording context
    selected_server: Option<ServerContext>,
    /// Whether macro is being edited
    editing_macro: Option<Macro>,
    /// Currently selected action for editing
    selected_action: Option<String>,
    /// Replay speed
    replay_speed: f64,
    /// Use original timing
    use_original_timing: bool,
    /// Show save dialog
    show_save_dialog: bool,
    /// Macro name for saving
    save_name: String,
    /// Playback progress
    playback_progress: f32,
    /// Is playing
    is_playing: bool,
}

impl MacroRecorderPanel {
    pub fn new() -> Self {
        Self {
            recorder: MacroRecorder::new(),
            recording_name: String::new(),
            recording_description: String::new(),
            selected_server: None,
            editing_macro: None,
            selected_action: None,
            replay_speed: 1.0,
            use_original_timing: false,
            show_save_dialog: false,
            save_name: String::new(),
            playback_progress: 0.0,
            is_playing: false,
        }
    }

    pub fn ui(&mut self, ui: &mut Ui) -> MacroRecorderResponse {
        let mut response = MacroRecorderResponse::default();

        // Header with recording controls
        self.render_header(ui, &mut response);

        ui.separator();

        match self.recorder.get_state() {
            RecorderState::Idle => {
                self.render_idle_state(ui, &mut response);
            }
            RecorderState::Recording => {
                self.render_recording_state(ui, &mut response);
            }
            RecorderState::Paused => {
                self.render_paused_state(ui, &mut response);
            }
            _ => {}
        }

        // Macro editing view
        if self.editing_macro.is_some() {
            ui.separator();
            let macro_data = self.editing_macro.clone().unwrap();
            self.render_macro_editor(ui, &macro_data, &mut response);
        }

        // Save dialog
        if self.show_save_dialog {
            self.render_save_dialog(ui, &mut response);
        }

        response
    }

    fn render_header(&mut self, ui: &mut Ui, _response: &mut MacroRecorderResponse) {
        ui.horizontal(|ui| {
            ui.heading("Macro Recorder");

            ui.with_layout(
                egui::Layout::right_to_left(egui::Align::Center),
                |ui| match self.recorder.get_state() {
                    RecorderState::Idle => {
                        if ui.button("⏺ Start Recording").clicked()
                            && !self.recording_name.is_empty()
                        {
                            self.recorder.start_recording(
                                &self.recording_name,
                                self.selected_server.clone().map(|s| MacroServerContext {
                                    server_id: s.id.clone(),
                                    server_name: s.name.clone(),
                                    host: s.host.clone(),
                                    username: s.username.clone(),
                                    initial_dir: String::from("/"),
                                    env_vars: std::collections::HashMap::new(),
                                }),
                            );
                        }
                    }
                    RecorderState::Recording => {
                        ui.colored_label(Color32::RED, "● REC");

                        if ui.button("⏸ Pause").clicked() {
                            self.recorder.pause_recording();
                        }
                        if ui.button("⏹ Stop").clicked() {
                            let m = self.recorder.stop_recording();
                            if let Some(macr) = m {
                                self.editing_macro = Some(macr);
                                self.show_save_dialog = true;
                            }
                        }
                    }
                    RecorderState::Paused => {
                        ui.colored_label(Color32::YELLOW, "⏸ PAUSED");

                        if ui.button("⏺ Resume").clicked() {
                            self.recorder.resume_recording();
                        }
                        if ui.button("⏹ Stop").clicked() {
                            let m = self.recorder.stop_recording();
                            if let Some(macr) = m {
                                self.editing_macro = Some(macr);
                                self.show_save_dialog = true;
                            }
                        }
                    }
                    _ => {}
                },
            );
        });
    }

    fn render_idle_state(&mut self, ui: &mut Ui, _response: &mut MacroRecorderResponse) {
        ui.label("Configure your recording:");
        ui.add_space(8.0);

        ui.label("Recording Name:");
        ui.text_edit_singleline(&mut self.recording_name);

        ui.label("Description (optional):");
        ui.text_edit_multiline(&mut self.recording_description);

        ui.add_space(8.0);

        ui.label("Server Context:");
        if let Some(ref server) = self.selected_server {
            ui.label(format!("{} ({}) ", server.name, server.host));
        } else {
            ui.label("No server selected - recording will be server-agnostic");
        }

        ui.add_space(16.0);

        ui.label("The macro recorder will capture:");
        ui.label("• SSH commands executed");
        ui.label("• File transfers (upload/download)");
        ui.label("• Directory changes");
        ui.label("• Input responses");
        ui.label("• Waits and pauses");
    }

    fn render_recording_state(&mut self, ui: &mut Ui, _response: &mut MacroRecorderResponse) {
        let elapsed = self.recorder.get_recording_duration();
        let action_count = self.recorder.get_buffered_action_count();

        ui.horizontal(|ui| {
            ui.label("Recording: ");
            ui.colored_label(Color32::RED, &self.recording_name);
        });

        ui.label(format!("Duration: {}s", elapsed.as_secs()));
        ui.label(format!("Actions captured: {}", action_count));

        ui.add_space(16.0);

        // Real-time preview of captured actions
        if let Some(current) = self.recorder.get_current_macro() {
            ui.label("Recent Actions:");
            egui::ScrollArea::vertical()
                .max_height(200.0)
                .show(ui, |ui| {
                    for action in current.actions.iter().rev().take(10) {
                        self.render_action_preview(ui, action);
                    }
                });
        }
    }

    fn render_paused_state(&mut self, ui: &mut Ui, _response: &mut MacroRecorderResponse) {
        ui.colored_label(Color32::YELLOW, "Recording is paused");
        ui.label("Click Resume to continue recording, or Stop to finish.");

        let elapsed = self.recorder.get_recording_duration();
        let action_count = self.recorder.get_buffered_action_count();

        ui.label(format!("Duration so far: {}s", elapsed.as_secs()));
        ui.label(format!("Actions captured: {}", action_count));
    }

    fn render_action_preview(&self, ui: &mut Ui, action: &MacroAction) {
        let (icon, text) = match &action.data {
            MacroActionData::SshCommand { command, .. } => ("$", command.clone()),
            MacroActionData::FileUpload {
                local_path,
                remote_path,
                ..
            } => ("↑", format!("Upload: {} → {}", local_path, remote_path)),
            MacroActionData::FileDownload {
                remote_path,
                local_path,
                ..
            } => ("↓", format!("Download: {} → {}", remote_path, local_path)),
            MacroActionData::Wait { duration_secs, .. } => {
                ("◷", format!("Wait {}s", duration_secs))
            }
            MacroActionData::ChangeDirectory { path } => ("📁", format!("cd {}", path)),
            MacroActionData::ProvideInput {
                input,
                is_sensitive,
                ..
            } => {
                let display = if *is_sensitive {
                    "***".to_string()
                } else {
                    input.clone()
                };
                ("⌨", format!("Input: {}", display))
            }
            MacroActionData::LocalCommand { command, .. } => ("⌘", command.clone()),
            _ => ("•", "Action".to_string()),
        };

        ui.horizontal(|ui| {
            ui.label(icon);
            ui.label(text);
            ui.label(format!("+{}ms", action.delay_ms));
        });
    }

    fn render_macro_editor(
        &mut self,
        ui: &mut Ui,
        macro_data: &Macro,
        response: &mut MacroRecorderResponse,
    ) {
        ui.heading("Macro Editor");

        // Playback controls
        ui.horizontal(|ui| {
            if ui.button("▶ Play").clicked() {
                self.is_playing = true;
                response.playback_requested = true;
            }
            if ui.button("⏸ Pause").clicked() {
                self.is_playing = false;
            }
            if ui.button("⏹ Stop").clicked() {
                self.is_playing = false;
                self.playback_progress = 0.0;
            }

            ui.separator();

            ui.label("Speed:");
            ui.add(egui::Slider::new(&mut self.replay_speed, 0.5..=3.0));

            ui.checkbox(&mut self.use_original_timing, "Use original timing");
        });

        // Progress bar
        if self.is_playing {
            ui.add(ProgressBar::new(self.playback_progress).text("Playing..."));
        }

        ui.separator();

        // Action list editor
        ui.label(format!("Actions ({} total):", macro_data.actions.len()));

        egui::ScrollArea::vertical()
            .max_height(300.0)
            .show(ui, |ui| {
                for (idx, action) in macro_data.actions.iter().enumerate() {
                    self.render_editable_action(ui, idx, action);
                }
            });

        // Variable extraction
        ui.separator();
        ui.label("Variable Extraction:");
        ui.label("The following variables can be extracted from this macro:");

        let suggestions = macro_data.suggest_variables();
        for var in &suggestions {
            ui.horizontal(|ui| {
                ui.label(format!("${{{}}}", var.name));
                ui.label(format!("(extracted from: {})", var.extraction_pattern));
            });
        }

        if suggestions.is_empty() {
            ui.label("No variables detected. Use {{variable_name}} syntax in commands to create variables.");
        }
    }

    fn render_editable_action(&mut self, ui: &mut Ui, index: usize, action: &MacroAction) {
        let is_selected = self.selected_action.as_ref() == Some(&action.id);

        let bg_color = if is_selected {
            Color32::from_rgb(50, 60, 80)
        } else {
            Color32::from_gray(40)
        };

        Frame::group(ui.style()).fill(bg_color).show(ui, |ui| {
            ui.set_min_width(ui.available_width());

            ui.horizontal(|ui| {
                ui.label(format!("{}.", index + 1));

                // Action type icon
                let icon = match action.action_type {
                    MacroActionType::SshCommand => "$",
                    MacroActionType::FileUpload => "↑",
                    MacroActionType::FileDownload => "↓",
                    MacroActionType::Wait => "◷",
                    MacroActionType::ChangeDirectory => "📁",
                    MacroActionType::ProvideInput => "⌨",
                    MacroActionType::LocalCommand => "⌘",
                    _ => "•",
                };
                ui.label(icon);

                // Action summary
                let summary = match &action.data {
                    MacroActionData::SshCommand { command, .. } => command.clone(),
                    MacroActionData::FileUpload {
                        local_path,
                        remote_path,
                        ..
                    } => {
                        format!("{} → {}", local_path, remote_path)
                    }
                    MacroActionData::FileDownload {
                        remote_path,
                        local_path,
                        ..
                    } => {
                        format!("{} → {}", remote_path, local_path)
                    }
                    MacroActionData::Wait { duration_secs, .. } => {
                        format!("Wait {}s", duration_secs)
                    }
                    _ => format!("{:?}", action.action_type),
                };

                ui.label(RichText::new(summary).monospace());

                // Edit button
                if ui.button("✎").clicked() {
                    self.selected_action = Some(action.id.clone());
                }

                // Enable/disable
                let mut enabled = action.enabled;
                if ui.checkbox(&mut enabled, "").changed() {
                    // Would need to update the macro action
                }
            });
        });
    }

    fn render_save_dialog(&mut self, ui: &mut Ui, response: &mut MacroRecorderResponse) {
        let id = ui.make_persistent_id("macro_save_dialog");
        egui::Window::new("Save Macro")
            .id(id)
            .collapsible(false)
            .resizable(false)
            .show(ui.ctx(), |ui| {
                ui.label("Save this macro to the library:");

                ui.label("Name:");
                ui.text_edit_singleline(&mut self.save_name);

                ui.horizontal(|ui| {
                    if ui.button("Save").clicked() && !self.save_name.is_empty() {
                        if let Some(ref mut macr) = self.editing_macro {
                            macr.name = self.save_name.clone();
                            response.save_macro = Some(macr.clone());
                            self.show_save_dialog = false;
                            self.save_name.clear();
                        }
                    }
                    if ui.button("Cancel").clicked() {
                        self.show_save_dialog = false;
                        self.editing_macro = None;
                    }
                });
            });
    }

    pub fn set_selected_server(&mut self, server: Option<ServerContext>) {
        self.selected_server = server;
    }

    pub fn load_macro(&mut self, macro_data: Macro) {
        self.editing_macro = Some(macro_data);
    }

    pub fn is_recording(&self) -> bool {
        self.recorder.is_recording()
    }
}

/// Response from macro recorder panel
#[derive(Debug, Default)]
pub struct MacroRecorderResponse {
    pub save_macro: Option<Macro>,
    pub playback_requested: bool,
    pub playback_stop_requested: bool,
}
