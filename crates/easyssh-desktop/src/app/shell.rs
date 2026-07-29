use super::*;

impl EasySshApp {
    pub(super) fn workspace_button(
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

    pub(super) fn topbar(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("topbar")
            .exact_height(56.0)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    let strings = crate::ui::localization::Strings::new(self.config.locale);
                    ui.add_space(10.0);
                    ui.label(egui::RichText::new(icon::TERMINAL).size(26.0).color(BLUE));
                    ui.label(egui::RichText::new("EasySSH").strong().size(20.0));
                    ui.label(egui::RichText::new("WORKBENCH").small().weak());
                    ui.add_space(14.0);
                    if matches!(
                        crate::ui::shell::Breakpoint::for_width(self.viewport_width),
                        crate::ui::shell::Breakpoint::Mobile
                    ) {
                        ui.menu_button(icon::LIST, |ui| {
                            for (workspace, key) in [
                                (Workspace::Home, crate::ui::localization::Key::Home),
                                (Workspace::Hosts, crate::ui::localization::Key::Hosts),
                                (
                                    Workspace::Transfers,
                                    crate::ui::localization::Key::Transfers,
                                ),
                                (Workspace::Keys, crate::ui::localization::Key::Keys),
                                (Workspace::Settings, crate::ui::localization::Key::Settings),
                            ] {
                                if ui.button(strings.text(key)).clicked() {
                                    self.config.workspace = workspace;
                                    self.save();
                                    ui.close_menu();
                                }
                            }
                            ui.separator();
                            if ui
                                .button(strings.text(crate::ui::localization::Key::NewHost))
                                .clicked()
                            {
                                self.add_host();
                                ui.close_menu();
                            }
                            if ui
                                .button(strings.text(crate::ui::localization::Key::Diagnostics))
                                .clicked()
                            {
                                self.diagnostics_open = true;
                                ui.close_menu();
                            }
                            if ui
                                .button(strings.text(crate::ui::localization::Key::CycleTheme))
                                .clicked()
                            {
                                self.config.theme = match self.config.theme {
                                    Theme::System => Theme::Dark,
                                    Theme::Dark => Theme::Light,
                                    Theme::Light => Theme::System,
                                };
                                self.save();
                                ui.close_menu();
                            }
                            if self.config.experimental.git_metadata_sync_ui
                                && ui
                                    .button(
                                        strings.text(crate::ui::localization::Key::GitMetadataSync),
                                    )
                                    .clicked()
                            {
                                self.sync_open = true;
                                ui.close_menu();
                            }
                        });
                        if icon_button(
                            ui,
                            icon::MAGNIFYING_GLASS,
                            strings.text(crate::ui::localization::Key::SearchCommands),
                        )
                        .clicked()
                        {
                            self.command_open = true;
                        }
                        if ui
                            .button(format!(
                                "{} {}",
                                icon::LIGHTNING,
                                strings.text(crate::ui::localization::Key::Connect)
                            ))
                            .clicked()
                        {
                            self.quick_open = true;
                        }
                    }
                    if !matches!(
                        crate::ui::shell::Breakpoint::for_width(self.viewport_width),
                        crate::ui::shell::Breakpoint::Mobile
                    ) && ui
                        .add_sized(
                            [270.0, 32.0],
                            egui::Button::new(format!(
                                "{}  {}",
                                icon::MAGNIFYING_GLASS,
                                strings.text(crate::ui::localization::Key::SearchCommands)
                            ))
                            .sense(egui::Sense::click()),
                        )
                        .clicked()
                    {
                        self.command_open = true;
                    }
                    if !matches!(
                        crate::ui::shell::Breakpoint::for_width(self.viewport_width),
                        crate::ui::shell::Breakpoint::Mobile
                    ) {
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if self.config.experimental.git_metadata_sync_ui
                                && icon_button(
                                    ui,
                                    icon::ARROWS_CLOCKWISE,
                                    strings.text(crate::ui::localization::Key::GitMetadataSync),
                                )
                                .clicked()
                            {
                                self.sync_open = true;
                            }
                            if icon_button(
                                ui,
                                icon::PLUG,
                                strings.text(crate::ui::localization::Key::Diagnostics),
                            )
                            .clicked()
                            {
                                self.diagnostics_open = true;
                            }
                            let theme_icon = if matches!(self.config.theme, Theme::Dark) {
                                icon::MOON
                            } else {
                                icon::SUN
                            };
                            if icon_button(
                                ui,
                                theme_icon,
                                strings.text(crate::ui::localization::Key::CycleTheme),
                            )
                            .clicked()
                            {
                                self.config.theme = match self.config.theme {
                                    Theme::System => Theme::Dark,
                                    Theme::Dark => Theme::Light,
                                    Theme::Light => Theme::System,
                                };
                                self.save();
                            }
                            if ui
                                .button(format!(
                                    "{} {}",
                                    icon::PLUS,
                                    strings.text(crate::ui::localization::Key::NewHost)
                                ))
                                .clicked()
                            {
                                self.add_host();
                            }
                            if ui
                                .button(format!(
                                    "{} {}",
                                    icon::LIGHTNING,
                                    strings.text(crate::ui::localization::Key::Connect)
                                ))
                                .clicked()
                            {
                                self.quick_open = true;
                            }
                            let agent_ready =
                                self.diagnostics_state.agent_available().unwrap_or(false);
                            let agent = strings.text(if agent_ready {
                                crate::ui::localization::Key::AgentReady
                            } else {
                                crate::ui::localization::Key::Agent
                            });
                            ui.label(
                                egui::RichText::new(format!("{} {}", icon::PLUG, agent))
                                    .small()
                                    .color(if agent_ready { GREEN } else { AMBER }),
                            );
                            ui.label(
                                egui::RichText::new(sync_status_label(GitSync::status(
                                    &self.config,
                                )))
                                .small()
                                .weak(),
                            );
                        });
                    }
                });
            });
        egui::TopBottomPanel::top("sessions")
            .exact_height(42.0)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    let strings = crate::ui::localization::Strings::new(self.config.locale);
                    ui.label(
                        egui::RichText::new(
                            strings.text(crate::ui::localization::Key::RecentlyLaunched),
                        )
                        .small()
                        .weak(),
                    );
                    ui.label(
                        egui::RichText::new(
                            strings.text(crate::ui::localization::Key::ExternalTerminalLaunches),
                        )
                        .small()
                        .weak(),
                    );
                    let sessions: Vec<SessionRecord> = self
                        .config
                        .sessions
                        .iter()
                        .filter(|s| !s.hidden)
                        .cloned()
                        .collect();
                    egui::ScrollArea::horizontal()
                        .id_salt("recent-sessions")
                        .show(ui, |ui| {
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
                                    if ui
                                        .button(
                                            strings.text(
                                                crate::ui::localization::Key::HideFromSessionBar,
                                            ),
                                        )
                                        .clicked()
                                    {
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
            });
    }

    pub(super) fn reconnect_session(&mut self, session: &SessionRecord) {
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

    pub(super) fn navigation(&mut self, ctx: &egui::Context) {
        if matches!(
            crate::ui::shell::Breakpoint::for_width(self.viewport_width),
            crate::ui::shell::Breakpoint::Mobile
        ) {
            return;
        }
        egui::SidePanel::left("navigation")
            .exact_width(190.0)
            .show(ctx, |ui| {
                let strings = crate::ui::localization::Strings::new(self.config.locale);
                ui.add_space(10.0);
                ui.label(
                    egui::RichText::new(strings.text(crate::ui::localization::Key::Workspaces))
                        .small()
                        .weak(),
                );
                ui.add_space(4.0);
                self.workspace_button(
                    ui,
                    Workspace::Home,
                    icon::HOUSE,
                    strings.text(crate::ui::localization::Key::Home),
                );
                self.workspace_button(
                    ui,
                    Workspace::Hosts,
                    icon::COMPUTER_TOWER,
                    strings.text(crate::ui::localization::Key::Hosts),
                );
                self.workspace_button(
                    ui,
                    Workspace::Transfers,
                    icon::ARROW_FAT_LINES_UP,
                    strings.text(crate::ui::localization::Key::Transfers),
                );
                self.workspace_button(
                    ui,
                    Workspace::Keys,
                    icon::KEY,
                    strings.text(crate::ui::localization::Key::Keys),
                );
                self.workspace_button(
                    ui,
                    Workspace::Settings,
                    icon::GEAR,
                    strings.text(crate::ui::localization::Key::Settings),
                );
                ui.add_space(12.0);
                ui.separator();
                ui.add_space(8.0);
                ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                    ui.label(egui::RichText::new(&self.status).small().weak().italics());
                });
            });
    }
}
