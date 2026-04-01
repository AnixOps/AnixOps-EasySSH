use gtk4::prelude::*;
use libadwaita::prelude::*;
use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::Mutex;
use std::time::{Duration, Instant};
use tokio::runtime::Runtime;
use tokio::sync::mpsc::UnboundedReceiver;

use crate::models::{Server, ServerGroup};
use crate::views::{
    EmptyView, MonitorPanel, MultiSessionTerminal, ServerDetailView, ServerListView,
};

/// Re-export SFTP entry from core for use in views
pub use easyssh_core::sftp::SftpEntry;

pub struct ActiveSession {
    pub session_id: String,
    pub server: Server,
    pub receiver: Option<UnboundedReceiver<String>>,
    pub start_time: Instant,
    pub terminal_content: String,
}

pub struct EasySSHApp {
    window: adw::ApplicationWindow,
    stack: gtk4::Stack,
    view_model: Arc<AppViewModel>,
    monitor_panel: MonitorPanel,
    multi_terminal: MultiSessionTerminal,
    active_sessions: RefCell<HashMap<String, ActiveSession>>,
    current_session_id: RefCell<Option<String>>,
    server_list: ServerListView,
    detail_view: ServerDetailView,
    empty_view: EmptyView,
}

#[derive(Clone)]
pub struct AppViewModel {
    core_state: Arc<Mutex<easyssh_core::AppState>>,
    runtime: Arc<Runtime>,
    ssh_manager: Arc<Mutex<easyssh_core::SshSessionManager>>,
}

impl AppViewModel {
    pub fn new() -> anyhow::Result<Self> {
        let core_state = Arc::new(Mutex::new(easyssh_core::AppState::new()));
        let runtime = Arc::new(
            tokio::runtime::Builder::new_multi_thread()
                .worker_threads(4)
                .thread_name("easyssh-runtime")
                .enable_all()
                .build()
                .map_err(|e| anyhow::anyhow!("Failed to create Tokio runtime: {}", e))?,
        );
        let ssh_manager = Arc::new(Mutex::new(easyssh_core::SshSessionManager::new()));
        Ok(Self {
            core_state,
            runtime,
            ssh_manager,
        })
    }

    pub fn init_database(&self) -> anyhow::Result<()> {
        let state = self.core_state.lock().unwrap();
        easyssh_core::init_database(&state)
            .map_err(|e| anyhow::anyhow!("Database init failed: {}", e))
    }

    pub fn get_servers(&self) -> anyhow::Result<Vec<ServerViewModel>> {
        let state = self.core_state.lock().unwrap();
        easyssh_core::get_servers(&state)
            .map(|servers| servers.into_iter().map(ServerViewModel::from).collect())
            .map_err(|e| anyhow::anyhow!("Failed to get servers: {}", e))
    }

    pub fn get_groups(&self) -> anyhow::Result<Vec<easyssh_core::GroupRecord>> {
        let state = self.core_state.lock().unwrap();
        easyssh_core::get_groups(&state).map_err(|e| anyhow::anyhow!("Failed to get groups: {}", e))
    }

    pub fn add_server(
        &self,
        name: &str,
        host: &str,
        port: i64,
        username: &str,
        auth_type: &str,
    ) -> anyhow::Result<()> {
        let state = self.core_state.lock().unwrap();
        let new_server = easyssh_core::NewServer {
            id: uuid::Uuid::new_v4().to_string(),
            name: name.to_string(),
            host: host.to_string(),
            port,
            username: username.to_string(),
            auth_type: auth_type.to_string(),
            identity_file: None,
            group_id: None,
            status: "active".to_string(),
        };
        easyssh_core::add_server(&state, &new_server)
            .map_err(|e| anyhow::anyhow!("Failed to add server: {}", e))
    }

    pub fn delete_server(&self, server_id: &str) -> anyhow::Result<()> {
        let state = self.core_state.lock().unwrap();
        easyssh_core::delete_server(&state, server_id)
            .map_err(|e| anyhow::anyhow!("Failed to delete server: {}", e))
    }

    pub fn get_saved_password(&self, server_id: &str) -> Option<String> {
        easyssh_core::keychain::get_password(server_id)
            .ok()
            .flatten()
    }

    pub fn save_password(&self, server_id: &str, password: &str) -> anyhow::Result<()> {
        easyssh_core::keychain::store_password(server_id, password)
            .map_err(|e| anyhow::anyhow!("Failed to save password: {}", e))
    }

    pub fn connect(
        &self,
        session_id: &str,
        host: &str,
        port: i64,
        username: &str,
        password: Option<&str>,
    ) -> anyhow::Result<()> {
        let rt = self.runtime.clone();
        let manager = self.ssh_manager.clone();
        let sid = session_id.to_string();
        let h = host.to_string();
        let u = username.to_string();
        let p = password.map(|s| s.to_string());
        rt.block_on(async move {
            let mut mgr = manager.lock().unwrap();
            mgr.connect(&sid, &h, port as u16, &u, p.as_deref())
                .await
                .map_err(|e| anyhow::anyhow!("SSH connection failed: {}", e))
        })
    }

    pub fn execute_stream(
        &self,
        session_id: &str,
        command: &str,
    ) -> anyhow::Result<UnboundedReceiver<String>> {
        let rt = self.runtime.clone();
        let manager = self.ssh_manager.clone();
        let sid = session_id.to_string();
        let cmd = command.to_string();
        rt.block_on(async move {
            let mut mgr = manager.lock().unwrap();
            mgr.execute_stream(&sid, &cmd)
                .await
                .map_err(|e| anyhow::anyhow!("Failed to start stream: {}", e))
        })
    }

    pub fn execute_via_sftp(&self, session_id: &str, command: &str) -> anyhow::Result<String> {
        let rt = self.runtime.clone();
        let manager = self.ssh_manager.clone();
        let sid = session_id.to_string();
        let cmd = command.to_string();
        rt.block_on(async move {
            let mgr = manager.lock().unwrap();
            // Use execute via SFTP through ssh_manager's execute method
            mgr.execute(&sid, &cmd)
                .await
                .map_err(|e| anyhow::anyhow!("SFTP command failed: {}", e))
        })
    }

    /// Write input to shell stdin
    pub fn write_shell_input(&self, session_id: &str, input: &[u8]) -> anyhow::Result<()> {
        let rt = self.runtime.clone();
        let manager = self.ssh_manager.clone();
        let sid = session_id.to_string();
        let data = input.to_vec();
        rt.block_on(async move {
            let mgr = manager.lock().unwrap();
            mgr.write_shell_input(&sid, &data)
                .await
                .map_err(|e| anyhow::anyhow!("Write failed: {}", e))
        })
    }

    /// Interrupt command (send Ctrl+C)
    pub fn interrupt_command(&self, session_id: &str) -> anyhow::Result<()> {
        let rt = self.runtime.clone();
        let manager = self.ssh_manager.clone();
        let sid = session_id.to_string();
        rt.block_on(async move {
            let mgr = manager.lock().unwrap();
            mgr.interrupt_command(&sid)
                .await
                .map_err(|e| anyhow::anyhow!("Interrupt failed: {}", e))
        })
    }

    pub fn disconnect(&self, session_id: &str) -> anyhow::Result<()> {
        let rt = self.runtime.clone();
        let manager = self.ssh_manager.clone();
        let sid = session_id.to_string();
        rt.block_on(async move {
            let mut mgr = manager.lock().unwrap();
            mgr.disconnect(&sid)
                .await
                .map_err(|e| anyhow::anyhow!("Disconnect failed: {}", e))
        })
    }

    pub fn init_sftp(&self, session_id: &str) {
        tracing::info!("SFTP init requested for {}", session_id);
    }

    pub fn is_sftp_initialized(&self, _session_id: &str) -> bool {
        // For now, assume SFTP is available if SSH session exists
        true
    }

    pub fn sftp_list_dir(&self, session_id: &str, path: &str) -> anyhow::Result<Vec<SftpEntry>> {
        let rt = self.runtime.clone();
        let state = self.core_state.clone();
        let sid = session_id.to_string();
        let p = path.to_string();
        rt.block_on(async move {
            let state = state.lock().unwrap();
            let sftp_manager = state.sftp_manager.lock().await;
            sftp_manager
                .list_dir(&sid, &p)
                .await
                .map_err(|e| anyhow::anyhow!("SFTP list failed: {}", e))
        })
    }

    pub fn sftp_mkdir(&self, session_id: &str, path: &str) -> anyhow::Result<()> {
        let rt = self.runtime.clone();
        let state = self.core_state.clone();
        let sid = session_id.to_string();
        let p = path.to_string();
        rt.block_on(async move {
            let state = state.lock().unwrap();
            let sftp_manager = state.sftp_manager.lock().await;
            sftp_manager
                .mkdir(&sid, &p)
                .await
                .map_err(|e| anyhow::anyhow!("SFTP mkdir failed: {}", e))
        })
    }

    pub fn sftp_remove(&self, session_id: &str, path: &str) -> anyhow::Result<()> {
        let rt = self.runtime.clone();
        let state = self.core_state.clone();
        let sid = session_id.to_string();
        let p = path.to_string();
        rt.block_on(async move {
            let state = state.lock().unwrap();
            let sftp_manager = state.sftp_manager.lock().await;
            sftp_manager
                .remove_file(&sid, &p)
                .await
                .map_err(|e| anyhow::anyhow!("SFTP remove failed: {}", e))
        })
    }

    pub fn sftp_rmdir(&self, session_id: &str, path: &str) -> anyhow::Result<()> {
        let rt = self.runtime.clone();
        let state = self.core_state.clone();
        let sid = session_id.to_string();
        let p = path.to_string();
        rt.block_on(async move {
            let state = state.lock().unwrap();
            let sftp_manager = state.sftp_manager.lock().await;
            sftp_manager
                .rmdir(&sid, &p)
                .await
                .map_err(|e| anyhow::anyhow!("SFTP rmdir failed: {}", e))
        })
    }

    pub fn sftp_upload(
        &self,
        session_id: &str,
        remote_path: &str,
        contents: &[u8],
    ) -> anyhow::Result<()> {
        let rt = self.runtime.clone();
        let state = self.core_state.clone();
        let sid = session_id.to_string();
        let p = remote_path.to_string();
        let data = contents.to_vec();
        rt.block_on(async move {
            let state = state.lock().unwrap();
            let sftp_manager = state.sftp_manager.lock().await;
            sftp_manager
                .upload(&sid, &p, &data)
                .await
                .map_err(|e| anyhow::anyhow!("SFTP upload failed: {}", e))
        })
    }

    pub fn sftp_download(&self, session_id: &str, remote_path: &str) -> anyhow::Result<Vec<u8>> {
        let rt = self.runtime.clone();
        let state = self.core_state.clone();
        let sid = session_id.to_string();
        let p = remote_path.to_string();
        rt.block_on(async move {
            let state = state.lock().unwrap();
            let sftp_manager = state.sftp_manager.lock().await;
            sftp_manager
                .download(&sid, &p)
                .await
                .map_err(|e| anyhow::anyhow!("SFTP download failed: {}", e))
        })
    }

    pub fn sftp_close(&self, session_id: &str) -> anyhow::Result<()> {
        let rt = self.runtime.clone();
        let state = self.core_state.clone();
        let sid = session_id.to_string();
        rt.block_on(async move {
            let state = state.lock().unwrap();
            let mut sftp_manager = state.sftp_manager.lock().await;
            sftp_manager
                .close_session(&sid)
                .await
                .map_err(|e| anyhow::anyhow!("SFTP close failed: {}", e))
        })
    }

    pub fn shutdown(&self) {
        tracing::info!("Shutting down AppViewModel...");
        let manager = self.ssh_manager.clone();
        let sessions: Vec<String> = {
            let mgr = manager.lock().unwrap();
            mgr.list_sessions().iter().map(|s| s.to_string()).collect()
        };
        for session_id in sessions {
            let _ = self.disconnect(&session_id);
        }
        tracing::info!("AppViewModel shutdown complete");
    }
}

#[derive(Clone, Debug)]
pub struct ServerViewModel {
    pub id: String,
    pub name: String,
    pub host: String,
    pub port: i64,
    pub username: String,
}

impl From<easyssh_core::ServerRecord> for ServerViewModel {
    fn from(s: easyssh_core::ServerRecord) -> Self {
        Self {
            id: s.id,
            name: s.name,
            host: s.host,
            port: s.port,
            username: s.username,
        }
    }
}

impl EasySSHApp {
    pub fn new(app: &adw::Application) -> Self {
        let view_model = Arc::new(AppViewModel::new().expect("Failed to create AppViewModel"));
        if let Err(e) = view_model.init_database() {
            tracing::error!("Failed to initialize database: {}", e);
        }

        let header = adw::HeaderBar::new();
        let add_button = gtk4::Button::from_icon_name("list-add-symbolic");
        add_button.set_tooltip_text(Some("Add Server"));
        header.pack_start(&add_button);

        // Monitor toggle button
        let monitor_button = gtk4::ToggleButton::from_icon_name("dashboard-symbolic");
        monitor_button.set_tooltip_text(Some("Monitor Panel (Ctrl+M)"));
        header.pack_start(&monitor_button);

        let search_entry = gtk4::SearchEntry::new();
        search_entry.set_placeholder_text(Some("Search servers..."));
        header.set_title_widget(Some(&search_entry));

        let menu_button = gtk4::MenuButton::new();
        menu_button.set_icon_name("open-menu-symbolic");
        header.pack_end(&menu_button);

        let server_list = ServerListView::new();
        let sidebar = gtk4::ScrolledWindow::new();
        sidebar.set_child(Some(&server_list.widget()));
        sidebar.set_width_request(280);

        let stack = gtk4::Stack::new();
        stack.set_transition_type(gtk4::StackTransitionType::Crossfade);

        let empty_view = EmptyView::new();
        stack.add_named(&empty_view.widget(), Some("empty"));

        let detail_view = ServerDetailView::new(view_model.clone());
        stack.add_named(&detail_view.widget(), Some("detail"));

        let multi_terminal = MultiSessionTerminal::new(view_model.clone());
        stack.add_named(&multi_terminal.widget(), Some("terminal"));

        let paned = gtk4::Paned::new(gtk4::Orientation::Horizontal);
        paned.set_start_child(Some(&sidebar));
        paned.set_end_child(Some(&stack));
        paned.set_resize_start_child(false);
        paned.set_shrink_start_child(false);

        let toolbar_view = adw::ToolbarView::new();
        toolbar_view.add_top_bar(&header);
        toolbar_view.set_content(Some(&paned));

        let breakpoint = adw::Breakpoint::new(adw::BreakpointCondition::new_length(
            adw::BreakpointConditionLengthType::MaxWidth,
            600.0,
            adw::LengthUnit::Px,
        ));
        breakpoint.add_setter(&paned, "collapsed", &true.to_value());

        let window = adw::ApplicationWindow::builder()
            .application(app)
            .title("EasySSH")
            .default_width(1400)
            .default_height(900)
            .content(&toolbar_view)
            .build();

        // Create monitor panel and add to stack
        let monitor_panel = MonitorPanel::new(view_model.clone());
        stack.add_named(&monitor_panel.widget(), Some("monitor"));

        // Setup periodic monitor refresh
        glib::timeout_add_local(
            Duration::from_millis(500),
            glib::clone!(@weak monitor_panel => move || {
                monitor_panel.poll_result();
                glib::ControlFlow::Continue
            }),
        );

        let easy_app = Self {
            window,
            stack,
            view_model: view_model.clone(),
            monitor_panel,
            multi_terminal,
            active_sessions: RefCell::new(HashMap::new()),
            current_session_id: RefCell::new(None),
            server_list,
            detail_view,
            empty_view,
        };

        easy_app.setup_signals(&add_button, &menu_button, &search_entry, &monitor_button);
        easy_app.setup_multi_terminal_signals();
        easy_app.setup_keyboard_shortcuts();
        easy_app.load_servers();

        easy_app
    }

    fn setup_signals(
        &self,
        add_button: &gtk4::Button,
        _menu_button: &gtk4::MenuButton,
        search_entry: &gtk4::SearchEntry,
        monitor_button: &gtk4::ToggleButton,
    ) {
        add_button.connect_clicked(glib::clone!(@weak self as app => move |_| {
            tracing::info!("Add server clicked");
        }));

        search_entry.connect_search_changed(glib::clone!(@weak self as app => move |entry| {
            let text = entry.text().to_string();
            app.filter_servers(&text);
        }));

        self.server_list.connect_selection_changed(
            glib::clone!(@weak self as app => move |server| {
                app.detail_view.set_server(server.clone());
                app.stack.set_visible_child_name("detail");
                // Uncheck monitor button when switching to detail
                monitor_button.set_active(false);
            }),
        );

        // Monitor button toggle
        monitor_button.connect_toggled(
            glib::clone!(@weak self as app, @weak monitor_button as btn => move |_| {
                if btn.is_active() {
                    app.stack.set_visible_child_name("monitor");
                    app.monitor_panel.set_session_id(app.current_session_id.borrow().clone());
                    app.monitor_panel.refresh();
                } else {
                    app.stack.set_visible_child_name("detail");
                }
            }),
        );
    }

    fn setup_multi_terminal_signals(&self) {
        self.multi_terminal.connect_tab_selected(
            glib::clone!(@weak self as app => move |session_id| {
                app.switch_to_session(session_id);
            }),
        );

        self.multi_terminal.connect_tab_closed(
            glib::clone!(@weak self as app => move |session_id| {
                app.close_session(session_id);
            }),
        );

        self.multi_terminal
            .connect_new_tab(glib::clone!(@weak self as app => move || {
                tracing::info!("New tab requested");
            }));
    }

    fn setup_keyboard_shortcuts(&self) {
        let controller = gtk4::EventControllerKey::new();
        controller.connect_key_pressed(glib::clone!(@weak self as app => move |_, key, _, modifier| {
            if modifier.contains(gtk4::gdk::ModifierType::CONTROL_MASK) {
                match key {
                    gtk4::gdk::Key::_1 => { app.switch_to_tab_index(0); glib::Propagation::Stop }
                    gtk4::gdk::Key::_2 => { app.switch_to_tab_index(1); glib::Propagation::Stop }
                    gtk4::gdk::Key::_3 => { app.switch_to_tab_index(2); glib::Propagation::Stop }
                    gtk4::gdk::Key::_4 => { app.switch_to_tab_index(3); glib::Propagation::Stop }
                    gtk4::gdk::Key::_5 => { app.switch_to_tab_index(4); glib::Propagation::Stop }
                    gtk4::gdk::Key::_6 => { app.switch_to_tab_index(5); glib::Propagation::Stop }
                    gtk4::gdk::Key::_7 => { app.switch_to_tab_index(6); glib::Propagation::Stop }
                    gtk4::gdk::Key::_8 => { app.switch_to_tab_index(7); glib::Propagation::Stop }
                    gtk4::gdk::Key::_9 => { app.switch_to_tab_index(8); glib::Propagation::Stop }
                    gtk4::gdk::Key::T | gtk4::gdk::Key::t => { glib::Propagation::Stop }
                    gtk4::gdk::Key::W | gtk4::gdk::Key::w => {
                        if let Some(session_id) = app.multi_terminal.active_session() {
                            app.close_session(&session_id);
                        }
                        glib::Propagation::Stop
                    }
                    _ => glib::Propagation::Proceed,
                }
            } else {
                glib::Propagation::Proceed
            }
        }));
        self.window.add_controller(controller);
    }

    fn switch_to_tab_index(&self, index: usize) {
        if let Some(session_id) = self.multi_terminal.switch_to_tab_index(index) {
            self.switch_to_session(&session_id);
        }
    }

    fn switch_to_session(&self, session_id: &str) {
        self.multi_terminal.switch_to_session(session_id);
        self.current_session_id
            .replace(Some(session_id.to_string()));
        // Update monitor panel session
        self.monitor_panel
            .set_session_id(Some(session_id.to_string()));
    }

    fn close_session(&self, session_id: &str) {
        let _ = self.view_model.disconnect(session_id);
        self.active_sessions.borrow_mut().remove(session_id);
        self.multi_terminal.close_session(session_id);
        if self.current_session_id.borrow().as_deref() == Some(session_id) {
            self.current_session_id.replace(None);
            // Clear monitor panel session
            self.monitor_panel.set_session_id(None);
        }
        if self.active_sessions.borrow().is_empty() {
            self.stack.set_visible_child_name("empty");
        }
    }

    pub fn create_session(
        &self,
        server: Server,
        password: Option<String>,
    ) -> anyhow::Result<String> {
        let session_id = uuid::Uuid::new_v4().to_string();
        self.view_model.connect(
            &session_id,
            &server.host,
            server.port,
            &server.username,
            password.as_deref(),
        )?;
        let receiver = self.view_model.execute_stream(&session_id, "")?;

        let active_session = ActiveSession {
            session_id: session_id.clone(),
            server: server.clone(),
            receiver: Some(receiver),
            start_time: Instant::now(),
            terminal_content: format!(
                "Connected to {} ({}@{}:{})\n\n",
                server.name, server.username, server.host, server.port
            ),
        };

        self.active_sessions
            .borrow_mut()
            .insert(session_id.clone(), active_session);
        let _ = self.multi_terminal.create_session(server.clone())?;
        self.stack.set_visible_child_name("terminal");
        self.current_session_id.replace(Some(session_id.clone()));
        // Set monitor panel session
        self.monitor_panel.set_session_id(Some(session_id.clone()));
        self.start_session_polling(&session_id);
        Ok(session_id)
    }

    fn start_session_polling(&self, session_id: &str) {
        let session_id = session_id.to_string();
        let sessions = self.active_sessions.clone();
        glib::timeout_add_local(std::time::Duration::from_millis(16), move || {
            let mut sessions_ref = sessions.borrow_mut();
            if let Some(session) = sessions_ref.get_mut(&session_id) {
                if let Some(ref mut receiver) = session.receiver {
                    while let Ok(chunk) = receiver.try_recv() {
                        session.terminal_content.push_str(&chunk);
                    }
                    glib::ControlFlow::Continue
                } else {
                    glib::ControlFlow::Break
                }
            } else {
                glib::ControlFlow::Break
            }
        });
    }

    fn load_servers(&self) {
        match self.view_model.get_servers() {
            Ok(servers) => {
                let servers: Vec<Server> = servers.into_iter().map(|s| s.into()).collect();
                self.server_list.set_servers(servers);
            }
            Err(e) => tracing::error!("Failed to load servers: {}", e),
        }
        match self.view_model.get_groups() {
            Ok(groups) => {
                let groups: Vec<ServerGroup> = groups.into_iter().map(|g| g.into()).collect();
                self.server_list.set_groups(groups);
            }
            Err(e) => tracing::error!("Failed to load groups: {}", e),
        }
    }

    fn filter_servers(&self, query: &str) {
        tracing::info!("Filtering servers with query: {}", query);
    }

    pub fn present(&self) {
        self.window.present();
    }

    pub fn shutdown(&self) {
        tracing::info!("Shutting down EasySSH...");
        let sessions_to_close: Vec<String> =
            self.active_sessions.borrow().keys().cloned().collect();
        for session_id in sessions_to_close {
            self.close_session(&session_id);
        }
        self.view_model.shutdown();
        tracing::info!("Shutdown complete");
    }
}

use gtk4::glib;
