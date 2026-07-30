use super::*;

impl EasySshApp {
    pub(super) fn files(&mut self, ctx: &egui::Context) {
        self.check_external_editor_change();
        egui::SidePanel::left("files_sources")
            .resizable(true)
            .default_width(220.0)
            .show(ctx, |ui| {
                ui.heading("Files");
                ui.label(egui::RichText::new("HOSTS").small().weak());
                let hosts = self.config.connections.clone();
                for host in hosts {
                    let selected = self.files_connection.as_deref() == Some(&host.id);
                    if ui
                        .selectable_label(selected, format!("{}  {}", icon::HARD_DRIVES, host.name))
                        .clicked()
                    {
                        self.files_select_host(host.id);
                    }
                }
                ui.separator();
                ui.label(egui::RichText::new("FAVORITES").small().weak());
                ui.label(
                    egui::RichText::new("No favorite directories")
                        .small()
                        .weak(),
                );
                ui.separator();
                ui.label(egui::RichText::new("RECENT").small().weak());
                for path in self.files_history.iter().rev().take(5) {
                    ui.label(egui::RichText::new(path).small().monospace());
                }
            });
        egui::SidePanel::right("files_inspector")
            .resizable(true)
            .default_width(260.0)
            .show(ctx, |ui| {
                if self.files_dual_pane {
                    ui.heading("Local files");
                    ui.label(
                        egui::RichText::new(self.local_path.to_string_lossy())
                            .small()
                            .monospace(),
                    );
                    ui.horizontal(|ui| {
                        if icon_button(ui, icon::ARROW_UP, "Parent directory").clicked() {
                            if let Some(parent) = self.local_path.parent() {
                                self.local_path = parent.to_path_buf();
                                self.refresh_local_files();
                            }
                        }
                        if icon_button(ui, icon::ARROW_CLOCKWISE, "Refresh local files").clicked() {
                            self.refresh_local_files();
                        }
                        if let Some(selected) = self.local_selected.clone() {
                            if ui.button("Upload").clicked() {
                                self.queue_local_upload(&selected);
                            }
                        }
                    });
                    ui.separator();
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        for entry in self.local_entries.clone() {
                            let glyph = if entry.is_directory {
                                icon::FOLDER
                            } else {
                                icon::FILE
                            };
                            let selected = self.local_selected.as_ref() == Some(&entry.path);
                            let response = ui.selectable_label(
                                selected,
                                format!("{}  {}  {}", glyph, entry.name, format_bytes(entry.size)),
                            );
                            if response.clicked() {
                                self.local_selected = Some(entry.path.clone());
                            }
                            if response.double_clicked() && entry.is_directory {
                                self.local_path = entry.path;
                                self.local_selected = None;
                                self.refresh_local_files();
                            }
                        }
                    });
                } else {
                    ui.heading("Properties");
                    if let Some(selected) = self.files_selected.as_ref().and_then(|path| {
                        self.files_entries.iter().find(|entry| &entry.path == path)
                    }) {
                        ui.label(egui::RichText::new(&selected.name).strong());
                        detail(ui, "Type", format!("{:?}", selected.entry_type));
                        detail(ui, "Size", format_bytes(selected.size));
                        detail(ui, "Permissions", selected.permissions.clone());
                        detail(
                            ui,
                            "Owner",
                            format!("{}:{}", selected.owner, selected.group),
                        );
                        if let Some(target) = &selected.link_target {
                            detail(ui, "Link", target.clone());
                        }
                    } else {
                        ui.label(egui::RichText::new("Select an entry").weak());
                    }
                }
                ui.separator();
                ui.label(egui::RichText::new(&self.files_status).small().weak());
            });
        egui::CentralPanel::default().show(ctx, |ui| {
            if let Some((path, image)) = self.file_preview_image.clone() {
                if self.file_preview_texture.is_none() {
                    self.file_preview_texture = Some(ctx.load_texture(
                        format!("remote-preview-{path}"),
                        image,
                        egui::TextureOptions::LINEAR,
                    ));
                }
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    if ui.button(format!("{} Files", icon::ARROW_LEFT)).clicked() {
                        self.file_preview_image = None;
                        self.file_preview_texture = None;
                    }
                    ui.heading(&path);
                    ui.label(
                        egui::RichText::new("Temporary local preview")
                            .small()
                            .weak(),
                    );
                });
                ui.separator();
                if let Some(texture) = &self.file_preview_texture {
                    let available = ui.available_size();
                    let original = texture.size_vec2();
                    let scale = (available.x / original.x)
                        .min(available.y / original.y)
                        .min(1.0);
                    ui.image((texture.id(), original * scale));
                }
                return;
            }
            if let Some(session) = self.file_edit_session.clone() {
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    if ui.button(format!("{} Files", icon::ARROW_LEFT)).clicked() {
                        self.file_edit_session = None;
                        self.file_editor_text.clear();
                    }
                    ui.heading(
                        std::path::Path::new(&session.remote_path)
                            .file_name()
                            .and_then(|name| name.to_str())
                            .unwrap_or(&session.remote_path),
                    );
                    ui.label(
                        egui::RichText::new(self.file_editor_status.label())
                            .small()
                            .weak(),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui
                            .button(format!("{} Upload changes", icon::UPLOAD_SIMPLE))
                            .clicked()
                        {
                            self.upload_editor_changes();
                        }
                        if ui.button("Save local copy").clicked() {
                            self.save_editor_locally();
                        }
                        if ui.button("Open externally").clicked() {
                            self.open_editor_in_system_app();
                        }
                    });
                });
                ui.separator();
                ui.horizontal(|ui| {
                    if icon_button(ui, icon::CHECK, "Select all editor text").clicked() {
                        self.select_all_editor_text();
                    }
                    ui.checkbox(&mut self.file_editor_word_wrap, "Wrap lines");
                    ui.label("Font size");
                    ui.add(
                        egui::DragValue::new(&mut self.file_editor_font_size)
                            .range(8.0..=32.0)
                            .speed(0.5),
                    );
                });
                ui.separator();
                if let Some(external_text) = self.file_editor_external_change.clone() {
                    ui.horizontal(|ui| {
                        ui.label("External editor changed the local working copy.");
                        if ui.button("Load external change").clicked() {
                            self.file_editor_text = external_text;
                            self.file_editor_external_change = None;
                            self.file_editor_status = FileEditorStatus::SavedLocally;
                        }
                        if ui.button("Keep editor text").clicked() {
                            self.file_editor_external_change = None;
                        }
                    });
                }
                ui.horizontal_wrapped(|ui| {
                    ui.label("Find");
                    let find_changed = ui
                        .add(
                            egui::TextEdit::singleline(&mut self.file_editor_find)
                                .hint_text("Text")
                                .desired_width(150.0),
                        )
                        .changed();
                    if find_changed {
                        self.file_editor_match_index = 0;
                    }
                    if ui.button("Previous").clicked() {
                        self.select_editor_match(true);
                    }
                    if ui.button("Next").clicked() {
                        self.select_editor_match(false);
                    }
                    let match_count = self.find_editor_matches().len();
                    if !self.file_editor_find.is_empty() {
                        ui.label(format!("{match_count} match(es)"));
                    }
                    ui.separator();
                    ui.label("Replace");
                    ui.add(
                        egui::TextEdit::singleline(&mut self.file_editor_replace)
                            .hint_text("Replacement")
                            .desired_width(150.0),
                    );
                    if ui.button("Replace all").clicked() {
                        self.replace_all_editor_matches();
                    }
                    ui.separator();
                    ui.label("Go to line");
                    ui.add(
                        egui::DragValue::new(&mut self.file_editor_go_to_line)
                            .range(1..=self.file_editor_text.lines().count().max(1))
                            .speed(1),
                    );
                    if ui.button("Go").clicked() {
                        self.go_to_editor_line();
                    }
                    ui.label(format!(
                        "Line {} / {}",
                        self.file_editor_cursor_line,
                        self.file_editor_text.lines().count().max(1)
                    ));
                });
                ui.separator();
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.label(egui::RichText::new("#").small().weak());
                        egui::ScrollArea::vertical()
                            .id_salt("file-editor-line-numbers")
                            .max_height(ui.available_height() - 12.0)
                            .show(ui, |ui| {
                                for line in 1..=self.file_editor_text.lines().count().max(1) {
                                    let text = if line == self.file_editor_cursor_line {
                                        egui::RichText::new(line.to_string()).strong()
                                    } else {
                                        egui::RichText::new(line.to_string()).weak()
                                    };
                                    ui.label(text);
                                }
                            });
                    });
                    let output = egui::TextEdit::multiline(&mut self.file_editor_text)
                        .id_source("file-editor")
                        .font(egui::FontId::monospace(self.file_editor_font_size))
                        .desired_width(if self.file_editor_word_wrap {
                            ui.available_width()
                        } else {
                            f32::INFINITY
                        })
                        .desired_rows(32)
                        .code_editor()
                        .show(ui);
                    if output.response.changed() {
                        self.file_editor_status = FileEditorStatus::LocalModified;
                    }
                    if let Some(range) = output.state.cursor.char_range() {
                        self.file_editor_cursor_line = self.file_editor_text[..]
                            .chars()
                            .take(range.primary.index)
                            .filter(|character| *character == '\n')
                            .count()
                            + 1;
                    }
                    if let Some((start, end)) = self.file_editor_pending_selection.take() {
                        let mut state = output.state;
                        state
                            .cursor
                            .set_char_range(Some(egui::text::CCursorRange::two(
                                egui::text::CCursor::new(start),
                                egui::text::CCursor::new(end),
                            )));
                        state.store(ui.ctx(), output.response.id);
                        output.response.request_focus();
                    }
                });
                return;
            }
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                if icon_button(ui, icon::ARROW_LEFT, "Back").clicked()
                    && self.files_history_index > 0
                {
                    self.files_history_index -= 1;
                    self.files_path = self.files_history[self.files_history_index].clone();
                    self.files_path_input = self.files_path.clone();
                    self.refresh_files(false);
                }
                if icon_button(ui, icon::ARROW_RIGHT, "Forward").clicked()
                    && self.files_history_index + 1 < self.files_history.len()
                {
                    self.files_history_index += 1;
                    self.files_path = self.files_history[self.files_history_index].clone();
                    self.files_path_input = self.files_path.clone();
                    self.refresh_files(false);
                }
                if icon_button(ui, icon::ARROW_UP, "Parent directory").clicked() {
                    self.files_go_up();
                }
                let path_response = ui.add(
                    egui::TextEdit::singleline(&mut self.files_path_input)
                        .desired_width(ui.available_width() * 0.52)
                        .hint_text("Remote path"),
                );
                if path_response.lost_focus()
                    && ui.input(|input| input.key_pressed(egui::Key::Enter))
                {
                    self.open_files_path(self.files_path_input.trim().to_owned());
                }
                if icon_button(ui, icon::ARROW_CLOCKWISE, "Refresh").clicked() {
                    self.refresh_files(false);
                }
                if icon_button(ui, icon::UPLOAD_SIMPLE, "Upload").clicked() {
                    self.config.workspace = Workspace::Transfers;
                }
                if ui.button("New folder").clicked() {
                    self.files_new_dir_name.clear();
                    self.files_create_dir_open = true;
                }
                if self.files_selected.is_some() && ui.button("Rename").clicked() {
                    self.files_rename_name = self
                        .files_selected
                        .as_ref()
                        .and_then(|path| {
                            self.files_entries.iter().find(|entry| &entry.path == path)
                        })
                        .map(|entry| entry.name.clone())
                        .unwrap_or_default();
                    self.files_rename_open = true;
                }
                if self.files_selected.is_some() && ui.button("Delete").clicked() {
                    self.files_delete_open = true;
                }
                ui.checkbox(&mut self.files_hidden, "Hidden");
                if ui
                    .add_enabled(
                        self.config.experimental.dual_pane_file_browsing,
                        egui::Checkbox::new(&mut self.files_dual_pane, "Two panes"),
                    )
                    .changed()
                    && self.files_dual_pane
                {
                    self.refresh_local_files();
                }
                if self.files_dual_pane {
                    if let Some(path) = self.files_selected.clone() {
                        if ui.button("Download selected").clicked() {
                            self.queue_remote_download(path);
                        }
                    }
                }
            });
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("Filter").small().weak());
                ui.add(egui::TextEdit::singleline(&mut self.files_filter).desired_width(240.0));
                if let Some(capabilities) = &self.files_capabilities {
                    ui.label(
                        egui::RichText::new(format!(
                            "{} / {:?}",
                            capabilities.uname, capabilities.platform
                        ))
                        .small()
                        .weak(),
                    );
                }
            });
            ui.separator();
            let mut entries = self.files_entries.clone();
            entries.sort_by_cached_key(|entry| entry.name.to_lowercase());
            egui::ScrollArea::vertical().show(ui, |ui| {
                if self.files_connection.is_none() {
                    ui.vertical_centered(|ui| {
                        ui.add_space(80.0);
                        ui.label(
                            egui::RichText::new(icon::FOLDER_OPEN)
                                .size(42.0)
                                .color(BLUE),
                        );
                        ui.heading("Select a host");
                        ui.label("Remote browsing uses system ssh.");
                    });
                }
                for entry in entries {
                    if !contains(&entry.name, &self.files_filter) {
                        continue;
                    }
                    let selected = self.files_selected.as_deref() == Some(&entry.path);
                    let glyph = match entry.entry_type {
                        RemoteEntryType::Directory => icon::FOLDER,
                        RemoteEntryType::Symlink => icon::LINK,
                        RemoteEntryType::File => icon::FILE,
                        RemoteEntryType::Other => icon::FILE,
                    };
                    let response = ui.selectable_label(
                        selected,
                        format!(
                            "{}  {:<32}  {:>10}  {}",
                            glyph,
                            entry.name,
                            format_bytes(entry.size),
                            entry.permissions
                        ),
                    );
                    if response.clicked() {
                        self.files_selected = Some(entry.path.clone());
                    }
                    if response.double_clicked() {
                        match entry.entry_type {
                            RemoteEntryType::Directory => self.open_files_path(entry.path.clone()),
                            RemoteEntryType::File
                                if self.config.experimental.image_preview
                                    && is_previewable_image(&entry.name) =>
                            {
                                self.open_remote_image_preview(entry.path.clone())
                            }
                            RemoteEntryType::File
                                if self.config.experimental.remote_text_editing =>
                            {
                                self.open_remote_text_file(entry.path.clone())
                            }
                            RemoteEntryType::File => {
                                self.files_status =
                                    "Remote editing is disabled in Experimental settings.".into()
                            }
                            _ => {
                                self.files_status =
                                    "This entry is not supported by the text editor.".into()
                            }
                        }
                    }
                }
            });
        });
    }
}
