use chrono::{DateTime, Utc};
use easyssh_core::{
    cancel, AppConfig, CommandSnippet, ConfigStore, Connection, ConnectionTarget, DisplayDensity,
    EditSession, ExternalTerminal, GitSync, LineEnding, OpenSsh, RemoteCapabilities, RemoteEntry,
    RemoteEntryType, RemoteFileService, ScpInvocation, SessionRecord, SshInvocation, SyncStatus,
    Theme, Transfer, TransferDirection, TransferStatus, Workspace, WorkspaceTempManager,
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

mod ui;
#[cfg(feature = "ui-test")]
mod ui_test;

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

fn main() -> eframe::Result<()> {
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
    let mut viewport = egui::ViewportBuilder::default().with_min_inner_size([1200.0, 760.0]);
    #[cfg(not(feature = "ui-test"))]
    let viewport = egui::ViewportBuilder::default().with_min_inner_size([1200.0, 760.0]);
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

struct EasySshApp {
    store: ConfigStore,
    config: AppConfig,
    openssh: OpenSsh,
    selected: Option<String>,
    search: String,
    command_query: String,
    command_open: bool,
    command_index: usize,
    quick_open: bool,
    quick_host: String,
    quick_user: String,
    quick_port: u16,
    editor_open: bool,
    snippet_editor: Option<String>,
    delete_host: Option<String>,
    delete_snippet: Option<String>,
    diagnostics_open: bool,
    sync_open: bool,
    agent_available: Option<bool>,
    copy_text: Option<String>,
    status: String,
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
    fn from_store(store: ConfigStore, inspect_agent: bool) -> Self {
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
        let agent_available = inspect_agent
            .then(|| {
                OpenSsh
                    .diagnostics(None)
                    .agent_keys
                    .map(|keys| keys.available)
            })
            .flatten();
        Self {
            store,
            config,
            openssh: OpenSsh,
            selected: None,
            search: String::new(),
            command_query: String::new(),
            command_open: false,
            command_index: 0,
            quick_open: false,
            quick_host: String::new(),
            quick_user: String::new(),
            quick_port: 22,
            editor_open: false,
            snippet_editor: None,
            delete_host: None,
            delete_snippet: None,
            diagnostics_open: false,
            sync_open: false,
            agent_available,
            copy_text: None,
            status: if interrupted == 0 {
                String::new()
            } else {
                format!("{interrupted} active transfer(s) marked interrupted.")
            },
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
        app.config.workspace = Workspace::Hosts;
        app.status = "UI TEST MODE - isolated configuration".into();
        app.test_mode = Some(mode);
        app
    }
    fn save(&mut self) {
        self.status = match self.store.save(&self.config) {
            Ok(()) => "Workbench saved".into(),
            Err(error) => error.to_string(),
        };
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
        self.selected = Some(host.id.clone());
        self.config.connections.push(host);
        self.editor_open = true;
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

    fn files_connection_record(&self) -> Option<Connection> {
        self.files_connection
            .as_ref()
            .and_then(|id| self.config.connections.iter().find(|item| &item.id == id))
            .cloned()
    }

    fn refresh_files(&mut self, record_history: bool) {
        let Some(connection) = self.files_connection_record() else {
            self.files_status = "Select a host to browse files.".into();
            return;
        };
        let capabilities = match self.files_capabilities.clone() {
            Some(value) => value,
            None => match self.remote_files.detect_capabilities(&connection) {
                Ok(value) => {
                    self.files_capabilities = Some(value.clone());
                    value
                }
                Err(error) => {
                    self.files_status = format!("Capability detection failed: {error}");
                    return;
                }
            },
        };
        match self.remote_files.list_dir(
            &connection,
            &capabilities,
            &self.files_path,
            self.files_hidden,
        ) {
            Ok(entries) => {
                self.files_entries = entries;
                self.files_status = format!("{} entries", self.files_entries.len());
                if record_history {
                    self.files_history.truncate(self.files_history_index + 1);
                    if self.files_history.last() != Some(&self.files_path) {
                        self.files_history.push(self.files_path.clone());
                    }
                    self.files_history_index = self.files_history.len().saturating_sub(1);
                }
            }
            Err(error) => self.files_status = error.to_string(),
        }
    }

    fn open_files_path(&mut self, path: String) {
        self.files_path = path;
        self.files_path_input = self.files_path.clone();
        self.files_selected = None;
        self.refresh_files(true);
    }

    fn files_go_up(&mut self) {
        let path = self.files_path.trim_end_matches('/');
        let parent = path
            .rsplit_once('/')
            .map(|(prefix, _)| prefix)
            .unwrap_or("");
        self.open_files_path(if parent.is_empty() {
            "/".into()
        } else {
            parent.into()
        });
    }

    fn files_select_host(&mut self, id: String) {
        self.files_connection = Some(id);
        self.files_capabilities = None;
        self.files_path = "/".into();
        self.files_path_input = "/".into();
        self.files_history = vec!["/".into()];
        self.files_history_index = 0;
        self.refresh_files(false);
    }

    fn refresh_local_files(&mut self) {
        let entries = fs::read_dir(&self.local_path)
            .map(|items| {
                items
                    .filter_map(Result::ok)
                    .filter_map(|item| {
                        let metadata = item.metadata().ok()?;
                        Some(LocalEntry {
                            name: item.file_name().to_string_lossy().into_owned(),
                            path: item.path(),
                            is_directory: metadata.is_dir(),
                            size: metadata.len(),
                        })
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        self.local_entries = entries;
        self.local_entries
            .sort_by_cached_key(|entry| (!entry.is_directory, entry.name.to_lowercase()));
    }

    fn queue_local_upload(&mut self, path: &Path) {
        if self.files_connection.is_none() {
            self.files_status = "Select a remote host before uploading.".into();
            return;
        }
        self.transfer_connection = self.files_connection.clone();
        self.transfer_local_path = path.to_string_lossy().into_owned();
        self.transfer_remote_path = self.files_path.clone();
        self.transfer_direction = TransferDirection::Upload;
        self.transfer_recursive = path.is_dir();
        self.config.workspace = Workspace::Transfers;
    }

    fn queue_remote_download(&mut self, remote_path: String) {
        self.transfer_connection = self.files_connection.clone();
        self.transfer_local_path = self.local_path.to_string_lossy().into_owned();
        self.transfer_remote_path = remote_path;
        self.transfer_direction = TransferDirection::Download;
        self.transfer_recursive = false;
        self.config.workspace = Workspace::Transfers;
    }

    fn remote_operation_context(&self) -> Option<(Connection, RemoteCapabilities)> {
        Some((
            self.files_connection_record()?,
            self.files_capabilities.clone()?,
        ))
    }

    fn create_remote_directory(&mut self) {
        let name = self.files_new_dir_name.trim();
        if name.is_empty() || name.contains(['/', '\\', '\0']) || name == "." || name == ".." {
            self.files_status = "Enter a single directory name without path separators.".into();
            return;
        }
        let Some((connection, capabilities)) = self.remote_operation_context() else {
            self.files_status = "Select and refresh a host before creating a directory.".into();
            return;
        };
        let path = remote_child_path(&self.files_path, name);
        match self
            .remote_files
            .create_dir(&connection, &capabilities, &path)
        {
            Ok(()) => {
                self.files_create_dir_open = false;
                self.files_new_dir_name.clear();
                self.refresh_files(false);
            }
            Err(error) => self.files_status = error.to_string(),
        }
    }

    fn rename_selected_remote(&mut self) {
        let name = self.files_rename_name.trim();
        if name.is_empty() || name.contains(['/', '\\', '\0']) || name == "." || name == ".." {
            self.files_status = "Enter a single new name without path separators.".into();
            return;
        }
        let Some(source) = self.files_selected.clone() else {
            return;
        };
        let Some((connection, capabilities)) = self.remote_operation_context() else {
            self.files_status = "Select and refresh a host before renaming.".into();
            return;
        };
        let destination = remote_child_path(&self.files_path, name);
        match self
            .remote_files
            .rename(&connection, &capabilities, &source, &destination)
        {
            Ok(()) => {
                self.files_rename_open = false;
                self.files_selected = None;
                self.refresh_files(false);
            }
            Err(error) => self.files_status = error.to_string(),
        }
    }

    fn delete_selected_remote(&mut self) {
        let Some(entry) = self
            .files_selected
            .as_ref()
            .and_then(|path| self.files_entries.iter().find(|entry| &entry.path == path))
            .cloned()
        else {
            return;
        };
        let Some((connection, capabilities)) = self.remote_operation_context() else {
            self.files_status = "Select and refresh a host before deleting.".into();
            return;
        };
        let result = if entry.entry_type == RemoteEntryType::Directory {
            self.remote_files
                .remove_dir(&connection, &capabilities, &entry.path)
        } else {
            self.remote_files
                .remove_file(&connection, &capabilities, &entry.path)
        };
        match result {
            Ok(()) => {
                self.files_delete_open = false;
                self.files_selected = None;
                self.refresh_files(false);
            }
            Err(error) => self.files_status = error.to_string(),
        }
    }

    fn open_remote_text_file(&mut self, path: String) {
        let Some(connection) = self.files_connection_record() else {
            self.files_status = "Select a host to open a file.".into();
            return;
        };
        let Some(capabilities) = self.files_capabilities.clone() else {
            self.files_status = "Refresh the directory before opening a file.".into();
            return;
        };
        let mut initial_remote = match self.remote_files.stat(&connection, &capabilities, &path) {
            Ok(state) => state,
            Err(error) => {
                self.files_status = error.to_string();
                return;
            }
        };
        if capabilities.has_sha256 {
            initial_remote.sha256 = self
                .remote_files
                .calculate_hash(&connection, &capabilities, &path)
                .ok();
        }
        let workspace = match self.temporary_workspace.as_ref() {
            Some(workspace) => workspace,
            None => match WorkspaceTempManager::create() {
                Ok(workspace) => {
                    self.temporary_workspace = Some(workspace);
                    self.temporary_workspace
                        .as_ref()
                        .expect("workspace inserted")
                }
                Err(error) => {
                    self.files_status = format!("Unable to create local working copy: {error}");
                    return;
                }
            },
        };
        let mut session = match workspace.create_session(&path, initial_remote) {
            Ok(session) => session,
            Err(error) => {
                self.files_status = format!("Unable to create local working copy: {error}");
                return;
            }
        };
        let transfer = Transfer::new(
            TransferDirection::Download,
            session.local_path.clone(),
            path,
            false,
        );
        let result = ScpInvocation::build(&self.openssh, &connection, &transfer)
            .and_then(|invocation| invocation.spawn())
            .and_then(wait_for_scp);
        if let Err(error) = result {
            self.files_status = format!("Download failed: {error}");
            return;
        }
        let bytes = match fs::read(&session.local_path) {
            Ok(bytes) => bytes,
            Err(error) => {
                self.files_status = format!("Unable to read downloaded working copy: {error}");
                return;
            }
        };
        if bytes.len() > 2 * 1024 * 1024 {
            self.files_status = "File is larger than the 2 MiB editor protection limit.".into();
            return;
        }
        session.file_info = easyssh_core::inspect_text(&bytes);
        if session.file_info.is_binary {
            self.files_status = "Binary file downloaded but not opened in the text editor.".into();
            return;
        }
        let content = match String::from_utf8(bytes) {
            Ok(content) => content,
            Err(_) => {
                self.files_status = "The file is not UTF-8 text; use Download instead.".into();
                return;
            }
        };
        self.file_editor_text = content;
        if let Err(error) = fs::copy(&session.local_path, &session.base_path) {
            self.files_status = format!("Unable to preserve initial working copy: {error}");
            return;
        }
        self.file_edit_session = Some(session);
        self.file_editor_status = FileEditorStatus::SavedLocally;
        self.file_editor_find.clear();
        self.file_editor_replace.clear();
        self.file_editor_match_index = 0;
        self.file_editor_go_to_line = 1;
        self.file_editor_cursor_line = 1;
        self.file_editor_pending_selection = None;
        self.file_editor_external_change = None;
        self.file_editor_last_modified = self
            .file_edit_session
            .as_ref()
            .and_then(|session| fs::metadata(&session.local_path).ok())
            .and_then(|metadata| metadata.modified().ok());
        self.files_status = "Opened a local working copy downloaded with system scp.".into();
    }

    fn open_remote_image_preview(&mut self, path: String) {
        let Some(connection) = self.files_connection_record() else {
            self.files_status = "Select a host to preview a file.".into();
            return;
        };
        let Some(capabilities) = self.files_capabilities.clone() else {
            self.files_status = "Refresh the directory before previewing a file.".into();
            return;
        };
        let state = match self.remote_files.stat(&connection, &capabilities, &path) {
            Ok(state) => state,
            Err(error) => {
                self.files_status = error.to_string();
                return;
            }
        };
        if state.size > 5 * 1024 * 1024 {
            self.files_status = "Image preview is limited to files smaller than 5 MiB.".into();
            return;
        }
        let workspace = match self.temporary_workspace.as_ref() {
            Some(workspace) => workspace,
            None => match WorkspaceTempManager::create() {
                Ok(workspace) => {
                    self.temporary_workspace = Some(workspace);
                    self.temporary_workspace
                        .as_ref()
                        .expect("workspace inserted")
                }
                Err(error) => {
                    self.files_status = format!("Unable to create preview copy: {error}");
                    return;
                }
            },
        };
        let session = match workspace.create_session(&path, state) {
            Ok(session) => session,
            Err(error) => {
                self.files_status = format!("Unable to create preview copy: {error}");
                return;
            }
        };
        let transfer = Transfer::new(
            TransferDirection::Download,
            session.local_path.clone(),
            path.clone(),
            false,
        );
        let result = ScpInvocation::build(&self.openssh, &connection, &transfer)
            .and_then(|invocation| invocation.spawn())
            .and_then(wait_for_scp);
        if let Err(error) = result {
            self.files_status = format!("Preview download failed: {error}");
            return;
        }
        let bytes = match fs::read(&session.local_path) {
            Ok(bytes) => bytes,
            Err(error) => {
                self.files_status = format!("Unable to read preview copy: {error}");
                return;
            }
        };
        match image::load_from_memory(&bytes) {
            Ok(image) => {
                let image = image.to_rgba8();
                let dimensions = [image.width() as usize, image.height() as usize];
                self.file_preview_image = Some((
                    path,
                    egui::ColorImage::from_rgba_unmultiplied(dimensions, image.as_raw()),
                ));
                self.file_preview_texture = None;
                self.files_status = "Image preview downloaded with system scp.".into();
            }
            Err(error) => self.files_status = format!("Unsupported image: {error}"),
        }
    }

    fn open_editor_in_system_app(&mut self) {
        let Some(session) = &self.file_edit_session else {
            return;
        };
        let result = if cfg!(windows) {
            Command::new("explorer.exe")
                .arg(&session.local_path)
                .spawn()
        } else if cfg!(target_os = "macos") {
            Command::new("open").arg(&session.local_path).spawn()
        } else {
            Command::new("xdg-open").arg(&session.local_path).spawn()
        };
        if let Err(error) = result {
            self.files_status = format!("Unable to open the local working copy: {error}");
        }
    }

    fn save_editor_locally(&mut self) -> bool {
        let Some(session) = &self.file_edit_session else {
            return false;
        };
        let text = match session.file_info.line_ending {
            LineEnding::Lf => self.file_editor_text.clone(),
            LineEnding::Crlf => self
                .file_editor_text
                .replace("\r\n", "\n")
                .replace('\n', "\r\n"),
        };
        match fs::write(&session.local_path, text) {
            Ok(()) => {
                self.file_editor_status = FileEditorStatus::SavedLocally;
                self.file_editor_last_modified = fs::metadata(&session.local_path)
                    .ok()
                    .and_then(|metadata| metadata.modified().ok());
                true
            }
            Err(error) => {
                self.file_editor_status = FileEditorStatus::UploadFailed;
                self.files_status = format!("Unable to save local working copy: {error}");
                false
            }
        }
    }

    fn editor_text_for_local_copy(&self) -> String {
        let line_ending = self
            .file_edit_session
            .as_ref()
            .map(|session| session.file_info.line_ending)
            .unwrap_or(LineEnding::Lf);
        match line_ending {
            LineEnding::Lf => self.file_editor_text.clone(),
            LineEnding::Crlf => self
                .file_editor_text
                .replace("\r\n", "\n")
                .replace('\n', "\r\n"),
        }
    }

    fn find_editor_matches(&self) -> Vec<(usize, usize)> {
        if self.file_editor_find.is_empty() {
            return Vec::new();
        }
        self.file_editor_text
            .match_indices(&self.file_editor_find)
            .map(|(start, matched)| {
                let start = self.file_editor_text[..start].chars().count();
                (start, start + matched.chars().count())
            })
            .collect()
    }

    fn select_editor_match(&mut self, backwards: bool) {
        let matches = self.find_editor_matches();
        if matches.is_empty() {
            self.file_editor_match_index = 0;
            self.files_status = "No matching text in this working copy.".into();
            return;
        }
        if backwards {
            self.file_editor_match_index = if self.file_editor_match_index == 0 {
                matches.len() - 1
            } else {
                self.file_editor_match_index - 1
            };
        } else {
            self.file_editor_match_index = (self.file_editor_match_index + 1) % matches.len();
        }
        self.file_editor_pending_selection = Some(matches[self.file_editor_match_index]);
    }

    fn replace_all_editor_matches(&mut self) {
        if self.file_editor_find.is_empty() {
            return;
        }
        let count = self
            .file_editor_text
            .matches(&self.file_editor_find)
            .count();
        if count == 0 {
            self.files_status = "No matching text to replace.".into();
            return;
        }
        self.file_editor_text = self
            .file_editor_text
            .replace(&self.file_editor_find, &self.file_editor_replace);
        self.file_editor_status = FileEditorStatus::LocalModified;
        self.file_editor_match_index = 0;
        self.files_status = format!("Replaced {count} match(es) in the local working copy.");
    }

    fn go_to_editor_line(&mut self) {
        let line_count = self.file_editor_text.lines().count().max(1);
        let target_line = self.file_editor_go_to_line.clamp(1, line_count);
        self.file_editor_go_to_line = target_line;
        let character_index = self
            .file_editor_text
            .split_inclusive('\n')
            .take(target_line.saturating_sub(1))
            .map(|line| line.chars().count())
            .sum();
        self.file_editor_pending_selection = Some((character_index, character_index));
    }

    fn check_external_editor_change(&mut self) {
        let Some(session) = self.file_edit_session.as_ref() else {
            return;
        };
        let Some(modified) = fs::metadata(&session.local_path)
            .ok()
            .and_then(|metadata| metadata.modified().ok())
        else {
            return;
        };
        if self.file_editor_last_modified == Some(modified) {
            return;
        }
        self.file_editor_last_modified = Some(modified);
        let Ok(bytes) = fs::read(&session.local_path) else {
            return;
        };
        let info = easyssh_core::inspect_text(&bytes);
        if info.is_binary {
            self.files_status =
                "External editor wrote unsupported non-UTF-8 or binary content.".into();
            return;
        }
        let Ok(text) = String::from_utf8(bytes) else {
            self.files_status =
                "External editor wrote unsupported non-UTF-8 or binary content.".into();
            return;
        };
        if text == self.editor_text_for_local_copy() {
            return;
        }
        if self.file_editor_status == FileEditorStatus::LocalModified {
            self.file_editor_external_change = Some(text);
            self.files_status =
                "External editor changed the working copy; choose which local version to keep."
                    .into();
        } else {
            self.file_editor_text = text;
            self.file_editor_status = FileEditorStatus::SavedLocally;
            self.files_status =
                "External editor saved the local working copy. Upload changes when ready.".into();
        }
    }

    fn download_remote_text_for_conflict(
        &mut self,
        connection: &Connection,
        remote_path: &str,
        state: easyssh_core::RemoteFileState,
    ) -> Option<String> {
        let workspace = self.temporary_workspace.as_ref()?;
        let session = workspace.create_session(remote_path, state).ok()?;
        let transfer = Transfer::new(
            TransferDirection::Download,
            session.local_path.clone(),
            remote_path.to_owned(),
            false,
        );
        ScpInvocation::build(&self.openssh, connection, &transfer)
            .and_then(|invocation| invocation.spawn())
            .and_then(wait_for_scp)
            .ok()?;
        let bytes = fs::read(session.local_path).ok()?;
        if bytes.len() > 2 * 1024 * 1024 || easyssh_core::inspect_text(&bytes).is_binary {
            return None;
        }
        String::from_utf8(bytes).ok()
    }

    fn upload_editor_changes(&mut self) {
        let Some(session) = self.file_edit_session.clone() else {
            return;
        };
        let Some(connection) = self.files_connection_record() else {
            self.file_editor_status = FileEditorStatus::UploadFailed;
            return;
        };
        let Some(capabilities) = self.files_capabilities.clone() else {
            self.file_editor_status = FileEditorStatus::UploadFailed;
            return;
        };
        if !self.save_editor_locally() {
            return;
        }
        self.file_editor_status = FileEditorStatus::CheckingRemote;
        let mut current =
            match self
                .remote_files
                .stat(&connection, &capabilities, &session.remote_path)
            {
                Ok(state) => state,
                Err(error) => {
                    self.file_editor_status = FileEditorStatus::Conflict;
                    self.files_status =
                        format!("Remote file is no longer safe to overwrite: {error}");
                    return;
                }
            };
        if session.initial_remote.sha256.is_some() {
            current.sha256 = self
                .remote_files
                .calculate_hash(&connection, &capabilities, &session.remote_path)
                .ok();
        }
        if easyssh_core::remote_state_changed(&session.initial_remote, &current) {
            self.file_editor_status = FileEditorStatus::Conflict;
            self.file_conflict_remote =
                self.download_remote_text_for_conflict(&connection, &session.remote_path, current);
            self.file_conflict_open = true;
            self.files_status =
                "Remote file changed since it was downloaded. Local changes were preserved.".into();
            return;
        }
        let temporary_path = remote_temporary_sibling(&session.remote_path, &session.id);
        let transfer = Transfer::new(
            TransferDirection::Upload,
            session.local_path.clone(),
            temporary_path.clone(),
            false,
        );
        self.file_editor_status = FileEditorStatus::Uploading;
        let result = ScpInvocation::build(&self.openssh, &connection, &transfer)
            .and_then(|invocation| invocation.spawn())
            .and_then(wait_for_scp)
            .and_then(|_| {
                self.remote_files
                    .atomic_replace(
                        &connection,
                        &capabilities,
                        &temporary_path,
                        &session.remote_path,
                    )
                    .map_err(|error| easyssh_core::OpenSshError::Failed(error.to_string()))
            });
        match result {
            Ok(()) => {
                self.file_editor_status = FileEditorStatus::SavedRemotely;
                self.files_status = "Saved remotely using a same-directory temporary file.".into();
                self.refresh_files(false);
            }
            Err(error) => {
                self.file_editor_status = FileEditorStatus::UploadFailed;
                self.files_status =
                    format!("Upload failed; the local working copy was retained: {error}");
            }
        }
    }

    fn workspace_button(
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

    fn topbar(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("topbar")
            .exact_height(56.0)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.add_space(10.0);
                    ui.label(egui::RichText::new(icon::TERMINAL).size(26.0).color(BLUE));
                    ui.label(egui::RichText::new("EasySSH").strong().size(20.0));
                    ui.label(egui::RichText::new("WORKBENCH").small().weak());
                    ui.add_space(14.0);
                    if ui
                        .add_sized(
                            [270.0, 32.0],
                            egui::Button::new(format!(
                                "{}  Search commands and hosts",
                                icon::MAGNIFYING_GLASS
                            ))
                            .sense(egui::Sense::click()),
                        )
                        .clicked()
                    {
                        self.command_open = true;
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if icon_button(ui, icon::ARROWS_CLOCKWISE, "Git metadata sync").clicked() {
                            self.sync_open = true;
                        }
                        if icon_button(ui, icon::PLUG, "SSH Agent diagnostics").clicked() {
                            self.diagnostics_open = true;
                        }
                        let theme_icon = if matches!(self.config.theme, Theme::Dark) {
                            icon::MOON
                        } else {
                            icon::SUN
                        };
                        if icon_button(ui, theme_icon, "Cycle theme").clicked() {
                            self.config.theme = match self.config.theme {
                                Theme::System => Theme::Dark,
                                Theme::Dark => Theme::Light,
                                Theme::Light => Theme::System,
                            };
                            self.save();
                        }
                        if ui.button(format!("{} New", icon::PLUS)).clicked() {
                            self.add_host();
                        }
                        if ui.button(format!("{} Connect", icon::LIGHTNING)).clicked() {
                            self.quick_open = true;
                        }
                        let agent = if self.agent_available.unwrap_or(false) {
                            "Agent ready"
                        } else {
                            "Agent"
                        };
                        ui.label(
                            egui::RichText::new(format!("{} {}", icon::PLUG, agent))
                                .small()
                                .color(if self.agent_available.unwrap_or(false) {
                                    GREEN
                                } else {
                                    AMBER
                                }),
                        );
                        ui.label(
                            egui::RichText::new(sync_status_label(GitSync::status(&self.config)))
                                .small()
                                .weak(),
                        );
                    });
                });
            });
        egui::TopBottomPanel::top("sessions")
            .exact_height(42.0)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("RECENT SESSIONS").small().weak());
                    let sessions: Vec<SessionRecord> = self
                        .config
                        .sessions
                        .iter()
                        .filter(|s| !s.hidden)
                        .cloned()
                        .collect();
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
                            if ui.button("Hide from session bar").clicked() {
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
    }

    fn reconnect_session(&mut self, session: &SessionRecord) {
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

    fn navigation(&mut self, ctx: &egui::Context) {
        egui::SidePanel::left("navigation")
            .exact_width(190.0)
            .show(ctx, |ui| {
                ui.add_space(10.0);
                ui.label(egui::RichText::new("WORKSPACES").small().weak());
                ui.add_space(4.0);
                self.workspace_button(ui, Workspace::Hosts, icon::COMPUTER_TOWER, "Hosts");
                self.workspace_button(ui, Workspace::Files, icon::FOLDER_OPEN, "Files");
                self.workspace_button(ui, Workspace::Snippets, icon::CODE, "Snippets");
                self.workspace_button(
                    ui,
                    Workspace::Forwarding,
                    icon::ARROWS_LEFT_RIGHT,
                    "Port forwarding",
                );
                self.workspace_button(
                    ui,
                    Workspace::Transfers,
                    icon::ARROW_FAT_LINES_UP,
                    "Transfers",
                );
                ui.add_space(12.0);
                ui.separator();
                ui.add_space(8.0);
                ui.label(egui::RichText::new("TOOLS").small().weak());
                if ui.button(format!("{}  Agent", icon::PLUG)).clicked() {
                    self.diagnostics_open = true;
                }
                if ui
                    .button(format!("{}  Sync", icon::ARROWS_CLOCKWISE))
                    .clicked()
                {
                    self.sync_open = true;
                }
                ui.add_enabled(
                    false,
                    egui::Button::new(format!("{}  Known Hosts", icon::SHIELD_CHECK)),
                );
                ui.add_enabled(
                    false,
                    egui::Button::new(format!("{}  Logs", icon::FILE_TEXT)),
                );
                ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                    ui.label(egui::RichText::new(&self.status).small().weak().italics());
                });
            });
    }

    fn hosts(&mut self, ctx: &egui::Context) {
        egui::SidePanel::right("inspector")
            .exact_width(330.0)
            .show(ctx, |ui| {
                self.inspector(ui);
            });
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.add_space(12.0);
            ui.horizontal(|ui| {
                ui.heading("Hosts");
                ui.label(
                    egui::RichText::new(format!("{} hosts", self.config.connections.len())).weak(),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button(format!("{} Add host", icon::PLUS)).clicked() {
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
                        if ui.button("Create host").clicked() {
                            self.add_host();
                        }
                    });
                }
            });
        });
    }

    fn host_row(&mut self, ui: &mut egui::Ui, host: &Connection) {
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
        }
        if response.double_clicked() {
            self.connect(host, false);
        }
        response.context_menu(|ui| {
            if ui.button("Connect").clicked() {
                self.connect(host, false);
                ui.close_menu();
            }
            if ui.button("Detailed log").clicked() {
                self.connect(host, true);
                ui.close_menu();
            }
            if ui.button("Edit").clicked() {
                self.selected = Some(host.id.clone());
                self.editor_open = true;
                ui.close_menu();
            }
        });
    }

    fn inspector(&mut self, ui: &mut egui::Ui) {
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
            if ui.button(format!("{} Connect", icon::PLAY)).clicked() {
                self.connect(&host, false);
            }
            if icon_button(ui, icon::WARNING, "Connect with detailed OpenSSH log").clicked() {
                self.connect(&host, true);
            }
            if icon_button(ui, icon::PENCIL, "Edit host").clicked() {
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

    fn snippets(&mut self, ctx: &egui::Context) {
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

    fn forwarding(&mut self, ctx: &egui::Context) {
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

    fn transfers(&mut self, ctx: &egui::Context) {
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

    fn files(&mut self, ctx: &egui::Context) {
        self.check_external_editor_change();
        egui::SidePanel::left("files_sources")
            .resizable(true)
            .default_width(220.0)
            .show(ctx, |ui| {
                ui.heading("Files");
                ui.label(egui::RichText::new("HOSTS").small().weak());
                let hosts = self.config.connections.clone();
                for host in hosts {
                    let selected = self.files_connection.as_deref() == Some(&host.id);
                    if ui
                        .selectable_label(selected, format!("{}  {}", icon::HARD_DRIVES, host.name))
                        .clicked()
                    {
                        self.files_select_host(host.id);
                    }
                }
                ui.separator();
                ui.label(egui::RichText::new("FAVORITES").small().weak());
                ui.label(
                    egui::RichText::new("No favorite directories")
                        .small()
                        .weak(),
                );
                ui.separator();
                ui.label(egui::RichText::new("RECENT").small().weak());
                for path in self.files_history.iter().rev().take(5) {
                    ui.label(egui::RichText::new(path).small().monospace());
                }
            });
        egui::SidePanel::right("files_inspector")
            .resizable(true)
            .default_width(260.0)
            .show(ctx, |ui| {
                if self.files_dual_pane {
                    ui.heading("Local files");
                    ui.label(
                        egui::RichText::new(self.local_path.to_string_lossy())
                            .small()
                            .monospace(),
                    );
                    ui.horizontal(|ui| {
                        if icon_button(ui, icon::ARROW_UP, "Parent directory").clicked() {
                            if let Some(parent) = self.local_path.parent() {
                                self.local_path = parent.to_path_buf();
                                self.refresh_local_files();
                            }
                        }
                        if icon_button(ui, icon::ARROW_CLOCKWISE, "Refresh local files").clicked() {
                            self.refresh_local_files();
                        }
                        if let Some(selected) = self.local_selected.clone() {
                            if ui.button("Upload").clicked() {
                                self.queue_local_upload(&selected);
                            }
                        }
                    });
                    ui.separator();
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        for entry in self.local_entries.clone() {
                            let glyph = if entry.is_directory {
                                icon::FOLDER
                            } else {
                                icon::FILE
                            };
                            let selected = self.local_selected.as_ref() == Some(&entry.path);
                            let response = ui.selectable_label(
                                selected,
                                format!("{}  {}  {}", glyph, entry.name, format_bytes(entry.size)),
                            );
                            if response.clicked() {
                                self.local_selected = Some(entry.path.clone());
                            }
                            if response.double_clicked() && entry.is_directory {
                                self.local_path = entry.path;
                                self.local_selected = None;
                                self.refresh_local_files();
                            }
                        }
                    });
                } else {
                    ui.heading("Properties");
                    if let Some(selected) = self.files_selected.as_ref().and_then(|path| {
                        self.files_entries.iter().find(|entry| &entry.path == path)
                    }) {
                        ui.label(egui::RichText::new(&selected.name).strong());
                        detail(ui, "Type", format!("{:?}", selected.entry_type));
                        detail(ui, "Size", format_bytes(selected.size));
                        detail(ui, "Permissions", selected.permissions.clone());
                        detail(
                            ui,
                            "Owner",
                            format!("{}:{}", selected.owner, selected.group),
                        );
                        if let Some(target) = &selected.link_target {
                            detail(ui, "Link", target.clone());
                        }
                    } else {
                        ui.label(egui::RichText::new("Select an entry").weak());
                    }
                }
                ui.separator();
                ui.label(egui::RichText::new(&self.files_status).small().weak());
            });
        egui::CentralPanel::default().show(ctx, |ui| {
            if let Some((path, image)) = self.file_preview_image.clone() {
                if self.file_preview_texture.is_none() {
                    self.file_preview_texture = Some(ctx.load_texture(
                        format!("remote-preview-{path}"),
                        image,
                        egui::TextureOptions::LINEAR,
                    ));
                }
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    if ui.button(format!("{} Files", icon::ARROW_LEFT)).clicked() {
                        self.file_preview_image = None;
                        self.file_preview_texture = None;
                    }
                    ui.heading(&path);
                    ui.label(
                        egui::RichText::new("Temporary local preview")
                            .small()
                            .weak(),
                    );
                });
                ui.separator();
                if let Some(texture) = &self.file_preview_texture {
                    let available = ui.available_size();
                    let original = texture.size_vec2();
                    let scale = (available.x / original.x)
                        .min(available.y / original.y)
                        .min(1.0);
                    ui.image((texture.id(), original * scale));
                }
                return;
            }
            if let Some(session) = self.file_edit_session.clone() {
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    if ui.button(format!("{} Files", icon::ARROW_LEFT)).clicked() {
                        self.file_edit_session = None;
                        self.file_editor_text.clear();
                    }
                    ui.heading(
                        std::path::Path::new(&session.remote_path)
                            .file_name()
                            .and_then(|name| name.to_str())
                            .unwrap_or(&session.remote_path),
                    );
                    ui.label(
                        egui::RichText::new(self.file_editor_status.label())
                            .small()
                            .weak(),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui
                            .button(format!("{} Upload changes", icon::UPLOAD_SIMPLE))
                            .clicked()
                        {
                            self.upload_editor_changes();
                        }
                        if ui.button("Save local copy").clicked() {
                            self.save_editor_locally();
                        }
                        if ui.button("Open externally").clicked() {
                            self.open_editor_in_system_app();
                        }
                    });
                });
                ui.separator();
                if let Some(external_text) = self.file_editor_external_change.clone() {
                    ui.horizontal(|ui| {
                        ui.label("External editor changed the local working copy.");
                        if ui.button("Load external change").clicked() {
                            self.file_editor_text = external_text;
                            self.file_editor_external_change = None;
                            self.file_editor_status = FileEditorStatus::SavedLocally;
                        }
                        if ui.button("Keep editor text").clicked() {
                            self.file_editor_external_change = None;
                        }
                    });
                }
                ui.horizontal_wrapped(|ui| {
                    ui.label("Find");
                    let find_changed = ui
                        .add(
                            egui::TextEdit::singleline(&mut self.file_editor_find)
                                .hint_text("Text")
                                .desired_width(150.0),
                        )
                        .changed();
                    if find_changed {
                        self.file_editor_match_index = 0;
                    }
                    if ui.button("Previous").clicked() {
                        self.select_editor_match(true);
                    }
                    if ui.button("Next").clicked() {
                        self.select_editor_match(false);
                    }
                    let match_count = self.find_editor_matches().len();
                    if !self.file_editor_find.is_empty() {
                        ui.label(format!("{match_count} match(es)"));
                    }
                    ui.separator();
                    ui.label("Replace");
                    ui.add(
                        egui::TextEdit::singleline(&mut self.file_editor_replace)
                            .hint_text("Replacement")
                            .desired_width(150.0),
                    );
                    if ui.button("Replace all").clicked() {
                        self.replace_all_editor_matches();
                    }
                    ui.separator();
                    ui.label("Go to line");
                    ui.add(
                        egui::DragValue::new(&mut self.file_editor_go_to_line)
                            .range(1..=self.file_editor_text.lines().count().max(1))
                            .speed(1),
                    );
                    if ui.button("Go").clicked() {
                        self.go_to_editor_line();
                    }
                    ui.label(format!(
                        "Line {} / {}",
                        self.file_editor_cursor_line,
                        self.file_editor_text.lines().count().max(1)
                    ));
                });
                ui.separator();
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.label(egui::RichText::new("#").small().weak());
                        egui::ScrollArea::vertical()
                            .id_salt("file-editor-line-numbers")
                            .max_height(ui.available_height() - 12.0)
                            .show(ui, |ui| {
                                for line in 1..=self.file_editor_text.lines().count().max(1) {
                                    let text = if line == self.file_editor_cursor_line {
                                        egui::RichText::new(line.to_string()).strong()
                                    } else {
                                        egui::RichText::new(line.to_string()).weak()
                                    };
                                    ui.label(text);
                                }
                            });
                    });
                    let output = egui::TextEdit::multiline(&mut self.file_editor_text)
                        .id_source("file-editor")
                        .font(egui::TextStyle::Monospace)
                        .desired_width(ui.available_width())
                        .desired_rows(32)
                        .code_editor()
                        .show(ui);
                    if output.response.changed() {
                        self.file_editor_status = FileEditorStatus::LocalModified;
                    }
                    if let Some(range) = output.state.cursor.char_range() {
                        self.file_editor_cursor_line = self.file_editor_text[..]
                            .chars()
                            .take(range.primary.index)
                            .filter(|character| *character == '\n')
                            .count()
                            + 1;
                    }
                    if let Some((start, end)) = self.file_editor_pending_selection.take() {
                        let mut state = output.state;
                        state
                            .cursor
                            .set_char_range(Some(egui::text::CCursorRange::two(
                                egui::text::CCursor::new(start),
                                egui::text::CCursor::new(end),
                            )));
                        state.store(ui.ctx(), output.response.id);
                        output.response.request_focus();
                    }
                });
                return;
            }
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                if icon_button(ui, icon::ARROW_LEFT, "Back").clicked()
                    && self.files_history_index > 0
                {
                    self.files_history_index -= 1;
                    self.files_path = self.files_history[self.files_history_index].clone();
                    self.files_path_input = self.files_path.clone();
                    self.refresh_files(false);
                }
                if icon_button(ui, icon::ARROW_RIGHT, "Forward").clicked()
                    && self.files_history_index + 1 < self.files_history.len()
                {
                    self.files_history_index += 1;
                    self.files_path = self.files_history[self.files_history_index].clone();
                    self.files_path_input = self.files_path.clone();
                    self.refresh_files(false);
                }
                if icon_button(ui, icon::ARROW_UP, "Parent directory").clicked() {
                    self.files_go_up();
                }
                let path_response = ui.add(
                    egui::TextEdit::singleline(&mut self.files_path_input)
                        .desired_width(ui.available_width() * 0.52)
                        .hint_text("Remote path"),
                );
                if path_response.lost_focus()
                    && ui.input(|input| input.key_pressed(egui::Key::Enter))
                {
                    self.open_files_path(self.files_path_input.trim().to_owned());
                }
                if icon_button(ui, icon::ARROW_CLOCKWISE, "Refresh").clicked() {
                    self.refresh_files(false);
                }
                if icon_button(ui, icon::UPLOAD_SIMPLE, "Upload").clicked() {
                    self.config.workspace = Workspace::Transfers;
                }
                if ui.button("New folder").clicked() {
                    self.files_new_dir_name.clear();
                    self.files_create_dir_open = true;
                }
                if self.files_selected.is_some() && ui.button("Rename").clicked() {
                    self.files_rename_name = self
                        .files_selected
                        .as_ref()
                        .and_then(|path| {
                            self.files_entries.iter().find(|entry| &entry.path == path)
                        })
                        .map(|entry| entry.name.clone())
                        .unwrap_or_default();
                    self.files_rename_open = true;
                }
                if self.files_selected.is_some() && ui.button("Delete").clicked() {
                    self.files_delete_open = true;
                }
                ui.checkbox(&mut self.files_hidden, "Hidden");
                if ui
                    .checkbox(&mut self.files_dual_pane, "Two panes")
                    .changed()
                    && self.files_dual_pane
                {
                    self.refresh_local_files();
                }
                if self.files_dual_pane {
                    if let Some(path) = self.files_selected.clone() {
                        if ui.button("Download selected").clicked() {
                            self.queue_remote_download(path);
                        }
                    }
                }
            });
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("Filter").small().weak());
                ui.add(egui::TextEdit::singleline(&mut self.files_filter).desired_width(240.0));
                if let Some(capabilities) = &self.files_capabilities {
                    ui.label(
                        egui::RichText::new(format!(
                            "{} / {:?}",
                            capabilities.uname, capabilities.platform
                        ))
                        .small()
                        .weak(),
                    );
                }
            });
            ui.separator();
            let mut entries = self.files_entries.clone();
            entries.sort_by_cached_key(|entry| entry.name.to_lowercase());
            egui::ScrollArea::vertical().show(ui, |ui| {
                if self.files_connection.is_none() {
                    ui.vertical_centered(|ui| {
                        ui.add_space(80.0);
                        ui.label(
                            egui::RichText::new(icon::FOLDER_OPEN)
                                .size(42.0)
                                .color(BLUE),
                        );
                        ui.heading("Select a host");
                        ui.label("Remote browsing uses system ssh.");
                    });
                }
                for entry in entries {
                    if !contains(&entry.name, &self.files_filter) {
                        continue;
                    }
                    let selected = self.files_selected.as_deref() == Some(&entry.path);
                    let glyph = match entry.entry_type {
                        RemoteEntryType::Directory => icon::FOLDER,
                        RemoteEntryType::Symlink => icon::LINK,
                        RemoteEntryType::File => icon::FILE,
                        RemoteEntryType::Other => icon::FILE,
                    };
                    let response = ui.selectable_label(
                        selected,
                        format!(
                            "{}  {:<32}  {:>10}  {}",
                            glyph,
                            entry.name,
                            format_bytes(entry.size),
                            entry.permissions
                        ),
                    );
                    if response.clicked() {
                        self.files_selected = Some(entry.path.clone());
                    }
                    if response.double_clicked() {
                        match entry.entry_type {
                            RemoteEntryType::Directory => self.open_files_path(entry.path.clone()),
                            RemoteEntryType::File if is_previewable_image(&entry.name) => {
                                self.open_remote_image_preview(entry.path.clone())
                            }
                            RemoteEntryType::File => self.open_remote_text_file(entry.path.clone()),
                            _ => {
                                self.files_status =
                                    "This entry is not supported by the text editor.".into()
                            }
                        }
                    }
                }
            });
        });
    }

    fn command_panel(&mut self, ctx: &egui::Context) {
        if !self.command_open {
            return;
        }
        let mut open = self.command_open;
        let mut close = false;
        let mut chosen: Option<CommandAction> = None;
        egui::Window::new("Command palette")
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .fixed_size([600.0, 410.0])
            .anchor(egui::Align2::CENTER_TOP, [0.0, 72.0])
            .show(ctx, |ui| {
                let response = ui.add(
                    egui::TextEdit::singleline(&mut self.command_query)
                        .hint_text("Search hosts, groups, tags, snippets, sessions, or actions")
                        .desired_width(f32::INFINITY)
                        .id(egui::Id::new("command-query")),
                );
                response.request_focus();
                let actions = self.command_actions();
                if ctx.input(|i| i.key_pressed(egui::Key::ArrowDown)) {
                    self.command_index =
                        (self.command_index + 1).min(actions.len().saturating_sub(1));
                }
                if ctx.input(|i| i.key_pressed(egui::Key::ArrowUp)) {
                    self.command_index = self.command_index.saturating_sub(1);
                }
                if ctx.input(|i| i.key_pressed(egui::Key::Enter)) {
                    chosen = actions.get(self.command_index).cloned();
                }
                if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
                    close = true;
                }
                egui::ScrollArea::vertical().show(ui, |ui| {
                    for (index, action) in actions.iter().enumerate() {
                        if ui
                            .selectable_label(index == self.command_index, action.label())
                            .clicked()
                        {
                            chosen = Some(action.clone());
                        }
                    }
                });
            });
        if let Some(action) = chosen {
            self.run_command(action);
            close = true;
        }
        if close {
            open = false;
        }
        self.command_open = open;
    }

    fn command_actions(&self) -> Vec<CommandAction> {
        let query = &self.command_query;
        let mut items = vec![
            CommandAction::NewHost,
            CommandAction::Switch(Workspace::Hosts),
            CommandAction::Switch(Workspace::Files),
            CommandAction::Switch(Workspace::Snippets),
            CommandAction::Switch(Workspace::Forwarding),
            CommandAction::Switch(Workspace::Transfers),
            CommandAction::OpenSync,
        ];
        for host in self.config.connections.iter().filter(|h| {
            contains(
                &format!("{} {} {}", h.name, target_text(h), h.tags.join(" ")),
                query,
            )
        }) {
            items.push(CommandAction::Host(host.id.clone(), host.name.clone()));
            items.push(CommandAction::Connect(
                host.id.clone(),
                host.name.clone(),
                false,
            ));
            items.push(CommandAction::Connect(
                host.id.clone(),
                host.name.clone(),
                true,
            ));
        }
        items.extend(
            self.config
                .snippets
                .iter()
                .filter(|s| contains(&format!("{} {}", s.name, s.content), query))
                .map(|s| CommandAction::Snippet(s.id.clone(), s.name.clone())),
        );
        items.extend(
            self.config
                .sessions
                .iter()
                .filter(|s| contains(&format!("{} {}", s.name, s.target), query))
                .map(|s| CommandAction::Session(s.id.clone(), s.name.clone())),
        );
        items
            .into_iter()
            .filter(|a| contains(&a.label(), query))
            .collect()
    }

    fn run_command(&mut self, action: CommandAction) {
        match action {
            CommandAction::NewHost => self.add_host(),
            CommandAction::Switch(workspace) => {
                self.config.workspace = workspace;
                self.save();
            }
            CommandAction::Host(id, _) => {
                self.selected = Some(id);
                self.config.workspace = Workspace::Hosts;
            }
            CommandAction::Connect(id, _, verbose) => {
                if let Some(host) = self
                    .config
                    .connections
                    .iter()
                    .find(|host| host.id == id)
                    .cloned()
                {
                    self.connect(&host, verbose);
                }
            }
            CommandAction::Snippet(id, _) => {
                if let Some(item) = self.config.snippets.iter().find(|s| s.id == id) {
                    self.status = "Snippet copied. Commands are never sent to a terminal.".into();
                    self.copy_text = Some(item.content.clone());
                }
                self.config.workspace = Workspace::Snippets;
            }
            CommandAction::Session(id, _) => {
                if let Some(session) = self.config.sessions.iter().find(|s| s.id == id).cloned() {
                    self.reconnect_session(&session);
                }
            }
            CommandAction::OpenSync => self.sync_open = true,
        }
    }

    fn host_editor(&mut self, ctx: &egui::Context) {
        if !self.editor_open {
            return;
        }
        let Some(index) = self
            .selected
            .as_ref()
            .and_then(|id| self.config.connections.iter().position(|h| &h.id == id))
        else {
            self.editor_open = false;
            return;
        };
        let mut open = self.editor_open;
        let mut save = false;
        egui::Window::new("Edit host")
            .open(&mut open)
            .resizable(true)
            .default_width(520.0)
            .show(ctx, |ui| {
                let host = &mut self.config.connections[index];
                ui.label("Display name");
                ui.text_edit_singleline(&mut host.name);
                let mut alias = matches!(host.target, ConnectionTarget::Alias { .. });
                ui.horizontal(|ui| {
                    ui.radio_value(&mut alias, false, "Address");
                    ui.radio_value(&mut alias, true, "OpenSSH alias");
                });
                if alias != matches!(host.target, ConnectionTarget::Alias { .. }) {
                    host.target = if alias {
                        ConnectionTarget::Alias {
                            alias: String::new(),
                        }
                    } else {
                        ConnectionTarget::Endpoint {
                            hostname: String::new(),
                            username: Some(String::new()),
                            port: 22,
                        }
                    };
                }
                match &mut host.target {
                    ConnectionTarget::Alias { alias } => {
                        ui.label("Alias");
                        ui.text_edit_singleline(alias);
                    }
                    ConnectionTarget::Endpoint {
                        hostname,
                        username,
                        port,
                    } => {
                        ui.label("Hostname");
                        ui.text_edit_singleline(hostname);
                        ui.horizontal(|ui| {
                            ui.vertical(|ui| {
                                ui.label("User");
                                ui.text_edit_singleline(username.get_or_insert_with(String::new));
                            });
                            ui.vertical(|ui| {
                                ui.label("Port");
                                ui.add(egui::DragValue::new(port).range(1..=65535));
                            });
                        });
                    }
                }
                ui.label("Tags (comma separated)");
                let mut tags = host.tags.join(", ");
                if ui.text_edit_singleline(&mut tags).changed() {
                    host.tags = tags
                        .split(',')
                        .map(str::trim)
                        .filter(|s| !s.is_empty())
                        .map(str::to_owned)
                        .collect();
                }
                ui.label("Local forwards (one per line)");
                edit_lines(ui, &mut host.local_forwards);
                ui.label("Remote forwards (one per line)");
                edit_lines(ui, &mut host.remote_forwards);
                ui.label("Dynamic forwards (one per line)");
                edit_lines(ui, &mut host.dynamic_forwards);
                ui.label("Notes");
                ui.add_sized(
                    [ui.available_width(), 70.0],
                    egui::TextEdit::multiline(&mut host.notes),
                );
                ui.checkbox(&mut host.favorite, "Favorite");
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("Save").clicked() {
                        save = true;
                    }
                });
            });
        if save {
            self.save();
            open = false;
        }
        self.editor_open = open;
    }

    fn snippet_editor(&mut self, ctx: &egui::Context) {
        let Some(id) = self.snippet_editor.clone() else {
            return;
        };
        let Some(index) = self.config.snippets.iter().position(|s| s.id == id) else {
            self.snippet_editor = None;
            return;
        };
        let mut open = true;
        let mut save = false;
        egui::Window::new("Edit snippet").open(&mut open).default_width(520.0).show(ctx, |ui| { let item = &mut self.config.snippets[index]; ui.label("Name"); ui.text_edit_singleline(&mut item.name); ui.label("Command text"); ui.add_sized([ui.available_width(), 180.0], egui::TextEdit::multiline(&mut item.content).code_editor()); ui.label(egui::RichText::new("Snippets can only be copied. They are never executed or injected into a terminal.").small().weak()); if ui.button("Save").clicked() { save = true; } });
        if save {
            self.save();
            open = false;
        }
        if !open {
            self.snippet_editor = None;
        }
    }

    fn quick_connect(&mut self, ctx: &egui::Context) {
        if !self.quick_open {
            return;
        }
        let mut open = self.quick_open;
        let mut launch = false;
        egui::Window::new("Quick connect")
            .open(&mut open)
            .resizable(false)
            .show(ctx, |ui| {
                ui.label("Host address");
                ui.text_edit_singleline(&mut self.quick_host);
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.label("User");
                        ui.text_edit_singleline(&mut self.quick_user);
                    });
                    ui.vertical(|ui| {
                        ui.label("Port");
                        ui.add(egui::DragValue::new(&mut self.quick_port).range(1..=65535));
                    });
                });
                if ui
                    .add_enabled(
                        !self.quick_host.trim().is_empty(),
                        egui::Button::new("Connect"),
                    )
                    .clicked()
                {
                    launch = true;
                }
            });
        if launch {
            let mut host = Connection::alias(&self.quick_host, &self.quick_host);
            host.target = ConnectionTarget::Endpoint {
                hostname: self.quick_host.clone(),
                username: (!self.quick_user.is_empty()).then(|| self.quick_user.clone()),
                port: self.quick_port,
            };
            self.connect(&host, false);
            open = false;
        }
        self.quick_open = open;
    }

    fn confirmation_dialogs(&mut self, ctx: &egui::Context) {
        if let Some(id) = self.delete_host.clone() {
            let mut open = true;
            egui::Window::new("Delete host?")
                .open(&mut open)
                .collapsible(false)
                .resizable(false)
                .show(ctx, |ui| {
                    ui.label("This removes only EasySSH workbench metadata.");
                    ui.label("It does not delete system SSH configuration, keys, or external terminals already launched.");
                    ui.horizontal(|ui| {
                        if ui.button(egui::RichText::new("Delete host").color(RED)).clicked() {
                            self.config.connections.retain(|h| h.id != id);
                            self.selected = None;
                            self.delete_host = None;
                            self.save();
                        } else if ui.button("Cancel").clicked() {
                            self.delete_host = None;
                        }
                    });
                });
            if !open {
                self.delete_host = None;
            }
        }
        if let Some(id) = self.delete_snippet.clone() {
            let mut open = true;
            egui::Window::new("Delete snippet?")
                .open(&mut open)
                .collapsible(false)
                .show(ctx, |ui| {
                    ui.label("This removes the saved snippet text.");
                    ui.horizontal(|ui| {
                        if ui
                            .button(egui::RichText::new("Delete").color(RED))
                            .clicked()
                        {
                            self.config.snippets.retain(|s| s.id != id);
                            self.delete_snippet = None;
                            self.save();
                        }
                        if ui.button("Cancel").clicked() {
                            self.delete_snippet = None;
                        }
                    });
                });
            if !open {
                self.delete_snippet = None;
            }
        }
    }

    fn diagnostics(&mut self, ctx: &egui::Context) {
        if !self.diagnostics_open {
            return;
        }
        let mut open = self.diagnostics_open;
        egui::Window::new("SSH Agent diagnostics")
            .open(&mut open)
            .show(ctx, |ui| {
                let report = self.openssh.diagnostics(None);
                self.agent_available = report.agent_keys.as_ref().map(|keys| keys.available);
                detail(
                    ui,
                    "OpenSSH",
                    report
                        .ssh_path
                        .map(|p| p.display().to_string())
                        .unwrap_or_else(|| "Not found".into()),
                );
                detail(
                    ui,
                    "Agent",
                    if report.agent_socket_configured {
                        "Configured".into()
                    } else {
                        "Not configured".into()
                    },
                );
                detail(
                    ui,
                    "Available keys",
                    report
                        .agent_keys
                        .map(|k| {
                            if k.available {
                                k.fingerprints.len().to_string()
                            } else {
                                "Unavailable".into()
                            }
                        })
                        .unwrap_or_else(|| "Unknown".into()),
                );
                ui.label(
                    egui::RichText::new(
                        "Key material and fingerprints are never saved or displayed.",
                    )
                    .small()
                    .weak(),
                );
            });
        self.diagnostics_open = open;
    }

    fn sync_panel(&mut self, ctx: &egui::Context) {
        if !self.sync_open {
            return;
        }
        let mut open = self.sync_open;
        let mut save = false;
        let mut action: Option<&str> = None;
        egui::Window::new("Git metadata sync")
            .open(&mut open).default_width(560.0).resizable(true)
            .show(ctx, |ui| {
                ui.label(egui::RichText::new("Explicit sync for non-sensitive workbench metadata").strong());
                ui.label(egui::RichText::new("Hosts, groups, tags, notes, forwards, proxy jumps, and snippets may reveal infrastructure structure. Private keys, passwords, credential data, key paths, terminal output, SSH diagnostics, working directories, and complete SSH commands are never written to Git.").small().weak());
                ui.add_space(8.0);
                ui.label("Display name"); ui.text_edit_singleline(&mut self.config.sync.display_name);
                ui.label("Repository folder"); ui.text_edit_singleline(self.config.sync.repository_path.get_or_insert_with(String::new));
                ui.label("Remote URL (optional, used only when initializing)"); ui.text_edit_singleline(self.config.sync.remote_url.get_or_insert_with(String::new));
                ui.label("Branch"); ui.text_edit_singleline(self.config.sync.branch.get_or_insert_with(|| "main".into()));
                ui.label("Display density");
                ui.horizontal(|ui| {
                    ui.selectable_value(&mut self.config.display_density, DisplayDensity::Compact, "Compact");
                    ui.selectable_value(&mut self.config.display_density, DisplayDensity::Comfortable, "Comfortable");
                    ui.selectable_value(&mut self.config.display_density, DisplayDensity::Large, "Large");
                });
                ui.add_space(8.0);
                detail(ui, "State", sync_status_label(GitSync::status(&self.config)).into());
                if let Some(time) = self.config.sync.last_success_at { detail(ui, "Last success", relative_time(time)); }
                if let Some(error) = &self.config.sync.last_error { ui.colored_label(RED, error); }
                ui.horizontal(|ui| {
                    if ui.button("Initialize repository").clicked() { action = Some("init"); }
                    if ui.add_enabled(GitSync::repository(&self.config).is_ok(), egui::Button::new("Pull")).clicked() { action = Some("pull"); }
                    if ui.add_enabled(GitSync::repository(&self.config).is_ok(), egui::Button::new("Push")).clicked() { action = Some("push"); }
                });
                if ui.button("Save settings").clicked() { save = true; }
            });
        if let Some(action) = action {
            let result = match action {
                "init" => self
                    .config
                    .sync
                    .repository_path
                    .as_deref()
                    .filter(|p| !p.trim().is_empty())
                    .ok_or_else(|| "Repository folder is required.".to_owned())
                    .and_then(|path| {
                        GitSync::init(
                            std::path::Path::new(path),
                            self.config
                                .sync
                                .remote_url
                                .as_deref()
                                .filter(|u| !u.trim().is_empty()),
                            self.config.sync.branch.as_deref().unwrap_or("main"),
                        )
                        .map_err(|e| e.to_string())
                    }),
                "pull" => GitSync::pull(&mut self.config)
                    .map(|changed| {
                        self.status = if changed {
                            "Metadata pulled. Review before pushing.".into()
                        } else {
                            "No metadata file exists on the remote branch.".into()
                        };
                    })
                    .map_err(|e| e.to_string()),
                _ => GitSync::push(&mut self.config).map_err(|e| e.to_string()),
            };
            match result {
                Ok(()) => {
                    self.config.sync.last_error = None;
                    self.status = if action == "push" {
                        "Metadata pushed to Git.".into()
                    } else if action == "init" {
                        "Git repository initialized.".into()
                    } else {
                        self.status.clone()
                    };
                    save = true;
                }
                Err(error) => {
                    self.config.sync.last_error = Some(error.clone());
                    self.status = error;
                    save = true;
                }
            }
        }
        if save {
            self.save();
        }
        self.sync_open = open;
    }

    fn shortcuts(&mut self, ctx: &egui::Context) {
        if ctx.input(|i| i.modifiers.command && i.key_pressed(egui::Key::K)) {
            self.command_open = true;
        }
        if ctx.input(|i| i.modifiers.command && i.key_pressed(egui::Key::N)) {
            self.add_host();
        }
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) && !self.command_open {
            self.search.clear();
        }
        if self.config.workspace == Workspace::Files {
            if self.file_edit_session.is_none()
                && self.files_selected.is_some()
                && ctx.input(|i| i.key_pressed(egui::Key::F2))
            {
                self.files_rename_name = self
                    .files_selected
                    .as_ref()
                    .and_then(|path| self.files_entries.iter().find(|entry| &entry.path == path))
                    .map(|entry| entry.name.clone())
                    .unwrap_or_default();
                self.files_rename_open = true;
            }
            if self.file_edit_session.is_none()
                && self.files_selected.is_some()
                && ctx.input(|i| i.key_pressed(egui::Key::Delete))
            {
                self.files_delete_open = true;
            }
            if ctx.input(|i| i.modifiers.command && i.key_pressed(egui::Key::S))
                && self.file_edit_session.is_some()
            {
                self.upload_editor_changes();
            }
            if ctx.input(|i| i.modifiers.command && i.key_pressed(egui::Key::R)) {
                self.refresh_files(false);
            }
            if ctx.input(|i| i.modifiers.command && i.key_pressed(egui::Key::U)) {
                self.config.workspace = Workspace::Transfers;
                self.transfer_connection = self.files_connection.clone();
                self.transfer_remote_path = self.files_path.clone();
            }
            if ctx.input(|i| i.key_pressed(egui::Key::Backspace)) {
                self.files_go_up();
            }
            let dropped = ctx.input(|i| i.raw.dropped_files.clone());
            if let Some(file) = dropped.first().and_then(|file| file.path.clone()) {
                self.transfer_connection = self.files_connection.clone();
                self.transfer_local_path = file.to_string_lossy().into_owned();
                self.transfer_remote_path = self.files_path.clone();
                self.transfer_direction = TransferDirection::Upload;
                self.config.workspace = Workspace::Transfers;
                self.status = "Local file queued for upload review.".into();
            }
        }
    }

    fn file_conflict_dialog(&mut self, ctx: &egui::Context) {
        if !self.file_conflict_open {
            return;
        }
        let mut open = true;
        egui::Window::new("Remote file changed")
            .collapsible(false)
            .resizable(true)
            .open(&mut open)
            .show(ctx, |ui| {
                ui.label("The remote file changed after this working copy was downloaded.");
                ui.label("EasySSH kept your local working copy and did not overwrite the server.");
                if let Some(session) = &self.file_edit_session {
                    ui.label(egui::RichText::new(&session.remote_path).monospace());
                }
                ui.separator();
                let initial = self
                    .file_edit_session
                    .as_ref()
                    .and_then(|session| fs::read_to_string(&session.base_path).ok())
                    .unwrap_or_else(|| "Initial version is unavailable.".into());
                let remote = self
                    .file_conflict_remote
                    .clone()
                    .unwrap_or_else(|| "Current remote version could not be downloaded for preview.".into());
                let local = self.file_editor_text.clone();
                ui.columns(3, |columns| {
                    for (column, (title, value)) in columns.iter_mut().zip([
                        ("My local version", local),
                        ("Initially downloaded", initial),
                        ("Current remote version", remote),
                    ]) {
                        column.label(egui::RichText::new(title).strong());
                        let mut display = value;
                        column.add_enabled(
                            false,
                            egui::TextEdit::multiline(&mut display)
                                .font(egui::TextStyle::Monospace)
                                .desired_rows(12),
                        );
                    }
                });
                ui.label("You can keep the local copy, close the editor, or manually compare it with a new download.");
                if ui.button("Keep local copy and close").clicked() {
                    self.file_edit_session = None;
                    self.file_editor_text.clear();
                    self.file_conflict_remote = None;
                    self.file_conflict_open = false;
                }
                if ui.button("Continue editing local copy").clicked() {
                    self.file_conflict_open = false;
                }
            });
        if !open {
            self.file_conflict_open = false;
        }
    }

    fn file_operation_dialogs(&mut self, ctx: &egui::Context) {
        if self.files_create_dir_open {
            let mut open = true;
            egui::Window::new("New remote folder")
                .collapsible(false)
                .resizable(false)
                .open(&mut open)
                .show(ctx, |ui| {
                    ui.label(egui::RichText::new(&self.files_path).monospace());
                    ui.text_edit_singleline(&mut self.files_new_dir_name);
                    if ui.button("Create folder").clicked() {
                        self.create_remote_directory();
                    }
                });
            if !open {
                self.files_create_dir_open = false;
            }
        }
        if self.files_rename_open {
            let mut open = true;
            egui::Window::new("Rename remote entry")
                .collapsible(false)
                .resizable(false)
                .open(&mut open)
                .show(ctx, |ui| {
                    ui.text_edit_singleline(&mut self.files_rename_name);
                    if ui.button("Rename").clicked() {
                        self.rename_selected_remote();
                    }
                });
            if !open {
                self.files_rename_open = false;
            }
        }
        if self.files_delete_open {
            let selected = self
                .files_selected
                .as_ref()
                .and_then(|path| self.files_entries.iter().find(|entry| &entry.path == path))
                .cloned();
            let mut open = true;
            egui::Window::new("Delete remote entry")
                .collapsible(false)
                .resizable(false)
                .open(&mut open)
                .show(ctx, |ui| {
                    if let Some(entry) = selected {
                        ui.colored_label(RED, format!("Delete {}?", entry.name));
                        if entry.entry_type == RemoteEntryType::Directory {
                            ui.label("This permanently deletes the directory and its contents.");
                        }
                        if ui.button("Delete permanently").clicked() {
                            self.delete_selected_remote();
                        }
                    }
                });
            if !open {
                self.files_delete_open = false;
            }
        }
    }

    #[cfg(feature = "ui-test")]
    fn handle_ui_test_bridge(&mut self, ctx: &egui::Context) {
        let Some(mode) = self.test_mode.clone() else {
            return;
        };
        let Some(request) = mode.take_bridge_request() else {
            return;
        };
        let response = match request["operation"].as_str() {
            Some("get_ui_tree") => json!({"success":true,"tree":self.ui_test_tree()}),
            Some("click") if request["element_id"].as_str() == Some("navigation.files") => {
                self.config.workspace = Workspace::Files;
                json!({"success":true,"tree":self.ui_test_tree()})
            }
            Some("click") if request["element_id"].as_str() == Some("files.toggle_dual_pane") => {
                self.files_dual_pane = !self.files_dual_pane;
                if self.files_dual_pane {
                    self.refresh_local_files();
                }
                json!({"success":true,"tree":self.ui_test_tree()})
            }
            Some("click") if request["element_id"].as_str() == Some("files.new_folder") => {
                self.files_new_dir_name.clear();
                self.files_create_dir_open = true;
                json!({"success":true,"tree":self.ui_test_tree()})
            }
            Some("click") if request["element_id"].as_str() == Some("navigation.transfers") => {
                self.config.workspace = Workspace::Transfers;
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
            Some("resize") => match (request["width"].as_f64(), request["height"].as_f64()) {
                (Some(width), Some(height)) if width >= 320.0 && height >= 320.0 => {
                    ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(egui::vec2(
                        width as f32,
                        height as f32,
                    )));
                    json!({"success":true,"width":width,"height":height})
                }
                _ => json!({"success":false,"error":"invalid window dimensions"}),
            },
            Some("send_key") if request["key"].as_str() == Some("Escape") => {
                self.search.clear();
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

    #[cfg(feature = "ui-test")]
    fn save_ui_test_screenshot(&mut self, ctx: &egui::Context) {
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
    fn ui_test_tree(&self) -> Value {
        let visible = self.config.workspace == Workspace::Transfers;
        let files_visible = self.config.workspace == Workspace::Files;
        json!({"id":"app.root","role":"window","text":"EasySSH [UI Test]","visible":true,"enabled":true,"state":{"ui.is_idle":true,"ui.animation_count":0,"ui.pending_task_count":self.transfer_children.len()},"children":[
          {"id":"navigation.files","role":"button","text":"Files","visible":true,"enabled":true,"selected":files_visible},
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
          {"id":"navigation.transfers","role":"button","text":"Transfers","visible":true,"enabled":true,"selected":visible},
          {"id":"transfers.page","role":"page","text":"Transfers","visible":visible,"enabled":true,"children":[
            {"id":"transfers.host_selector","role":"combobox","text":"Host","visible":visible,"enabled":true},
            {"id":"transfers.connection_status","role":"status","text":"Disconnected","visible":visible,"enabled":true},
            {"id":"transfers.local_path","role":"textbox","text":"Local path","value":self.transfer_local_path,"visible":visible,"enabled":true},
            {"id":"transfers.remote_path","role":"textbox","text":"Remote path","value":self.transfer_remote_path,"visible":visible,"enabled":true},
            {"id":"transfers.upload_button","role":"button","text":"Upload","visible":visible,"enabled":true},
            {"id":"transfers.download_button","role":"button","text":"Download","visible":visible,"enabled":true},
            {"id":"transfers.transfer_queue","role":"list","text":"Transfer queue","visible":visible,"enabled":true},
            {"id":"transfers.empty_state","role":"status","text":"No transfers yet","visible":visible,"enabled":true}
          ]}
        ]})
    }
}

impl eframe::App for EasySshApp {
    fn update(&mut self, ctx: &egui::Context, _: &mut eframe::Frame) {
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
            Workspace::Hosts => self.hosts(ctx),
            Workspace::Files => self.files(ctx),
            Workspace::Snippets => self.snippets(ctx),
            Workspace::Forwarding => self.forwarding(ctx),
            Workspace::Transfers => self.transfers(ctx),
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
    }
}

#[derive(Clone)]
enum CommandAction {
    NewHost,
    OpenSync,
    Switch(Workspace),
    Host(String, String),
    Connect(String, String, bool),
    Snippet(String, String),
    Session(String, String),
}
impl CommandAction {
    fn label(&self) -> String {
        match self {
            Self::NewHost => format!("{} New host", icon::PLUS),
            Self::OpenSync => format!("{} Open Git metadata sync", icon::ARROWS_CLOCKWISE),
            Self::Switch(Workspace::Hosts) => "Go to Hosts".into(),
            Self::Switch(Workspace::Files) => "Go to Files".into(),
            Self::Switch(Workspace::Snippets) => "Go to Snippets".into(),
            Self::Switch(Workspace::Forwarding) => "Go to Port forwarding".into(),
            Self::Switch(Workspace::Transfers) => "Go to Transfers".into(),
            Self::Host(_, name) => format!("Open host: {name}"),
            Self::Connect(_, name, false) => format!("Connect: {name}"),
            Self::Connect(_, name, true) => format!("Detailed log: {name}"),
            Self::Snippet(_, name) => format!("Copy snippet: {name}"),
            Self::Session(_, name) => format!("Reconnect session: {name}"),
        }
    }
}

fn icon_button(ui: &mut egui::Ui, glyph: &str, tooltip: &str) -> egui::Response {
    crate::ui::components::icon_button(ui, glyph, tooltip)
}
fn section(ui: &mut egui::Ui, label: &str) {
    ui.add_space(10.0);
    ui.label(egui::RichText::new(label).small().strong().color(BLUE));
}
fn detail(ui: &mut egui::Ui, label: &str, value: String) {
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new(label).weak());
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(value);
        });
    });
}
fn edit_lines(ui: &mut egui::Ui, values: &mut Vec<String>) {
    let mut text = values.join("\n");
    if ui
        .add_sized(
            [ui.available_width(), 46.0],
            egui::TextEdit::multiline(&mut text).desired_rows(2),
        )
        .changed()
    {
        *values = text
            .lines()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_owned)
            .collect();
    }
}
fn target_text(host: &Connection) -> String {
    match &host.target {
        ConnectionTarget::Alias { alias } => alias.clone(),
        ConnectionTarget::Endpoint {
            hostname,
            username,
            port,
        } => match username.as_deref().filter(|u| !u.is_empty()) {
            Some(user) => format!("{user}@{hostname}:{port}"),
            None => format!("{hostname}:{port}"),
        },
    }
}
fn relative_time(time: DateTime<Utc>) -> String {
    let seconds = (Utc::now() - time).num_seconds().max(0);
    if seconds < 60 {
        "now".into()
    } else if seconds < 3600 {
        format!("{}m", seconds / 60)
    } else if seconds < 86_400 {
        format!("{}h", seconds / 3600)
    } else {
        format!("{}d", seconds / 86_400)
    }
}
fn contains(value: &str, query: &str) -> bool {
    query.trim().is_empty() || value.to_lowercase().contains(&query.trim().to_lowercase())
}
fn host_matches(host: &Connection, query: &str, group: &str) -> bool {
    contains(
        &format!(
            "{} {} {} {}",
            host.name,
            target_text(host),
            host.tags.join(" "),
            group
        ),
        query,
    )
}
fn single_line(value: &str) -> String {
    value.lines().next().unwrap_or_default().to_owned()
}
fn count_forwards(values: &[String]) -> String {
    if values.is_empty() {
        "None".into()
    } else {
        values.len().to_string()
    }
}

fn format_bytes(size: u64) -> String {
    const UNITS: [&str; 4] = ["B", "KB", "MB", "GB"];
    let mut value = size as f64;
    let mut index = 0;
    while value >= 1024.0 && index + 1 < UNITS.len() {
        value /= 1024.0;
        index += 1;
    }
    if index == 0 {
        format!("{} {}", size, UNITS[index])
    } else {
        format!("{value:.1} {}", UNITS[index])
    }
}

fn remote_temporary_sibling(path: &str, session_id: &str) -> String {
    let (directory, name) = path.rsplit_once('/').unwrap_or((".", path));
    let directory = if directory.is_empty() { "/" } else { directory };
    format!("{directory}/.easyssh-{name}-{session_id}.tmp")
}

fn remote_child_path(directory: &str, name: &str) -> String {
    if directory == "/" {
        format!("/{name}")
    } else {
        format!("{}/{}", directory.trim_end_matches('/'), name)
    }
}

fn is_previewable_image(name: &str) -> bool {
    Path::new(name)
        .extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| {
            matches!(
                extension.to_ascii_lowercase().as_str(),
                "png" | "jpg" | "jpeg" | "gif" | "bmp" | "webp"
            )
        })
}

fn sync_status_label(status: SyncStatus) -> &'static str {
    match status {
        SyncStatus::Unconfigured => "Sync: not configured",
        SyncStatus::Clean => "Sync: clean",
        SyncStatus::LocalChanges => "Sync: local changes",
        SyncStatus::RemoteUpdates => "Sync: remote updates",
        SyncStatus::Conflict => "Sync: conflict",
        SyncStatus::Failed => "Sync: unavailable",
    }
}

fn transfer_status_badge(status: TransferStatus) -> (&'static str, egui::Color32) {
    match status {
        TransferStatus::Queued => ("Queued", AMBER),
        TransferStatus::Pending => ("Pending", AMBER),
        TransferStatus::Authorizing => ("Authorizing", AMBER),
        TransferStatus::Transferring => ("Transferring", BLUE),
        TransferStatus::Completed => ("Completed", GREEN),
        TransferStatus::Failed => ("Failed", RED),
        TransferStatus::Cancelled => ("Cancelled", AMBER),
        TransferStatus::Interrupted => ("Interrupted", RED),
    }
}

#[allow(dead_code)]
fn apply_theme(ctx: &egui::Context, theme: Theme, density: DisplayDensity) {
    let scale = match density {
        DisplayDensity::Compact => 0.9,
        DisplayDensity::Comfortable => 1.0,
        DisplayDensity::Large => 1.15,
    };
    let mut style = (*ctx.style()).clone();
    style.text_styles.insert(
        egui::TextStyle::Body,
        egui::FontId::proportional(16.0 * scale),
    );
    style.text_styles.insert(
        egui::TextStyle::Button,
        egui::FontId::proportional(16.0 * scale),
    );
    style.text_styles.insert(
        egui::TextStyle::Heading,
        egui::FontId::proportional(22.0 * scale),
    );
    style.text_styles.insert(
        egui::TextStyle::Small,
        egui::FontId::proportional(13.0 * scale),
    );
    style.text_styles.insert(
        egui::TextStyle::Monospace,
        egui::FontId::monospace(15.0 * scale),
    );
    style.spacing.interact_size.y = 32.0 * scale;
    style.spacing.button_padding = egui::vec2(8.0 * scale, 4.0 * scale);
    ctx.set_style(style);
    let dark = match theme {
        Theme::Dark => true,
        Theme::Light => false,
        Theme::System => ctx.system_theme().unwrap_or(egui::Theme::Dark) == egui::Theme::Dark,
    };
    let mut visuals = if dark {
        egui::Visuals::dark()
    } else {
        egui::Visuals::light()
    };
    visuals.panel_fill = if dark {
        egui::Color32::from_rgb(18, 20, 27)
    } else {
        egui::Color32::from_rgb(246, 247, 251)
    };
    visuals.window_fill = if dark {
        egui::Color32::from_rgb(30, 33, 43)
    } else {
        egui::Color32::WHITE
    };
    visuals.extreme_bg_color = visuals.window_fill;
    visuals.faint_bg_color = if dark {
        egui::Color32::from_rgb(25, 28, 38)
    } else {
        egui::Color32::from_rgb(236, 238, 246)
    };
    visuals.selection.bg_fill = BLUE.gamma_multiply(0.6);
    visuals.widgets.hovered.bg_stroke.color = BLUE;
    visuals.widgets.active.bg_stroke.color = BLUE;
    visuals.hyperlink_color = BLUE;
    visuals.window_rounding = egui::Rounding::same(6.0);
    visuals.widgets.inactive.rounding = egui::Rounding::same(4.0);
    visuals.widgets.hovered.rounding = egui::Rounding::same(4.0);
    visuals.widgets.active.rounding = egui::Rounding::same(4.0);
    ctx.set_visuals(visuals);
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn target_includes_endpoint_user_and_port() {
        let mut c = Connection::alias("x", "prod");
        c.target = ConnectionTarget::Endpoint {
            hostname: "host".into(),
            username: Some("ops".into()),
            port: 2200,
        };
        assert_eq!(target_text(&c), "ops@host:2200");
    }
    #[test]
    fn relative_time_is_compact() {
        assert_eq!(relative_time(Utc::now()), "now");
    }
}
