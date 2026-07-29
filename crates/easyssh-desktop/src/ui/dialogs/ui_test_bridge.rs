#[cfg(feature = "ui-test")]
use super::*;

#[cfg(feature = "ui-test")]
impl EasySshApp {
    pub(super) fn handle_ui_test_bridge(&mut self, ctx: &egui::Context) {
        let Some(mode) = self.test_mode.clone() else {
            return;
        };
        let Some(request) = mode.take_bridge_request() else {
            return;
        };
        let response = match request["operation"].as_str() {
            Some("get_ui_tree") => json!({"success":true,"tree":self.ui_test_tree()}),
            Some("click") if request["element_id"].as_str() == Some("navigation.home") => {
                self.config.workspace = Workspace::Home;
                json!({"success":true,"tree":self.ui_test_tree()})
            }
            Some("click") if request["element_id"].as_str() == Some("navigation.hosts") => {
                self.config.workspace = Workspace::Hosts;
                json!({"success":true,"tree":self.ui_test_tree()})
            }
            Some("click") if request["element_id"].as_str() == Some("hosts.add") => {
                self.config.workspace = Workspace::Hosts;
                self.add_host();
                json!({"success":true,"tree":self.ui_test_tree()})
            }
            Some("click")
                if request["element_id"].as_str() == Some("files.toggle_dual_pane")
                    && self.remote_file_browser_enabled() =>
            {
                self.files_dual_pane = !self.files_dual_pane;
                if self.files_dual_pane {
                    self.refresh_local_files();
                }
                json!({"success":true,"tree":self.ui_test_tree()})
            }
            Some("click")
                if request["element_id"].as_str() == Some("files.new_folder")
                    && self.remote_file_browser_enabled() =>
            {
                self.files_new_dir_name.clear();
                self.files_create_dir_open = true;
                json!({"success":true,"tree":self.ui_test_tree()})
            }
            Some("click") if request["element_id"].as_str() == Some("navigation.transfers") => {
                self.config.workspace = Workspace::Transfers;
                json!({"success":true,"tree":self.ui_test_tree()})
            }
            Some("click") if request["element_id"].as_str() == Some("navigation.keys") => {
                self.config.workspace = Workspace::Keys;
                self.diagnostics_state.request(ctx);
                json!({"success":true,"tree":self.ui_test_tree()})
            }
            Some("click") if request["element_id"].as_str() == Some("navigation.settings") => {
                self.config.workspace = Workspace::Settings;
                json!({"success":true,"tree":self.ui_test_tree()})
            }
            Some("click") if request["element_id"].as_str() == Some("settings.theme.light") => {
                self.config.theme = Theme::Light;
                json!({"success":true,"tree":self.ui_test_tree()})
            }
            Some("click") if request["element_id"].as_str() == Some("settings.theme.dark") => {
                self.config.theme = Theme::Dark;
                self.save();
                json!({"success":true,"tree":self.ui_test_tree()})
            }
            Some("click") if request["element_id"].as_str() == Some("keys.refresh") => {
                self.diagnostics_state.request(ctx);
                json!({"success":true,"tree":self.ui_test_tree()})
            }
            Some("click")
                if request["element_id"].as_str() == Some("settings.experimental.git_sync") =>
            {
                self.config.experimental.git_metadata_sync_ui =
                    !self.config.experimental.git_metadata_sync_ui;
                self.save();
                json!({"success":true,"tree":self.ui_test_tree()})
            }
            Some("click") if request["element_id"].as_str() == Some("hosts.editor.discard") => {
                self.close_host_form(true);
                json!({"success":true,"tree":self.ui_test_tree()})
            }
            Some("click") if request["element_id"].as_str() == Some("hosts.editor.close") => {
                if self.host_form.as_ref().is_some_and(|form| form.dirty()) {
                    self.host_form.as_mut().expect("host form").confirm_discard = true;
                } else {
                    self.close_host_form(true);
                }
                json!({"success":true,"tree":self.ui_test_tree()})
            }
            Some("double_click")
                if request["element_id"].as_str() == Some("navigation.transfers") =>
            {
                self.config.workspace = Workspace::Transfers;
                json!({"success":true,"tree":self.ui_test_tree()})
            }
            Some("type") if request["element_id"].as_str() == Some("transfers.local_path") => {
                if let Some(text) = request["text"].as_str() {
                    self.transfer_local_path = text.into();
                    json!({"success":true,"tree":self.ui_test_tree()})
                } else {
                    json!({"success":false,"error":"text is required"})
                }
            }
            Some("type") if request["element_id"].as_str() == Some("hosts.editor.name") => {
                if let (Some(form), Some(text)) =
                    (self.host_form.as_mut(), request["text"].as_str())
                {
                    form.draft.name = text.into();
                    json!({"success":true,"tree":self.ui_test_tree()})
                } else {
                    json!({"success":false,"error":"host editor is not open"})
                }
            }
            Some("resize") => match (request["width"].as_f64(), request["height"].as_f64()) {
                (Some(width), Some(height)) if width >= 320.0 && height >= 320.0 => {
                    self.viewport_width = width as f32;
                    ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(egui::vec2(
                        width as f32,
                        height as f32,
                    )));
                    json!({"success":true,"width":width,"height":height})
                }
                _ => json!({"success":false,"error":"invalid window dimensions"}),
            },
            Some("send_key") if request["key"].as_str() == Some("Escape") => {
                if self.editor_open {
                    if self.host_form.as_ref().is_some_and(|form| form.dirty()) {
                        self.host_form.as_mut().expect("host form").confirm_discard = true;
                    } else {
                        self.close_host_form(true);
                    }
                } else {
                    self.search.clear();
                }
                json!({"success":true,"tree":self.ui_test_tree()})
            }
            Some("drag") => {
                json!({"success":false,"error":"no drag target is registered in the current Transfers view"})
            }
            Some("screenshot") => {
                let name = request["name"]
                    .as_str()
                    .filter(|name| {
                        !name.is_empty()
                            && name
                                .chars()
                                .all(|ch| ch.is_ascii_alphanumeric() || ch == '-' || ch == '_')
                    })
                    .unwrap_or("window");
                let path = mode.root.join("screenshots").join(format!("{name}.png"));
                self.test_screenshot_path = Some(path.clone());
                ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot);
                ctx.request_repaint();
                json!({"success":true,"path":format!("screenshots/{name}.png")})
            }
            _ => json!({"success":false,"error":"bridge operation is not allowed"}),
        };
        mode.write_bridge_response(&response);
    }
}
