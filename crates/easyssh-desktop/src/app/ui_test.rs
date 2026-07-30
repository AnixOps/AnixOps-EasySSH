#[cfg(feature = "ui-test")]
use super::*;

#[cfg(feature = "ui-test")]
impl EasySshApp {
    #[cfg(feature = "ui-test")]
    pub(super) fn save_ui_test_screenshot(&mut self, ctx: &egui::Context) {
        let Some(path) = self.test_screenshot_path.take() else {
            return;
        };
        let image = ctx.input(|input| {
            input.events.iter().find_map(|event| match event {
                egui::Event::Screenshot { image, .. } => Some(image.clone()),
                _ => None,
            })
        });
        let Some(image) = image else {
            self.test_screenshot_path = Some(path);
            return;
        };
        let bytes = image
            .pixels
            .iter()
            .flat_map(|pixel| pixel.to_array())
            .collect::<Vec<_>>();
        let _ = image::save_buffer(
            path,
            &bytes,
            image.size[0] as u32,
            image.size[1] as u32,
            image::ColorType::Rgba8,
        );
    }

    #[cfg(feature = "ui-test")]
    pub(super) fn ui_test_tree(&self) -> Value {
        let strings = crate::ui::localization::Strings::new(self.config.locale);
        let page: crate::ui::pages::Page = self.config.workspace.into();
        let visible = page == crate::ui::pages::Page::Transfers;
        let navigation_visible = !matches!(
            crate::ui::shell::Breakpoint::for_width(self.viewport_width),
            crate::ui::shell::Breakpoint::Mobile
        );
        let files_visible =
            self.remote_file_browser_enabled() && self.config.workspace == Workspace::Files;
        let home_visible = page == crate::ui::pages::Page::Home;
        let hosts_visible =
            page == crate::ui::pages::Page::Hosts && self.config.workspace != Workspace::Files;
        let keys_visible = page == crate::ui::pages::Page::Keys;
        let settings_visible = page == crate::ui::pages::Page::Settings;
        let open_dialog_count = [
            (crate::ui::dialogs::Dialog::HostEditor, self.editor_open),
            (crate::ui::dialogs::Dialog::QuickConnect, self.quick_open),
            (
                crate::ui::dialogs::Dialog::Diagnostics,
                self.diagnostics_open,
            ),
            (crate::ui::dialogs::Dialog::Sync, self.sync_open),
        ]
        .iter()
        .filter(|(_, open)| *open)
        .count();
        let breakpoint = crate::ui::shell::Breakpoint::for_width(self.viewport_width).name();
        json!({"id":"app.root","role":"window","text":"EasySSH [UI Test]","visible":true,"enabled":true,"state":{"ui.is_idle":true,"ui.animation_count":0,"ui.pending_task_count":self.transfer_children.len(),"host_count":self.config.connections.len(),"open_dialog_count":open_dialog_count,"responsive_breakpoint":breakpoint,"diagnostics_loading":matches!(self.diagnostics_state.status, state::diagnostics::Status::Loading)},"children":[
          {"id":"navigation.home","role":"button","text":strings.text(crate::ui::localization::Key::Home),"visible":navigation_visible,"enabled":true,"selected":home_visible},
          {"id":"navigation.hosts","role":"button","text":strings.text(crate::ui::localization::Key::Hosts),"visible":navigation_visible,"enabled":true,"selected":hosts_visible},
          {"id":"navigation.transfers","role":"button","text":strings.text(crate::ui::localization::Key::Transfers),"visible":navigation_visible,"enabled":true,"selected":visible},
          {"id":"navigation.keys","role":"button","text":strings.text(crate::ui::localization::Key::Keys),"visible":navigation_visible,"enabled":true,"selected":keys_visible},
          {"id":"navigation.settings","role":"button","text":strings.text(crate::ui::localization::Key::Settings),"visible":navigation_visible,"enabled":true,"selected":settings_visible},
          {"id":"home.page","role":"page","text":strings.text(crate::ui::localization::Key::Home),"visible":home_visible,"enabled":true,"children":[
            {"id":"home.quick_connect","role":"textbox","text":strings.text(crate::ui::localization::Key::QuickConnect),"value":self.quick_host,"visible":home_visible,"enabled":true},
            {"id":"home.favorites","role":"list","text":strings.text(crate::ui::localization::Key::Favorites),"visible":home_visible,"enabled":true},
            {"id":"home.recent_sessions","role":"list","text":strings.text(crate::ui::localization::Key::RecentConnections),"visible":home_visible,"enabled":true}
          ]},
          {"id":"hosts.page","role":"page","text":strings.text(crate::ui::localization::Key::Hosts),"visible":hosts_visible,"enabled":true,"children":[
            {"id":"hosts.search","role":"textbox","text":"Search hosts","value":self.search,"visible":hosts_visible,"enabled":true},
            {"id":"hosts.add","role":"button","text":"Add host","visible":hosts_visible,"enabled":true},
            {"id":"hosts.groups","role":"button","text":"Groups","visible":hosts_visible,"enabled":true},
            {"id":"hosts.list","role":"list","text":"Hosts","visible":hosts_visible,"enabled":true},
            {"id":"hosts.inspector","role":"complementary","text":"Host inspector","visible":hosts_visible && self.inspector_open,"enabled":true}
          ]},
          {"id":"hosts.editor","role":"dialog","text":"Edit host","visible":self.editor_open,"enabled":true},
          {"id":"hosts.editor.name","role":"textbox","text":"Display name","value":self.host_form.as_ref().map(|form| form.draft.name.clone()).unwrap_or_default(),"visible":self.editor_open,"enabled":true},
          {"id":"hosts.editor.close","role":"button","text":"Close editor","visible":self.editor_open,"enabled":true},
          {"id":"hosts.editor.discard","role":"button","text":"Discard changes","visible":self.host_form.as_ref().is_some_and(|form| form.confirm_discard),"enabled":true},
          {"id":"hosts.group_settings","role":"dialog","text":"Group settings","visible":self.group_settings_open,"enabled":true},
          {"id":"keys.page","role":"page","text":"Keys","visible":keys_visible,"enabled":true,"children":[{"id":"keys.refresh","role":"button","text":"Refresh diagnostics","visible":keys_visible,"enabled":!matches!(self.diagnostics_state.status, state::diagnostics::Status::Loading)}]},
          {"id":"settings.page","role":"page","text":strings.text(crate::ui::localization::Key::Settings),"visible":settings_visible,"enabled":true,"children":[
            {"id":"settings.theme.light","role":"button","text":"Light","visible":settings_visible,"enabled":true},
            {"id":"settings.theme.dark","role":"button","text":"Dark","visible":settings_visible,"enabled":true},
            {"id":"settings.locale.system","role":"button","text":"System","visible":settings_visible,"enabled":true},
            {"id":"settings.locale.en","role":"button","text":"English","visible":settings_visible,"enabled":true},
            {"id":"settings.locale.zh_cn","role":"button","text":"Chinese","visible":settings_visible,"enabled":true},
            {"id":"settings.experimental.remote_file_browser","role":"checkbox","text":strings.text(crate::ui::localization::Key::RemoteFileBrowser),"visible":settings_visible,"enabled":true,"checked":self.config.experimental.remote_file_browser},
            {"id":"settings.experimental.git_sync","role":"checkbox","text":"Git metadata sync","visible":settings_visible,"enabled":true,"checked":self.config.experimental.git_metadata_sync_ui}
          ]},
          {"id":"files.page","role":"page","text":"Files","visible":files_visible,"enabled":true,"children":[
            {"id":"files.hosts","role":"list","text":"Hosts","visible":files_visible,"enabled":true},
            {"id":"files.path","role":"textbox","text":"Remote path","value":self.files_path,"visible":files_visible,"enabled":true},
            {"id":"files.filter","role":"textbox","text":"Filter","value":self.files_filter,"visible":files_visible,"enabled":true},
            {"id":"files.refresh","role":"button","text":"Refresh","visible":files_visible,"enabled":true},
            {"id":"files.new_folder","role":"button","text":"New folder","visible":files_visible,"enabled":true},
            {"id":"files.toggle_dual_pane","role":"checkbox","text":"Two panes","visible":files_visible,"enabled":true,"checked":self.files_dual_pane},
            {"id":"files.entries","role":"list","text":"Remote entries","visible":files_visible,"enabled":true},
            {"id":"files.properties","role":"complementary","text":"Properties","visible":files_visible,"enabled":true}
          ]},
          {"id":"files.create_folder_dialog","role":"dialog","text":"New remote folder","visible":self.files_create_dir_open,"enabled":true},
          {"id":"transfers.page","role":"page","text":strings.text(crate::ui::localization::Key::Transfers),"visible":visible,"enabled":true,"children":[
            {"id":"transfers.host_selector","role":"combobox","text":"Host","visible":visible,"enabled":true},
            {"id":"transfers.connection_status","role":"status","text":"Disconnected","visible":visible,"enabled":true},
            {"id":"transfers.local_path","role":"textbox","text":"Local path","value":self.transfer_local_path,"visible":visible,"enabled":true},
            {"id":"transfers.remote_path","role":"textbox","text":"Remote path","value":self.transfer_remote_path,"visible":visible,"enabled":true},
            {"id":"transfers.upload_button","role":"button","text":"Upload","visible":visible,"enabled":true},
            {"id":"transfers.download_button","role":"button","text":"Download","visible":visible,"enabled":true},
            {"id":"transfers.transfer_queue","role":"list","text":"Transfer queue","visible":visible,"enabled":true},
            {"id":"transfers.empty_state","role":"status","text":"No transfers yet","visible":visible,"enabled":true}
          ]},
          {"id":"toast","role":"status","text":self.toast.as_ref().map(|toast| toast.message.clone()).unwrap_or_default(),"visible":self.toast.is_some(),"enabled":true,"kind":self.toast.as_ref().map(|toast| format!("{:?}", toast.kind)),"children":[
            {"id":"toast.close","role":"button","text":"Close","visible":self.toast.as_ref().is_some_and(|toast| toast.kind == ToastKind::Error),"enabled":true}
          ]}
        ]})
    }
}
