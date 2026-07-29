use super::*;

impl EasySshApp {
    pub(super) fn settings(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            let strings = crate::ui::localization::Strings::new(self.config.locale);
            ui.add_space(22.0);
            ui.heading(strings.text(crate::ui::localization::Key::Settings));
            let mut changed = false;
            ui.collapsing("Appearance", |ui| {
                ui.horizontal(|ui| {
                    ui.label("Theme");
                    changed |= ui.selectable_value(&mut self.config.theme, Theme::System, "System").changed();
                    changed |= ui.selectable_value(&mut self.config.theme, Theme::Light, "Light").changed();
                    changed |= ui.selectable_value(&mut self.config.theme, Theme::Dark, "Dark").changed();
                });
                ui.horizontal(|ui| {
                    ui.label("Density");
                    changed |= ui.selectable_value(&mut self.config.display_density, DisplayDensity::Compact, "Compact").changed();
                    changed |= ui.selectable_value(&mut self.config.display_density, DisplayDensity::Comfortable, "Comfortable").changed();
                    changed |= ui.selectable_value(&mut self.config.display_density, DisplayDensity::Large, "Large").changed();
                });
                ui.horizontal(|ui| {
                    ui.label("Language");
                    changed |= ui.selectable_value(&mut self.config.locale, Locale::System, "System").changed();
                    changed |= ui.selectable_value(&mut self.config.locale, Locale::En, "English").changed();
                    changed |= ui.selectable_value(&mut self.config.locale, Locale::ZhCn, "Chinese").changed();
                });
            });
            ui.collapsing("Terminal", |ui| {
                ui.label("External terminals are launched through the system OpenSSH command.");
            });
            ui.collapsing("SSH", |ui| {
                ui.label("Authentication remains delegated to OpenSSH and SSH Agent.");
            });
            ui.collapsing("File Transfers", |ui| {
                ui.label("Transfers are tracked in the Transfers workspace with cancel and retry controls.");
            });
            ui.collapsing("Shortcuts", |ui| {
                ui.label("Ctrl/Cmd+K opens the command palette.");
            });
            ui.collapsing("Experimental", |ui| {
                changed |= ui
                    .checkbox(
                        &mut self.config.experimental.remote_file_browser,
                        strings.text(crate::ui::localization::Key::RemoteFileBrowser),
                    )
                    .changed();
                changed |= ui.checkbox(&mut self.config.experimental.remote_text_editing, "Remote text editing").changed();
                changed |= ui.checkbox(&mut self.config.experimental.image_preview, "Image preview").changed();
                changed |= ui.checkbox(&mut self.config.experimental.dual_pane_file_browsing, "Dual-pane file browsing").changed();
                changed |= ui.checkbox(&mut self.config.experimental.git_metadata_sync_ui, "Git metadata sync").changed();
                if self.config.experimental.git_metadata_sync_ui
                    && ui.button(format!("{} Open sync settings", icon::ARROWS_CLOCKWISE)).clicked()
                {
                    self.sync_open = true;
                }
            });
            if changed {
                self.save();
            }
        });
    }
}
