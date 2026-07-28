use easyssh_core::{
    AppConfig, ConfigStore, Connection, ConnectionTarget, OpenSsh, TerminalSession, Theme,
};
use eframe::egui;

fn main() -> eframe::Result<()> {
    eframe::run_native(
        "EasySSH",
        eframe::NativeOptions::default(),
        Box::new(|_| Ok(Box::<EasySshApp>::default())),
    )
}

struct EasySshApp {
    store: ConfigStore,
    config: AppConfig,
    openssh: OpenSsh,
    selected: Option<String>,
    quick_host: String,
    quick_user: String,
    quick_port: u16,
    diagnostics_open: bool,
    status: String,
    session: Option<TerminalSession>,
    terminal_output: String,
    terminal_input: String,
}
impl Default for EasySshApp {
    fn default() -> Self {
        let store = ConfigStore::default_path().expect("configuration path");
        let config = store.load().unwrap_or_else(|_| AppConfig::new());
        Self {
            store,
            config,
            openssh: OpenSsh,
            selected: None,
            quick_host: String::new(),
            quick_user: String::new(),
            quick_port: 22,
            diagnostics_open: false,
            status: String::new(),
            session: None,
            terminal_output: String::new(),
            terminal_input: String::new(),
        }
    }
}
impl EasySshApp {
    fn save(&mut self) {
        self.status = match self.store.save(&self.config) {
            Ok(()) => "Saved connection metadata".into(),
            Err(error) => error.to_string(),
        };
    }
    fn connect(&mut self, connection: &Connection) {
        match TerminalSession::connect(&self.openssh, connection, 120, 36) {
            Ok(session) => {
                self.session = Some(session);
                self.terminal_output.clear();
                self.status = "SSH session started; waiting for OpenSSH and SSH Agent".into();
            }
            Err(error) => self.status = error.to_string(),
        }
    }
    fn poll_terminal(&mut self) {
        if let Some(session) = &self.session {
            while let Some(bytes) = session.try_read() {
                self.terminal_output
                    .push_str(&String::from_utf8_lossy(&bytes));
            }
        }
    }
}
impl eframe::App for EasySshApp {
    fn update(&mut self, ctx: &egui::Context, _: &mut eframe::Frame) {
        self.poll_terminal();
        if matches!(self.config.theme, Theme::Dark) {
            ctx.set_visuals(egui::Visuals::dark());
        }
        egui::SidePanel::left("hosts")
            .resizable(true)
            .show(ctx, |ui| {
                ui.heading("EasySSH");
                ui.small("OpenSSH + SSH Agent sessions");
                ui.separator();
                if ui.button("+ Host").clicked() {
                    self.config
                        .connections
                        .push(Connection::alias("New host", "host-alias"));
                    self.save();
                }
                ui.horizontal(|ui| {
                    ui.label("Quick connect");
                });
                ui.text_edit_singleline(&mut self.quick_host);
                ui.text_edit_singleline(&mut self.quick_user);
                ui.add(egui::DragValue::new(&mut self.quick_port).range(1..=65535));
                if ui.button("Open session").clicked() && !self.quick_host.is_empty() {
                    let mut connection = Connection::alias(&self.quick_host, &self.quick_host);
                    connection.target = ConnectionTarget::Endpoint {
                        hostname: self.quick_host.clone(),
                        username: (!self.quick_user.is_empty()).then(|| self.quick_user.clone()),
                        port: self.quick_port,
                    };
                    self.connect(&connection);
                }
                ui.separator();
                for connection in self.config.connections.clone() {
                    let selected = self.selected.as_deref() == Some(&connection.id);
                    if ui
                        .selectable_label(
                            selected,
                            format!(
                                "{}{}",
                                if connection.favorite { "* " } else { "" },
                                connection.name
                            ),
                        )
                        .clicked()
                    {
                        self.selected = Some(connection.id.clone());
                    }
                }
                ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                    if ui.button("SSH Agent status").clicked() {
                        self.diagnostics_open = true;
                    }
                });
            });
        egui::CentralPanel::default().show(ctx, |ui| {
            if let Some(id) = &self.selected {
                if let Some(index) = self.config.connections.iter().position(|item| &item.id == id) {
                    let connect = {
                        let connection = &mut self.config.connections[index];
                        ui.heading(&connection.name);
                        ui.label("Connection metadata only. Authentication is handled by the system OpenSSH agent.");
                        ui.text_edit_singleline(&mut connection.name);
                        ui.checkbox(&mut connection.favorite, "Favorite");
                        ui.text_edit_multiline(&mut connection.notes);
                        ui.button("Connect").clicked()
                    };
                    if connect {
                        let connection = self.config.connections[index].clone();
                        self.connect(&connection);
                    }
                    if ui.button("Save").clicked() {
                        self.save();
                    }
                }
            } else {
                ui.heading("Sessions");
                ui.label("Choose a host or use Quick connect.");
            }
            ui.separator(); ui.label(&self.status);
            if let Some(session) = &mut self.session { ui.separator(); ui.heading("Terminal"); egui::ScrollArea::vertical().stick_to_bottom(true).show(ui, |ui| { ui.monospace(&self.terminal_output); }); let response = ui.add(egui::TextEdit::singleline(&mut self.terminal_input).hint_text("Type into the SSH session")); if response.lost_focus() && ui.input(|input| input.key_pressed(egui::Key::Enter)) { let input = std::mem::take(&mut self.terminal_input); if let Err(error) = session.write(format!("{input}\n").as_bytes()) { self.status = error.to_string(); } } }
        });
        if self.diagnostics_open {
            egui::Window::new("SSH Agent status")
                .open(&mut self.diagnostics_open)
                .show(ctx, |ui| {
                    let diagnostics = self.openssh.diagnostics(
                        self.quick_host
                            .is_empty()
                            .then_some("localhost")
                            .or(Some(&self.quick_host)),
                    );
                    ui.label(format!(
                        "ssh: {}",
                        diagnostics
                            .ssh_path
                            .as_ref()
                            .map(|p| p.display().to_string())
                            .unwrap_or_else(|| "not found".into())
                    ));
                    ui.label(format!(
                        "scp: {}",
                        diagnostics
                            .scp_path
                            .as_ref()
                            .map(|p| p.display().to_string())
                            .unwrap_or_else(|| "not found".into())
                    ));
                    ui.label(format!(
                        "Agent environment: {}",
                        if diagnostics.agent_socket_configured {
                            "configured"
                        } else {
                            "not configured"
                        }
                    ));
                    if let Some(version) = diagnostics.ssh_version {
                        ui.label(version);
                    }
                    if let Some(keys) = diagnostics.agent_keys {
                        ui.label(if keys.available {
                            format!("Agent keys: {}", keys.fingerprints.len())
                        } else {
                            format!("Agent unavailable: {}", keys.raw_error.unwrap_or_default())
                        });
                        for key in keys.fingerprints {
                            ui.monospace(key);
                        }
                    }
                });
        }
    }
}
