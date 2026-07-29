use super::*;

impl EasySshApp {
    pub(super) fn snippets(&mut self, ctx: &egui::Context) {
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

    pub(super) fn forwarding(&mut self, ctx: &egui::Context) {
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

    pub(super) fn transfers(&mut self, ctx: &egui::Context) {
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
                        egui::RichText::new("Transfer history is kept locally on this device.")
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
                                TransferStatus::Queued => {}
                                TransferStatus::Pending
                                | TransferStatus::Authorizing
                                | TransferStatus::Transferring => {
                                    if icon_button(ui, icon::X, "Cancel transfer").clicked() {
                                        cancel_id = Some(transfer.id.clone());
                                    }
                                }
                                TransferStatus::Failed
                                | TransferStatus::Cancelled
                                | TransferStatus::Interrupted => {
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
}
