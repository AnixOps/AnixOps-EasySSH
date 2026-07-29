use super::*;

impl EasySshApp {
    pub(super) fn home(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            let strings = crate::ui::localization::Strings::new(self.config.locale);
            ui.add_space(22.0);
            ui.heading(strings.text(crate::ui::localization::Key::Home));
            ui.label(
                egui::RichText::new(
                    strings.text(crate::ui::localization::Key::LocalFirstWorkspace),
                )
                .weak(),
            );
            ui.add_space(14.0);
            ui.group(|ui| {
                ui.label(
                    egui::RichText::new(strings.text(crate::ui::localization::Key::QuickConnect))
                        .strong(),
                );
                ui.horizontal(|ui| {
                    ui.add_sized(
                        [360.0, 32.0],
                        egui::TextEdit::singleline(&mut self.quick_host)
                            .hint_text("user@example.com:22"),
                    );
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
                });
                ui.label(
                    egui::RichText::new(
                        strings.text(crate::ui::localization::Key::UsesSystemOpenSsh),
                    )
                    .small(),
                );
            });
            ui.add_space(16.0);
            ui.columns(2, |columns| {
                columns[0].heading(strings.text(crate::ui::localization::Key::Favorites));
                let favorites: Vec<Connection> = self
                    .config
                    .connections
                    .iter()
                    .filter(|c| c.favorite)
                    .cloned()
                    .collect();
                if favorites.is_empty() {
                    columns[0].label(strings.text(crate::ui::localization::Key::NoFavoriteHosts));
                }
                for host in favorites {
                    if columns[0]
                        .button(format!("{}  {}", icon::STAR, host.name))
                        .clicked()
                    {
                        self.selected = Some(host.id.clone());
                        self.config.workspace = Workspace::Hosts;
                        self.inspector_open = true;
                    }
                }
                columns[1].heading(strings.text(crate::ui::localization::Key::RecentConnections));
                let recent: Vec<SessionRecord> = self
                    .config
                    .sessions
                    .iter()
                    .filter(|s| !s.hidden)
                    .take(5)
                    .cloned()
                    .collect();
                if recent.is_empty() {
                    columns[1].label(strings.text(crate::ui::localization::Key::NoSessions));
                }
                for session in recent {
                    if columns[1]
                        .button(format!("{}  {}", icon::ARROW_SQUARE_OUT, session.name))
                        .clicked()
                    {
                        self.reconnect_session(&session);
                    }
                }
            });
            ui.add_space(16.0);
            if ui
                .button(format!(
                    "{} {}",
                    icon::UPLOAD_SIMPLE,
                    strings.text(crate::ui::localization::Key::ImportSshConfig)
                ))
                .clicked()
            {
                let discovery = scan_default_ssh_config();
                self.ssh_config_aliases = discovery.aliases;
                self.ssh_config_warnings = discovery.warnings;
                self.ssh_config_scanned = true;
            }
            if self.ssh_config_scanned {
                ui.add_space(8.0);
                ui.label(
                    egui::RichText::new(strings.text(crate::ui::localization::Key::FromSshConfig))
                        .strong(),
                );
                if self.ssh_config_aliases.is_empty() {
                    ui.label(strings.text(crate::ui::localization::Key::NoHostAliases));
                }
                let existing_aliases: std::collections::BTreeSet<&str> = self
                    .config
                    .connections
                    .iter()
                    .filter_map(|connection| match &connection.target {
                        ConnectionTarget::Alias { alias } => Some(alias.as_str()),
                        ConnectionTarget::Endpoint { .. } => None,
                    })
                    .collect();
                let mut import_one = None;
                for alias in &self.ssh_config_aliases {
                    ui.horizontal(|ui| {
                        ui.label(format!("{} {}", icon::TERMINAL, alias));
                        if existing_aliases.contains(alias.as_str()) {
                            ui.label(
                                egui::RichText::new(
                                    strings.text(crate::ui::localization::Key::Added),
                                )
                                .small()
                                .weak(),
                            );
                        } else if ui
                            .small_button(strings.text(crate::ui::localization::Key::Add))
                            .clicked()
                        {
                            import_one = Some(alias.clone());
                        }
                    });
                }
                if ui
                    .add_enabled(
                        !self.ssh_config_aliases.is_empty(),
                        egui::Button::new(strings.text(crate::ui::localization::Key::AddAll)),
                    )
                    .clicked()
                {
                    self.import_ssh_config_aliases();
                }
                if let Some(alias) = import_one {
                    self.ssh_config_aliases = vec![alias];
                    self.import_ssh_config_aliases();
                    let discovery = scan_default_ssh_config();
                    self.ssh_config_aliases = discovery.aliases;
                }
                for warning in &self.ssh_config_warnings {
                    ui.label(egui::RichText::new(warning).small().color(AMBER));
                }
            }
        });
    }
}
