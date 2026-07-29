use super::*;

impl EasySshApp {
    pub(super) fn host_row(&mut self, ui: &mut egui::Ui, host: &Connection) {
        let strings = crate::ui::localization::Strings::new(self.config.locale);
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
            self.inspector_open = true;
        }
        if response.double_clicked() {
            self.connect(host, false);
        }
        response.context_menu(|ui| {
            if ui
                .button(strings.text(crate::ui::localization::Key::Connect))
                .clicked()
            {
                self.connect(host, false);
                ui.close_menu();
            }
            if ui.button("Detailed log").clicked() {
                self.connect(host, true);
                ui.close_menu();
            }
            if ui.button("Edit").clicked() {
                self.selected = Some(host.id.clone());
                self.host_form = Some(state::host_form::State::existing(host.clone()));
                self.editor_open = true;
                ui.close_menu();
            }
        });
    }

    pub(super) fn inspector(&mut self, ui: &mut egui::Ui) {
        let strings = crate::ui::localization::Strings::new(self.config.locale);
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
            if ui
                .button(format!(
                    "{} {}",
                    icon::PLAY,
                    strings.text(crate::ui::localization::Key::Connect)
                ))
                .clicked()
            {
                self.connect(&host, false);
            }
            if self.remote_file_browser_enabled()
                && icon_button(ui, icon::FOLDER_OPEN, "Open files for this host").clicked()
            {
                self.files_connection = Some(host.id.clone());
                self.config.workspace = Workspace::Files;
            }
            if icon_button(ui, icon::WARNING, "Connect with detailed OpenSSH log").clicked() {
                self.connect(&host, true);
            }
            if icon_button(ui, icon::PENCIL, "Edit host").clicked() {
                self.host_form = Some(state::host_form::State::existing(host.clone()));
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
}
