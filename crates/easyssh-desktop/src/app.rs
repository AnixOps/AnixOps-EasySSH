use chrono::{DateTime, Utc};
use easyssh_core::{
    cancel, scan_default_ssh_config, AppConfig, CommandSnippet, ConfigStore, Connection,
    ConnectionTarget, DisplayDensity, EditSession, ExternalTerminal, GitSync, LineEnding, Locale,
    OpenSsh, RemoteCapabilities, RemoteEntry, RemoteEntryType, RemoteFileService, ScpInvocation,
    SessionRecord, SshInvocation, SyncStatus, Theme, Transfer, TransferDirection, TransferStatus,
    Workspace, WorkspaceTempManager,
};
use eframe::egui;
use egui_phosphor::regular as icon;
#[cfg(feature = "ui-test")]
use serde_json::{json, Value};
use std::collections::HashMap;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Child, Command};
use std::thread::JoinHandle;
use std::time::SystemTime;
#[path = "app/dialogs.rs"]
mod app_dialogs;
#[path = "app/shared.rs"]
mod app_shared;
#[path = "app/shell.rs"]
mod app_shell;
#[cfg(test)]
#[path = "app/tests.rs"]
mod app_tests;
#[path = "app/ui_test.rs"]
mod app_ui_test;
#[path = "ui/dialogs/file_operations.rs"]
mod dialog_file_operations;
#[path = "ui/dialogs/ui_test_bridge.rs"]
mod dialog_ui_test_bridge;
#[path = "ui/pages/file_actions.rs"]
mod page_file_actions;
#[path = "ui/pages/file_editor.rs"]
mod page_file_editor;
#[path = "ui/pages/file_upload.rs"]
mod page_file_upload;
#[path = "ui/pages/files.rs"]
mod page_files;
#[path = "ui/pages/home.rs"]
mod page_home;
#[path = "ui/pages/host_details.rs"]
mod page_host_details;
#[path = "ui/pages/hosts.rs"]
mod page_hosts;
#[path = "ui/pages/keys.rs"]
mod page_keys;
#[path = "ui/pages/operations.rs"]
mod page_operations;
#[path = "ui/pages/settings.rs"]
mod page_settings;
#[path = "state/mod.rs"]
mod state;
#[cfg(feature = "ui-test")]
use crate::ui_test;
use app_shared::*;
const BLUE: egui::Color32 = egui::Color32::from_rgb(111, 124, 255);
const GREEN: egui::Color32 = crate::ui::tokens::DARK.success;
const AMBER: egui::Color32 = crate::ui::tokens::DARK.warning;
const RED: egui::Color32 = crate::ui::tokens::DARK.danger;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FileEditorStatus {
    LocalModified,
    SavedLocally,
    CheckingRemote,
    Uploading,
    SavedRemotely,
    Conflict,
    UploadFailed,
}

#[derive(Debug, Clone)]
struct LocalEntry {
    path: PathBuf,
    name: String,
    is_directory: bool,
    size: u64,
}

struct RunningTransfer {
    child: Child,
    stdout: JoinHandle<String>,
    stderr: JoinHandle<String>,
}

fn read_scp_output(mut reader: impl Read + Send + 'static) -> JoinHandle<String> {
    std::thread::spawn(move || {
        const LIMIT: usize = 16 * 1024;
        let mut bytes = Vec::new();
        let _ = reader.by_ref().take(LIMIT as u64).read_to_end(&mut bytes);
        String::from_utf8_lossy(&bytes).trim().to_owned()
    })
}

fn running_transfer(mut child: Child) -> RunningTransfer {
    let stdout = child.stdout.take().expect("scp stdout is piped");
    let stderr = child.stderr.take().expect("scp stderr is piped");
    RunningTransfer {
        child,
        stdout: read_scp_output(stdout),
        stderr: read_scp_output(stderr),
    }
}

fn finished_transfer_output(process: RunningTransfer) -> String {
    let stdout = process.stdout.join().unwrap_or_default();
    let stderr = process.stderr.join().unwrap_or_default();
    match (stdout.is_empty(), stderr.is_empty()) {
        (true, true) => String::new(),
        (false, true) => stdout,
        (true, false) => stderr,
        (false, false) => format!("{stdout}\n{stderr}"),
    }
}

/// Adds OpenSSH aliases as alias-only connections. Repeating an import is a
/// no-op for aliases already present in the workspace.
fn import_aliases(connections: &mut Vec<Connection>, aliases: &[String]) -> (usize, usize) {
    let mut added = 0;
    let mut skipped = 0;
    for alias in aliases {
        if connections.iter().any(|connection| {
            matches!(&connection.target, ConnectionTarget::Alias { alias: existing } if existing == alias)
        }) {
            skipped += 1;
        } else {
            connections.push(Connection::alias(alias, alias));
            added += 1;
        }
    }
    (added, skipped)
}

fn wait_for_scp(child: Child) -> Result<(), easyssh_core::OpenSshError> {
    let mut process = running_transfer(child);
    let status = process.child.wait()?;
    let output = finished_transfer_output(process);
    if status.success() {
        Ok(())
    } else if output.is_empty() {
        Err(easyssh_core::OpenSshError::Failed(
            "system scp exited unsuccessfully".into(),
        ))
    } else {
        Err(easyssh_core::OpenSshError::Failed(output))
    }
}

impl FileEditorStatus {
    fn label(self) -> &'static str {
        match self {
            Self::LocalModified => "Local modified",
            Self::SavedLocally => "Saved to local working copy",
            Self::CheckingRemote => "Checking remote",
            Self::Uploading => "Uploading",
            Self::SavedRemotely => "Saved remotely",
            Self::Conflict => "Conflict: remote file changed",
            Self::UploadFailed => "Upload failed",
        }
    }
}

pub fn run() -> eframe::Result<()> {
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
    #[cfg(feature = "ui-test")]
    let mut viewport = egui::ViewportBuilder::default()
        .with_inner_size([1280.0, 800.0])
        .with_min_inner_size([680.0, 520.0]);
    #[cfg(not(feature = "ui-test"))]
    let viewport = egui::ViewportBuilder::default()
        .with_inner_size([1280.0, 800.0])
        .with_min_inner_size([680.0, 520.0]);
    #[cfg(feature = "ui-test")]
    if test_mode.is_some() {
        viewport = viewport.with_inner_size([1280.0, 800.0]);
    }
    eframe::run_native(
        title,
        eframe::NativeOptions {
            // Hosts uses a permanent three-column layout (navigation, object
            // list, inspector). Keep enough room for targets and toolbar
            // actions instead of allowing panels to collapse into each other.
            viewport,
            ..Default::default()
        },
        Box::new(move |cc| {
            let mut fonts = egui::FontDefinitions::default();
            egui_phosphor::add_to_fonts(&mut fonts, egui_phosphor::Variant::Regular);
            install_system_cjk_fallback(&mut fonts);
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

fn install_system_cjk_fallback(fonts: &mut egui::FontDefinitions) {
    #[cfg(windows)]
    {
        const FONT_PATH: &str = r"C:\Windows\Fonts\simhei.ttf";
        if let Ok(bytes) = fs::read(FONT_PATH) {
            let name = "easyssh-system-cjk".to_owned();
            fonts
                .font_data
                .insert(name.clone(), egui::FontData::from_owned(bytes));
            for family in [egui::FontFamily::Proportional, egui::FontFamily::Monospace] {
                fonts.families.entry(family).or_default().push(name.clone());
            }
        }
    }
}

struct EasySshApp {
    store: ConfigStore,
    config: AppConfig,
    openssh: OpenSsh,
    selected: Option<String>,
    inspector_open: bool,
    search: String,
    command_query: String,
    command_open: bool,
    command_index: usize,
    quick_open: bool,
    quick_host: String,
    quick_user: String,
    quick_port: u16,
    editor_open: bool,
    host_form: Option<state::host_form::State>,
    editor_test_status: Option<String>,
    snippet_editor: Option<String>,
    delete_host: Option<String>,
    delete_snippet: Option<String>,
    diagnostics_open: bool,
    sync_open: bool,
    diagnostics_state: state::diagnostics::State,
    ssh_config_aliases: Vec<String>,
    ssh_config_warnings: Vec<String>,
    ssh_config_scanned: bool,
    viewport_width: f32,
    copy_text: Option<String>,
    status: String,
    toast: Option<String>,
    transfers: Vec<Transfer>,
    transfer_children: HashMap<String, RunningTransfer>,
    transfer_connection: Option<String>,
    transfer_local_path: String,
    transfer_remote_path: String,
    transfer_recursive: bool,
    transfer_direction: TransferDirection,
    remote_files: RemoteFileService,
    files_connection: Option<String>,
    files_path: String,
    files_path_input: String,
    files_filter: String,
    files_hidden: bool,
    files_entries: Vec<RemoteEntry>,
    files_selected: Option<String>,
    files_capabilities: Option<RemoteCapabilities>,
    files_history: Vec<String>,
    files_history_index: usize,
    files_status: String,
    temporary_workspace: Option<WorkspaceTempManager>,
    file_edit_session: Option<EditSession>,
    file_editor_text: String,
    file_editor_status: FileEditorStatus,
    file_editor_find: String,
    file_editor_replace: String,
    file_editor_match_index: usize,
    file_editor_go_to_line: usize,
    file_editor_cursor_line: usize,
    file_editor_pending_selection: Option<(usize, usize)>,
    file_editor_last_modified: Option<SystemTime>,
    file_editor_external_change: Option<String>,
    file_conflict_open: bool,
    file_conflict_remote: Option<String>,
    file_preview_image: Option<(String, egui::ColorImage)>,
    file_preview_texture: Option<egui::TextureHandle>,
    files_dual_pane: bool,
    local_path: PathBuf,
    local_entries: Vec<LocalEntry>,
    local_selected: Option<PathBuf>,
    files_create_dir_open: bool,
    files_new_dir_name: String,
    files_rename_open: bool,
    files_rename_name: String,
    files_delete_open: bool,
    #[cfg(feature = "ui-test")]
    test_mode: Option<ui_test::UiTestMode>,
    #[cfg(feature = "ui-test")]
    test_ready: bool,
    #[cfg(feature = "ui-test")]
    test_screenshot_path: Option<PathBuf>,
}

impl Default for EasySshApp {
    fn default() -> Self {
        let store = ConfigStore::default_path().expect("configuration path");
        Self::from_store(store, true)
    }
}

impl EasySshApp {
    fn from_store(store: ConfigStore, _inspect_agent: bool) -> Self {
        let mut config = store.load().unwrap_or_else(|_| AppConfig::new());
        let mut interrupted = 0usize;
        for transfer in &mut config.transfer_history {
            if matches!(
                transfer.status,
                TransferStatus::Queued
                    | TransferStatus::Pending
                    | TransferStatus::Authorizing
                    | TransferStatus::Transferring
            ) {
                transfer.status = TransferStatus::Interrupted;
                transfer.finished_at = Some(Utc::now());
                transfer.output = "Interrupted because EasySSH was closed.".into();
                interrupted += 1;
            }
        }
        config.transfer_history.truncate(100);
        let transfers = config.transfer_history.clone();
        let diagnostics_state = state::diagnostics::State::default();
        Self {
            store,
            config,
            openssh: OpenSsh,
            selected: None,
            inspector_open: false,
            search: String::new(),
            command_query: String::new(),
            command_open: false,
            command_index: 0,
            quick_open: false,
            quick_host: String::new(),
            quick_user: String::new(),
            quick_port: 22,
            editor_open: false,
            host_form: None,
            editor_test_status: None,
            snippet_editor: None,
            delete_host: None,
            delete_snippet: None,
            diagnostics_open: false,
            sync_open: false,
            diagnostics_state,
            ssh_config_aliases: Vec::new(),
            ssh_config_warnings: Vec::new(),
            ssh_config_scanned: false,
            viewport_width: 1280.0,
            copy_text: None,
            status: if interrupted == 0 {
                String::new()
            } else {
                format!("{interrupted} active transfer(s) marked interrupted.")
            },
            toast: None,
            transfers,
            transfer_children: HashMap::new(),
            transfer_connection: None,
            transfer_local_path: String::new(),
            transfer_remote_path: String::new(),
            transfer_recursive: false,
            transfer_direction: TransferDirection::Upload,
            remote_files: RemoteFileService::new(OpenSsh),
            files_connection: None,
            files_path: "/".into(),
            files_path_input: "/".into(),
            files_filter: String::new(),
            files_hidden: false,
            files_entries: Vec::new(),
            files_selected: None,
            files_capabilities: None,
            files_history: vec!["/".into()],
            files_history_index: 0,
            files_status: "Select a host to browse files.".into(),
            temporary_workspace: None,
            file_edit_session: None,
            file_editor_text: String::new(),
            file_editor_status: FileEditorStatus::SavedLocally,
            file_editor_find: String::new(),
            file_editor_replace: String::new(),
            file_editor_match_index: 0,
            file_editor_go_to_line: 1,
            file_editor_cursor_line: 1,
            file_editor_pending_selection: None,
            file_editor_last_modified: None,
            file_editor_external_change: None,
            file_conflict_open: false,
            file_conflict_remote: None,
            file_preview_image: None,
            file_preview_texture: None,
            files_dual_pane: false,
            local_path: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
            local_entries: Vec::new(),
            local_selected: None,
            files_create_dir_open: false,
            files_new_dir_name: String::new(),
            files_rename_open: false,
            files_rename_name: String::new(),
            files_delete_open: false,
            #[cfg(feature = "ui-test")]
            test_mode: None,
            #[cfg(feature = "ui-test")]
            test_ready: false,
            #[cfg(feature = "ui-test")]
            test_screenshot_path: None,
        }
    }

    #[cfg(feature = "ui-test")]
    fn ui_test(mode: ui_test::UiTestMode) -> Self {
        let store = ConfigStore::at(mode.root.join("config").join("connections.json"));
        let mut app = Self::from_store(store, false);
        app.config.theme = Theme::Dark;
        app.config.workspace = Workspace::Home;
        app.status = "UI TEST MODE - isolated configuration".into();
        app.test_mode = Some(mode);
        app
    }
    fn save(&mut self) {
        let _ = self.try_save();
    }

    fn try_save(&mut self) -> bool {
        match self.store.save(&self.config) {
            Ok(()) => {
                self.status = "Workbench saved".into();
                true
            }
            Err(error) => {
                self.status = error.to_string();
                self.toast = Some(format!("Unable to save changes: {error}"));
                false
            }
        }
    }

    fn remote_file_browser_enabled(&self) -> bool {
        self.config.experimental.remote_file_browser
    }

    fn import_ssh_config_aliases(&mut self) -> (usize, usize) {
        let (added, skipped) =
            import_aliases(&mut self.config.connections, &self.ssh_config_aliases);
        if added > 0 && self.try_save() {
            self.toast = Some(format!("Added {added} SSH Config host(s)."));
        } else if added == 0 {
            self.toast = Some("All SSH Config hosts are already added.".into());
        }
        (added, skipped)
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
        self.selected = None;
        self.host_form = Some(state::host_form::State::new(host));
        self.editor_test_status = None;
        self.editor_open = true;
    }

    fn close_host_form(&mut self, discard: bool) {
        if discard {
            self.host_form = None;
            self.editor_open = false;
        }
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
        if self.transfer_connection.is_none() {
            self.status = "Select a host before starting a transfer.".into();
            return;
        }
        let mut transfer = Transfer::new(
            self.transfer_direction,
            PathBuf::from(self.transfer_local_path.trim()),
            self.transfer_remote_path.trim().to_owned(),
            self.transfer_recursive,
        );
        transfer.status = TransferStatus::Queued;
        transfer.started_at = Some(Utc::now());
        self.transfers.insert(0, transfer);
        self.pump_transfers();
        self.persist_transfer_history();
    }

    fn persist_transfer_history(&mut self) {
        self.transfers.truncate(100);
        self.config.transfer_history = self.transfers.clone();
        self.save();
    }

    fn pump_transfers(&mut self) {
        while self.transfer_children.len() < 2 {
            let Some(index) = self
                .transfers
                .iter()
                .position(|item| item.status == TransferStatus::Queued)
            else {
                break;
            };
            let mut transfer = self.transfers[index].clone();
            let Some(connection) = self
                .transfer_connection
                .as_ref()
                .and_then(|id| self.config.connections.iter().find(|item| &item.id == id))
                .cloned()
            else {
                transfer.status = TransferStatus::Failed;
                transfer.output = "Select a host before starting a transfer.".into();
                self.transfers[index] = transfer;
                continue;
            };
            transfer.status = TransferStatus::Authorizing;
            match ScpInvocation::build(&self.openssh, &connection, &transfer)
                .and_then(|invocation| invocation.spawn())
            {
                Ok(child) => {
                    self.transfer_children
                        .insert(transfer.id.clone(), running_transfer(child));
                    self.transfers[index] = transfer;
                    self.status = "Waiting for system SSH Agent authorization.".into();
                }
                Err(error) => {
                    transfer.status = TransferStatus::Failed;
                    transfer.finished_at = Some(Utc::now());
                    transfer.output = error.to_string();
                    self.transfers[index] = transfer;
                }
            }
        }
    }

    fn poll_transfers(&mut self, ctx: &egui::Context) {
        let active: Vec<String> = self.transfer_children.keys().cloned().collect();
        let mut changed = false;
        for id in active {
            let result = self
                .transfer_children
                .get_mut(&id)
                .map(|process| process.child.try_wait());
            match result {
                Some(Ok(Some(exit))) => {
                    let output = self
                        .transfer_children
                        .remove(&id)
                        .map(finished_transfer_output)
                        .unwrap_or_default();
                    if let Some(transfer) = self.transfers.iter_mut().find(|item| item.id == id) {
                        transfer.finished_at = Some(Utc::now());
                        transfer.status = if exit.success() {
                            TransferStatus::Completed
                        } else {
                            TransferStatus::Failed
                        };
                        transfer.output = if exit.success() && output.is_empty() {
                            "Completed".into()
                        } else if exit.success() {
                            output
                        } else if output.is_empty() {
                            "The system scp process exited unsuccessfully.".into()
                        } else {
                            output
                        };
                    }
                    self.pump_transfers();
                    changed = true;
                }
                Some(Ok(None)) => {
                    if let Some(transfer) = self.transfers.iter_mut().find(|item| item.id == id) {
                        transfer.status = TransferStatus::Transferring;
                    }
                }
                Some(Err(error)) => {
                    let output = self
                        .transfer_children
                        .remove(&id)
                        .map(finished_transfer_output)
                        .unwrap_or_default();
                    if let Some(transfer) = self.transfers.iter_mut().find(|item| item.id == id) {
                        transfer.status = TransferStatus::Failed;
                        transfer.finished_at = Some(Utc::now());
                        transfer.output = if output.is_empty() {
                            error.to_string()
                        } else {
                            output
                        };
                    }
                    self.pump_transfers();
                    changed = true;
                }
                None => {}
            }
        }
        if !self.transfer_children.is_empty() {
            ctx.request_repaint_after(std::time::Duration::from_millis(250));
        }
        if changed {
            self.persist_transfer_history();
        }
    }

    fn cancel_transfer(&mut self, id: &str) {
        if let Some(mut process) = self.transfer_children.remove(id) {
            let _ = cancel(&mut process.child);
            let _ = finished_transfer_output(process);
        }
        if let Some(transfer) = self.transfers.iter_mut().find(|item| item.id == id) {
            transfer.status = TransferStatus::Cancelled;
            transfer.finished_at = Some(Utc::now());
            transfer.output = "Cancelled by user.".into();
        }
        self.persist_transfer_history();
    }
}

impl eframe::App for EasySshApp {
    fn update(&mut self, ctx: &egui::Context, _: &mut eframe::Frame) {
        self.diagnostics_state.poll();
        if self.config.workspace == Workspace::Files && !self.remote_file_browser_enabled() {
            self.config.workspace = Workspace::Hosts;
            self.toast = Some("Remote file browser is disabled in Experimental settings.".into());
        }
        self.viewport_width = ctx.input(|input| input.screen_rect().width());
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
        #[cfg(feature = "ui-test")]
        self.handle_ui_test_bridge(ctx);
        #[cfg(feature = "ui-test")]
        self.save_ui_test_screenshot(ctx);
        #[cfg(feature = "ui-test")]
        if self.test_mode.is_some() {
            ctx.request_repaint_after(std::time::Duration::from_millis(25));
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
            Workspace::Home => self.home(ctx),
            Workspace::Hosts => self.hosts(ctx),
            Workspace::Files if self.remote_file_browser_enabled() => self.files(ctx),
            Workspace::Files => self.hosts(ctx),
            Workspace::Snippets => self.snippets(ctx),
            Workspace::Forwarding => self.forwarding(ctx),
            Workspace::Transfers => self.transfers(ctx),
            Workspace::Keys => self.keys(ctx),
            Workspace::Settings => self.settings(ctx),
        };
        self.command_panel(ctx);
        self.host_editor(ctx);
        self.snippet_editor(ctx);
        self.quick_connect(ctx);
        self.confirmation_dialogs(ctx);
        self.diagnostics(ctx);
        self.sync_panel(ctx);
        self.file_conflict_dialog(ctx);
        self.file_operation_dialogs(ctx);
        if let Some(message) = &self.toast {
            egui::Area::new(egui::Id::new("save-toast"))
                .anchor(egui::Align2::RIGHT_BOTTOM, [-16.0, -16.0])
                .show(ctx, |ui| {
                    egui::Frame::popup(ui.style()).show(ui, |ui| {
                        ui.label(message);
                    });
                });
        }
    }
}
