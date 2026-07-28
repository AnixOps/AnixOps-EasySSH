use chrono::{DateTime, Utc};
use easyssh_core::{
    cancel, AppConfig, CommandSnippet, ConfigStore, Connection, ConnectionTarget, DisplayDensity,
    ExternalTerminal, GitSync, OpenSsh, ScpInvocation, SessionRecord, SshInvocation, SyncStatus,
    Theme, Transfer, TransferDirection, TransferStatus, Workspace,
};
use eframe::egui;
use egui_phosphor::regular as icon;
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Child;

mod ui;
#[cfg(feature = "ui-test")]
mod ui_test;

const BLUE: egui::Color32 = egui::Color32::from_rgb(111, 124, 255);
const GREEN: egui::Color32 = crate::ui::tokens::DARK.success;
const AMBER: egui::Color32 = crate::ui::tokens::DARK.warning;
const RED: egui::Color32 = crate::ui::tokens::DARK.danger;

fn main() -> eframe::Result<()> {
    #[cfg(feature = "ui-test")]
    let test_mode = ui_test::UiTestMode::from_args()
        .map_err(|error| eframe::Error::AppCreation(Box::new(std::io::Error::other(error))))?;
    #[cfg(feature = "ui-test")]
    let title = if test_mode.is_some() {
        "EasySSH [UI Test]"
    } else {
        "EasySSH"
    };
    #[cfg(not(feature = "ui-test"))]
    let title = "EasySSH";
    eframe::run_native(
        title,
        eframe::NativeOptions {
            // Hosts uses a permanent three-column layout (navigation, object
            // list, inspector). Keep enough room for targets and toolbar
            // actions instead of allowing panels to collapse into each other.
            viewport: egui::ViewportBuilder::default().with_min_inner_size([1200.0, 760.0]),
            ..Default::default()
        },
        Box::new(move |cc| {
            let mut fonts = egui::FontDefinitions::default();
            egui_phosphor::add_to_fonts(&mut fonts, egui_phosphor::Variant::Regular);
            cc.egui_ctx.set_fonts(fonts);
            #[cfg(feature = "ui-test")]
            let app = test_mode
                .clone()
                .map(EasySshApp::ui_test)
                .unwrap_or_default();
            #[cfg(not(feature = "ui-test"))]
            let app = EasySshApp::default();
            Ok(Box::new(app))
        }),
    )
}

struct EasySshApp {
    store: ConfigStore,
    config: AppConfig,
    openssh: OpenSsh,
    selected: Option<String>,
    search: String,
    command_query: String,
    command_open: bool,
    command_index: usize,
    quick_open: bool,
    quick_host: String,
    quick_user: String,
    quick_port: u16,
    editor_open: bool,
    snippet_editor: Option<String>,
    delete_host: Option<String>,
    delete_snippet: Option<String>,
    diagnostics_open: bool,
    sync_open: bool,
    agent_available: Option<bool>,
    copy_text: Option<String>,
    status: String,
    transfers: Vec<Transfer>,
    transfer_children: HashMap<String, Child>,
    transfer_connection: Option<String>,
    transfer_local_path: String,
    transfer_remote_path: String,
    transfer_recursive: bool,
    transfer_direction: TransferDirection,
    #[cfg(feature = "ui-test")]
    test_mode: Option<ui_test::UiTestMode>,
    #[cfg(feature = "ui-test")]
    test_ready: bool,
}

impl Default for EasySshApp {
    fn default() -> Self {
        let store = ConfigStore::default_path().expect("configuration path");
        Self::from_store(store, true)
    }
}

impl EasySshApp {
    fn from_store(store: ConfigStore, inspect_agent: bool) -> Self {
        let config = store.load().unwrap_or_else(|_| AppConfig::new());
        let agent_available = inspect_agent
            .then(|| {
                OpenSsh
                    .diagnostics(None)
                    .agent_keys
                    .map(|keys| keys.available)
            })
            .flatten();
        Self {
            store,
            config,
            openssh: OpenSsh,
            selected: None,
            search: String::new(),
            command_query: String::new(),
            command_open: false,
            command_index: 0,
            quick_open: false,
            quick_host: String::new(),
            quick_user: String::new(),
            quick_port: 22,
            editor_open: false,
            snippet_editor: None,
            delete_host: None,
            delete_snippet: None,
            diagnostics_open: false,
            sync_open: false,
            agent_available,
            copy_text: None,
            status: String::new(),
            transfers: Vec::new(),
            transfer_children: HashMap::new(),
            transfer_connection: None,
            transfer_local_path: String::new(),
            transfer_remote_path: String::new(),
            transfer_recursive: false,
            transfer_direction: TransferDirection::Upload,
            #[cfg(feature = "ui-test")]
            test_mode: None,
            #[cfg(feature = "ui-test")]
            test_ready: false,
        }
    }

    #[cfg(feature = "ui-test")]
    fn ui_test(mode: ui_test::UiTestMode) -> Self {
        let store = ConfigStore::at(mode.root.join("config").join("connections.json"));
        let mut app = Self::from_store(store, false);
        app.config.theme = Theme::Dark;
        app.config.workspace = Workspace::Hosts;
        app.status = "UI TEST MODE - isolated configuration".into();
        app.test_mode = Some(mode);
        app
    }
    fn save(&mut self) {
        self.status = match self.store.save(&self.config) {
            Ok(()) => "Workbench saved".into(),
            Err(error) => error.to_string(),
        };
    }

    fn selected_connection(&self) -> Option<Connection> {
        self.selected
            .as_ref()
            .and_then(|id| self.config.connections.iter().find(|c| &c.id == id))
            .cloned()
    }

    fn connect(&mut self, connection: &Connection, verbose: bool) {
        let result =
            SshInvocation::for_connection_with_verbosity(&self.openssh, connection, verbose)
                .and_then(|invocation| ExternalTerminal::launch(&connection.name, &invocation));
        let launched = result.is_ok();
        self.config
            .sessions
            .retain(|item| item.connection_id.as_deref() != Some(&connection.id));
        self.config.sessions.insert(
            0,
            SessionRecord {
                id: uuid::Uuid::new_v4().to_string(),
                connection_id: Some(connection.id.clone()),
                name: connection.name.clone(),
                target: target_text(connection),
                launched_at: Utc::now(),
                verbose,
                launched,
                hidden: false,
            },
        );
        self.config.sessions.truncate(20);
        self.status = match result {
            Ok(()) if verbose => "External terminal launched with detailed OpenSSH logging.".into(),
            Ok(()) => "External terminal launched.".into(),
            Err(error) => error.to_string(),
        };
        self.save();
    }

    fn add_host(&mut self) {
        let mut host = Connection::alias("New host", "");
        host.target = ConnectionTarget::Endpoint {
            hostname: String::new(),
            username: Some(String::new()),
            port: 22,
        };
        self.selected = Some(host.id.clone());
        self.config.connections.push(host);
        self.editor_open = true;
    }

    fn add_snippet(&mut self) {
        let item = CommandSnippet {
            id: uuid::Uuid::new_v4().to_string(),
            name: "New snippet".into(),
            content: String::new(),
        };
        self.snippet_editor = Some(item.id.clone());
        self.config.snippets.push(item);
    }

    fn start_transfer(&mut self) {
        let Some(connection) = self
            .transfer_connection
            .as_ref()
            .and_then(|id| self.config.connections.iter().find(|item| &item.id == id))
            .cloned()
        else {
            self.status = "Select a host before starting a transfer.".into();
            return;
        };
        let mut transfer = Transfer::new(
            self.transfer_direction,
            PathBuf::from(self.transfer_local_path.trim()),
            self.transfer_remote_path.trim().to_owned(),
            self.transfer_recursive,
        );
        transfer.status = TransferStatus::Authorizing;
        transfer.started_at = Some(Utc::now());
        match ScpInvocation::build(&self.openssh, &connection, &transfer)
            .and_then(|invocation| invocation.spawn())
        {
            Ok(child) => {
                self.transfer_children.insert(transfer.id.clone(), child);
                self.transfers.insert(0, transfer);
                self.status = "Transfer started in the system OpenSSH environment.".into();
            }
            Err(error) => {
                transfer.status = TransferStatus::Failed;
                transfer.finished_at = Some(Utc::now());
                transfer.output = error.to_string();
                self.status = transfer.output.clone();
                self.transfers.insert(0, transfer);
            }
        }
    }

    fn poll_transfers(&mut self, ctx: &egui::Context) {
        let active: Vec<String> = self.transfer_children.keys().cloned().collect();
        for id in active {
            let result = self.transfer_children.get_mut(&id).map(Child::try_wait);
            match result {
                Some(Ok(Some(exit))) => {
                    self.transfer_children.remove(&id);
                    if let Some(transfer) = self.transfers.iter_mut().find(|item| item.id == id) {
                        transfer.finished_at = Some(Utc::now());
                        transfer.status = if exit.success() {
                            TransferStatus::Completed
                        } else {
                            TransferStatus::Failed
                        };
                        transfer.output = if exit.success() {
                            "Completed".into()
                        } else {
                            "The system scp process exited unsuccessfully.".into()
                        };
                    }
                }
                Some(Ok(None)) => {
                    if let Some(transfer) = self.transfers.iter_mut().find(|item| item.id == id) {
                        transfer.status = TransferStatus::Transferring;
                    }
                }
                Some(Err(error)) => {
                    self.transfer_children.remove(&id);
                    if let Some(transfer) = self.transfers.iter_mut().find(|item| item.id == id) {
                        transfer.status = TransferStatus::Failed;
                        transfer.finished_at = Some(Utc::now());
                        transfer.output = error.to_string();
                    }
                }
                None => {}
            }
        }
        if !self.transfer_children.is_empty() {
            ctx.request_repaint_after(std::time::Duration::from_millis(250));
        }
    }

    fn cancel_transfer(&mut self, id: &str) {
        if let Some(mut child) = self.transfer_children.remove(id) {
            let _ = cancel(&mut child);
        }
        if let Some(transfer) = self.transfers.iter_mut().find(|item| item.id == id) {
            transfer.status = TransferStatus::Cancelled;
            transfer.finished_at = Some(Utc::now());
            transfer.output = "Cancelled by user.".into();
        }
    }

    fn workspace_button(
        &mut self,
        ui: &mut egui::Ui,
        workspace: Workspace,
        glyph: &str,
        label: &str,
    ) {
        let selected = self.config.workspace == workspace;
        if ui
            .add_sized(
                [ui.available_width(), 30.0],
                egui::SelectableLabel::new(selected, format!("{}  {}", glyph, label)),
            )
            .clicked()
        {
            self.config.workspace = workspace;
            self.search.clear();
            self.save();
        }
    }

    fn topbar(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("topbar")
            .exact_height(56.0)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.add_space(10.0);
                    ui.label(egui::RichText::new(icon::TERMINAL).size(26.0).color(BLUE));
                    ui.label(egui::RichText::new("EasySSH").strong().size(20.0));
                    ui.label(egui::RichText::new("WORKBENCH").small().weak());
                    ui.add_space(14.0);
                    if ui
                        .add_sized(
                            [270.0, 32.0],
                            egui::Button::new(format!(
                                "{}  Search commands and hosts",
                                icon::MAGNIFYING_GLASS
                            ))
                            .sense(egui::Sense::click()),
                        )
                        .clicked()
                    {
                        self.command_open = true;
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if icon_button(ui, icon::ARROWS_CLOCKWISE, "Git metadata sync").clicked() {
                            self.sync_open = true;
                        }
                        if icon_button(ui, icon::PLUG, "SSH Agent diagnostics").clicked() {
                            self.diagnostics_open = true;
                        }
                        let theme_icon = if matches!(self.config.theme, Theme::Dark) {
                            icon::MOON
                        } else {
                            icon::SUN
                        };
                        if icon_button(ui, theme_icon, "Cycle theme").clicked() {
                            self.config.theme = match self.config.theme {
                                Theme::System => Theme::Dark,
                                Theme::Dark => Theme::Light,
                                Theme::Light => Theme::System,
                            };
                            self.save();
                        }
                        if ui.button(format!("{} New", icon::PLUS)).clicked() {
                            self.add_host();
                        }
                        if ui.button(format!("{} Connect", icon::LIGHTNING)).clicked() {
                            self.quick_open = true;
                        }
                        let agent = if self.agent_available.unwrap_or(false) {
                            "Agent ready"
                        } else {
                            "Agent"
                        };
                        ui.label(
                            egui::RichText::new(format!("{} {}", icon::PLUG, agent))
                                .small()
                                .color(if self.agent_available.unwrap_or(false) {
                                    GREEN
                                } else {
                                    AMBER
                                }),
                        );
                        ui.label(
                            egui::RichText::new(sync_status_label(GitSync::status(&self.config)))
                                .small()
                                .weak(),
                        );
                    });
                });
            });
        egui::TopBottomPanel::top("sessions")
            .exact_height(42.0)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("RECENT SESSIONS").small().weak());
                    let sessions: Vec<SessionRecord> = self
                        .config
                        .sessions
                        .iter()
                        .filter(|s| !s.hidden)
                        .cloned()
                        .collect();
                    for session in sessions {
                        let response = ui.add(
                            egui::Button::new(format!(
                                "{} {}  {}",
                                if session.launched {
                                    icon::ARROW_SQUARE_OUT
                                } else {
                                    icon::WARNING
                                },
                                session.name,
                                relative_time(session.launched_at)
                            ))
                            .small(),
                        );
                        if response.clicked() {
                            self.reconnect_session(&session);
                        }
                        response.context_menu(|ui| {
                            if ui.button("Hide from session bar").clicked() {
                                if let Some(item) = self
                                    .config
                                    .sessions
                                    .iter_mut()
                                    .find(|item| item.id == session.id)
                                {
                                    item.hidden = true;
                                }
                                self.save();
                                ui.close_menu();
                            }
                        });
                    }
                });
            });
    }

    fn reconnect_session(&mut self, session: &SessionRecord) {
        if let Some(connection) = session
            .connection_id
            .as_ref()
            .and_then(|id| self.config.connections.iter().find(|c| &c.id == id))
            .cloned()
        {
            self.connect(&connection, session.verbose);
        } else {
            self.status = "This session's host is no longer available.".into();
        }
    }

    fn navigation(&mut self, ctx: &egui::Context) {
        egui::SidePanel::left("navigation")
            .exact_width(190.0)
            .show(ctx, |ui| {
                ui.add_space(10.0);
                ui.label(egui::RichText::new("WORKSPACES").small().weak());
                ui.add_space(4.0);
                self.workspace_button(ui, Workspace::Hosts, icon::COMPUTER_TOWER, "Hosts");
                self.workspace_button(ui, Workspace::Snippets, icon::CODE, "Snippets");
                self.workspace_button(
                    ui,
                    Workspace::Forwarding,
                    icon::ARROWS_LEFT_RIGHT,
                    "Port forwarding",
                );
                self.workspace_button(
                    ui,
                    Workspace::Transfers,
                    icon::ARROW_FAT_LINES_UP,
                    "Transfers",
                );
                ui.add_space(12.0);
                ui.separator();
                ui.add_space(8.0);
                ui.label(egui::RichText::new("TOOLS").small().weak());
                if ui.button(format!("{}  Agent", icon::PLUG)).clicked() {
                    self.diagnostics_open = true;
                }
                if ui
                    .button(format!("{}  Sync", icon::ARROWS_CLOCKWISE))
                    .clicked()
                {
                    self.sync_open = true;
                }
                ui.add_enabled(
                    false,
                    egui::Button::new(format!("{}  Known Hosts", icon::SHIELD_CHECK)),
                );
                ui.add_enabled(
                    false,
                    egui::Button::new(format!("{}  Logs", icon::FILE_TEXT)),
                );
                ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                    ui.label(egui::RichText::new(&self.status).small().weak().italics());
                });
            });
    }

    fn hosts(&mut self, ctx: &egui::Context) {
        egui::SidePanel::right("inspector")
            .exact_width(330.0)
            .show(ctx, |ui| {
                self.inspector(ui);
            });
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.add_space(12.0);
            ui.horizontal(|ui| {
                ui.heading("Hosts");
                ui.label(
                    egui::RichText::new(format!("{} hosts", self.config.connections.len())).weak(),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button(format!("{} Add host", icon::PLUS)).clicked() {
                        self.add_host();
                    }
                });
            });
            ui.add(
                egui::TextEdit::singleline(&mut self.search)
                    .hint_text("Search hosts, groups, or tags")
                    .desired_width(f32::INFINITY),
            );
            ui.add_space(8.0);
            let mut groups: Vec<Option<String>> = self
                .config
                .groups
                .iter()
                .map(|g| Some(g.id.clone()))
                .collect();
            if self.config.connections.iter().any(|c| c.group_id.is_none()) {
                groups.push(None);
            }
            if groups.is_empty() {
                groups.push(None);
            }
            egui::ScrollArea::vertical().show(ui, |ui| {
                for group_id in groups {
                    let title = group_id
                        .as_ref()
                        .and_then(|id| self.config.groups.iter().find(|g| &g.id == id))
                        .map(|g| g.name.clone())
                        .unwrap_or_else(|| "Ungrouped".into());
                    let entries: Vec<Connection> = self
                        .config
                        .connections
                        .iter()
                        .filter(|c| c.group_id == group_id && host_matches(c, &self.search, &title))
                        .cloned()
                        .collect();
                    if entries.is_empty() {
                        continue;
                    }
                    ui.label(egui::RichText::new(title).small().strong().color(BLUE));
                    for host in entries {
                        self.host_row(ui, &host);
                    }
                    ui.add_space(8.0);
                }
                if self.config.connections.is_empty() {
                    ui.add_space(64.0);
                    ui.vertical_centered(|ui| {
                        ui.label(
                            egui::RichText::new(icon::COMPUTER_TOWER)
                                .size(40.0)
                                .color(BLUE),
                        );
                        ui.label("No hosts yet");
                        if ui.button("Create host").clicked() {
                            self.add_host();
                        }
                    });
                }
            });
        });
    }

    fn host_row(&mut self, ui: &mut egui::Ui, host: &Connection) {
        let selected = self.selected.as_deref() == Some(&host.id);
        let recent = self
            .config
            .sessions
            .iter()
            .find(|s| s.connection_id.as_deref() == Some(&host.id))
            .map(|s| relative_time(s.launched_at))
            .unwrap_or_else(|| "Never used".into());
        let response = ui.add_sized(
            [ui.available_width(), 60.0],
            egui::SelectableLabel::new(
                selected,
                egui::RichText::new(format!(
                    "{}{}\n{}    {}",
                    if host.favorite {
                        format!("{} ", icon::STAR)
                    } else {
                        String::new()
                    },
                    host.name,
                    target_text(host),
                    recent
                ))
                .size(15.0),
            ),
        );
        if response.clicked() {
            self.selected = Some(host.id.clone());
        }
        if response.double_clicked() {
            self.connect(host, false);
        }
        response.context_menu(|ui| {
            if ui.button("Connect").clicked() {
                self.connect(host, false);
                ui.close_menu();
            }
            if ui.button("Detailed log").clicked() {
                self.connect(host, true);
                ui.close_menu();
            }
            if ui.button("Edit").clicked() {
                self.selected = Some(host.id.clone());
                self.editor_open = true;
                ui.close_menu();
            }
        });
    }

    fn inspector(&mut self, ui: &mut egui::Ui) {
        ui.add_space(10.0);
        ui.label(egui::RichText::new("INSPECTOR").small().weak());
        let Some(host) = self.selected_connection() else {
            ui.add_space(48.0);
            ui.vertical_centered(|ui| {
                ui.label(egui::RichText::new(icon::SIDEBAR_SIMPLE).size(34.0).weak());
                ui.label(egui::RichText::new("Select a host").weak());
            });
            return;
        };
        ui.heading(&host.name);
        ui.label(egui::RichText::new(target_text(&host)).monospace().weak());
        ui.horizontal(|ui| {
            if ui.button(format!("{} Connect", icon::PLAY)).clicked() {
                self.connect(&host, false);
            }
            if icon_button(ui, icon::WARNING, "Connect with detailed OpenSSH log").clicked() {
                self.connect(&host, true);
            }
            if icon_button(ui, icon::PENCIL, "Edit host").clicked() {
                self.editor_open = true;
            }
        });
        ui.separator();
        section(ui, "OVERVIEW");
        detail(
            ui,
            "Group",
            host.group_id
                .as_ref()
                .and_then(|id| self.config.groups.iter().find(|g| &g.id == id))
                .map(|g| g.name.clone())
                .unwrap_or_else(|| "Ungrouped".into()),
        );
        detail(
            ui,
            "Tags",
            if host.tags.is_empty() {
                "None".into()
            } else {
                host.tags.join(", ")
            },
        );
        section(ui, "CONNECTION");
        detail(ui, "Target", target_text(&host));
        detail(
            ui,
            "Proxy jump",
            host.proxy_jump.clone().unwrap_or_else(|| "None".into()),
        );
        section(ui, "FORWARDING");
        detail(ui, "Local", count_forwards(&host.local_forwards));
        detail(ui, "Remote", count_forwards(&host.remote_forwards));
        detail(ui, "Dynamic", count_forwards(&host.dynamic_forwards));
        section(ui, "NOTES");
        ui.label(if host.notes.is_empty() {
            "No notes"
        } else {
            &host.notes
        });
        ui.separator();
        ui.horizontal(|ui| {
            if icon_button(ui, icon::COPY, "Copy target").clicked() {
                ui.output_mut(|o| o.copied_text = target_text(&host));
            }
            if icon_button(ui, icon::STAR, "Toggle favorite").clicked() {
                if let Some(item) = self.config.connections.iter_mut().find(|c| c.id == host.id) {
                    item.favorite = !item.favorite;
                }
                self.save();
            }
            if icon_button(ui, icon::TRASH, "Delete host").clicked() {
                self.delete_host = Some(host.id.clone());
            }
        });
    }

    fn snippets(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.add_space(12.0);
            ui.horizontal(|ui| {
                ui.heading("Snippets");
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button(format!("{} New snippet", icon::PLUS)).clicked() {
                        self.add_snippet();
                    }
                });
            });
            ui.add(
                egui::TextEdit::singleline(&mut self.search)
                    .hint_text("Search snippets")
                    .desired_width(f32::INFINITY),
            );
            ui.add_space(8.0);
            let items: Vec<CommandSnippet> = self
                .config
                .snippets
                .iter()
                .filter(|s| contains(&format!("{} {}", s.name, s.content), &self.search))
                .cloned()
                .collect();
            egui::ScrollArea::vertical().show(ui, |ui| {
                for item in items {
                    ui.horizontal(|ui| {
                        ui.vertical(|ui| {
                            ui.label(egui::RichText::new(&item.name).strong());
                            ui.label(
                                egui::RichText::new(single_line(&item.content))
                                    .monospace()
                                    .small()
                                    .weak(),
                            );
                        });
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if icon_button(ui, icon::TRASH, "Delete snippet").clicked() {
                                self.delete_snippet = Some(item.id.clone());
                            }
                            if icon_button(ui, icon::PENCIL, "Edit snippet").clicked() {
                                self.snippet_editor = Some(item.id.clone());
                            }
                            if icon_button(ui, icon::COPY, "Copy snippet").clicked() {
                                ui.output_mut(|o| o.copied_text = item.content.clone());
                                self.status =
                                    "Snippet copied. Commands are never sent to a terminal.".into();
                            }
                        });
                    });
                    ui.separator();
                }
            });
        });
    }

    fn forwarding(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.add_space(12.0);
            ui.heading("Port forwarding");
            ui.label(
                egui::RichText::new("Definitions are applied only when you connect with OpenSSH.")
                    .weak(),
            );
            ui.add_space(8.0);
            egui::ScrollArea::vertical().show(ui, |ui| {
                let hosts = self.config.connections.clone();
                for host in hosts {
                    for (kind, values) in [
                        ("Local", &host.local_forwards),
                        ("Remote", &host.remote_forwards),
                        ("Dynamic", &host.dynamic_forwards),
                    ] {
                        for value in values {
                            ui.horizontal(|ui| {
                                ui.label(egui::RichText::new(kind).color(BLUE));
                                ui.label(egui::RichText::new(value).monospace());
                                ui.label(egui::RichText::new(&host.name).weak());
                                if icon_button(ui, icon::ARROW_RIGHT, "Open host inspector")
                                    .clicked()
                                {
                                    self.selected = Some(host.id.clone());
                                    self.config.workspace = Workspace::Hosts;
                                }
                            });
                        }
                    }
                }
            });
        });
    }

    fn transfers(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.add_space(12.0);
            ui.horizontal(|ui| {
                ui.heading("Transfers");
                ui.label(
                    egui::RichText::new("System scp with your existing SSH authentication").weak(),
                );
            });
            ui.separator();
            ui.horizontal(|ui| {
                ui.radio_value(
                    &mut self.transfer_direction,
                    TransferDirection::Upload,
                    "Upload",
                );
                ui.radio_value(
                    &mut self.transfer_direction,
                    TransferDirection::Download,
                    "Download",
                );
                ui.checkbox(&mut self.transfer_recursive, "Recursive");
            });
            egui::ComboBox::from_label("Host")
                .selected_text(
                    self.transfer_connection
                        .as_ref()
                        .and_then(|id| self.config.connections.iter().find(|item| &item.id == id))
                        .map(|item| item.name.as_str())
                        .unwrap_or("Select a host"),
                )
                .show_ui(ui, |ui| {
                    for host in &self.config.connections {
                        ui.selectable_value(
                            &mut self.transfer_connection,
                            Some(host.id.clone()),
                            &host.name,
                        );
                    }
                });
            ui.label("Local path");
            ui.text_edit_singleline(&mut self.transfer_local_path);
            ui.label("Remote path");
            ui.text_edit_singleline(&mut self.transfer_remote_path);
            let ready = self.transfer_connection.is_some()
                && !self.transfer_local_path.trim().is_empty()
                && !self.transfer_remote_path.trim().is_empty();
            if ui
                .add_enabled(
                    ready,
                    egui::Button::new(format!("{} Start transfer", icon::PLAY)),
                )
                .clicked()
            {
                self.start_transfer();
            }
            ui.add_space(12.0);
            ui.separator();
            if self.transfers.is_empty() {
                ui.add_space(36.0);
                ui.vertical_centered(|ui| {
                    ui.label(
                        egui::RichText::new(icon::ARROW_FAT_LINES_UP)
                            .size(36.0)
                            .color(BLUE),
                    );
                    ui.label("No transfers yet");
                    ui.label(
                        egui::RichText::new(
                            "Transfer records are kept only while this app is open.",
                        )
                        .small()
                        .weak(),
                    );
                });
                return;
            }
            let mut cancel_id = None;
            let mut retry_id = None;
            egui::ScrollArea::vertical().show(ui, |ui| {
                for transfer in self.transfers.clone() {
                    ui.horizontal(|ui| {
                        ui.vertical(|ui| {
                            ui.label(
                                egui::RichText::new(match transfer.direction {
                                    TransferDirection::Upload => "Upload",
                                    TransferDirection::Download => "Download",
                                })
                                .strong(),
                            );
                            ui.label(
                                egui::RichText::new(format!(
                                    "{}  <->  {}",
                                    transfer.local_path.display(),
                                    transfer.remote_path
                                ))
                                .monospace()
                                .small()
                                .weak(),
                            );
                            if !transfer.output.is_empty() {
                                ui.label(egui::RichText::new(&transfer.output).small().weak());
                            }
                        });
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            match transfer.status {
                                TransferStatus::Pending
                                | TransferStatus::Authorizing
                                | TransferStatus::Transferring => {
                                    if icon_button(ui, icon::X, "Cancel transfer").clicked() {
                                        cancel_id = Some(transfer.id.clone());
                                    }
                                }
                                TransferStatus::Failed | TransferStatus::Cancelled => {
                                    if icon_button(
                                        ui,
                                        icon::ARROW_CLOCKWISE,
                                        "Retry with the same paths",
                                    )
                                    .clicked()
                                    {
                                        retry_id = Some(transfer.id.clone());
                                    }
                                }
                                TransferStatus::Completed => {}
                            }
                            let (label, color) = transfer_status_badge(transfer.status);
                            crate::ui::components::status_badge(ui, label, color);
                        });
                    });
                    ui.separator();
                }
            });
            if let Some(id) = cancel_id {
                self.cancel_transfer(&id);
            }
            if let Some(id) = retry_id {
                if let Some(item) = self.transfers.iter().find(|item| item.id == id) {
                    self.transfer_local_path = item.local_path.to_string_lossy().into_owned();
                    self.transfer_remote_path = item.remote_path.clone();
                    self.transfer_recursive = item.recursive;
                    self.transfer_direction = item.direction;
                }
                self.status = "Review the transfer form and start the retry.".into();
            }
        });
    }

    fn command_panel(&mut self, ctx: &egui::Context) {
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

    fn command_actions(&self) -> Vec<CommandAction> {
        let query = &self.command_query;
        let mut items = vec![
            CommandAction::NewHost,
            CommandAction::Switch(Workspace::Hosts),
            CommandAction::Switch(Workspace::Snippets),
            CommandAction::Switch(Workspace::Forwarding),
            CommandAction::Switch(Workspace::Transfers),
            CommandAction::OpenSync,
        ];
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

    fn run_command(&mut self, action: CommandAction) {
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
                self.config.workspace = Workspace::Snippets;
            }
            CommandAction::Session(id, _) => {
                if let Some(session) = self.config.sessions.iter().find(|s| s.id == id).cloned() {
                    self.reconnect_session(&session);
                }
            }
            CommandAction::OpenSync => self.sync_open = true,
        }
    }

    fn host_editor(&mut self, ctx: &egui::Context) {
        if !self.editor_open {
            return;
        }
        let Some(index) = self
            .selected
            .as_ref()
            .and_then(|id| self.config.connections.iter().position(|h| &h.id == id))
        else {
            self.editor_open = false;
            return;
        };
        let mut open = self.editor_open;
        let mut save = false;
        egui::Window::new("Edit host")
            .open(&mut open)
            .resizable(true)
            .default_width(520.0)
            .show(ctx, |ui| {
                let host = &mut self.config.connections[index];
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
                ui.checkbox(&mut host.favorite, "Favorite");
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("Save").clicked() {
                        save = true;
                    }
                });
            });
        if save {
            self.save();
            open = false;
        }
        self.editor_open = open;
    }

    fn snippet_editor(&mut self, ctx: &egui::Context) {
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

    fn quick_connect(&mut self, ctx: &egui::Context) {
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
            let mut host = Connection::alias(&self.quick_host, &self.quick_host);
            host.target = ConnectionTarget::Endpoint {
                hostname: self.quick_host.clone(),
                username: (!self.quick_user.is_empty()).then(|| self.quick_user.clone()),
                port: self.quick_port,
            };
            self.connect(&host, false);
            open = false;
        }
        self.quick_open = open;
    }

    fn confirmation_dialogs(&mut self, ctx: &egui::Context) {
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

    fn diagnostics(&mut self, ctx: &egui::Context) {
        if !self.diagnostics_open {
            return;
        }
        let mut open = self.diagnostics_open;
        egui::Window::new("SSH Agent diagnostics")
            .open(&mut open)
            .show(ctx, |ui| {
                let report = self.openssh.diagnostics(None);
                self.agent_available = report.agent_keys.as_ref().map(|keys| keys.available);
                detail(
                    ui,
                    "OpenSSH",
                    report
                        .ssh_path
                        .map(|p| p.display().to_string())
                        .unwrap_or_else(|| "Not found".into()),
                );
                detail(
                    ui,
                    "Agent",
                    if report.agent_socket_configured {
                        "Configured".into()
                    } else {
                        "Not configured".into()
                    },
                );
                detail(
                    ui,
                    "Available keys",
                    report
                        .agent_keys
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
            });
        self.diagnostics_open = open;
    }

    fn sync_panel(&mut self, ctx: &egui::Context) {
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

    fn shortcuts(&mut self, ctx: &egui::Context) {
        if ctx.input(|i| i.modifiers.command && i.key_pressed(egui::Key::K)) {
            self.command_open = true;
        }
        if ctx.input(|i| i.modifiers.command && i.key_pressed(egui::Key::N)) {
            self.add_host();
        }
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) && !self.command_open {
            self.search.clear();
        }
    }
}

impl eframe::App for EasySshApp {
    fn update(&mut self, ctx: &egui::Context, _: &mut eframe::Frame) {
        #[cfg(feature = "ui-test")]
        if !self.test_ready {
            if let Some(mode) = &self.test_mode {
                mode.mark_ready();
                self.test_ready = true;
            }
        }
        #[cfg(feature = "ui-test")]
        if self
            .test_mode
            .as_ref()
            .is_some_and(ui_test::UiTestMode::stop_requested)
        {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            return;
        }
        if let Some(text) = self.copy_text.take() {
            ctx.output_mut(|output| output.copied_text = text);
        }
        crate::ui::theme::apply(ctx, self.config.theme, self.config.display_density);
        self.poll_transfers(ctx);
        self.shortcuts(ctx);
        self.topbar(ctx);
        self.navigation(ctx);
        match self.config.workspace {
            Workspace::Hosts => self.hosts(ctx),
            Workspace::Snippets => self.snippets(ctx),
            Workspace::Forwarding => self.forwarding(ctx),
            Workspace::Transfers => self.transfers(ctx),
        };
        self.command_panel(ctx);
        self.host_editor(ctx);
        self.snippet_editor(ctx);
        self.quick_connect(ctx);
        self.confirmation_dialogs(ctx);
        self.diagnostics(ctx);
        self.sync_panel(ctx);
    }
}

#[derive(Clone)]
enum CommandAction {
    NewHost,
    OpenSync,
    Switch(Workspace),
    Host(String, String),
    Connect(String, String, bool),
    Snippet(String, String),
    Session(String, String),
}
impl CommandAction {
    fn label(&self) -> String {
        match self {
            Self::NewHost => format!("{} New host", icon::PLUS),
            Self::OpenSync => format!("{} Open Git metadata sync", icon::ARROWS_CLOCKWISE),
            Self::Switch(Workspace::Hosts) => "Go to Hosts".into(),
            Self::Switch(Workspace::Snippets) => "Go to Snippets".into(),
            Self::Switch(Workspace::Forwarding) => "Go to Port forwarding".into(),
            Self::Switch(Workspace::Transfers) => "Go to Transfers".into(),
            Self::Host(_, name) => format!("Open host: {name}"),
            Self::Connect(_, name, false) => format!("Connect: {name}"),
            Self::Connect(_, name, true) => format!("Detailed log: {name}"),
            Self::Snippet(_, name) => format!("Copy snippet: {name}"),
            Self::Session(_, name) => format!("Reconnect session: {name}"),
        }
    }
}

fn icon_button(ui: &mut egui::Ui, glyph: &str, tooltip: &str) -> egui::Response {
    crate::ui::components::icon_button(ui, glyph, tooltip)
}
fn section(ui: &mut egui::Ui, label: &str) {
    ui.add_space(10.0);
    ui.label(egui::RichText::new(label).small().strong().color(BLUE));
}
fn detail(ui: &mut egui::Ui, label: &str, value: String) {
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new(label).weak());
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(value);
        });
    });
}
fn edit_lines(ui: &mut egui::Ui, values: &mut Vec<String>) {
    let mut text = values.join("\n");
    if ui
        .add_sized(
            [ui.available_width(), 46.0],
            egui::TextEdit::multiline(&mut text).desired_rows(2),
        )
        .changed()
    {
        *values = text
            .lines()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_owned)
            .collect();
    }
}
fn target_text(host: &Connection) -> String {
    match &host.target {
        ConnectionTarget::Alias { alias } => alias.clone(),
        ConnectionTarget::Endpoint {
            hostname,
            username,
            port,
        } => match username.as_deref().filter(|u| !u.is_empty()) {
            Some(user) => format!("{user}@{hostname}:{port}"),
            None => format!("{hostname}:{port}"),
        },
    }
}
fn relative_time(time: DateTime<Utc>) -> String {
    let seconds = (Utc::now() - time).num_seconds().max(0);
    if seconds < 60 {
        "now".into()
    } else if seconds < 3600 {
        format!("{}m", seconds / 60)
    } else if seconds < 86_400 {
        format!("{}h", seconds / 3600)
    } else {
        format!("{}d", seconds / 86_400)
    }
}
fn contains(value: &str, query: &str) -> bool {
    query.trim().is_empty() || value.to_lowercase().contains(&query.trim().to_lowercase())
}
fn host_matches(host: &Connection, query: &str, group: &str) -> bool {
    contains(
        &format!(
            "{} {} {} {}",
            host.name,
            target_text(host),
            host.tags.join(" "),
            group
        ),
        query,
    )
}
fn single_line(value: &str) -> String {
    value.lines().next().unwrap_or_default().to_owned()
}
fn count_forwards(values: &[String]) -> String {
    if values.is_empty() {
        "None".into()
    } else {
        values.len().to_string()
    }
}

fn sync_status_label(status: SyncStatus) -> &'static str {
    match status {
        SyncStatus::Unconfigured => "Sync: not configured",
        SyncStatus::Clean => "Sync: clean",
        SyncStatus::LocalChanges => "Sync: local changes",
        SyncStatus::RemoteUpdates => "Sync: remote updates",
        SyncStatus::Conflict => "Sync: conflict",
        SyncStatus::Failed => "Sync: unavailable",
    }
}

fn transfer_status_badge(status: TransferStatus) -> (&'static str, egui::Color32) {
    match status {
        TransferStatus::Pending => ("Pending", AMBER),
        TransferStatus::Authorizing => ("Authorizing", AMBER),
        TransferStatus::Transferring => ("Transferring", BLUE),
        TransferStatus::Completed => ("Completed", GREEN),
        TransferStatus::Failed => ("Failed", RED),
        TransferStatus::Cancelled => ("Cancelled", AMBER),
    }
}

#[allow(dead_code)]
fn apply_theme(ctx: &egui::Context, theme: Theme, density: DisplayDensity) {
    let scale = match density {
        DisplayDensity::Compact => 0.9,
        DisplayDensity::Comfortable => 1.0,
        DisplayDensity::Large => 1.15,
    };
    let mut style = (*ctx.style()).clone();
    style.text_styles.insert(
        egui::TextStyle::Body,
        egui::FontId::proportional(16.0 * scale),
    );
    style.text_styles.insert(
        egui::TextStyle::Button,
        egui::FontId::proportional(16.0 * scale),
    );
    style.text_styles.insert(
        egui::TextStyle::Heading,
        egui::FontId::proportional(22.0 * scale),
    );
    style.text_styles.insert(
        egui::TextStyle::Small,
        egui::FontId::proportional(13.0 * scale),
    );
    style.text_styles.insert(
        egui::TextStyle::Monospace,
        egui::FontId::monospace(15.0 * scale),
    );
    style.spacing.interact_size.y = 32.0 * scale;
    style.spacing.button_padding = egui::vec2(8.0 * scale, 4.0 * scale);
    ctx.set_style(style);
    let dark = match theme {
        Theme::Dark => true,
        Theme::Light => false,
        Theme::System => ctx.system_theme().unwrap_or(egui::Theme::Dark) == egui::Theme::Dark,
    };
    let mut visuals = if dark {
        egui::Visuals::dark()
    } else {
        egui::Visuals::light()
    };
    visuals.panel_fill = if dark {
        egui::Color32::from_rgb(18, 20, 27)
    } else {
        egui::Color32::from_rgb(246, 247, 251)
    };
    visuals.window_fill = if dark {
        egui::Color32::from_rgb(30, 33, 43)
    } else {
        egui::Color32::WHITE
    };
    visuals.extreme_bg_color = visuals.window_fill;
    visuals.faint_bg_color = if dark {
        egui::Color32::from_rgb(25, 28, 38)
    } else {
        egui::Color32::from_rgb(236, 238, 246)
    };
    visuals.selection.bg_fill = BLUE.gamma_multiply(0.6);
    visuals.widgets.hovered.bg_stroke.color = BLUE;
    visuals.widgets.active.bg_stroke.color = BLUE;
    visuals.hyperlink_color = BLUE;
    visuals.window_rounding = egui::Rounding::same(6.0);
    visuals.widgets.inactive.rounding = egui::Rounding::same(4.0);
    visuals.widgets.hovered.rounding = egui::Rounding::same(4.0);
    visuals.widgets.active.rounding = egui::Rounding::same(4.0);
    ctx.set_visuals(visuals);
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn target_includes_endpoint_user_and_port() {
        let mut c = Connection::alias("x", "prod");
        c.target = ConnectionTarget::Endpoint {
            hostname: "host".into(),
            username: Some("ops".into()),
            port: 2200,
        };
        assert_eq!(target_text(&c), "ops@host:2200");
    }
    #[test]
    fn relative_time_is_compact() {
        assert_eq!(relative_time(Utc::now()), "now");
    }
}
