use super::*;

impl EasySshApp {
    pub(super) fn command_panel(&mut self, ctx: &egui::Context) {
        if !self.command_open {
            return;
        }
        let mut open = self.command_open;
        let mut close = false;
        let mut chosen: Option<CommandAction> = None;
        egui::Window::new("Command palette")
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .fixed_size([600.0, 410.0])
            .anchor(egui::Align2::CENTER_TOP, [0.0, 72.0])
            .show(ctx, |ui| {
                let response = ui.add(
                    egui::TextEdit::singleline(&mut self.command_query)
                        .hint_text("Search hosts, groups, tags, snippets, sessions, or actions")
                        .desired_width(f32::INFINITY)
                        .id(egui::Id::new("command-query")),
                );
                response.request_focus();
                let actions = self.command_actions();
                if ctx.input(|i| i.key_pressed(egui::Key::ArrowDown)) {
                    self.command_index =
                        (self.command_index + 1).min(actions.len().saturating_sub(1));
                }
                if ctx.input(|i| i.key_pressed(egui::Key::ArrowUp)) {
                    self.command_index = self.command_index.saturating_sub(1);
                }
                if ctx.input(|i| i.key_pressed(egui::Key::Enter)) {
                    chosen = actions.get(self.command_index).cloned();
                }
                if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
                    close = true;
                }
                egui::ScrollArea::vertical().show(ui, |ui| {
                    for (index, action) in actions.iter().enumerate() {
                        if ui
                            .selectable_label(index == self.command_index, action.label())
                            .clicked()
                        {
                            chosen = Some(action.clone());
                        }
                    }
                });
            });
        if let Some(action) = chosen {
            self.run_command(action);
            close = true;
        }
        if close {
            open = false;
        }
        self.command_open = open;
    }

    pub(super) fn command_actions(&self) -> Vec<CommandAction> {
        let query = &self.command_query;
        let mut items = vec![
            CommandAction::NewHost,
            CommandAction::Switch(Workspace::Home),
            CommandAction::Switch(Workspace::Hosts),
            CommandAction::Switch(Workspace::Transfers),
            CommandAction::Switch(Workspace::Keys),
            CommandAction::Switch(Workspace::Settings),
        ];
        if self.config.experimental.git_metadata_sync_ui {
            items.push(CommandAction::OpenSync);
        }
        for host in self.config.connections.iter().filter(|h| {
            contains(
                &format!("{} {} {}", h.name, target_text(h), h.tags.join(" ")),
                query,
            )
        }) {
            items.push(CommandAction::Host(host.id.clone(), host.name.clone()));
            items.push(CommandAction::Connect(
                host.id.clone(),
                host.name.clone(),
                false,
            ));
            items.push(CommandAction::Connect(
                host.id.clone(),
                host.name.clone(),
                true,
            ));
        }
        items.extend(
            self.config
                .snippets
                .iter()
                .filter(|s| contains(&format!("{} {}", s.name, s.content), query))
                .map(|s| CommandAction::Snippet(s.id.clone(), s.name.clone())),
        );
        items.extend(
            self.config
                .sessions
                .iter()
                .filter(|s| contains(&format!("{} {}", s.name, s.target), query))
                .map(|s| CommandAction::Session(s.id.clone(), s.name.clone())),
        );
        items
            .into_iter()
            .filter(|a| contains(&a.label(), query))
            .collect()
    }

    pub(super) fn run_command(&mut self, action: CommandAction) {
        match action {
            CommandAction::NewHost => self.add_host(),
            CommandAction::Switch(workspace) => {
                self.config.workspace = workspace;
                self.save();
            }
            CommandAction::Host(id, _) => {
                self.selected = Some(id);
                self.config.workspace = Workspace::Hosts;
            }
            CommandAction::Connect(id, _, verbose) => {
                if let Some(host) = self
                    .config
                    .connections
                    .iter()
                    .find(|host| host.id == id)
                    .cloned()
                {
                    self.connect(&host, verbose);
                }
            }
            CommandAction::Snippet(id, _) => {
                if let Some(item) = self.config.snippets.iter().find(|s| s.id == id) {
                    self.status = "Snippet copied. Commands are never sent to a terminal.".into();
                    self.copy_text = Some(item.content.clone());
                }
            }
            CommandAction::Session(id, _) => {
                if let Some(session) = self.config.sessions.iter().find(|s| s.id == id).cloned() {
                    self.reconnect_session(&session);
                }
            }
            CommandAction::OpenSync if self.config.experimental.git_metadata_sync_ui => {
                self.sync_open = true
            }
            CommandAction::OpenSync => {
                self.status = "Git metadata sync is disabled in Experimental settings.".into()
            }
        }
    }

    pub(super) fn host_editor(&mut self, ctx: &egui::Context) {
        if !self.editor_open {
            return;
        }
        let Some(form) = &mut self.host_form else {
            self.editor_open = false;
            return;
        };
        let mut open = self.editor_open;
        let mut save = false;
        let mut save_and_connect = false;
        let mut validate = false;
        egui::Window::new("Edit host")
            .open(&mut open)
            .resizable(true)
            .default_width(520.0)
            .show(ctx, |ui| {
                let host = &mut form.draft;
                ui.label("Display name");
                ui.text_edit_singleline(&mut host.name);
                let mut alias = matches!(host.target, ConnectionTarget::Alias { .. });
                ui.horizontal(|ui| {
                    ui.radio_value(&mut alias, false, "Address");
                    ui.radio_value(&mut alias, true, "OpenSSH alias");
                });
                if alias != matches!(host.target, ConnectionTarget::Alias { .. }) {
                    host.target = if alias {
                        ConnectionTarget::Alias {
                            alias: String::new(),
                        }
                    } else {
                        ConnectionTarget::Endpoint {
                            hostname: String::new(),
                            username: Some(String::new()),
                            port: 22,
                        }
                    };
                }
                match &mut host.target {
                    ConnectionTarget::Alias { alias } => {
                        ui.label("Alias");
                        ui.text_edit_singleline(alias);
                    }
                    ConnectionTarget::Endpoint {
                        hostname,
                        username,
                        port,
                    } => {
                        ui.label("Hostname");
                        ui.text_edit_singleline(hostname);
                        ui.horizontal(|ui| {
                            ui.vertical(|ui| {
                                ui.label("User");
                                ui.text_edit_singleline(username.get_or_insert_with(String::new));
                            });
                            ui.vertical(|ui| {
                                ui.label("Port");
                                ui.add(egui::DragValue::new(port).range(1..=65535));
                            });
                        });
                    }
                }
                ui.horizontal(|ui| {
                    ui.checkbox(&mut host.favorite, "Favorite");
                    if ui.button("Validate setup").clicked() {
                        validate = true;
                    }
                });
                if let Some(status) = &self.editor_test_status {
                    ui.label(egui::RichText::new(status).small().weak());
                }
                ui.collapsing("Advanced", |ui| {
                    ui.label("Tags (comma separated)");
                    let mut tags = host.tags.join(", ");
                    if ui.text_edit_singleline(&mut tags).changed() {
                        host.tags = tags
                            .split(',')
                            .map(str::trim)
                            .filter(|s| !s.is_empty())
                            .map(str::to_owned)
                            .collect();
                    }
                    ui.label("Jump host");
                    ui.text_edit_singleline(host.proxy_jump.get_or_insert_with(String::new));
                    ui.label("Startup command");
                    ui.text_edit_singleline(host.remote_command.get_or_insert_with(String::new));
                    ui.label("Local forwards (one per line)");
                    edit_lines(ui, &mut host.local_forwards);
                    ui.label("Remote forwards (one per line)");
                    edit_lines(ui, &mut host.remote_forwards);
                    ui.label("Dynamic forwards (one per line)");
                    edit_lines(ui, &mut host.dynamic_forwards);
                    ui.label("Notes");
                    ui.add_sized(
                        [ui.available_width(), 70.0],
                        egui::TextEdit::multiline(&mut host.notes),
                    );
                });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("Save and connect").clicked() {
                        save = true;
                        save_and_connect = true;
                    }
                    if ui.button("Save").clicked() {
                        save = true;
                    }
                });
            });
        if validate {
            self.editor_test_status = Some(match form.validate() {
                Ok(()) => "Parsed successfully. TCP and authentication occur only when the external terminal launches.".into(),
                Err(error) => format!("Fix connection details: {error}"),
            });
        }
        if save {
            let valid = self
                .host_form
                .as_mut()
                .is_some_and(|form| form.validate().is_ok());
            if valid {
                let host = self
                    .host_form
                    .as_ref()
                    .map(|form| form.draft.clone())
                    .expect("host form");
                let previous = if let Some(index) = self
                    .config
                    .connections
                    .iter()
                    .position(|item| item.id == host.id)
                {
                    Some((
                        index,
                        std::mem::replace(&mut self.config.connections[index], host.clone()),
                    ))
                } else {
                    self.config.connections.push(host.clone());
                    None
                };
                if self.try_save() {
                    self.selected = Some(host.id.clone());
                    open = false;
                    self.host_form = None;
                    if save_and_connect {
                        self.connect(&host, false);
                    }
                } else if let Some((index, original)) = previous {
                    self.config.connections[index] = original;
                } else {
                    self.config.connections.retain(|item| item.id != host.id);
                }
            } else {
                self.editor_test_status = self
                    .host_form
                    .as_ref()
                    .and_then(|form| form.validation.clone());
            }
        } else if !open && self.host_form.as_ref().is_some_and(|form| form.dirty()) {
            self.host_form.as_mut().expect("host form").confirm_discard = true;
            open = true;
        } else if !open {
            self.host_form = None;
        }
        self.editor_open = open;
        if self
            .host_form
            .as_ref()
            .is_some_and(|form| form.confirm_discard)
        {
            egui::Window::new("Discard changes?")
                .collapsible(false)
                .resizable(false)
                .show(ctx, |ui| {
                    let strings = crate::ui::localization::Strings::new(self.config.locale);
                    ui.label("Discard unsaved host changes?");
                    ui.horizontal(|ui| {
                        if ui
                            .button(strings.text(crate::ui::localization::Key::DiscardChanges))
                            .clicked()
                        {
                            self.close_host_form(true);
                        }
                        if ui
                            .button(strings.text(crate::ui::localization::Key::KeepEditing))
                            .clicked()
                        {
                            self.host_form.as_mut().expect("host form").confirm_discard = false;
                        }
                    });
                });
        }
    }

    pub(super) fn snippet_editor(&mut self, ctx: &egui::Context) {
        let Some(id) = self.snippet_editor.clone() else {
            return;
        };
        let Some(index) = self.config.snippets.iter().position(|s| s.id == id) else {
            self.snippet_editor = None;
            return;
        };
        let mut open = true;
        let mut save = false;
        egui::Window::new("Edit snippet").open(&mut open).default_width(520.0).show(ctx, |ui| { let item = &mut self.config.snippets[index]; ui.label("Name"); ui.text_edit_singleline(&mut item.name); ui.label("Command text"); ui.add_sized([ui.available_width(), 180.0], egui::TextEdit::multiline(&mut item.content).code_editor()); ui.label(egui::RichText::new("Snippets can only be copied. They are never executed or injected into a terminal.").small().weak()); if ui.button("Save").clicked() { save = true; } });
        if save {
            self.save();
            open = false;
        }
        if !open {
            self.snippet_editor = None;
        }
    }

    pub(super) fn quick_connect(&mut self, ctx: &egui::Context) {
        if !self.quick_open {
            return;
        }
        let mut open = self.quick_open;
        let mut launch = false;
        egui::Window::new("Quick connect")
            .open(&mut open)
            .resizable(false)
            .show(ctx, |ui| {
                ui.label("Host address");
                ui.text_edit_singleline(&mut self.quick_host);
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.label("User");
                        ui.text_edit_singleline(&mut self.quick_user);
                    });
                    ui.vertical(|ui| {
                        ui.label("Port");
                        ui.add(egui::DragValue::new(&mut self.quick_port).range(1..=65535));
                    });
                });
                if ui
                    .add_enabled(
                        !self.quick_host.trim().is_empty(),
                        egui::Button::new("Connect"),
                    )
                    .clicked()
                {
                    launch = true;
                }
            });
        if launch {
            let (user, hostname, port) =
                parse_quick_target(&self.quick_host, &self.quick_user, self.quick_port);
            let mut host = Connection::alias(&hostname, &hostname);
            host.target = ConnectionTarget::Endpoint {
                hostname,
                username: user,
                port,
            };
            self.connect(&host, false);
            open = false;
        }
        self.quick_open = open;
    }

    pub(super) fn confirmation_dialogs(&mut self, ctx: &egui::Context) {
        if let Some(id) = self.delete_host.clone() {
            let mut open = true;
            egui::Window::new("Delete host?")
                .open(&mut open)
                .collapsible(false)
                .resizable(false)
                .show(ctx, |ui| {
                    ui.label("This removes only EasySSH workbench metadata.");
                    ui.label("It does not delete system SSH configuration, keys, or external terminals already launched.");
                    ui.horizontal(|ui| {
                        if ui.button(egui::RichText::new("Delete host").color(RED)).clicked() {
                            self.config.connections.retain(|h| h.id != id);
                            self.selected = None;
                            self.delete_host = None;
                            self.save();
                        } else if ui.button("Cancel").clicked() {
                            self.delete_host = None;
                        }
                    });
                });
            if !open {
                self.delete_host = None;
            }
        }
        if let Some(id) = self.delete_snippet.clone() {
            let mut open = true;
            egui::Window::new("Delete snippet?")
                .open(&mut open)
                .collapsible(false)
                .show(ctx, |ui| {
                    ui.label("This removes the saved snippet text.");
                    ui.horizontal(|ui| {
                        if ui
                            .button(egui::RichText::new("Delete").color(RED))
                            .clicked()
                        {
                            self.config.snippets.retain(|s| s.id != id);
                            self.delete_snippet = None;
                            self.save();
                        }
                        if ui.button("Cancel").clicked() {
                            self.delete_snippet = None;
                        }
                    });
                });
            if !open {
                self.delete_snippet = None;
            }
        }
    }

    pub(super) fn diagnostics(&mut self, ctx: &egui::Context) {
        if !self.diagnostics_open {
            return;
        }
        let mut open = self.diagnostics_open;
        egui::Window::new("SSH Agent diagnostics")
            .open(&mut open)
            .show(ctx, |ui| {
                let strings = crate::ui::localization::Strings::new(self.config.locale);
                self.diagnostics_state.poll();
                let report = match &self.diagnostics_state.status {
                    state::diagnostics::Status::Ready(report) => Some(report),
                    state::diagnostics::Status::Loading => {
                        ui.spinner();
                        ui.label("Refreshing diagnostics...");
                        None
                    }
                    state::diagnostics::Status::Failed(error) => {
                        ui.label(egui::RichText::new(error).small().color(AMBER));
                        None
                    }
                    _ => None,
                };
                detail(
                    ui,
                    "OpenSSH",
                    report
                        .and_then(|report| report.ssh_path.as_ref())
                        .map(|p| p.display().to_string())
                        .unwrap_or_else(|| "Not found".into()),
                );
                detail(
                    ui,
                    "Agent",
                    if report
                        .as_ref()
                        .is_some_and(|report| report.agent_socket_configured)
                    {
                        "Configured".into()
                    } else {
                        "Not configured".into()
                    },
                );
                detail(
                    ui,
                    "Available keys",
                    report
                        .and_then(|report| report.agent_keys.as_ref())
                        .map(|k| {
                            if k.available {
                                k.fingerprints.len().to_string()
                            } else {
                                "Unavailable".into()
                            }
                        })
                        .unwrap_or_else(|| "Unknown".into()),
                );
                ui.label(
                    egui::RichText::new(
                        "Key material and fingerprints are never saved or displayed.",
                    )
                    .small()
                    .weak(),
                );
                if ui
                    .add_enabled(
                        !matches!(
                            self.diagnostics_state.status,
                            state::diagnostics::Status::Loading
                        ),
                        egui::Button::new(strings.text(crate::ui::localization::Key::Refresh)),
                    )
                    .clicked()
                {
                    self.diagnostics_state.request(ctx);
                }
            });
        self.diagnostics_open = open;
    }

    pub(super) fn sync_panel(&mut self, ctx: &egui::Context) {
        if !self.sync_open {
            return;
        }
        let mut open = self.sync_open;
        let mut save = false;
        let mut action: Option<&str> = None;
        egui::Window::new("Git metadata sync")
            .open(&mut open).default_width(560.0).resizable(true)
            .show(ctx, |ui| {
                ui.label(egui::RichText::new("Explicit sync for non-sensitive workbench metadata").strong());
                ui.label(egui::RichText::new("Hosts, groups, tags, notes, forwards, proxy jumps, and snippets may reveal infrastructure structure. Private keys, passwords, credential data, key paths, terminal output, SSH diagnostics, working directories, and complete SSH commands are never written to Git.").small().weak());
                ui.add_space(8.0);
                ui.label("Display name"); ui.text_edit_singleline(&mut self.config.sync.display_name);
                ui.label("Repository folder"); ui.text_edit_singleline(self.config.sync.repository_path.get_or_insert_with(String::new));
                ui.label("Remote URL (optional, used only when initializing)"); ui.text_edit_singleline(self.config.sync.remote_url.get_or_insert_with(String::new));
                ui.label("Branch"); ui.text_edit_singleline(self.config.sync.branch.get_or_insert_with(|| "main".into()));
                ui.label("Display density");
                ui.horizontal(|ui| {
                    ui.selectable_value(&mut self.config.display_density, DisplayDensity::Compact, "Compact");
                    ui.selectable_value(&mut self.config.display_density, DisplayDensity::Comfortable, "Comfortable");
                    ui.selectable_value(&mut self.config.display_density, DisplayDensity::Large, "Large");
                });
                ui.add_space(8.0);
                detail(ui, "State", sync_status_label(GitSync::status(&self.config)).into());
                if let Some(time) = self.config.sync.last_success_at { detail(ui, "Last success", relative_time(time)); }
                if let Some(error) = &self.config.sync.last_error { ui.colored_label(RED, error); }
                ui.horizontal(|ui| {
                    if ui.button("Initialize repository").clicked() { action = Some("init"); }
                    if ui.add_enabled(GitSync::repository(&self.config).is_ok(), egui::Button::new("Pull")).clicked() { action = Some("pull"); }
                    if ui.add_enabled(GitSync::repository(&self.config).is_ok(), egui::Button::new("Push")).clicked() { action = Some("push"); }
                });
                if ui.button("Save settings").clicked() { save = true; }
            });
        if let Some(action) = action {
            let result = match action {
                "init" => self
                    .config
                    .sync
                    .repository_path
                    .as_deref()
                    .filter(|p| !p.trim().is_empty())
                    .ok_or_else(|| "Repository folder is required.".to_owned())
                    .and_then(|path| {
                        GitSync::init(
                            std::path::Path::new(path),
                            self.config
                                .sync
                                .remote_url
                                .as_deref()
                                .filter(|u| !u.trim().is_empty()),
                            self.config.sync.branch.as_deref().unwrap_or("main"),
                        )
                        .map_err(|e| e.to_string())
                    }),
                "pull" => GitSync::pull(&mut self.config)
                    .map(|changed| {
                        self.status = if changed {
                            "Metadata pulled. Review before pushing.".into()
                        } else {
                            "No metadata file exists on the remote branch.".into()
                        };
                    })
                    .map_err(|e| e.to_string()),
                _ => GitSync::push(&mut self.config).map_err(|e| e.to_string()),
            };
            match result {
                Ok(()) => {
                    self.config.sync.last_error = None;
                    self.status = if action == "push" {
                        "Metadata pushed to Git.".into()
                    } else if action == "init" {
                        "Git repository initialized.".into()
                    } else {
                        self.status.clone()
                    };
                    save = true;
                }
                Err(error) => {
                    self.config.sync.last_error = Some(error.clone());
                    self.status = error;
                    save = true;
                }
            }
        }
        if save {
            self.save();
        }
        self.sync_open = open;
    }
}
