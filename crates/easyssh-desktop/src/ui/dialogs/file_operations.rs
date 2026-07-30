use super::*;

impl EasySshApp {
    pub(super) fn shortcuts(&mut self, ctx: &egui::Context) {
        if ctx.input(|i| i.modifiers.command && i.key_pressed(egui::Key::K)) {
            self.command_open = true;
        }
        if ctx.input(|i| i.modifiers.command && i.key_pressed(egui::Key::N)) {
            self.add_host();
        }
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) && !self.command_open {
            self.search.clear();
        }
        if self.remote_file_browser_enabled() && self.config.workspace == Workspace::Files {
            if self.file_edit_session.is_none()
                && self.files_selected.is_some()
                && ctx.input(|i| i.key_pressed(egui::Key::F2))
            {
                self.files_rename_name = self
                    .files_selected
                    .as_ref()
                    .and_then(|path| self.files_entries.iter().find(|entry| &entry.path == path))
                    .map(|entry| entry.name.clone())
                    .unwrap_or_default();
                self.files_rename_open = true;
            }
            if self.file_edit_session.is_none()
                && self.files_selected.is_some()
                && ctx.input(|i| i.key_pressed(egui::Key::Delete))
            {
                self.files_delete_open = true;
            }
            if ctx.input(|i| i.modifiers.command && i.key_pressed(egui::Key::S))
                && self.file_edit_session.is_some()
            {
                self.upload_editor_changes();
            }
            if ctx.input(|i| i.modifiers.command && i.key_pressed(egui::Key::A))
                && self.file_edit_session.is_some()
            {
                self.select_all_editor_text();
            }
            if ctx.input(|i| i.modifiers.command && i.key_pressed(egui::Key::R)) {
                self.refresh_files(false);
            }
            if ctx.input(|i| i.modifiers.command && i.key_pressed(egui::Key::U)) {
                self.config.workspace = Workspace::Transfers;
                self.transfer_connection = self.files_connection.clone();
                self.transfer_remote_path = self.files_path.clone();
            }
            if ctx.input(|i| i.key_pressed(egui::Key::Backspace)) {
                self.files_go_up();
            }
            let dropped = ctx.input(|i| i.raw.dropped_files.clone());
            if let Some(file) = dropped.first().and_then(|file| file.path.clone()) {
                self.transfer_connection = self.files_connection.clone();
                self.transfer_local_path = file.to_string_lossy().into_owned();
                self.transfer_remote_path = self.files_path.clone();
                self.transfer_direction = TransferDirection::Upload;
                self.config.workspace = Workspace::Transfers;
                self.status = "Local file queued for upload review.".into();
            }
        }
    }

    pub(super) fn file_conflict_dialog(&mut self, ctx: &egui::Context) {
        if !self.file_conflict_open {
            return;
        }
        let mut open = true;
        egui::Window::new("Remote file changed")
            .collapsible(false)
            .resizable(true)
            .open(&mut open)
            .show(ctx, |ui| {
                ui.label("The remote file changed after this working copy was downloaded.");
                ui.label("EasySSH kept your local working copy and did not overwrite the server.");
                if let Some(session) = &self.file_edit_session {
                    ui.label(egui::RichText::new(&session.remote_path).monospace());
                }
                ui.separator();
                let initial = self
                    .file_edit_session
                    .as_ref()
                    .and_then(|session| fs::read_to_string(&session.base_path).ok())
                    .unwrap_or_else(|| "Initial version is unavailable.".into());
                let remote = self
                    .file_conflict_remote
                    .clone()
                    .unwrap_or_else(|| "Current remote version could not be downloaded for preview.".into());
                let local = self.file_editor_text.clone();
                ui.columns(3, |columns| {
                    for (column, (title, value)) in columns.iter_mut().zip([
                        ("My local version", local),
                        ("Initially downloaded", initial),
                        ("Current remote version", remote),
                    ]) {
                        column.label(egui::RichText::new(title).strong());
                        let mut display = value;
                        column.add_enabled(
                            false,
                            egui::TextEdit::multiline(&mut display)
                                .font(egui::TextStyle::Monospace)
                                .desired_rows(12),
                        );
                    }
                });
                ui.label("You can keep the local copy, close the editor, or manually compare it with a new download.");
                if ui.button("Keep local copy and close").clicked() {
                    self.file_edit_session = None;
                    self.file_editor_text.clear();
                    self.file_conflict_remote = None;
                    self.file_conflict_open = false;
                }
                if ui.button("Continue editing local copy").clicked() {
                    self.file_conflict_open = false;
                }
            });
        if !open {
            self.file_conflict_open = false;
        }
    }

    pub(super) fn file_operation_dialogs(&mut self, ctx: &egui::Context) {
        if self.files_create_dir_open {
            let mut open = true;
            egui::Window::new("New remote folder")
                .collapsible(false)
                .resizable(false)
                .open(&mut open)
                .show(ctx, |ui| {
                    ui.label(egui::RichText::new(&self.files_path).monospace());
                    ui.text_edit_singleline(&mut self.files_new_dir_name);
                    if ui.button("Create folder").clicked() {
                        self.create_remote_directory();
                    }
                });
            if !open {
                self.files_create_dir_open = false;
            }
        }
        if self.files_rename_open {
            let mut open = true;
            egui::Window::new("Rename remote entry")
                .collapsible(false)
                .resizable(false)
                .open(&mut open)
                .show(ctx, |ui| {
                    ui.text_edit_singleline(&mut self.files_rename_name);
                    if ui.button("Rename").clicked() {
                        self.rename_selected_remote();
                    }
                });
            if !open {
                self.files_rename_open = false;
            }
        }
        if self.files_delete_open {
            let selected = self
                .files_selected
                .as_ref()
                .and_then(|path| self.files_entries.iter().find(|entry| &entry.path == path))
                .cloned();
            let mut open = true;
            egui::Window::new("Delete remote entry")
                .collapsible(false)
                .resizable(false)
                .open(&mut open)
                .show(ctx, |ui| {
                    if let Some(entry) = selected {
                        ui.colored_label(RED, format!("Delete {}?", entry.name));
                        if entry.entry_type == RemoteEntryType::Directory {
                            ui.label("This permanently deletes the directory and its contents.");
                        }
                        if ui.button("Delete permanently").clicked() {
                            self.delete_selected_remote();
                        }
                    }
                });
            if !open {
                self.files_delete_open = false;
            }
        }
    }
}
