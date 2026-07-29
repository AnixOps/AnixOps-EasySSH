use super::*;

impl EasySshApp {
    pub(super) fn hosts(&mut self, ctx: &egui::Context) {
        let strings = crate::ui::localization::Strings::new(self.config.locale);
        if self.inspector_open
            && matches!(
                crate::ui::shell::Breakpoint::for_width(self.viewport_width),
                crate::ui::shell::Breakpoint::Desktop
            )
        {
            egui::SidePanel::right("inspector")
                .exact_width(330.0)
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("HOST INSPECTOR").small().weak());
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if icon_button(ui, icon::X, "Close inspector").clicked() {
                                self.inspector_open = false;
                            }
                        });
                    });
                    self.inspector(ui);
                });
        }
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.add_space(12.0);
            ui.horizontal(|ui| {
                ui.heading(strings.text(crate::ui::localization::Key::Hosts));
                ui.label(
                    egui::RichText::new(format!("{} hosts", self.config.connections.len())).weak(),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if icon_button(ui, icon::SIDEBAR_SIMPLE, "Open host inspector").clicked() {
                        self.inspector_open = true;
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
                        if ui
                            .button(strings.text(crate::ui::localization::Key::NewHost))
                            .clicked()
                        {
                            self.add_host();
                        }
                    });
                }
            });
        });
        if self.inspector_open
            && !matches!(
                crate::ui::shell::Breakpoint::for_width(self.viewport_width),
                crate::ui::shell::Breakpoint::Desktop
            )
        {
            let mut open = true;
            egui::Window::new("Host inspector")
                .open(&mut open)
                .default_width(360.0)
                .show(ctx, |ui| self.inspector(ui));
            self.inspector_open = open;
        }
    }
}
