use super::*;

impl EasySshApp {
    pub(super) fn keys(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            let strings = crate::ui::localization::Strings::new(self.config.locale);
            ui.add_space(22.0);
            ui.heading(strings.text(crate::ui::localization::Key::Keys));
            ui.label(egui::RichText::new("SSH Agent and system OpenSSH diagnostics").weak());
            ui.add_space(12.0);
            if matches!(
                self.diagnostics_state.status,
                state::diagnostics::Status::Idle
            ) {
                self.diagnostics_state.request(ctx);
            }
            self.diagnostics_state.poll();
            let report = match &self.diagnostics_state.status {
                state::diagnostics::Status::Ready(report) => Some(report),
                state::diagnostics::Status::Loading => {
                    ui.spinner();
                    ui.label(strings.text(crate::ui::localization::Key::Checking));
                    None
                }
                state::diagnostics::Status::Failed(error) => {
                    ui.colored_label(AMBER, error);
                    None
                }
                state::diagnostics::Status::Idle => None,
            };
            let pending = matches!(
                self.diagnostics_state.status,
                state::diagnostics::Status::Loading
            );
            detail(
                ui,
                "OpenSSH",
                report
                    .and_then(|report| report.ssh_path.as_ref())
                    .map(|p| p.display().to_string())
                    .unwrap_or_else(|| {
                        if pending {
                            strings.text(crate::ui::localization::Key::Checking).into()
                        } else {
                            strings.text(crate::ui::localization::Key::NotFound).into()
                        }
                    }),
            );
            detail(
                ui,
                "Agent",
                if pending {
                    strings.text(crate::ui::localization::Key::Checking).into()
                } else if report
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
                "Discovered identities",
                report
                    .and_then(|report| report.agent_keys.as_ref())
                    .map(|k| k.fingerprints.len().to_string())
                    .unwrap_or_else(|| {
                        if pending {
                            strings.text(crate::ui::localization::Key::Checking).into()
                        } else {
                            strings.text(crate::ui::localization::Key::NotFound).into()
                        }
                    }),
            );
            ui.add_space(8.0);
            ui.label(
                egui::RichText::new(
                    "Passwords, private key contents, and key paths are never stored by EasySSH.",
                )
                .small()
                .weak(),
            );
            if ui
                .button(format!(
                    "{} {}",
                    icon::ARROWS_CLOCKWISE,
                    strings.text(crate::ui::localization::Key::Refresh)
                ))
                .clicked()
            {
                self.diagnostics_state.request(ctx);
            }
        });
    }
}
