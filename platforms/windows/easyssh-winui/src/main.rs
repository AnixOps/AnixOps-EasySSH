#![allow(dead_code)]

use eframe::egui;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::mpsc as std_mpsc;
use std::time::Instant;
use tokio::sync::mpsc;
use tracing::{error, info};
use uuid::Uuid;

#[cfg(feature = "code-editor")]
mod code_editor;
#[cfg(feature = "code-editor")]
use code_editor::{CodeEditor, FileInfo};

mod bridge;
mod viewmodels;
mod ws_server;
mod design;
mod apple_design;
mod search;
mod snippets;
mod snippets_ui;
mod layout_manager;
mod split_layout;
mod terminal;
mod file_icons;
mod transfer_queue;
mod file_preview;
mod sftp_file_manager;
mod notifications;
mod notification_panel;
mod hotkeys;
mod performance;
mod performance_panel;
mod hotkey_helpers;
mod settings;
mod app_settings;
mod theme_system;
mod port_forward_dialog;
mod pages;
mod startup;
mod user_experience;

use startup::{global_profiler, DeferredInitQueue};

#[cfg(feature = "remote-desktop")]
mod remote_desktop_ui;
#[cfg(feature = "remote-desktop")]
mod embedded_rdp;

#[cfg(feature = "workflow")]
mod workflow_editor;
#[cfg(feature = "workflow")]
mod macro_recorder_ui;
#[cfg(feature = "workflow")]
mod scheduled_tasks_ui;
#[cfg(feature = "workflow")]
mod batch_results_ui;
#[cfg(feature = "workflow")]
mod workflow_panel;

#[cfg(feature = "ai-terminal")]
mod ai_terminal;
#[cfg(feature = "ai-terminal")]
mod ai_terminal_ui;


#[cfg(feature = "ai-terminal")]
use ai_terminal_ui::AiTerminalUi;

use design::{DesignTheme, AccessibilitySettings};
use apple_design::MicrointeractionState;
use viewmodels::{AppViewModel, GroupViewModel, ServerViewModel};
use ws_server::{update_ui_debug, WsControlServer};
use search::{GlobalSearchEngine, FilterCriteria, ConnectionStatusFilter, QuickAction};
use snippets::{SnippetCategory, SnippetInputDialog, SnippetManager};
use layout_manager::SplitLayoutManager;
use terminal::{WebGlTerminalManager, RenderStats};
use notifications::NotificationManager;
use notification_panel::{NotificationPanel, NotificationSettingsPanel};
use split_layout::{PanelId, PanelType, DropTarget};
use hotkeys::{HotkeyManager, CommandPalette, HotkeySettingsUI};
use settings::SettingsPanel;
use performance_panel::PerformancePanel;

#[cfg(feature = "remote-desktop")]
use remote_desktop_ui::{RemoteDesktopManagerUI, render_remote_desktop_panel};
#[cfg(feature = "remote-desktop")]
use embedded_rdp::RemoteDesktopViewerManager;

use port_forward_dialog::PortForwardDialog;
use theme_system::{ThemeManager, ThemeGallery, ThemeEditor};
use user_experience::{UXManager, ToastNotification, LoadingOperation, OnboardingWizard, OnboardingAction, QuickTip};
use app_settings::SettingsManager;


fn main() -> eframe::Result {
    // Start global startup profiler
    let profiler = global_profiler();
    let _main_phase = profiler.lock().unwrap().start_phase("total");
    let _early_phase = profiler.lock().unwrap().start_phase("early_init");

    tracing_subscriber::fmt::init();
    info!("Starting EasySSH for Windows");

    // Initialize accessibility settings from system preferences (WCAG 2.1 AA)
    let a11y = AccessibilitySettings::global();
    a11y.detect_system_settings();
    info!(
        "Accessibility settings: high_contrast={}, reduced_motion={}, large_text={}, rtl={}",
        a11y.is_high_contrast(),
        a11y.is_reduced_motion(),
        a11y.is_large_text(),
        a11y.is_rtl()
    );

    profiler.lock().unwrap().end_phase("early_init");

    // Create shared Tokio runtime for the entire application
    let _rt_phase = profiler.lock().unwrap().start_phase("tokio_runtime");
    let rt = Arc::new(tokio::runtime::Runtime::new().expect("Failed to create Tokio runtime"));
    profiler.lock().unwrap().end_phase("tokio_runtime");

    // Initialize AppViewModel (includes database) with profiling
    let _vm_phase = profiler.lock().unwrap().start_phase("viewmodel_init");
    let ws_view_model = Arc::new(Mutex::new(
        AppViewModel::new_with_runtime(rt.clone()).expect("Failed to init")
    ));
    profiler.lock().unwrap().end_phase("viewmodel_init");
    profiler.lock().unwrap().mark_db_initialized();

    // Start WebSocket debug API server (non-blocking)
    let ws_vm_clone = ws_view_model.clone();
    let rt_clone = rt.clone();
    std::thread::spawn(move || {
        rt_clone.block_on(async move {
            let ws_server = WsControlServer::new(8765, ws_vm_clone);
            if let Err(e) = ws_server.start().await {
                error!("WebSocket server error: {}", e);
            }
        });
    });

    info!("WebSocket API available at ws://localhost:8765");

    // Setup deferred initialization for non-critical components
    let mut deferred = DeferredInitQueue::new();

    // Deferred: Pre-load themes and search index
    deferred.add(|| {
        tracing::debug!("Deferred: Pre-loading themes...");
    });

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1000.0, 700.0])
            .with_min_inner_size([600.0, 400.0]),
        ..Default::default()
    };

    // Run deferred tasks after UI is shown
    deferred.run_all();

    eframe::run_native(
        "EasySSH",
        options,
        Box::new(move |cc| {
            let _ui_phase = global_profiler().lock().unwrap().start_phase("ui_init");
            let app = EasySSHApp::new(cc, rt.clone());
            global_profiler().lock().unwrap().end_phase("ui_init");
            global_profiler().lock().unwrap().mark_ui_ready();

            // Log startup report
            let report = global_profiler().lock().unwrap().generate_report();
            info!("{}", report.format());

            Ok(Box::new(app))
        }),
    )
}

#[derive(Clone)]
struct SessionTab {
    session_id: String,
    #[allow(dead_code)]
    server_id: String,
    title: String,
    output: String,
    #[allow(dead_code)]
    input: String,
    #[allow(dead_code)]
    connected: bool,
}

#[derive(Clone)]
struct MonitorSnapshot {
    cpu: f32,
    memory: f32,
    disk: f32,
    uptime: String,
    net_in: String,
    net_out: String,
    load: String,
    net_prev: Option<(u64, u64, std::time::Instant)>,
    refreshed_at: std::time::Instant,
    has_errors: bool,
}

struct MonitorRefreshResult {
    request_seq: u64,
    session_id: String,
    snapshot: MonitorSnapshot,
}

struct EasySSHApp {
    view_model: Arc<Mutex<AppViewModel>>,
    servers: Vec<ServerViewModel>,
    groups: Vec<GroupViewModel>,
    selected_server: Option<String>,
    search_query: String,
    show_add_dialog: bool,
    new_server: NewServerForm,
    add_error: Option<String>,
    show_edit_dialog: bool,
    edit_server: EditServerForm,
    edit_error: Option<String>,
    editing_server_id: Option<String>,
    show_connect_dialog: bool,
    connect_server: Option<ServerViewModel>,
    password: String,
    save_password: bool,
    use_saved_password: bool,
    connect_status: ConnectStatus,
    connect_error: Option<String>,
    current_session_id: Option<String>,
    terminal_output: String,
    command_input: String,
    is_terminal_active: bool,
    command_receiver: Option<mpsc::UnboundedReceiver<String>>,

    // Server monitoring
    show_monitor: bool,
    monitor_cpu: f32,
    monitor_memory: f32,
    monitor_disk: f32,
    monitor_uptime: String,
    monitor_net_in: String,
    monitor_net_out: String,
    monitor_load: String,
    monitor_available: bool,
    monitor_net_prev: Option<(u64, u64, std::time::Instant)>,
    monitor_refreshing: bool,
    monitor_request_seq: u64,
    monitor_result_rx: Option<std_mpsc::Receiver<MonitorRefreshResult>>,
    command_running: bool,
    last_monitor_refresh: Option<std::time::Instant>,

    // Auto-reconnect
    auto_reconnect: bool,

    // Session timing
    session_start_time: Option<std::time::Instant>,

    // Termius-like features
    favorites: HashSet<String>,
    tags: HashMap<String, Vec<String>>,
    tag_input: String,
    session_tabs: Vec<SessionTab>,
    active_tab: Option<String>,

    // Server groups (group_name -> server_ids)
    #[allow(dead_code)]
    server_groups: HashMap<String, Vec<String>>,
    selected_group: Option<String>,

    // Command history
    command_history: Vec<String>,
    history_index: Option<usize>,

    // File browser
    show_file_browser: bool,
    file_current_path: String,
    file_entries: Vec<FileEntry>,
    selected_file: Option<String>,
    file_error: Option<String>,
    show_new_folder_dialog: bool,
    new_folder_name: String,
    show_rename_dialog: bool,
    rename_target: Option<String>,
    rename_new_name: String,
    last_file_refresh: Option<std::time::Instant>,

    // File editing
    editing_file: Option<String>,      // Remote path being edited
    temp_file_path: Option<String>,     // Local temp file path

    // Global search system
    search_engine: GlobalSearchEngine,
    show_global_search: bool,
    global_search_query: String,
    global_search_results: Vec<search::SearchResult>,
    global_search_selected: Option<usize>,
    search_filter: FilterCriteria,
    show_search_filters: bool,

    // Snippets system
    snippet_manager: SnippetManager,
    show_snippets_panel: bool,
    snippet_search: String,
    selected_snippet_category: Option<SnippetCategory>,
    snippet_input_dialog: SnippetInputDialog,
    show_add_snippet_dialog: bool,
    new_snippet_form: NewSnippetForm,
    // Split layout system
    split_layout_manager: SplitLayoutManager,
    panel_states: HashMap<PanelId, PanelState>,
    show_layout_menu: bool,
    drag_drop_target: Option<(PanelId, DropTarget)>,
    snippet_export_path: Option<String>,
    snippet_import_path: Option<String>,
    snippet_action_message: Option<(String, std::time::Instant)>, // (message, timestamp)

    // === High-Performance WebGL Terminal (60fps) ===
    /// WebGL terminal manager for 60fps rendering
    terminal_manager: Option<WebGlTerminalManager>,
    /// Use WebGL terminal vs legacy text mode
    use_webgl_terminal: bool,
    /// Terminal render stats (FPS, frame time)
    terminal_stats: RenderStats,
    /// Last stats update time
    last_terminal_stats_update: Instant,

    // === Interactive Terminal Input ===
    /// Terminal has focus for direct keyboard input
    terminal_has_focus: bool,
    /// Input buffer for character composition (for IME/Chinese input)
    terminal_input_buffer: String,

    // Apple Design System
    theme: DesignTheme,
    interaction_states: HashMap<String, MicrointeractionState>,

    // Notification system
    notification_manager: Arc<NotificationManager>,
    notification_panel: NotificationPanel,
    notification_settings_panel: NotificationSettingsPanel,
    show_notification_panel: bool,
    show_notification_settings: bool,
n    // Performance monitoring panel
    performance_panel: PerformancePanel,
    show_performance_panel: bool,

    // === Professional Hotkey System ===
    /// Global and app hotkey manager
    hotkey_manager: Arc<Mutex<HotkeyManager>>,
    /// Command palette (VS Code style)
    command_palette: CommandPalette,
    /// Hotkey settings UI
    hotkey_settings: HotkeySettingsUI,
    /// Show hotkey settings
    show_hotkey_settings: bool,
    /// Keyboard shortcut cheatsheet
    shortcut_cheatsheet: hotkeys::ShortcutCheatsheet,
    /// Show shortcut cheatsheet
    show_shortcut_cheatsheet: bool,
    /// Current terminal zoom level
    terminal_font_zoom: f32,
    /// Fullscreen mode
    is_fullscreen: bool,

    // Settings panel with import/export
    settings_panel: SettingsPanel,
    // Settings manager for persistence
    settings_manager: Arc<app_settings::SettingsManager>,

    // === Port Forwarding Manager ===
    /// Port forwarding dialog for managing SSH tunnels
    port_forward_dialog: PortForwardDialog,

    // === Enterprise Password Vault ===
    // Enterprise password vault UI
    enterprise_vault: pages::enterprise_vault_ui::EnterpriseVaultWindow,

    // === Remote Desktop Integration ===
    /// Remote desktop connection manager
    #[cfg(feature = "remote-desktop")]
    remote_desktop_manager: RemoteDesktopManagerUI,
    /// Show remote desktop panel
    #[cfg(feature = "remote-desktop")]
    show_remote_desktop: bool,
    /// Active RDP/VNC viewers
    #[cfg(feature = "remote-desktop")]
    rdp_viewer_manager: Option<RemoteDesktopViewerManager>,
    /// Currently selected remote desktop session
    #[cfg(feature = "remote-desktop")]
    active_remote_session: Option<String>,
    /// Toggle for clipboard synchronization
    #[cfg(feature = "remote-desktop")]
    clipboard_sync_enabled: bool,
    /// Drag and drop file transfer enabled
    #[cfg(feature = "remote-desktop")]
    drag_drop_enabled: bool,

    // === Professional Theme System ===
    // Theme manager with all themes
    theme_manager: ThemeManager,
    // Theme gallery browser
    theme_gallery: ThemeGallery,
    // Theme editor for custom themes
    theme_editor: ThemeEditor,
    // Track last applied theme ID for theme change detection
    last_applied_theme_id: String,

    // === AI Terminal Assistant ===
    /// AI terminal assistant for intelligent command help
    #[cfg(feature = "ai-terminal")]
    ai_terminal: Option<AiTerminalUi>,
    /// Show AI assistant panel
    #[cfg(feature = "ai-terminal")]
    show_ai_assistant: bool,
    /// AI assistant toggle state
    #[cfg(feature = "ai-terminal")]
    ai_assistant_enabled: bool,
    // Note: runtime is declared below in the Shared Runtime section

    // === Professional Code Editor ===
    /// Built-in code editor for editing remote and local files
    #[cfg(feature = "code-editor")]
    code_editor: CodeEditor,
    /// Show code editor panel
    #[cfg(feature = "code-editor")]
    show_code_editor: bool,
    /// Currently editing file info
    #[cfg(feature = "code-editor")]
    current_editing_file: Option<FileInfo>,

    // === Shared Runtime (startup optimization) ===
    /// Shared Tokio runtime for all async operations
    runtime: Arc<tokio::runtime::Runtime>,

    // === User Experience Enhancements ===
    /// UX manager for loading states, errors, onboarding
    ux_manager: UXManager,
    /// Onboarding wizard for first-time users
    onboarding_wizard: OnboardingWizard,
    /// Show onboarding dialog
    show_onboarding: bool,
    /// Show quick tips
    show_quick_tips: bool,
    /// Quick tip queue
    quick_tip_queue: Vec<QuickTip>,

    // === Server Deletion Confirmation ===
    /// Show delete confirmation dialog
    show_delete_confirm: bool,
    /// Server ID pending deletion
    pending_delete_server_id: Option<String>,
    /// Server name pending deletion (for display)
    pending_delete_server_name: Option<String>,
}


#[derive(Default)]
struct PanelState {
    // Terminal state
    terminal_output: String,
    command_input: String,
    command_history: Vec<String>,
    history_index: Option<usize>,

    // SFTP state
    file_current_path: String,
    file_entries: Vec<FileEntry>,
    selected_file: Option<String>,
    file_error: Option<String>,
    last_file_refresh: Option<std::time::Instant>,

    // Monitor state
    monitor_cpu: f32,
    monitor_memory: f32,
    monitor_disk: f32,
    monitor_uptime: String,
    monitor_net_in: String,
    monitor_net_out: String,
    monitor_load: String,
    monitor_available: bool,
    monitor_net_prev: Option<(u64, u64, std::time::Instant)>,
    monitor_refreshing: bool,
    monitor_request_seq: u64,
    last_monitor_refresh: Option<std::time::Instant>,

    // Panel metadata
    session_id: Option<String>,
    server_id: Option<String>,
}
#[derive(Clone, Debug)]
struct FileEntry {
    name: String,
    path: String,
    is_dir: bool,
    size: String,
    mtime: String,
}

#[derive(Default)]
struct NewServerForm {
    name: String,
    host: String,
    port: String,
    username: String,
    auth_type: AuthType,
    group_id: Option<String>,
}

#[derive(Default)]
struct EditServerForm {
    name: String,
    host: String,
    port: String,
    username: String,
    auth_type: AuthType,
    group_id: Option<String>,
}

#[derive(Default, PartialEq)]
enum AuthType {
    #[default]
    Password,
    Key,
}

#[derive(Default, PartialEq)]
enum ConnectStatus {
    #[default]
    Idle,
    Connecting,
    #[allow(dead_code)]
    Connected,
    Error,
}

#[derive(Default)]
struct NewSnippetForm {
    name: String,
    content: String,
    description: String,
    category: SnippetCategory,
    tags: String,
}

impl EasySSHApp {
    /// Maximum terminal output size to prevent memory issues
    const MAX_TERMINAL_CHARS: usize = 100_000;

    fn truncate_terminal(&mut self) {
        if self.terminal_output.len() > Self::MAX_TERMINAL_CHARS {
            let truncate_pos = self.terminal_output.len() - Self::MAX_TERMINAL_CHARS;
            if let Some(pos) = self.terminal_output[..truncate_pos].find('\n') {
                self.terminal_output = format!("[...truncated {} bytes...]\n{}",
                    truncate_pos - pos - 1,
                    &self.terminal_output[pos + 1..]);
            } else {
                self.terminal_output = self.terminal_output[truncate_pos..].to_string();
            }
        }
    }

    fn new(cc: &eframe::CreationContext<'_>, runtime: Arc<tokio::runtime::Runtime>) -> Self {
        // Initialize accessible theme with system accessibility settings (fast path)
        let theme_phase = global_profiler().lock().unwrap().start_phase("theme_init");
        let mut theme = if AccessibilitySettings::global().is_high_contrast() {
            DesignTheme::high_contrast()
        } else {
            DesignTheme::dark()
        };
        theme.apply_accessibility_settings();
        theme.apply_to_ctx(&cc.egui_ctx);
        global_profiler().lock().unwrap().end_phase(&theme_phase);

        // Load settings from config file
        let settings_phase = global_profiler().lock().unwrap().start_phase("settings_load");

        // Initialize settings manager (this loads from disk and applies language)
        let settings_manager = Arc::new(SettingsManager::new());

        let mut settings_panel = SettingsPanel::default();
        // Initialize settings panel with the settings manager (syncs UI state)
        settings_panel.initialize(settings_manager.clone());

        let (loaded_theme, loaded_accessibility) = Self::load_ui_settings();

        // Apply loaded settings
        if let Some(theme_mode) = loaded_theme {
            settings_panel.ui_theme_mode = theme_mode;
            // Apply the loaded theme immediately
            let mut new_theme = match theme_mode {
                crate::settings::UiThemeMode::Light => DesignTheme::light(),
                crate::settings::UiThemeMode::Dark => DesignTheme::dark(),
                crate::settings::UiThemeMode::System => {
                    // For system mode, check current egui visuals
                    if cc.egui_ctx.style().visuals.dark_mode {
                        DesignTheme::dark()
                    } else {
                        DesignTheme::light()
                    }
                }
            };
            new_theme.apply_accessibility_settings();
            new_theme.apply_to_ctx(&cc.egui_ctx);
            theme = new_theme;
            info!("Loaded and applied UI theme mode from config: {:?}", theme_mode);
        }

        if let Some((high_contrast, reduced_motion, large_text)) = loaded_accessibility {
            settings_panel.high_contrast = high_contrast;
            settings_panel.reduce_motion = reduced_motion;
            settings_panel.large_text = large_text;

            // Update global accessibility settings
            let settings = AccessibilitySettings::global();
            settings.high_contrast.store(high_contrast, std::sync::atomic::Ordering::Relaxed);
            settings.reduced_motion.store(reduced_motion, std::sync::atomic::Ordering::Relaxed);
            settings.large_text.store(large_text, std::sync::atomic::Ordering::Relaxed);

            // Re-apply theme with accessibility settings
            let mut updated_theme = theme.clone();
            updated_theme.apply_accessibility_settings();
            updated_theme.apply_to_ctx(&cc.egui_ctx);
            theme = updated_theme;
            info!("Loaded and applied accessibility settings from config");
        }
        global_profiler().lock().unwrap().end_phase(&settings_phase);

        // Critical: Get view model from main() instead of creating new one
        // This is passed via the eframe::run_native closure
        // For now, we create it here but reuse the database connection
        let vm_phase = global_profiler().lock().unwrap().start_phase("vm_load");
        let view_model = Arc::new(Mutex::new(
            AppViewModel::new_with_runtime(runtime.clone()).expect("Failed to init")
        ));
        let servers = view_model.lock().unwrap().get_servers();
        let groups = view_model.lock().unwrap().get_groups();
        global_profiler().lock().unwrap().end_phase(&vm_phase);

        // Initialize terminal manager lazily (don't create WebView yet)
        let terminal_phase = global_profiler().lock().unwrap().start_phase("terminal_init");
        // WebGL terminal is expensive - delay actual WebView creation
        let terminal_manager = None; // Will be created on first terminal use
        global_profiler().lock().unwrap().end_phase(&terminal_phase);

        // Initialize notification manager (shared instance)
        let notify_phase = global_profiler().lock().unwrap().start_phase("notify_init");
        let notification_manager = Arc::new(NotificationManager::new("EasySSH"));
        let notification_panel = NotificationPanel::new(notification_manager.clone());
        let notification_settings_panel = NotificationSettingsPanel::new(notification_manager.clone());
        global_profiler().lock().unwrap().end_phase(&notify_phase);

        // Clone theme for enterprise_vault (before theme is moved into Self)
        let theme_for_vault = theme.clone();

        // Initialize hotkey manager and load saved config
        let hotkey_manager = {
            let mut manager = HotkeyManager::new();
            if let Err(e) = manager.load_from_file() {
                info!("Could not load hotkey config: {}. Using defaults.", e);
            }
            Arc::new(Mutex::new(manager))
        };

        // Clone for callback setup
        let hotkey_manager_for_callback = hotkey_manager.clone();

        Self {
            view_model,
            servers,
            groups,
            selected_server: None,
            search_query: String::new(),
            show_add_dialog: false,
            new_server: NewServerForm::default(),
            add_error: None,
            show_connect_dialog: false,
            connect_server: None,
            password: String::new(),
            save_password: false,
            use_saved_password: false,
            connect_status: ConnectStatus::Idle,
            connect_error: None,
            current_session_id: None,
            terminal_output: String::new(),
            command_input: String::new(),
            is_terminal_active: false,
            command_receiver: None,
            command_running: false,
            auto_reconnect: false,
            session_start_time: None,
            favorites: HashSet::new(),
            tags: HashMap::new(),
            tag_input: String::new(),
            session_tabs: Vec::new(),
            active_tab: None,
            server_groups: HashMap::new(),
            selected_group: None,
            command_history: Vec::new(),
            history_index: None,
            show_file_browser: false,
            file_current_path: String::from("/"),
            file_entries: Vec::new(),
            selected_file: None,
            file_error: None,
            show_new_folder_dialog: false,
            new_folder_name: String::new(),
            show_rename_dialog: false,
            rename_target: None,
            rename_new_name: String::new(),
            last_file_refresh: None,
            editing_file: None,
            temp_file_path: None,
            show_monitor: false,
            monitor_cpu: 0.0,
            monitor_memory: 0.0,
            monitor_disk: 0.0,
            monitor_uptime: String::from("-"),
            monitor_net_in: String::from("-"),
            monitor_net_out: String::from("-"),
            monitor_load: String::from("-"),
            monitor_available: false,
            monitor_net_prev: None,
            monitor_refreshing: false,
            monitor_request_seq: 0,
            monitor_result_rx: None,
            last_monitor_refresh: None,
            // Global search (lightweight)
            search_engine: GlobalSearchEngine::new(),
            show_global_search: false,
            global_search_query: String::new(),
            global_search_results: Vec::new(),
            global_search_selected: None,
            search_filter: FilterCriteria::default(),
            show_search_filters: false,
            // Snippets system (lightweight)
            snippet_manager: SnippetManager::new(),
            show_snippets_panel: true,
            snippet_search: String::new(),
            selected_snippet_category: None,
            snippet_input_dialog: SnippetInputDialog::default(),
            show_add_snippet_dialog: false,
            new_snippet_form: NewSnippetForm::default(),
            snippet_export_path: None,
            snippet_import_path: None,
            snippet_action_message: None,
            // Split layout system
            split_layout_manager: SplitLayoutManager::default(),
            panel_states: HashMap::new(),
            show_layout_menu: false,
            drag_drop_target: None,
            // WebGL Terminal - lazily initialized
            terminal_manager,
            use_webgl_terminal: true,
            terminal_stats: RenderStats::default(),
            last_terminal_stats_update: Instant::now(),
            // Interactive Terminal Input
            terminal_has_focus: false,
            terminal_input_buffer: String::new(),

            // Apple Design System
            theme,
            interaction_states: HashMap::new(),

            // Notification system (shared instance)
            notification_manager,
            notification_panel,
            notification_settings_panel,
            show_notification_panel: false,
            show_notification_settings: false,

            // Hotkey system (lightweight) - using pre-initialized manager
            hotkey_manager,
            command_palette: CommandPalette::new(),
            hotkey_settings: {
                let mut settings = HotkeySettingsUI::new();
                // Set up auto-save callback
                let manager_arc = hotkey_manager_for_callback.clone();
                settings.set_save_callback(move || {
                    if let Ok(manager) = manager_arc.lock() {
                        if let Err(e) = manager.save_to_file() {
                            error!("Failed to save hotkey config: {}", e);
                        } else {
                            info!("Hotkey configuration auto-saved");
                        }
                    }
                });
                settings
            },
            show_hotkey_settings: false,
            shortcut_cheatsheet: hotkeys::ShortcutCheatsheet::new(),
            show_shortcut_cheatsheet: false,
            terminal_font_zoom: 1.0,
            is_fullscreen: false,

            // Settings panel
            settings_panel,
            // Settings manager for persistence
            settings_manager,

            // Port Forwarding Manager
            port_forward_dialog: PortForwardDialog::new(),

            // Enterprise Password Vault (may be expensive - consider deferring)
            enterprise_vault: pages::enterprise_vault_ui::EnterpriseVaultWindow::new(theme_for_vault),

            // Remote Desktop Integration
            #[cfg(feature = "remote-desktop")]
            remote_desktop_manager: RemoteDesktopManagerUI::new(),
            #[cfg(feature = "remote-desktop")]
            show_remote_desktop: false,
            #[cfg(feature = "remote-desktop")]
            rdp_viewer_manager: None,
            #[cfg(feature = "remote-desktop")]
            active_remote_session: None,
            #[cfg(feature = "remote-desktop")]
            clipboard_sync_enabled: true,
            #[cfg(feature = "remote-desktop")]
            drag_drop_enabled: true,

            // Theme system (lightweight)
            theme_manager: ThemeManager::new(),
            theme_gallery: ThemeGallery::default(),
            theme_editor: ThemeEditor::default(),
            // Initialize with current theme ID to avoid false change detection on startup
            last_applied_theme_id: ThemeManager::new().current_theme.id.clone(),

            // AI Terminal Assistant - uses shared runtime, no extra runtime creation
            #[cfg(feature = "ai-terminal")]
            ai_terminal: None,
            #[cfg(feature = "ai-terminal")]
            show_ai_assistant: false,
            #[cfg(feature = "ai-terminal")]
            ai_assistant_enabled: true,

            // Code Editor (may be expensive)
            #[cfg(feature = "code-editor")]
            code_editor: CodeEditor::new(),
            #[cfg(feature = "code-editor")]
            show_code_editor: false,
            #[cfg(feature = "code-editor")]
            current_editing_file: None,

            // Shared runtime
            runtime,

            // User Experience Enhancements
            ux_manager: UXManager::new(),
            onboarding_wizard: OnboardingWizard::default(),
            show_onboarding: true, // Show on first launch
            show_quick_tips: true,
            quick_tip_queue: Vec::new(),

            // Server Deletion Confirmation
            show_delete_confirm: false,
            pending_delete_server_id: None,
            pending_delete_server_name: None,
        }
    }

    fn get_interaction_state(&mut self, id: &str) -> &MicrointeractionState {
        if !self.interaction_states.contains_key(id) {
            self.interaction_states.insert(id.to_string(), MicrointeractionState::new());
        }
        self.interaction_states.get(id).unwrap()
    }

    fn refresh_servers(&mut self) {
        let vm = self.view_model.lock().unwrap();
        self.servers = vm.get_servers();
    }

    fn add_server(&mut self) {
        if self.new_server.name.is_empty() {
            self.add_error = Some("Name is required".to_string());
            return;
        }
        if self.new_server.host.is_empty() {
            self.add_error = Some("Host is required".to_string());
            return;
        }
        if self.new_server.username.is_empty() {
            self.add_error = Some("Username is required".to_string());
            return;
        }

        let port: i64 = self.new_server.port.parse().unwrap_or(22);
        let auth = match self.new_server.auth_type {
            AuthType::Password => "password",
            AuthType::Key => "key",
        };

        let vm = self.view_model.lock().unwrap();
        match vm.add_server(
            &self.new_server.name,
            &self.new_server.host,
            port,
            &self.new_server.username,
            auth,
        ) {
            Ok(_) => {
                let server_name = self.new_server.name.clone();
                info!("Server added successfully: {}", server_name);
                self.show_add_dialog = false;
                self.new_server = NewServerForm::default();
                self.add_error = None;
                drop(vm);
                self.refresh_servers();
                info!("Server list refreshed, total servers: {}", self.servers.len());
                // Show success toast to user
                self.ux_manager.show_toast(
                    ToastNotification::success("服务器添加成功", &format!("服务器 '{}' 已添加到列表", server_name))
                );
            }
            Err(e) => {
                error!("Failed to add server: {}", e);
                self.add_error = Some(format!("Failed to add server: {}", e));
            }
        }
    }

    fn start_connect(&mut self) {
        if let Some(server_id) = self.selected_server.as_ref() {
            if let Some(server) = self.servers.iter().find(|s| s.id == *server_id).cloned() {
                self.show_connect_dialog = true;
                self.connect_server = Some(server.clone());

                let vm = self.view_model.lock().unwrap();
                if let Some(saved_pwd) = vm.get_saved_password(&server.id) {
                    self.password = saved_pwd;
                    self.use_saved_password = true;
                    self.save_password = true;
                } else {
                    self.password.clear();
                    self.use_saved_password = false;
                    self.save_password = true;
                }

                self.connect_status = ConnectStatus::Idle;
                self.connect_error = None;
            }
        }
    }

    fn do_connect(&mut self) {
        if let Some(ref server) = self.connect_server {
            self.connect_status = ConnectStatus::Connecting;
            self.connect_error = None;

            let session_id = Uuid::new_v4().to_string();
            let password = if self.password.is_empty() {
                None
            } else {
                Some(self.password.clone())
            };

            let vm = self.view_model.lock().unwrap();

            if self.save_password {
                if let Some(ref pwd) = password {
                    if let Err(e) = vm.save_password(&server.id, pwd) {
                        error!("Failed to save password: {}", e);
                        self.connect_error = Some(format!("Save password failed: {}", e));
                    }
                }
            }

            match vm.connect(
                &session_id,
                &server.host,
                server.port,
                &server.username,
                password.as_deref(),
            ) {
                Ok(_) => {
                    info!("Connected to {}", server.name);
                    self.show_connect_dialog = false;
                    self.connect_status = ConnectStatus::Idle;
                    self.current_session_id = Some(session_id.clone());
                    self.is_terminal_active = true;
                    self.command_running = false;
                    self.session_start_time = Some(std::time::Instant::now());
                    self.terminal_output = format!(
                        "Connected to {} ({}@{}:{})\n\n",
                        server.name, server.username, server.host, server.port
                    );

                    // Create session tab (Termius-like multi-session)
                    let tab = SessionTab {
                        session_id: session_id.clone(),
                        server_id: server.id.clone(),
                        title: format!("{}@{}", server.username, server.host),
                        output: self.terminal_output.clone(),
                        input: String::new(),
                        connected: true,
                    };
                    self.session_tabs.push(tab);
                    self.active_tab = Some(session_id.clone());

                    // Start persistent shell stream once
                    match vm.execute_stream(&session_id, "") {
                        Ok(receiver) => {
                            self.command_receiver = Some(receiver);
                        }
                        Err(e) => {
                            self.terminal_output
                                .push_str(&format!("[stream init failed] {}\n", e));
                        }
                    }
                }
                Err(e) => {
                    error!("Failed to connect to {}: {}", server.name, e);
                    self.connect_status = ConnectStatus::Error;
                    self.connect_error = Some(e.to_string());
                }
            }
        }
    }

    fn poll_command_output(&mut self) {
        if let Some(receiver) = self.command_receiver.as_mut() {
            while let Ok(chunk) = receiver.try_recv() {
                self.terminal_output.push_str(&chunk);
                if let Some(active) = self.active_tab.clone() {
                    if let Some(tab) = self.session_tabs.iter_mut().find(|t| t.session_id == active) {
                        tab.output.push_str(&chunk);
                    }
                }
            }
        }
        // Prevent memory bloat from large terminal outputs
        self.truncate_terminal();
    }

    fn execute_command(&mut self) {
        if let Some(ref session_id) = self.current_session_id {
            let cmd = self.command_input.trim().to_string();
            if cmd.is_empty() {
                return;
            }

            // Add to history (avoid duplicates)
            if !self.command_history.contains(&cmd) {
                self.command_history.push(cmd.clone());
                if self.command_history.len() > 100 {
                    self.command_history.remove(0);
                }
            }
            self.history_index = None;

            self.terminal_output.push_str(&format!("$ {}\n", cmd));

            // Write to existing persistent shell stdin
            // IMPORTANT: Don't hold view_model lock during async operations to avoid deadlock
            let line = format!("{}\n", cmd);
            let sid = session_id.clone();
            let write_result = {
                let vm = self.view_model.lock().unwrap();
                vm.write_shell_input(&sid, line.as_bytes())
            };

            match write_result {
                Ok(_) => {}
                Err(e) => {
                    self.terminal_output.push_str(&format!("Error writing to shell: {}\n", e));
                    error!("Shell write failed: {}", e);
                }
            }

            self.command_input.clear();
        }
    }

    fn navigate_history(&mut self, up: bool) {
        if self.command_history.is_empty() {
            return;
        }

        if up {
            if self.history_index.is_none() {
                self.history_index = Some(self.command_history.len().saturating_sub(1));
            } else {
                self.history_index = Some(self.history_index.unwrap().saturating_sub(1));
            }
        } else {
            if let Some(idx) = self.history_index {
                if idx >= self.command_history.len() - 1 {
                    self.history_index = None;
                    self.command_input.clear();
                    return;
                }
                self.history_index = Some(idx + 1);
            } else {
                return;
            }
        }

        if let Some(idx) = self.history_index {
            if let Some(cmd) = self.command_history.get(idx) {
                self.command_input = cmd.clone();
            }
        }
    }

    fn stop_current_command(&mut self) {
        if let Some(session_id) = self.current_session_id.clone() {
            let vm = self.view_model.lock().unwrap();
            let _ = vm.interrupt_command(&session_id);
            let _ = vm.write_shell_input(&session_id, &[0x03]);
            let _ = vm.write_shell_input(&session_id, b"\n");
            self.terminal_output.push_str("\n^C\n");
        }
    }

    fn disconnect(&mut self) {
        let session_id = self.current_session_id.clone();

        // Remove tab from session_tabs
        if let Some(ref sid) = session_id {
            self.session_tabs.retain(|t| t.session_id != *sid);
        }

        // Switch to another tab if available
        if let Some(tab) = self.session_tabs.last() {
            self.active_tab = Some(tab.session_id.clone());
            self.current_session_id = Some(tab.session_id.clone());
            self.terminal_output = tab.output.clone();
            self.is_terminal_active = true;
        } else {
            self.current_session_id = None;
            self.command_receiver = None;
            self.command_running = false;
            self.is_terminal_active = false;
            self.active_tab = None;
            self.session_start_time = None;
        }

        self.connect_status = ConnectStatus::Idle;
        self.show_connect_dialog = false;
        self.terminal_output.push_str("\nDisconnected.\n");

        if let Some(session_id) = session_id {
            let vm = self.view_model.lock().unwrap();
            let _ = vm.interrupt_command(&session_id);
            let _ = vm.sftp_close(&session_id);
            if let Err(e) = vm.disconnect(&session_id) {
                error!("Disconnect error: {}", e);
            }
        }

        // Close file browser
        self.file_entries.clear();
        self.file_current_path = String::from("/");
        self.show_file_browser = false;

        // Close monitor
        self.show_monitor = false;
        self.last_monitor_refresh = None;
        self.monitor_available = false;
        self.monitor_cpu = 0.0;
        self.monitor_memory = 0.0;
        self.monitor_disk = 0.0;
        self.monitor_load = String::from("-");
        self.monitor_net_in = String::from("-");
        self.monitor_net_out = String::from("-");
        self.monitor_uptime = String::from("-");
        self.monitor_net_prev = None;
        self.monitor_refreshing = false;
        self.monitor_result_rx = None;
    }

    // ==================== File Browser ====================

    fn refresh_file_list(&mut self) {
        if let Some(ref session_id) = self.current_session_id {
            let vm = self.view_model.lock().unwrap();

            match vm.sftp_list_dir(session_id, &self.file_current_path) {
                Ok(entries) => {
                    self.file_entries = entries
                        .into_iter()
                        .map(|e| FileEntry {
                            name: e.name.clone(),
                            path: e.path.clone(),
                            is_dir: e.file_type == "directory",
                            size: e.size_display(),
                            mtime: e.mtime_display(),
                        })
                        .collect();
                    self.file_error = None;  // 成功时清除错误
                }
                Err(e) => {
                    let err_str = e.to_string();
                    if err_str.contains("not initialized") {
                        // SFTP还在初始化中，保持当前状态
                        if self.file_error.is_none() || !self.file_error.as_ref().unwrap().contains("Initializing") {
                            self.file_error = Some(String::from("Initializing SFTP..."));
                        }
                    } else {
                        // 实际错误，显示给用户
                        self.file_error = Some(format!("SFTP error: {}", err_str));
                    }
                    self.file_entries.clear();
                }
            }
        }
    }

    fn navigate_to_parent(&mut self) {
        if self.file_current_path != "/" {
            if let Some(pos) = self.file_current_path.rfind('/') {
                if pos == 0 {
                    self.file_current_path = String::from("/");
                } else {
                    self.file_current_path = self.file_current_path[..pos].to_string();
                }
                self.refresh_file_list();
            }
        }
    }

    fn navigate_to_dir(&mut self, path: &str) {
        self.file_current_path = path.to_string();
        self.refresh_file_list();
    }

    fn create_folder(&mut self) {
        if let Some(ref session_id) = self.current_session_id {
            let new_path = if self.file_current_path == "/" {
                format!("/{}", self.new_folder_name)
            } else {
                format!("{}/{}", self.file_current_path, self.new_folder_name)
            };

            let result = {
                let vm = self.view_model.lock().unwrap();
                vm.sftp_mkdir(session_id, &new_path)
            };

            match result {
                Ok(_) => {
                    self.show_new_folder_dialog = false;
                    self.new_folder_name.clear();
                    self.refresh_file_list();
                }
                Err(e) => {
                    self.file_error = Some(format!("Failed to create folder: {}", e));
                }
            }
        }
    }

    fn delete_selected(&mut self) {
        if let (Some(ref session_id), Some(ref selected)) = (&self.current_session_id, &self.selected_file) {
            let is_dir = self.file_entries.iter().any(|e| &e.path == selected && e.is_dir);
            let result = {
                let vm = self.view_model.lock().unwrap();
                if is_dir {
                    vm.sftp_rmdir(session_id, selected)
                } else {
                    vm.sftp_remove(session_id, selected)
                }
            };

            match result {
                Ok(_) => {
                    self.selected_file = None;
                    self.refresh_file_list();
                }
                Err(e) => {
                    self.file_error = Some(format!("Failed to delete: {}", e));
                }
            }
        }
    }

    fn start_rename(&mut self) {
        if let Some(ref selected) = self.selected_file {
            self.rename_target = Some(selected.clone());
            self.rename_new_name = self.file_entries
                .iter()
                .find(|e| &e.path == selected)
                .map(|e| e.name.clone())
                .unwrap_or_default();
            self.show_rename_dialog = true;
        }
    }

    fn do_rename(&mut self) {
        if let (Some(ref session_id), Some(ref target), Some(new_name)) =
            (&self.current_session_id, &self.rename_target, Some(&self.rename_new_name))
        {
            if new_name.is_empty() {
                return;
            }

            let parent = if let Some(pos) = target.rfind('/') {
                &target[..pos + 1]
            } else {
                ""
            };
            let new_path = if parent.is_empty() || parent == "/" {
                format!("/{}", new_name)
            } else {
                format!("{}{}", parent, new_name)
            };

            let result = {
                let vm = self.view_model.lock().unwrap();
                vm.sftp_rename(session_id, target, &new_path)
            };

            match result {
                Ok(_) => {
                    self.show_rename_dialog = false;
                    self.rename_target = None;
                    self.refresh_file_list();
                }
                Err(e) => {
                    self.file_error = Some(format!("Failed to rename: {}", e));
                }
            }
        }
    }

    /// Download file, open in system editor, watch for changes, auto-upload on save
    fn edit_file(&mut self) {
        if let (Some(ref session_id), Some(ref selected)) = (&self.current_session_id, &self.selected_file) {
            // Don't edit directories
            if self.file_entries.iter().any(|e| &e.path == selected && e.is_dir) {
                self.file_error = Some("Cannot edit directories".to_string());
                return;
            }

            let remote_path = selected.clone();
            let sid = session_id.clone();

            // Download file to temp directory
            let (temp_path, content) = {
                let vm = self.view_model.lock().unwrap();
                // Use user's temp directory
                let temp_dir = std::env::var_os("TEMP")
                    .or_else(|| std::env::var_os("TMP"))
                    .map(std::path::PathBuf::from)
                    .unwrap_or_else(|| std::path::PathBuf::from(r"C:\Users\z7299\AppData\Local\Temp"));
                let file_name = remote_path.split('/').last().unwrap_or("edit.tmp");
                let temp_path = temp_dir.join(format!("easyssh_{}", file_name));
                match vm.sftp_download(&sid, &remote_path, temp_path.to_str().unwrap_or("")) {
                    Ok(data) => {
                        (temp_path, data)
                    }
                    Err(e) => {
                        self.file_error = Some(format!("Download failed: {}", e));
                        return;
                    }
                }
            };

            // Ensure parent directory exists
            if let Some(parent) = temp_path.parent() {
                if !parent.exists() {
                    if let Err(e) = std::fs::create_dir_all(parent) {
                        self.file_error = Some(format!("Cannot create temp dir: {}", e));
                        return;
                    }
                }
            }

            // Write content to temp file
            if let Err(e) = std::fs::write(&temp_path, &content) {
                self.file_error = Some(format!("Cannot write temp file: {}", e));
                return;
            }

            // Store state
            self.editing_file = Some(remote_path.clone());
            self.temp_file_path = Some(temp_path.to_string_lossy().to_string());
            self.file_error = Some(format!("Editing {} - save file to auto-upload", remote_path));

            // Open in system default editor
            #[cfg(target_os = "windows")]
            {
                std::process::Command::new("notepad")
                    .arg(&temp_path)
                    .spawn()
                    .ok();
            }
            #[cfg(not(target_os = "windows"))]
            {
                std::process::Command::new("xdg-open")
                    .arg(&temp_path)
                    .spawn()
                    .ok();
            }

            // Start file watcher in background thread
            let remote_path_clone = remote_path.clone();
            let sid_clone = sid.clone();
            let vm_clone = self.view_model.clone();
            let temp_path_for_watcher = temp_path.clone();

            std::thread::spawn(move || {
                // Wait for file to be modified (user saves in editor)
                // Simple approach: poll for file modification time change
                let original_mtime = std::fs::metadata(&temp_path_for_watcher)
                    .and_then(|m| m.modified())
                    .ok();

                loop {
                    std::thread::sleep(std::time::Duration::from_secs(1));

                    // Check if we're still supposed to be watching
                    // (simplified: just wait for a few seconds and assume user saved)

                    // Check if file was modified
                    if let Ok(current_mtime) = std::fs::metadata(&temp_path_for_watcher).and_then(|m| m.modified()) {
                        if original_mtime.map(|ot| ot != current_mtime).unwrap_or(false) {
                            // File was modified, upload it
                            if let Ok(new_content) = std::fs::read(&temp_path_for_watcher) {
                                let vm = vm_clone.lock().unwrap();
                                if let Err(e) = vm.sftp_upload(&sid_clone, &remote_path_clone, &new_content) {
                                    error!("Auto-upload failed: {}", e);
                                } else {
                                    info!("File auto-uploaded: {}", remote_path_clone);
                                }
                            }
                            break;
                        }
                    }

                    // Timeout after 5 minutes
                    if std::time::Instant::now().elapsed().as_secs() > 300 {
                        info!("File edit timeout, stopping watcher");
                        break;
                    }
                }

                // Clean up temp file
                let _ = std::fs::remove_file(&temp_path_for_watcher);
            });
        }
    }

    fn fmt_bytes(bytes: u64) -> String {
        const KB: u64 = 1024;
        const MB: u64 = KB * 1024;
        const GB: u64 = MB * 1024;
        if bytes >= GB {
            format!("{:.1} GB", bytes as f64 / GB as f64)
        } else if bytes >= MB {
            format!("{:.1} MB", bytes as f64 / MB as f64)
        } else if bytes >= KB {
            format!("{:.1} KB", bytes as f64 / KB as f64)
        } else {
            format!("{} B", bytes)
        }
    }

    fn fmt_rate(bytes_per_sec: f64) -> String {
        const KB: f64 = 1024.0;
        const MB: f64 = KB * 1024.0;
        const GB: f64 = MB * 1024.0;
        if bytes_per_sec >= GB {
            format!("{:.1} GB/s", bytes_per_sec / GB)
        } else if bytes_per_sec >= MB {
            format!("{:.1} MB/s", bytes_per_sec / MB)
        } else if bytes_per_sec >= KB {
            format!("{:.1} KB/s", bytes_per_sec / KB)
        } else {
            format!("{:.0} B/s", bytes_per_sec)
        }
    }

    fn is_ignored_iface(iface: &str) -> bool {
        iface == "lo"
            || iface.starts_with("docker")
            || iface.starts_with("br-")
            || iface.starts_with("veth")
            || iface.starts_with("flannel")
            || iface.starts_with("cni")
            || iface.starts_with("tun")
            || iface.starts_with("tap")
            || iface.starts_with("virbr")
    }

    fn parse_net_totals(output: &str) -> Option<(u64, u64)> {
        let mut total_in: u64 = 0;
        let mut total_out: u64 = 0;
        let mut has_data = false;

        for line in output.lines().skip(2) {
            let parts: Vec<&str> = line.split(':').collect();
            if parts.len() < 2 {
                continue;
            }

            let iface = parts[0].trim();
            if Self::is_ignored_iface(iface) {
                continue;
            }

            let data: Vec<&str> = parts[1].split_whitespace().collect();
            if data.len() >= 10 {
                if let (Ok(rx), Ok(tx)) = (data[0].parse::<u64>(), data[8].parse::<u64>()) {
                    total_in = total_in.saturating_add(rx);
                    total_out = total_out.saturating_add(tx);
                    has_data = true;
                }
            }
        }

        if has_data {
            Some((total_in, total_out))
        } else {
            None
        }
    }

    fn collect_monitor_snapshot(
        vm: &AppViewModel,
        session_id: &str,
        prev_net: Option<(u64, u64, std::time::Instant)>,
    ) -> MonitorSnapshot {
        info!("Monitor: collect_monitor_snapshot starting for session {}", session_id);
        let mut has_errors = false;

        // Helper to execute with timeout via SFTP session (avoids shell channel conflicts)
        fn exec_with_timeout(vm: &AppViewModel, session_id: &str, cmd: &str) -> anyhow::Result<String> {
            info!("Monitor: executing '{}' via SFTP session...", cmd);
            let start = std::time::Instant::now();
            // Use execute_via_sftp which runs on independent SFTP connection with built-in timeout
            let result = vm.execute_via_sftp(session_id, cmd);
            let elapsed = start.elapsed();
            match &result {
                Ok(_) => info!("Monitor: '{}' completed in {:?}", cmd, elapsed),
                Err(e) => error!("Monitor: '{}' failed after {:?}: {}", cmd, elapsed, e),
            }
            result
        }

        info!("Monitor: starting CPU collection...");
        // CPU
        let cpu = match exec_with_timeout(vm, session_id, "cat /proc/stat | head -1") {
            Ok(output) => {
                let mut cpu = 0.0f32;
                if let Some(line) = output.lines().next() {
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if parts.len() >= 5 {
                        if let (Ok(user), Ok(nice), Ok(system), Ok(idle)) =
                            (parts[1].parse::<u64>(), parts[2].parse::<u64>(), parts[3].parse::<u64>(), parts[4].parse::<u64>())
                        {
                            let total: u64 = user + nice + system + idle;
                            if total > 0 {
                                cpu = ((user + nice + system) as f32 / total as f32) * 100.0;
                            }
                        }
                    }
                }
                cpu
            }
            Err(e) => {
                error!("CPU command failed: {}", e);
                has_errors = true;
                0.0f32
            }
        };

        // Memory
        let memory = match exec_with_timeout(vm, session_id, "cat /proc/meminfo | head -3") {
            Ok(output) => {
                let mut mem_total: u64 = 0;
                let mut mem_available: u64 = 0;
                for line in output.lines() {
                    if line.starts_with("MemTotal:") {
                        if let Some(val) = line.split_whitespace().nth(1) {
                            mem_total = val.parse::<u64>().unwrap_or(0) * 1024;
                        }
                    }
                    if line.starts_with("MemAvailable:") {
                        if let Some(val) = line.split_whitespace().nth(1) {
                            mem_available = val.parse::<u64>().unwrap_or(0) * 1024;
                        }
                    }
                }
                if mem_total > 0 {
                    ((mem_total - mem_available) as f32 / mem_total as f32) * 100.0
                } else {
                    0.0f32
                }
            }
            Err(e) => {
                error!("MEM command failed: {}", e);
                has_errors = true;
                0.0f32
            }
        };

        // Disk
        let disk = match exec_with_timeout(vm, session_id, "df -B1 / | tail -1") {
            Ok(output) => {
                let mut disk = 0.0f32;
                if let Some(line) = output.lines().next() {
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if parts.len() >= 6 {
                        if let (Ok(total), Ok(used)) = (parts[1].parse::<u64>(), parts[2].parse::<u64>()) {
                            if total > 0 {
                                disk = (used as f32 / total as f32) * 100.0;
                            }
                        }
                    }
                }
                disk
            }
            Err(e) => {
                error!("DISK command failed: {}", e);
                has_errors = true;
                0.0f32
            }
        };

        // Load
        let load = match exec_with_timeout(vm, session_id, "cat /proc/loadavg") {
            Ok(output) => {
                let parts: Vec<&str> = output.split_whitespace().take(3).collect();
                if parts.is_empty() { String::from("-") } else { parts.join(" ") }
            }
            Err(e) => {
                error!("LOAD command failed: {}", e);
                has_errors = true;
                String::from("-")
            }
        };

        // Network (rate from successive refresh snapshots)
        let net_totals = match exec_with_timeout(vm, session_id, "cat /proc/net/dev") {
            Ok(output) => Self::parse_net_totals(&output),
            Err(e) => {
                error!("NET command failed: {}", e);
                has_errors = true;
                None
            }
        };

        let now = std::time::Instant::now();
        let (net_in, net_out, net_prev) = if let Some((current_in, current_out)) = net_totals {
            let display = if let Some((prev_in, prev_out, prev_t)) = prev_net {
                let dt = now.duration_since(prev_t).as_secs_f64();
                if dt > 0.0 {
                    let in_rate = current_in.saturating_sub(prev_in) as f64 / dt;
                    let out_rate = current_out.saturating_sub(prev_out) as f64 / dt;
                    (Self::fmt_rate(in_rate), Self::fmt_rate(out_rate))
                } else {
                    (
                        format!("{} total", Self::fmt_bytes(current_in)),
                        format!("{} total", Self::fmt_bytes(current_out)),
                    )
                }
            } else {
                (
                    format!("{} total", Self::fmt_bytes(current_in)),
                    format!("{} total", Self::fmt_bytes(current_out)),
                )
            };
            (display.0, display.1, Some((current_in, current_out, now)))
        } else {
            (String::from("-"), String::from("-"), prev_net)
        };

        // Uptime
        let uptime = match exec_with_timeout(vm, session_id, "cat /proc/uptime") {
            Ok(output) => {
                let mut result = String::from("-");
                if let Some(uptime_secs) = output.split_whitespace().next() {
                    if let Ok(secs) = uptime_secs.parse::<u64>() {
                        let days = secs / 86400;
                        let hours = (secs % 86400) / 3600;
                        let mins = (secs % 3600) / 60;
                        result = if days > 0 {
                            format!("{}d {}h {}m", days, hours, mins)
                        } else if hours > 0 {
                            format!("{}h {}m", hours, mins)
                        } else {
                            format!("{}m", mins)
                        };
                    }
                }
                result
            }
            Err(e) => {
                error!("UPTIME command failed: {}", e);
                has_errors = true;
                String::from("-")
            }
        };

        MonitorSnapshot {
            cpu,
            memory,
            disk,
            uptime,
            net_in,
            net_out,
            load,
            net_prev,
            refreshed_at: now,
            has_errors,
        }
    }

    fn poll_monitor_result(&mut self) {
        let mut clear_receiver = false;
        if let Some(rx) = self.monitor_result_rx.as_mut() {
            info!("Monitor: poll_monitor_result checking for results...");
            loop {
                match rx.try_recv() {
                    Ok(result) => {
                        info!("Monitor: received result for #{} (current waiting for #{})", result.request_seq, self.monitor_request_seq);
                        // 首先检查 request_seq 是否匹配
                        if result.request_seq != self.monitor_request_seq {
                            // 这是旧请求的结果，直接丢弃，等待新请求的结果
                            info!("Monitor: result seq mismatch, discarding");
                            continue;
                        }

                        // request_seq 匹配，这是当前等待的结果
                        // 检查 session_id 是否匹配（用户可能切换了会话）
                        if self.current_session_id.as_deref() != Some(result.session_id.as_str()) {
                            // session 已经切换，这个结果无效，但要重置 refreshing 状态
                            info!("Monitor: session mismatch, marking unavailable");
                            self.monitor_refreshing = false;
                            self.monitor_available = false;
                            clear_receiver = true;
                            break;
                        }

                        // 正常处理结果
                        info!("Monitor: applying result for #{} (has_errors={})", result.request_seq, result.snapshot.has_errors);
                        self.monitor_available = !result.snapshot.has_errors;
                        self.monitor_cpu = result.snapshot.cpu;
                        self.monitor_memory = result.snapshot.memory;
                        self.monitor_disk = result.snapshot.disk;
                        self.monitor_load = result.snapshot.load;
                        self.monitor_net_in = result.snapshot.net_in;
                        self.monitor_net_out = result.snapshot.net_out;
                        self.monitor_uptime = result.snapshot.uptime;
                        self.monitor_net_prev = result.snapshot.net_prev;
                        self.last_monitor_refresh = Some(result.snapshot.refreshed_at);
                        self.monitor_refreshing = false;
                        clear_receiver = true;
                        break;
                    }
                    Err(std_mpsc::TryRecvError::Empty) => {
                        // 暂时没有结果，继续等待
                        break;
                    }
                    Err(std_mpsc::TryRecvError::Disconnected) => {
                        // 后台线程断开（可能 panic 了）
                        error!("Monitor: receiver disconnected unexpectedly");
                        self.monitor_refreshing = false;
                        self.monitor_available = false;
                        clear_receiver = true;
                        break;
                    }
                }
            }
        }

        if clear_receiver {
            self.monitor_result_rx = None;
        }
    }

    fn refresh_monitor(&mut self) {
        if self.current_session_id.is_none() || self.monitor_refreshing {
            return;
        }

        let session_id = self.current_session_id.clone().unwrap();
        let session_id_for_log = session_id.clone();
        let vm = {
            let guard = self.view_model.lock().unwrap();
            guard.clone()
        };

        let prev_net = self.monitor_net_prev;
        self.monitor_request_seq = self.monitor_request_seq.wrapping_add(1);
        let request_seq = self.monitor_request_seq;
        self.monitor_refreshing = true;

        let (tx, rx) = std_mpsc::channel();
        self.monitor_result_rx = Some(rx);

        info!("Monitor: starting background refresh #{} for session {}", request_seq, session_id_for_log);

        std::thread::spawn(move || {
            info!("Monitor: background thread started for #{} (session {})", request_seq, session_id);
            let snapshot = EasySSHApp::collect_monitor_snapshot(&vm, &session_id, prev_net);
            info!("Monitor: snapshot collected for #{} (has_errors={})", request_seq, snapshot.has_errors);
            let send_result = tx.send(MonitorRefreshResult {
                request_seq,
                session_id,
                snapshot,
            });
            match send_result {
                Ok(_) => info!("Monitor: result sent for #{} successfully", request_seq),
                Err(_) => error!("Monitor: failed to send result for #{}", request_seq),
            }
        });

        info!("Monitor: background thread spawned for #{} (session {})", request_seq, session_id_for_log);
    }

    // ==================== Global Search ====================
    fn render_global_search(&mut self, ctx: &egui::Context) {
        let need_update = ctx.input(|i| {
            i.events.iter().any(|e| matches!(e, egui::Event::Text(_)))
        });

        if need_update || self.global_search_results.is_empty() {
            self.update_search_results();
        }

        self.handle_search_keyboard(ctx);

        let screen_rect = ctx.screen_rect();
        ctx.layer_painter(egui::LayerId::new(egui::Order::Background, "search_overlay".into()))
            .rect_filled(screen_rect, 0.0, egui::Color32::from_black_alpha(180));

        let window_width = 700.0;
        let window_height = 500.0;
        let window_pos = egui::Pos2::new(
            (screen_rect.width() - window_width) / 2.0,
            (screen_rect.height() - window_height) / 3.0,
        );

        egui::Window::new("🔍 Global Search")
            .fixed_pos(window_pos)
            .fixed_size([window_width, window_height])
            .collapsible(false)
            .resizable(false)
            .title_bar(false)
            .frame(egui::Frame {
                fill: egui::Color32::from_rgb(35, 38, 46),
                rounding: egui::Rounding::same(12.0),
                stroke: egui::Stroke::new(1.0, egui::Color32::from_rgb(64, 156, 255)),
                shadow: egui::epaint::Shadow {
                    blur: 24.0,
                    spread: 0.0,
                    offset: egui::Vec2::new(0.0, 8.0),
                    color: egui::Color32::from_black_alpha(100),
                },
                ..Default::default()
            })
            .show(ctx, |ui| {
                ui.vertical(|ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("🔍").size(20.0));
                        let search_edit = egui::TextEdit::singleline(&mut self.global_search_query)
                            .hint_text("Search servers, commands, snippets... (Ctrl+Shift+F)")
                            .font(egui::TextStyle::Heading)
                            .desired_width(ui.available_width() - 80.0);
                        let response = ui.add(search_edit);
                        if self.global_search_selected.is_some() {
                            response.request_focus();
                        }
                        let filter_active = self.show_search_filters || self.has_active_filters();
                        let filter_btn = egui::Button::new("⚙")
                            .fill(if filter_active { egui::Color32::from_rgb(64, 156, 255) } else { egui::Color32::from_rgb(50, 55, 65) })
                            .min_size([36.0, 36.0].into());
                        if ui.add(filter_btn).clicked() {
                            self.show_search_filters = !self.show_search_filters;
                        }
                    });

                    ui.add_space(4.0);

                    if self.show_search_filters {
                        self.render_search_filters(ui);
                    }

                    ui.separator();

                    let total_results = self.global_search_results.len();
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new(format!("{} results", total_results))
                            .small()
                            .color(egui::Color32::from_rgb(150, 160, 175)));
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.add(egui::Button::new("★ Favs").small().fill(
                                if self.search_filter.only_favorites { egui::Color32::from_rgb(64, 156, 255) } else { egui::Color32::from_rgb(50, 55, 65) }
                            )).clicked() {
                                self.search_filter.only_favorites = !self.search_filter.only_favorites;
                                self.update_search_results();
                            }
                            if ui.add(egui::Button::new("🕐 Recent").small().fill(
                                if self.search_filter.only_recent { egui::Color32::from_rgb(64, 156, 255) } else { egui::Color32::from_rgb(50, 55, 65) }
                            )).clicked() {
                                self.search_filter.only_recent = !self.search_filter.only_recent;
                                self.update_search_results();
                            }
                        });
                    });

                    ui.add_space(4.0);

                    egui::ScrollArea::vertical()
                        .max_height(window_height - 150.0)
                        .show(ui, |ui| {
                            if self.global_search_results.is_empty() {
                                ui.vertical_centered(|ui| {
                                    ui.add_space(50.0);
                                    ui.label(egui::RichText::new("No results found").size(16.0).color(egui::Color32::from_rgb(150, 160, 175)));
                                    ui.label(egui::RichText::new("Try a different search term").small().color(egui::Color32::from_rgb(120, 130, 145)));
                                });
                            } else {
                                let results: Vec<_> = self.global_search_results.clone();
                                for (idx, result) in results.iter().enumerate() {
                                    let is_selected = self.global_search_selected == Some(idx);
                                    self.render_search_result_item(ui, result, is_selected, idx);
                                }
                            }
                        });

                    ui.separator();

                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("↑↓ Navigate • Enter Execute • Esc Close • Ctrl+D Delete").small().color(egui::Color32::from_rgb(120, 130, 145)));
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.add(egui::Button::new("Clear History").small()).clicked() {
                                self.search_engine.clear_history();
                            }
                        });
                    });
                });
            });
    }

    fn render_search_result_item(&mut self, ui: &mut egui::Ui, result: &search::SearchResult, is_selected: bool, idx: usize) {
        let bg_color = if is_selected { egui::Color32::from_rgb(64, 120, 200) } else { egui::Color32::TRANSPARENT };
        let text_color = if is_selected { egui::Color32::WHITE } else { egui::Color32::from_rgb(220, 225, 235) };
        let subtitle_color = if is_selected { egui::Color32::from_rgb(200, 210, 230) } else { egui::Color32::from_rgb(150, 160, 175) };

        let response = egui::Frame::none()
            .fill(bg_color)
            .rounding(egui::Rounding::same(6.0))
            .inner_margin(egui::Margin::same(8.0))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new(&result.icon).size(18.0));
                    ui.vertical(|ui| {
                        let action_hint = match result.action {
                            QuickAction::Connect => "[Connect]",
                            QuickAction::Execute => "[Execute]",
                            QuickAction::Edit => "[Edit]",
                            QuickAction::Delete => "[Delete]",
                            QuickAction::FilterByTag => "[Filter]",
                            QuickAction::FilterByGroup => "[Filter]",
                            QuickAction::CopyToClipboard => "[Copy]",
                        };
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new(&result.title).color(text_color).size(14.0).strong());
                            ui.label(egui::RichText::new(action_hint).small().color(if is_selected { egui::Color32::from_rgb(180, 200, 255) } else { egui::Color32::from_rgb(100, 130, 180) }));
                        });
                        ui.label(egui::RichText::new(&result.subtitle).color(subtitle_color).size(12.0));
                        ui.horizontal_wrapped(|ui| {
                            if let Some(tags) = result.metadata.get("tags") {
                                if !tags.is_empty() {
                                    for tag in tags.split(", ").take(3) {
                                        ui.label(egui::RichText::new(format!("# {}", tag))
                                            .small()
                                            .color(if is_selected { egui::Color32::from_rgb(200, 220, 255) } else { egui::Color32::from_rgb(64, 156, 255) }));
                                    }
                                }
                            }
                            if let Some(is_fav) = result.metadata.get("is_favorite") {
                                if is_fav == "true" {
                                    ui.label(egui::RichText::new("★ Favorite").small().color(egui::Color32::from_rgb(255, 207, 80)));
                                }
                            }
                            if let Some(is_recent) = result.metadata.get("is_recent") {
                                if is_recent == "true" {
                                    ui.label(egui::RichText::new("🕐 Recent").small().color(egui::Color32::from_rgb(100, 220, 150)));
                                }
                            }
                            if let Some(is_conn) = result.metadata.get("is_connected") {
                                if is_conn == "true" {
                                    ui.label(egui::RichText::new("● Active").small().color(egui::Color32::from_rgb(72, 199, 116)));
                                }
                            }
                        });
                    });
                });
            }).response.interact(egui::Sense::click());

        if response.clicked() {
            self.global_search_selected = Some(idx);
            self.execute_selected_search_result();
        }
        if response.hovered() {
            self.global_search_selected = Some(idx);
        }
    }

    fn render_search_filters(&mut self, ui: &mut egui::Ui) {
        egui::Frame::none()
            .fill(egui::Color32::from_rgb(45, 50, 60))
            .rounding(egui::Rounding::same(8.0))
            .inner_margin(egui::Margin::same(10.0))
            .show(ui, |ui| {
                ui.label(egui::RichText::new("Filters").strong().color(egui::Color32::from_rgb(220, 225, 235)));
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    ui.label("Status: ");
                    let status_options = vec![
                        ("All", ConnectionStatusFilter::All),
                        ("Connected", ConnectionStatusFilter::Connected),
                        ("Disconnected", ConnectionStatusFilter::Disconnected)
                    ];
                    for (label, status) in &status_options {
                        let is_selected = self.search_filter.connection_status.as_ref() == Some(status);
                        if ui.add(egui::Button::new(*label).small().fill(
                            if is_selected { egui::Color32::from_rgb(64, 156, 255) } else { egui::Color32::from_rgb(50, 55, 65) }
                        )).clicked() {
                            self.search_filter.connection_status = if is_selected { None } else { Some(status.clone()) };
                            self.update_search_results();
                        }
                    }
                });
                ui.add_space(4.0);
                if ui.add(egui::Button::new("Clear All Filters").small().fill(egui::Color32::from_rgb(80, 60, 60))).clicked() {
                    self.search_filter = FilterCriteria::default();
                    self.update_search_results();
                }
            });
        ui.add_space(4.0);
    }

    fn handle_search_keyboard(&mut self, ctx: &egui::Context) {
        let total_results = self.global_search_results.len();
        if total_results == 0 { return; }

        if ctx.input(|i| i.key_pressed(egui::Key::ArrowDown)) {
            let current = self.global_search_selected.unwrap_or(0);
            self.global_search_selected = Some((current + 1) % total_results);
        }
        if ctx.input(|i| i.key_pressed(egui::Key::ArrowUp)) {
            let current = self.global_search_selected.unwrap_or(0);
            self.global_search_selected = Some(if current == 0 { total_results - 1 } else { current - 1 });
        }
        if ctx.input(|i| i.key_pressed(egui::Key::Enter)) {
            self.execute_selected_search_result();
        }
        if ctx.input(|i| i.modifiers.ctrl && i.key_pressed(egui::Key::D)) {
            if let Some(selected) = self.global_search_selected {
                if let Some(result) = self.global_search_results.get(selected).cloned() {
                    if result.result_type == search::SearchResultType::Server {
                        self.quick_delete_server(&result.id);
                    }
                }
            }
        }
    }

    fn update_search_results(&mut self) {
        let active_sessions: Vec<String> = self.session_tabs.iter().map(|t| t.server_id.clone()).collect();
        self.global_search_results = self.search_engine.search(
            &self.global_search_query,
            &self.servers,
            &self.favorites,
            &self.command_history,
            &self.tags,
            &self.search_filter,
            &active_sessions,
        );
        if !self.global_search_results.is_empty() {
            if self.global_search_selected.is_none() {
                self.global_search_selected = Some(0);
            } else {
                let max_idx = self.global_search_results.len().saturating_sub(1);
                if self.global_search_selected.unwrap() > max_idx {
                    self.global_search_selected = Some(max_idx);
                }
            }
        } else {
            self.global_search_selected = None;
        }
    }

    fn execute_selected_search_result(&mut self) {
        if let Some(selected) = self.global_search_selected {
            if let Some(result) = self.global_search_results.get(selected).cloned() {
                self.search_engine.add_to_history(&self.global_search_query, Some(result.id.clone()));
                match result.action {
                    QuickAction::Connect => {
                        if let Some(server) = self.servers.iter().find(|s| s.id == result.id).cloned() {
                            self.selected_server = Some(result.id.clone());
                            self.search_engine.recent_usage_mut().record_connection(server.id.clone(), server.name.clone());
                            self.start_connect();
                        }
                        self.show_global_search = false;
                    }
                    QuickAction::Execute => {
                        if let Some(content) = result.metadata.get("command") {
                            if let Some(ref _session_id) = self.current_session_id {
                                self.command_input = content.clone();
                                self.execute_command();
                            }
                        } else if let Some(content) = result.metadata.get("content") {
                            self.command_input = content.clone();
                        }
                        self.show_global_search = false;
                    }
                    QuickAction::Edit => {
                        self.selected_server = Some(result.id.clone());
                        self.show_global_search = false;
                    }
                    QuickAction::Delete => {
                        self.quick_delete_server(&result.id);
                        self.update_search_results();
                    }
                    QuickAction::FilterByTag => {
                        if let Some(tag) = result.metadata.get("tag") {
                            if !self.search_filter.tags.contains(tag) {
                                self.search_filter.tags.push(tag.clone());
                                self.global_search_query.clear();
                                self.update_search_results();
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    fn quick_delete_server(&mut self, server_id: &str) {
        let vm = self.view_model.lock().unwrap();
        if let Err(e) = vm.delete_server(server_id) {
            error!("Failed to delete server: {}", e);
        }
        drop(vm);
        self.refresh_servers();
        self.global_search_results.retain(|r| r.id != server_id);
    }


    fn has_active_filters(&self) -> bool {
        !self.search_filter.tags.is_empty()
            || self.search_filter.group_id.is_some()
            || self.search_filter.connection_status.is_some()
            || self.search_filter.only_favorites
            || self.search_filter.only_recent
    }

    fn render_snippets_panel(&mut self, ctx: &egui::Context) {
        snippets_ui::render_snippets_panel(
            ctx,
            &mut self.show_snippets_panel,
            &mut self.snippet_manager,
            &mut self.selected_snippet_category,
            &mut self.snippet_search,
            &mut self.snippet_action_message,
            &mut self.show_add_snippet_dialog,
            &mut self.snippet_input_dialog,
            &mut self.command_input,
            &mut self.new_snippet_form,
        );
    }
    fn render_snippet_dialog(&mut self, _ctx: &egui::Context) {
        // Stub - snippet dialog rendering
    }
    fn render_add_snippet_dialog(&mut self, ctx: &egui::Context) {
        snippets_ui::render_add_snippet_dialog(
            ctx,
            &mut self.show_add_snippet_dialog,
            &mut self.new_snippet_form,
            &mut self.snippet_manager,
            &mut self.snippet_action_message,
        );
    }

    #[cfg(feature = "code-editor")]
    fn render_code_editor_panel(&mut self, ctx: &egui::Context) {
        egui::Window::new("Code Editor")
            .collapsible(true)
            .resizable(true)
            .default_size([800.0, 600.0])
            .frame(egui::Frame {
                fill: egui::Color32::from_rgb(30, 30, 30),
                stroke: egui::Stroke::new(1.0, egui::Color32::from_rgb(60, 70, 85)),
                ..Default::default()
            })
            .show(ctx, |ui| {
                // Toolbar
                ui.horizontal(|ui| {
                    ui.heading("📄 Professional Code Editor");
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        // Theme selector
                        egui::ComboBox::from_id_source("editor_theme")
                            .selected_text(self.code_editor.theme_manager.current_theme().name.clone())
                            .show_ui(ui, |ui| {
                                for theme_name in self.code_editor.theme_manager.list_themes() {
                                    if ui.selectable_label(false, &theme_name).clicked() {
                                        if let Some(theme) = self.code_editor.theme_manager.load_theme(&theme_name) {
                                            self.code_editor.set_theme(theme);
                                        }
                                    }
                                }
                            });

                        ui.separator();

                        // Toggle minimap
                        if ui.button(if self.code_editor.show_minimap { "🗺️ Map" } else { "Map" }).clicked() {
                            self.code_editor.show_minimap = !self.code_editor.show_minimap;
                        }

                        // Toggle terminal
                        if ui.button(if self.code_editor.show_terminal { "💻 Term" } else { "Term" }).clicked() {
                            self.code_editor.show_terminal = !self.code_editor.show_terminal;
                        }

                        // Find/Replace
                        if ui.button("🔍 Find").clicked() {
                            self.code_editor.show_find_replace = !self.code_editor.show_find_replace;
                        }

                        ui.separator();

                        // Save button
                        if ui.button("💾 Save").clicked() {
                            if let Some(ref file_info) = self.current_editing_file {
                                if let Err(e) = self.code_editor.save(&file_info.path) {
                                    self.code_editor.add_line_to_terminal(&format!("Save error: {}", e));
                                }
                            }
                        }

                        // Close button
                        if ui.button("✕").clicked() {
                            self.show_code_editor = false;
                        }
                    });
                });

                ui.separator();

                // File info
                if let Some(ref file_info) = self.current_editing_file {
                    ui.horizontal(|ui| {
                        let path_display = if file_info.path.is_empty() { "Untitled" } else { &file_info.path };
                        ui.label(format!("📄 {}", path_display));
                        if file_info.is_remote {
                            ui.colored_label(egui::Color32::YELLOW, "🌐 Remote");
                        }
                        if file_info.is_dirty {
                            ui.colored_label(egui::Color32::RED, "● Unsaved");
                        }
                        ui.separator();
                        ui.label(format!("Lang: {:?}", file_info.language));
                        ui.label(format!("Lines: {} | Chars: {}", file_info.line_count, file_info.char_count));
                    });
                    ui.separator();
                }

                // Editor area
                let _available_size = ui.available_size();
                let response = self.code_editor.render(ui);

                // Handle editor response
                if response.clicked() {
                    // Handle click in editor
                }
            });
    }

    /// Sync UI theme (DesignTheme) with terminal theme selection.
    /// Called when user selects a different theme in the ThemeGallery.
    fn sync_ui_theme_with_terminal_theme(&mut self, ctx: &egui::Context) {
        let current_theme_id = &self.theme_manager.current_theme.id;

        // Only apply if theme has actually changed
        if self.last_applied_theme_id != *current_theme_id {
            tracing::info!("Theme changed from {} to {}, updating UI theme", self.last_applied_theme_id, current_theme_id);

            // Determine UI theme based on terminal theme's background brightness
            let bg = self.theme_manager.current_theme.palette.background;
            let brightness = (bg.r() as u16 + bg.g() as u16 + bg.b() as u16) / 3;

            // Update DesignTheme based on terminal theme brightness
            self.theme = if brightness > 128 {
                DesignTheme::light()
            } else {
                DesignTheme::dark()
            };

            // Apply accessibility settings and update the UI
            self.theme.apply_accessibility_settings();
            self.theme.apply_to_ctx(ctx);

            // Save the new theme ID
            self.last_applied_theme_id = current_theme_id.clone();

            // Persist theme settings
            if let Err(e) = self.theme_manager.save() {
                tracing::warn!("Failed to save theme settings: {}", e);
            }
        }
    }

    /// Handle settings changes from the settings panel (theme, accessibility, and language)
    /// Also syncs terminal theme with UI theme when needed
    fn handle_settings_changes(&mut self, ctx: &egui::Context) {
        use crate::settings::UiThemeMode;
        use crate::design::{DesignTheme, AccessibilitySettings};

        // Check for pending language changes from settings panel
        if let Some(new_language) = self.settings_panel.take_pending_language_change() {
            info!("Applying language change from settings: {}", new_language);

            // The language has already been applied via set_language in the settings panel
            // We just need to trigger a UI refresh. The settings manager has already persisted it.

            // Show a toast notification to inform the user
            self.ux_manager.show_toast(
                ToastNotification::success(
                    "语言已更改 / Language Updated",
                    format!("应用语言已更改为: {}", new_language)
                )
            );

            // Request a repaint to refresh the UI with new language
            ctx.request_repaint();
        }

        // Check for pending theme changes from settings panel
        let theme_changed = if let Some(new_theme_mode) = self.settings_panel.take_pending_theme_change() {
            info!("Applying theme change from settings: {:?}", new_theme_mode);

            // Create the appropriate DesignTheme
            let mut new_theme = match new_theme_mode {
                UiThemeMode::Light => DesignTheme::light(),
                UiThemeMode::Dark => DesignTheme::dark(),
                UiThemeMode::System => {
                    // Detect system theme from egui's current visuals
                    let is_system_dark = ctx.style().visuals.dark_mode;
                    if is_system_dark {
                        DesignTheme::dark()
                    } else {
                        DesignTheme::light()
                    }
                }
            };

            // Apply accessibility settings to the theme
            new_theme.apply_accessibility_settings();

            // Update the app's theme
            self.theme = new_theme;

            // CRITICAL: Actually apply the theme to the UI context
            self.theme.apply_to_ctx(ctx);

            info!("Theme changed to {:?} and applied to UI", new_theme_mode);
            true
        } else {
            false
        };

        // Check for pending accessibility changes
        if self.settings_panel.take_pending_accessibility_change() {
            let (high_contrast, reduced_motion, large_text) = self.settings_panel.get_accessibility_settings();
            info!("Applying accessibility changes: high_contrast={}, reduced_motion={}, large_text={}",
                  high_contrast, reduced_motion, large_text);

            // Update global accessibility settings
            let settings = AccessibilitySettings::global();
            settings.high_contrast.store(high_contrast, std::sync::atomic::Ordering::Relaxed);
            settings.reduced_motion.store(reduced_motion, std::sync::atomic::Ordering::Relaxed);
            settings.large_text.store(large_text, std::sync::atomic::Ordering::Relaxed);

            // Re-apply current theme with new accessibility settings
            let mut updated_theme = self.theme.clone();
            updated_theme.apply_accessibility_settings();
            self.theme = updated_theme;

            // CRITICAL: Apply updated theme to the UI context
            self.theme.apply_to_ctx(ctx);

            info!("Accessibility settings applied");
        } else if theme_changed {
            // If theme changed but no accessibility change, we still need to ensure
            // accessibility settings are applied to the new theme
            let (high_contrast, reduced_motion, large_text) = self.settings_panel.get_accessibility_settings();
            let settings = AccessibilitySettings::global();
            settings.high_contrast.store(high_contrast, std::sync::atomic::Ordering::Relaxed);
            settings.reduced_motion.store(reduced_motion, std::sync::atomic::Ordering::Relaxed);
            settings.large_text.store(large_text, std::sync::atomic::Ordering::Relaxed);
        }

        // Save settings to disk after any changes
        if let Err(e) = self.save_all_settings() {
            tracing::warn!("Failed to save settings: {}", e);
        }
    }

    /// Save all settings (UI theme and accessibility) to configuration file
    fn save_all_settings(&self) -> anyhow::Result<()> {
        let config_path = Self::get_config_path()
            .ok_or_else(|| anyhow::anyhow!("Could not determine config path"))?;

        // Build settings object
        let settings = serde_json::json!({
            "ui_theme_mode": match self.settings_panel.ui_theme_mode {
                crate::settings::UiThemeMode::Dark => "dark",
                crate::settings::UiThemeMode::Light => "light",
                crate::settings::UiThemeMode::System => "system",
            },
            "accessibility": {
                "high_contrast": self.settings_panel.high_contrast,
                "reduced_motion": self.settings_panel.reduce_motion,
                "large_text": self.settings_panel.large_text,
            },
            "version": "1.0",
        });

        // Ensure config directory exists
        if let Some(parent) = config_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        // Write config file
        let content = serde_json::to_string_pretty(&settings)?;
        std::fs::write(config_path, content)?;

        info!("Settings saved successfully");
        Ok(())
    }

    /// Get the configuration file path for general settings
    fn get_config_path() -> Option<std::path::PathBuf> {
        dirs::config_dir().map(|p| p.join("easyssh").join("settings.json"))
    }

    /// Load UI settings from configuration file
    /// Returns: (ui_theme_mode, accessibility_settings)
    fn load_ui_settings() -> (Option<crate::settings::UiThemeMode>, Option<(bool, bool, bool)>) {
        if let Some(config_path) = Self::get_config_path() {
            if config_path.exists() {
                if let Ok(content) = std::fs::read_to_string(&config_path) {
                    if let Ok(config) = serde_json::from_str::<serde_json::Value>(&content) {
                        // Parse UI theme mode
                        let theme_mode = config.get("ui_theme_mode")
                            .and_then(|v| v.as_str())
                            .map(|mode| match mode {
                                "light" => crate::settings::UiThemeMode::Light,
                                "system" => crate::settings::UiThemeMode::System,
                                _ => crate::settings::UiThemeMode::Dark,
                            });

                        // Parse accessibility settings
                        let accessibility = config.get("accessibility").map(|acc| {
                            let high_contrast = acc.get("high_contrast")
                                .and_then(|v| v.as_bool())
                                .unwrap_or(false);
                            let reduced_motion = acc.get("reduced_motion")
                                .and_then(|v| v.as_bool())
                                .unwrap_or(false);
                            let large_text = acc.get("large_text")
                                .and_then(|v| v.as_bool())
                                .unwrap_or(false);
                            (high_contrast, reduced_motion, large_text)
                        });

                        return (theme_mode, accessibility);
                    }
                }
            }
        }
        (None, None)
    }
}

impl eframe::App for EasySSHApp {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        // Update UX manager for animations and timed events
        self.ux_manager.update();

        // === Handle theme changes from settings panel ===
        self.handle_settings_changes(ctx);

        self.poll_command_output();
        self.poll_monitor_result();

        // === Auto-refresh file browser when SFTP is initializing ===
        if self.show_file_browser && self.current_session_id.is_some() {
            let should_refresh = self.file_error.as_ref()
                .map(|e| e.contains("Initializing") || e.contains("not initialized"))
                .unwrap_or(false);

            if should_refresh {
                // Try to refresh every 500ms until SFTP is ready
                let now = std::time::Instant::now();
                let last_refresh = self.last_file_refresh.unwrap_or(now);
                if now.duration_since(last_refresh).as_millis() > 500 {
                    self.refresh_file_list();
                    self.last_file_refresh = Some(now);
                }
            }
        }

        if self.command_receiver.is_some() || self.monitor_refreshing {
            ctx.request_repaint_after(std::time::Duration::from_millis(16));
        }

        // === Professional Hotkey System ===
        // Process hotkeys before UI to ensure they work even when modals are open
        self.process_hotkeys(ctx, frame);

        // Push UI heartbeat/debug state for websocket diagnostics
        update_ui_debug(
            self.current_session_id.is_some(),
            self.current_session_id.clone(),
            self.terminal_output.len(),
            self.command_input.len(),
        );

        // === Onboarding for first-time users ===
        if self.show_onboarding && self.onboarding_wizard.state.should_show() {
            self.render_onboarding(ctx);
        }

        // === Render toast notifications ===
        self.ux_manager.render_toasts(ctx, &self.theme);

        egui::TopBottomPanel::top("top_bar")
            .frame(egui::Frame {
                fill: egui::Color32::from_rgb(30, 34, 42),
                ..Default::default()
            })
            .show(ctx, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.label(egui::RichText::new("🖥").size(20.0));
                ui.label(egui::RichText::new("EasySSH").heading().color(egui::Color32::from_rgb(220, 225, 235)));
                ui.add_space(20.0);
                ui.label(egui::RichText::new("Ctrl+C: Interrupt | Ctrl+L: Clear | Ctrl+Shift+H/V: Split | Ctrl+1-9: Tabs").small().color(egui::Color32::from_rgb(120, 128, 145)));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    // Main Settings button
                    let settings_btn = egui::Button::new("⚙️")
                        .fill(egui::Color32::from_rgb(60, 70, 85))
                        .rounding(4.0)
                        .min_size([44.0, 44.0].into());
                    if ui.add(settings_btn).on_hover_text("Settings (Import/Export/Cloud)").clicked() {
                        self.settings_panel.toggle();
                    }
                    ui.add_space(8.0);
// Theme Gallery button
                    let theme_btn = egui::Button::new("🎨")
                        .fill(egui::Color32::from_rgb(60, 70, 85))
                        .rounding(4.0)
                        .min_size([44.0, 44.0].into());
                    if ui.add(theme_btn).on_hover_text("Theme Gallery - Browse and customize themes").clicked() {
                        self.theme_gallery.open();
                    }
                    ui.add_space(8.0);

                    // Enterprise Vault button
                    let vault_btn = egui::Button::new("🔐")
                        .fill(egui::Color32::from_rgb(60, 70, 85))
                        .rounding(4.0)
                        .min_size([44.0, 44.0].into());
                    if ui.add(vault_btn).on_hover_text("Enterprise Password Vault").clicked() {
                        self.enterprise_vault.open = true;
                    }
                    ui.add_space(8.0);

                    #[cfg(feature = "remote-desktop")]
                    {
                        // Remote Desktop button
                        let rdp_btn = egui::Button::new("ud83duddbcufe0f")
                            .fill(egui::Color32::from_rgb(60, 70, 85))
                            .rounding(4.0)
                            .min_size([44.0, 44.0].into());
                        if ui.add(rdp_btn).on_hover_text("Remote Desktop").clicked() {
                            self.show_remote_desktop = !self.show_remote_desktop;
                        }
                        ui.add_space(8.0);
                    }

                    #[cfg(feature = "ai-terminal")]
                    {
                        // AI Assistant button
                        let ai_btn_fill = if self.show_ai_assistant {
                            egui::Color32::from_rgb(64, 156, 255) // Active state
                        } else {
                            egui::Color32::from_rgb(60, 70, 85)
                        };
                        let ai_btn = egui::Button::new("ud83euddd0")  // Scientist emoji
                            .fill(ai_btn_fill)
                            .rounding(4.0)
                            .min_size([44.0, 44.0].into());
                        if ui.add(ai_btn).on_hover_text("AI Assistant - Command help, completion, and security audit").clicked() {
                            self.show_ai_assistant = !self.show_ai_assistant;
                        }
                        ui.add_space(8.0);
                    }

                    #[cfg(feature = "code-editor")]
                    {
                        // Code Editor button
                        let editor_btn_fill = if self.show_code_editor {
                            egui::Color32::from_rgb(80, 150, 100) // Active state
                        } else {
                            egui::Color32::from_rgb(60, 70, 85)
                        };
                        let editor_btn = egui::Button::new("📝")
                            .fill(editor_btn_fill)
                            .rounding(4.0)
                            .min_size([44.0, 44.0].into());
                        if ui.add(editor_btn).on_hover_text("Code Editor - Professional editor with syntax highlighting (Ctrl+Shift+E)").clicked() {
                            self.show_code_editor = !self.show_code_editor;
                            if self.show_code_editor && self.current_editing_file.is_none() {
                                // Create new untitled file
                                self.code_editor.load_content("untitled", "");
                                self.current_editing_file = Some(FileInfo {
                                    path: "untitled".to_string(),
                                    content: String::new(),
                                    is_remote: false,
                                    language: code_editor::syntax_highlighter::Language::PlainText,
                                    line_count: 0,
                                    char_count: 0,
                                    is_dirty: false,
                                    modified: false,
                                    encoding: "UTF-8".to_string(),
                                    line_ending: "LF".to_string(),
                                });
                            }
                        }
                        ui.add_space(8.0);
                    }

                    // Split Layout buttons
                    let hsplit_btn = egui::Button::new("◫")
                        .fill(egui::Color32::from_rgb(60, 70, 85))
                        .rounding(4.0)
                        .min_size([44.0, 44.0].into());
                    if ui.add(hsplit_btn).on_hover_text("Split Horizontally (Ctrl+Shift+H)").clicked() {
                        self.split_panel_horizontal();
                    }
                    ui.add_space(4.0);

                    let vsplit_btn = egui::Button::new("◪")
                        .fill(egui::Color32::from_rgb(60, 70, 85))
                        .rounding(4.0)
                        .min_size([44.0, 44.0].into());
                    if ui.add(vsplit_btn).on_hover_text("Split Vertically (Ctrl+Shift+V)").clicked() {
                        self.split_panel_vertical();
                    }
                    ui.add_space(8.0);

                    let add_btn = egui::Button::new("+ Add Server")
                        .fill(egui::Color32::from_rgb(64, 156, 255))
                        .rounding(4.0)
                        .min_size([120.0, 44.0].into());
                    if ui.add(add_btn).clicked() {
                        self.show_add_dialog = true;
                        self.new_server = NewServerForm::default();
                        self.add_error = None;
                    }
                });
            });
        });

        if self.show_add_dialog {
            egui::Window::new("Add New Server")
                .collapsible(false)
                .resizable(false)
                .default_size([400.0, 380.0])
                .frame(egui::Frame {
                    fill: egui::Color32::from_rgb(42, 48, 58),
                    stroke: egui::Stroke::new(1.0, egui::Color32::from_rgb(60, 70, 85)),
                    ..Default::default()
                })
                .show(ctx, |ui| {
                    ui.vertical_centered(|ui| {
                        ui.add_space(10.0);
                        ui.label(egui::RichText::new("➕ Add New Server").heading().color(egui::Color32::from_rgb(220, 225, 235)));
                        ui.add_space(10.0);
                    });
                    ui.separator();

                    if let Some(ref err) = self.add_error {
                        ui.colored_label(egui::Color32::from_rgb(255, 87, 87), err);
                        ui.separator();
                    }

                    egui::Grid::new("add_server_grid")
                        .num_columns(2)
                        .spacing([10.0, 12.0])
                        .show(ui, |ui| {
                            ui.label(egui::RichText::new("Name:").color(egui::Color32::from_rgb(180, 190, 205)));
                            ui.add(egui::TextEdit::singleline(&mut self.new_server.name).hint_text("My Server"));
                            ui.end_row();

                            ui.label(egui::RichText::new("Host:").color(egui::Color32::from_rgb(180, 190, 205)));
                            ui.add(egui::TextEdit::singleline(&mut self.new_server.host).hint_text("192.168.1.100"));
                            ui.end_row();

                            ui.label(egui::RichText::new("Port:").color(egui::Color32::from_rgb(180, 190, 205)));
                            ui.add(egui::TextEdit::singleline(&mut self.new_server.port).hint_text("22"));
                            ui.end_row();

                            ui.label(egui::RichText::new("Username:").color(egui::Color32::from_rgb(180, 190, 205)));
                            ui.add(egui::TextEdit::singleline(&mut self.new_server.username).hint_text("root"));
                            ui.end_row();

                            ui.label(egui::RichText::new("Auth:").color(egui::Color32::from_rgb(180, 190, 205)));
                            ui.horizontal(|ui| {
                                ui.radio_value(
                                    &mut self.new_server.auth_type,
                                    AuthType::Password,
                                    "🔐 Password",
                                );
                                ui.radio_value(
                                    &mut self.new_server.auth_type,
                                    AuthType::Key,
                                    "🔑 SSH Key",
                                );
                            });
                            ui.end_row();
                        });

                    ui.add_space(10.0);
                    ui.separator();
                    ui.add_space(5.0);

                    if self.new_server.port.is_empty() {
                        ui.label(egui::RichText::new("Default port: 22").small().color(egui::Color32::from_rgb(120, 130, 150)));
                    }

                    ui.separator();

                        ui.horizontal(|ui| {
                            if ui.add(egui::Button::new("Cancel").min_size([80.0, 44.0].into())).clicked() {
                                self.show_add_dialog = false;
                            }
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                if ui.add(egui::Button::new("Add Server").min_size([120.0, 44.0].into())).clicked() {
                                    self.add_server();
                                }
                            });
                        });
                });
        }

        if self.show_connect_dialog {
            let server_info = self
                .connect_server
                .as_ref()
                .map(|s| (s.name.clone(), s.username.clone(), s.host.clone(), s.port));

            if let Some((name, username, host, port)) = server_info {
                egui::Window::new(format!("Connect to {}", name))
                    .collapsible(false)
                    .resizable(false)
                    .default_size([400.0, 300.0])
                    .show(ctx, |ui| {
                        ui.heading(format!("{}@{}:{}", username, host, port));
                        ui.colored_label(egui::Color32::GRAY, "Auth: Password or SSH Agent");
                        ui.separator();

                        if let Some(ref err) = self.connect_error {
                            ui.colored_label(egui::Color32::RED, format!("Error: {}", err));
                            ui.label(egui::RichText::new("Check: firewall, SSH port (22), credentials").size(11.0).color(egui::Color32::GRAY));
                            ui.separator();
                        }

                        match self.connect_status {
                            ConnectStatus::Idle => {
                                if self.use_saved_password && !self.password.is_empty() {
                                    ui.horizontal(|ui| {
                                        ui.colored_label(
                                            egui::Color32::GREEN,
                                            "Saved password loaded from keychain",
                                        );
                                        if ui.add(egui::Button::new("Clear").min_size([80.0, 44.0].into())).clicked() {
                                            self.password.clear();
                                            self.use_saved_password = false;
                                            self.save_password = false;
                                        }
                                    });
                                    ui.add_space(5.0);
                                }

                                ui.horizontal(|ui| {
                                    ui.label("Password:");
                                    ui.add(
                                        egui::TextEdit::singleline(&mut self.password)
                                            .password(true),
                                    );
                                });

                                ui.checkbox(&mut self.save_password, "Save password to keychain");
                                ui.checkbox(&mut self.auto_reconnect, "Auto-reconnect on disconnect");

                                ui.separator();

                                ui.horizontal(|ui| {
                                    if ui.add(egui::Button::new("Cancel").min_size([80.0, 44.0].into())).clicked() {
                                        self.show_connect_dialog = false;
                                    }
                                    ui.with_layout(
                                        egui::Layout::right_to_left(egui::Align::Center),
                                        |ui| {
                                            if ui.add(egui::Button::new("Connect").min_size([120.0, 44.0].into())).clicked() {
                                                self.do_connect();
                                            }
                                        },
                                    );
                                });
                            }
                            ConnectStatus::Connecting => {
                                ui.label("Connecting...");
                                ui.spinner();
                            }
                            ConnectStatus::Connected => {
                                ui.colored_label(
                                    egui::Color32::GREEN,
                                    "Connected successfully!",
                                );
                            }
                            ConnectStatus::Error => {
                                ui.colored_label(egui::Color32::RED, "Connection failed");
                                ui.separator();
                                if ui.add(egui::Button::new("Close").min_size([80.0, 44.0].into())).clicked() {
                                    self.show_connect_dialog = false;
                                    self.connect_status = ConnectStatus::Idle;
                                }
                                if ui.add(egui::Button::new("Retry").min_size([80.0, 44.0].into())).clicked() {
                                    self.connect_status = ConnectStatus::Idle;
                                    self.connect_error = None;
                                }
                            }
                        }
                    });
            }
        }

        // File Browser Panel (centered popup)
        if self.show_file_browser && self.current_session_id.is_some() {
            egui::Window::new("File Manager")
                .collapsible(false)
                .resizable(true)
                .default_size([500.0, 400.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.heading("📁 File Manager");
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.add(egui::Button::new("× Close").min_size([100.0, 44.0].into())).clicked() {
                                self.show_file_browser = false;
                            }
                        });
                    });
                    ui.separator();

                    // Current path display
                    ui.horizontal(|ui| {
                        if ui.button("↰ Back").clicked() {
                            self.navigate_to_parent();
                        }
                        if ui.button("⟳ Refresh").clicked() {
                            self.refresh_file_list();
                        }
                        ui.label(egui::RichText::new(&self.file_current_path).monospace().size(12.0));
                    });
                    ui.separator();

                    // Error display
                    if let Some(ref err) = self.file_error {
                        ui.colored_label(egui::Color32::RED, err);
                        ui.separator();
                    }

                    // Action buttons
                    ui.horizontal(|ui| {
                        if ui.add(egui::Button::new("+ New Folder").min_size([120.0, 44.0].into())).clicked() {
                            self.show_new_folder_dialog = true;
                            self.new_folder_name.clear();
                        }
                        if ui.add(egui::Button::new("✎ Rename").min_size([100.0, 44.0].into())).clicked() {
                            self.start_rename();
                        }
                        if ui.add(egui::Button::new("🗑 Delete").min_size([100.0, 44.0].into())).clicked() {
                            self.delete_selected();
                        }
                        // File edit buttons
                        let has_selection = self.selected_file.is_some();
                        let is_file = self.selected_file.as_ref().map(|p| {
                            !self.file_entries.iter().any(|e| &e.path == p && e.is_dir)
                        }).unwrap_or(false);

                        if ui.add_enabled(has_selection && is_file, egui::Button::new("✏ Edit").min_size([80.0, 44.0].into())).clicked() {
                            self.edit_file();
                        }
                    });
                    ui.separator();

                    // File list
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        // Collect paths first to avoid borrow issues
                        let entries_with_paths: Vec<(String, String, bool)> = self.file_entries
                            .iter()
                            .map(|e| (e.path.clone(), e.name.clone(), e.is_dir))
                            .collect();

                        for (path, name, is_dir) in entries_with_paths {
                            let icon = if is_dir { "📁" } else { "📄" };
                            let is_selected = self.selected_file.as_ref() == Some(&path);

                            let label = {
                                let entry = self.file_entries.iter().find(|e| e.path == path);
                                let size = entry.map(|e| e.size.clone()).unwrap_or_default();
                                let mtime = entry.map(|e| e.mtime.clone()).unwrap_or_default();
                                format!("{} {}  {}  {}", icon, name, size, mtime)
                            };
                            let response = ui.selectable_label(is_selected, label);

                            if response.clicked() {
                                self.selected_file = Some(path.clone());
                            }

                            // Double-click for directories
                            if is_dir && response.double_clicked() {
                                self.navigate_to_dir(&path);
                            }
                        }
                    });
                });

            // New folder dialog
            if self.show_new_folder_dialog {
                egui::Window::new("New Folder")
                    .collapsible(false)
                    .resizable(false)
                    .default_size([300.0, 120.0])
                    .show(ctx, |ui| {
                        ui.label("Folder name:");
                        ui.text_edit_singleline(&mut self.new_folder_name);

                        ui.horizontal(|ui| {
                            if ui.add(egui::Button::new("Cancel").min_size([80.0, 44.0].into())).clicked() {
                                self.show_new_folder_dialog = false;
                            }
                            if ui.add(egui::Button::new("Create").min_size([80.0, 44.0].into())).clicked() {
                                self.create_folder();
                            }
                        });
                    });
            }

            // Rename dialog
            if self.show_rename_dialog {
                egui::Window::new("Rename")
                    .collapsible(false)
                    .resizable(false)
                    .default_size([300.0, 120.0])
                    .show(ctx, |ui| {
                        ui.label("New name:");
                        ui.text_edit_singleline(&mut self.rename_new_name);

                        ui.horizontal(|ui| {
                            if ui.add(egui::Button::new("Cancel").min_size([80.0, 44.0].into())).clicked() {
                                self.show_rename_dialog = false;
                            }
                            if ui.add(egui::Button::new("Rename").min_size([80.0, 44.0].into())).clicked() {
                                self.do_rename();
                            }
                        });
                    });
            }
        }

        egui::SidePanel::left("server_list")
            .width_range(220.0..=350.0)
            .frame(egui::Frame {
                fill: egui::Color32::from_rgb(38, 42, 50),
                ..Default::default()
            })
            .show(ctx, |ui| {
                // Header with icon and title
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("📡").size(18.0));
                    ui.label(egui::RichText::new("Servers").heading().color(egui::Color32::from_rgb(220, 225, 235)));
                });
                ui.add_space(8.0);
                ui.separator();

                // Filter buttons with better styling
                ui.horizontal_wrapped(|ui| {
                    let all_btn = egui::Button::new("All")
                        .fill(if self.selected_group.is_none() { egui::Color32::from_rgb(64, 156, 255) } else { egui::Color32::from_rgb(50, 55, 65) });
                    if ui.add(all_btn).clicked() {
                        self.selected_group = None;
                    }

                    let favs_btn = egui::Button::new("★ Favs")
                        .fill(if self.selected_group.as_ref() == Some(&"__favorites__".to_string()) { egui::Color32::from_rgb(64, 156, 255) } else { egui::Color32::from_rgb(50, 55, 65) });
                    if ui.add(favs_btn).clicked() {
                        self.selected_group = Some("__favorites__".to_string());
                    }
                });
                ui.add_space(6.0);

                // Search with better styling
                ui.horizontal(|ui| {
                    ui.label("🔍");
                    ui.text_edit_singleline(&mut self.search_query);
                });
                ui.add_space(4.0);
                ui.separator();

                egui::ScrollArea::vertical().show(ui, |ui| {
                    // Build a map of group_id -> servers
                    let mut ungrouped: Vec<&ServerViewModel> = Vec::new();
                    let mut grouped: std::collections::HashMap<String, Vec<&ServerViewModel>> = std::collections::HashMap::new();

                    for server in &self.servers {
                        // Search filter
                        let matches_search = if self.search_query.is_empty() {
                            true
                        } else {
                            let query = self.search_query.to_lowercase();
                            server.name.to_lowercase().contains(&query) || server.host.to_lowercase().contains(&query)
                        };

                        if !matches_search {
                            continue;
                        }

                        if let Some(ref group_id) = server.group_id {
                            grouped.entry(group_id.clone()).or_default().push(server);
                        } else {
                            ungrouped.push(server);
                        }
                    }

                    // Macro to render a server button
                    macro_rules! render_server_btn {
                        ($server:expr) => {
                            let is_sel = self.selected_server.as_ref() == Some(&$server.id);
                            let has_sess = self.current_session_id.is_some() && is_sel;
                            let is_fav = self.favorites.contains(&$server.id);
                            let ic = if has_sess { "● " } else if is_fav { "★ " } else { "  " };
                            let nc = if is_sel { egui::Color32::WHITE } else { egui::Color32::from_rgb(220, 225, 235) };
                            let bg = if is_sel { egui::Color32::from_rgb(64, 156, 255) } else { egui::Color32::from_rgb(48, 52, 62) };
                            let btn = egui::Button::new(egui::RichText::new(format!("{}{}\n{}@{}:{}", ic, $server.name, $server.username, $server.host, $server.port)).color(nc).size(13.0))
                                .fill(bg).rounding(6.0)
                                .stroke(if is_sel { egui::Stroke::new(2.0, egui::Color32::from_rgb(100, 180, 255)) } else { egui::Stroke::new(1.0, egui::Color32::from_rgb(60, 65, 75)) })
                                .frame(false);
                            ui.add_space(2.0);
                            if ui.add(btn).clicked() { self.selected_server = Some($server.id.clone()); }
                        };
                    }

                    // Render groups
                    for group in &self.groups {
                        let servers_in_group = grouped.get(&group.id);
                        if servers_in_group.is_none() || servers_in_group.unwrap().is_empty() {
                            continue;
                        }

                        // Group header
                        let is_group_selected = self.selected_group.as_ref() == Some(&group.id);
                        let group_header_bg = if is_group_selected {
                            egui::Color32::from_rgb(50, 60, 80)
                        } else {
                            egui::Color32::from_rgb(40, 48, 58)
                        };

                        ui.add_space(6.0);
                        let header_btn = egui::Button::new(
                            egui::RichText::new(format!("📂 {}", group.name))
                                .color(egui::Color32::from_rgb(180, 195, 215))
                                .size(13.0)
                        )
                        .fill(group_header_bg)
                        .rounding(4.0)
                        .frame(false);

                        if ui.add(header_btn).clicked() {
                            if is_group_selected {
                                self.selected_group = None;
                            } else {
                                self.selected_group = Some(group.id.clone());
                            }
                        }

                        // Show servers in this group if selected
                        if is_group_selected {
                            for server in servers_in_group.unwrap() {
                                render_server_btn!(server);
                            }
                        }
                    }

                    // Favorites section
                    let favorite_servers: Vec<_> = self.servers.iter()
                        .filter(|s| self.favorites.contains(&s.id) && s.group_id.is_none())
                        .filter(|s| {
                            if self.search_query.is_empty() { return true; }
                            let q = self.search_query.to_lowercase();
                            s.name.to_lowercase().contains(&q) || s.host.to_lowercase().contains(&q)
                        })
                        .collect();

                    if !favorite_servers.is_empty() {
                        ui.add_space(6.0);
                        let fav_btn = egui::Button::new(
                            egui::RichText::new("★ Favorites").color(egui::Color32::from_rgb(255, 207, 80)).size(13.0)
                        )
                        .fill(egui::Color32::from_rgb(50, 48, 40))
                        .rounding(4.0)
                        .frame(false);
                        if ui.add(fav_btn).clicked() {
                            if self.selected_group.as_ref() == Some(&"__favorites__".to_string()) {
                                self.selected_group = None;
                            } else {
                                self.selected_group = Some("__favorites__".to_string());
                            }
                        }
                        if self.selected_group.as_ref() == Some(&"__favorites__".to_string()) {
                            for server in favorite_servers {
                                render_server_btn!(server);
                            }
                        }
                    }

                    // Ungrouped servers
                    ui.add_space(6.0);
                    if !ungrouped.is_empty() {
                        let ungrouped_btn = egui::Button::new(
                            egui::RichText::new("📡 No Group").color(egui::Color32::from_rgb(150, 160, 180)).size(13.0)
                        )
                        .fill(egui::Color32::from_rgb(40, 45, 52))
                        .rounding(4.0)
                        .frame(false);

                        let is_ungrouped_selected = self.selected_group.as_ref() == Some(&"__ungrouped__".to_string());
                        if is_ungrouped_selected {
                            if ui.add(ungrouped_btn).clicked() {
                                self.selected_group = None;
                            }
                            for server in &ungrouped {
                                render_server_btn!(server);
                            }
                        } else {
                            if ui.add(ungrouped_btn).clicked() {
                                self.selected_group = Some("__ungrouped__".to_string());
                            }
                        }
                    }
                });
            });

        // Snippets Panel
        self.render_snippets_panel(ctx);

        // Code Editor Panel
        #[cfg(feature = "code-editor")]
        if self.show_code_editor {
            self.render_code_editor_panel(ctx);
        }

        // ==================== Split Layout Main Content ====================
        // Use the split layout manager to render all panels
        egui::CentralPanel::default()
            .frame(egui::Frame {
                fill: egui::Color32::from_rgb(30, 32, 38),
                ..Default::default()
            })
            .show(ctx, |ui| {
                // Get session ID for terminal panels
                let current_session = self.current_session_id.clone();

                // Use raw pointer to work around borrow checker limitations
                // This is safe because the closure is immediately executed and doesn't outlive self
                let self_ptr: *mut Self = self;

                // Render the split layout using the pointer workaround
                unsafe { &mut *self_ptr }.split_layout_manager.render(ui, |ui, panel_id, content, is_active| {
                    // SAFETY: The pointer is valid for the duration of this closure
                    // because the closure is executed immediately within the render call
                    let app = unsafe { &mut *self_ptr };

                    // Click to activate panel
                    if is_active {
                        ui.ctx().set_visuals(ui.ctx().style().visuals.clone());
                    }

                    // Render panel content based on type
                    match content.panel_type {
                        PanelType::Terminal => {
                            app.render_terminal_panel(ui, panel_id, content, is_active, current_session.as_deref());
                        }
                        PanelType::SftpBrowser => {
                            app.render_sftp_panel(ui, panel_id, content, is_active);
                        }
                        PanelType::Monitor => {
                            app.render_monitor_panel(ui, panel_id, content, is_active);
                        }
                        PanelType::ServerList => {
                            app.render_serverlist_panel(ui, panel_id, content, is_active);
                        }
                    }
                });
            });

        // ==================== Global Search Dialog (Raycast/Alfred style) ====================
        if self.show_global_search {
            self.render_global_search(ctx);
        }

        // ==================== Snippets Dialogs ====================
        self.render_snippet_dialog(ctx);
        self.render_add_snippet_dialog(ctx);

        // ==================== Port Forwarding Dialog ====================
        {
            let vm = self.view_model.lock().unwrap();
            let pf_vm = vm.get_port_forward_vm();
            self.port_forward_dialog.render(ctx, &pf_vm);
        }

        // ==================== Notification Panel ====================
        if self.notification_panel.visible {
            egui::Window::new("Notifications")
                .collapsible(false)
                .resizable(false)
                .default_pos([600.0, 80.0])
                .frame(egui::Frame::none())
                .show(ctx, |ui| {
                    self.notification_panel.show(ui);
                });
        }

        // ==================== Notification Settings ====================
        if self.notification_settings_panel.visible {
            egui::Window::new("Notification Settings")
                .collapsible(false)
                .resizable(false)
                .default_pos([400.0, 100.0])
                .frame(egui::Frame::none())
                .show(ctx, |ui| {
                    self.notification_settings_panel.show(ui);
                });
        }

        // ==================== Settings Panel (Import/Export/Cloud) ====================
        let vm = self.view_model.clone();
        self.settings_panel.render(ctx, &vm);

        // ==================== Professional Theme System ====================
        self.theme_gallery.render(ctx, &mut self.theme_manager, &mut self.theme_editor);
        self.theme_editor.render(ctx, &mut self.theme_manager);

        // Sync UI theme with terminal theme when user selects a theme in the gallery
        // This ensures the UI theme matches the terminal theme (light/dark)
        self.sync_ui_theme_with_terminal_theme(ctx);

        // ==================== Enterprise Password Vault ====================
        if self.enterprise_vault.open {
            self.enterprise_vault.render(ctx);
        }
        // ==================== Remote Desktop Panel ====================
        #[cfg(feature = "remote-desktop")]
        if self.show_remote_desktop {
            render_remote_desktop_panel(ctx, &mut self.remote_desktop_manager);
        }

        // ==================== AI Terminal Assistant ====================
        #[cfg(feature = "ai-terminal")]
        if self.show_ai_assistant && self.ai_assistant_enabled {
            // Initialize AI terminal if not already done
            if self.ai_terminal.is_none() {
                // Use the shared runtime directly (no longer Option-wrapped)
                self.ai_terminal = Some(AiTerminalUi::new(self.runtime.clone()));
            }

            // Show AI terminal panel
            if let Some(ref mut ai) = self.ai_terminal {
                ai.show(ctx, &self.command_input, &self.terminal_output);
            }
        }

        // ==================== Shortcut Cheatsheet ====================
        if self.show_shortcut_cheatsheet {
            if let Ok(mgr) = self.hotkey_manager.lock() {
                self.shortcut_cheatsheet.render(ctx, &mgr, &self.theme);
            }
        }
    }
}

// ==================== Command Palette ====================
// Note: Command palette rendering is handled in update method via self.command_palette.render()
// Note: Hotkey settings rendering is handled via self.hotkey_settings.render()

impl EasySSHApp {
    // ==================== User Experience Helpers ====================

    /// Render onboarding wizard for first-time users
    fn render_onboarding(&mut self, ctx: &egui::Context) {
        let mut should_close = false;
        let mut action = None;

        egui::Window::new("欢迎使用 EasySSH")
            .default_size([700.0, 550.0])
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
            .show(ctx, |ui| {
                action = self.onboarding_wizard.render(ui, &self.theme);
            });

        if let Some(action) = action {
            match action {
                OnboardingAction::Next => {
                    self.onboarding_wizard.state.next_step();
                }
                OnboardingAction::Previous => {
                    self.onboarding_wizard.state.previous_step();
                }
                OnboardingAction::Skip => {
                    should_close = true;
                }
                OnboardingAction::Finish => {
                    self.onboarding_wizard.state.complete();
                    should_close = true;

                    // Show welcome toast
                    self.ux_manager.show_toast(
                        ToastNotification::success("准备就绪！", "您可以开始添加服务器并连接了")
                            .with_action("添加服务器", || {})
                    );
                }
                OnboardingAction::AddServer => {
                    self.show_add_dialog = true;
                    should_close = true;
                }
                OnboardingAction::OpenSettings => {
                    self.settings_panel.toggle();
                    should_close = true;
                }
                OnboardingAction::OpenHelp => {
                    should_close = true;
                }
            }
        }

        if should_close {
            self.show_onboarding = false;
        }
    }

    /// Show loading overlay for an operation
    fn show_loading(&mut self, operation: LoadingOperation, cancellable: bool) {
        self.ux_manager.loading_states.start(operation, cancellable);
    }

    /// Update loading progress
    fn update_loading_progress(&mut self, operation: &LoadingOperation, progress: f32) {
        self.ux_manager.loading_states.update_progress(operation, progress);
    }

    /// Complete loading operation
    fn complete_loading(&mut self, operation: &LoadingOperation) {
        self.ux_manager.loading_states.complete(operation);
    }

    /// Show error with recovery options
    fn show_error(&mut self, error: &str) {
        self.ux_manager.error_queue.push_error(error);

        // Also show toast for immediate feedback
        self.ux_manager.show_toast(
            ToastNotification::error("操作失败", "请查看详情并尝试恢复")
        );
    }

    /// Show success message
    fn show_success(&mut self, message: impl Into<String>) {
        self.ux_manager.show_toast(
            ToastNotification::success("完成", message)
        );
    }

    /// Add a quick tip to show
    fn add_quick_tip(&mut self, icon: impl Into<String>, message: impl Into<String>) {
        self.quick_tip_queue.push(
            QuickTip::new(icon, message)
        );
    }

    // ==================== Split Layout Panel Renderers ====================

    /// Render a terminal panel in split layout
    fn render_terminal_panel(&mut self, ui: &mut egui::Ui, _panel_id: PanelId, _content: &split_layout::PanelContent, is_active: bool, session_id: Option<&str>) {
        let term_bg = self.theme_manager.current_theme.palette.background;
        let term_fg = self.theme_manager.current_theme.palette.foreground;
        let term_cursor = self.theme_manager.current_theme.palette.cursor;

        // Panel header with active indicator
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("🖥 Terminal").strong());
            if is_active {
                ui.colored_label(egui::Color32::GREEN, "●");
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.small_button("×").clicked() {
                    // Close handled by split layout manager
                }
            });
        });
        ui.separator();

        if session_id.is_some() && self.is_terminal_active {
            // Terminal content
            let available_height = ui.available_height() - 50.0;
            egui::Frame {
                fill: term_bg,
                rounding: egui::Rounding::same(4.0),
                stroke: egui::Stroke::new(1.0, term_fg.linear_multiply(0.3)),
                ..Default::default()
            }.show(ui, |ui| {
                ui.set_min_height(available_height.max(100.0));

                egui::ScrollArea::vertical()
                    .auto_shrink([false; 2])
                    .stick_to_bottom(true)
                    .show(ui, |ui| {
                        ui.label(egui::RichText::new(&self.terminal_output)
                            .monospace()
                            .size(14.0 * self.terminal_font_zoom)
                            .color(term_fg));
                    });
            });

            ui.separator();

            // Command input
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("❯").color(term_cursor));

                let enter_pressed = ui.input(|i| i.key_pressed(egui::Key::Enter));

                if ui.input(|i| i.key_pressed(egui::Key::ArrowUp)) {
                    self.navigate_history(true);
                }
                if ui.input(|i| i.key_pressed(egui::Key::ArrowDown)) {
                    self.navigate_history(false);
                }

                let response = ui.add(
                    egui::TextEdit::singleline(&mut self.command_input)
                        .font(egui::TextStyle::Monospace)
                        .desired_width(ui.available_width() - 100.0),
                );

                if enter_pressed && !self.command_input.is_empty() {
                    self.execute_command();
                }

                ui.memory_mut(|m| m.request_focus(response.id));

                if ui.add(egui::Button::new("Execute").min_size([80.0, 36.0].into())).clicked() {
                    self.execute_command();
                }
            });
        } else {
            // No active session
            ui.centered_and_justified(|ui| {
                ui.vertical_centered(|ui| {
                    ui.label(egui::RichText::new("🖥").size(48.0));
                    ui.add_space(16.0);
                    ui.label(egui::RichText::new("No active terminal session").size(16.0));
                    ui.add_space(8.0);
                    if ui.button("Connect to a server").clicked() {
                        if let Some(server_id) = self.selected_server.clone() {
                            if let Some(server) = self.servers.iter().find(|s| s.id == server_id).cloned() {
                                self.show_connect_dialog = true;
                                self.connect_server = Some(server);
                            }
                        }
                    }
                });
            });
        }
    }

    /// Render SFTP panel in split layout
    fn render_sftp_panel(&mut self, ui: &mut egui::Ui, _panel_id: PanelId, _content: &split_layout::PanelContent, _is_active: bool) {
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("📁 SFTP Browser").strong());
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.small_button("×").clicked() {
                    // Close handled by split layout manager
                }
            });
        });
        ui.separator();

        if self.show_file_browser {
            ui.label(format!("Path: {}", self.file_current_path));
            ui.separator();

            egui::ScrollArea::vertical().show(ui, |ui| {
                if self.file_current_path != "/" {
                    if ui.button("📁 ..").clicked() {
                        self.navigate_to_parent();
                    }
                }

                // Collect entries first to avoid borrow checker issues
                let entries: Vec<_> = self.file_entries.iter().map(|e| (e.name.clone(), e.path.clone(), e.is_dir, e.size.clone())).collect();
                for (name, path, is_dir, size) in entries {
                    let icon = if is_dir { "📁" } else { "📄" };
                    let label = format!("{} {} ({})", icon, name, size);
                    if ui.button(&label).clicked() {
                        if is_dir {
                            self.navigate_to_dir(&path);
                        } else {
                            self.selected_file = Some(path);
                        }
                    }
                }
            });
        } else {
            ui.centered_and_justified(|ui| {
                ui.label("SFTP not connected");
            });
        }
    }

    /// Render monitor panel in split layout
    fn render_monitor_panel(&mut self, ui: &mut egui::Ui, _panel_id: PanelId, _content: &split_layout::PanelContent, _is_active: bool) {
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("📊 System Monitor").strong());
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.small_button("×").clicked() {
                    // Close handled by split layout manager
                }
                if ui.small_button("🔄").clicked() {
                    self.refresh_monitor();
                }
            });
        });
        ui.separator();

        if self.show_monitor {
            egui::Grid::new("monitor_grid")
                .num_columns(2)
                .spacing([10.0, 5.0])
                .show(ui, |ui| {
                    ui.label("CPU:");
                    let cpu_color = if self.monitor_cpu > 80.0 { egui::Color32::RED }
                        else if self.monitor_cpu > 50.0 { egui::Color32::YELLOW }
                        else { egui::Color32::GREEN };
                    ui.colored_label(cpu_color, format!("{:.1}%", self.monitor_cpu));
                    ui.end_row();

                    ui.label("Memory:");
                    let mem_color = if self.monitor_memory > 80.0 { egui::Color32::RED }
                        else if self.monitor_memory > 50.0 { egui::Color32::YELLOW }
                        else { egui::Color32::GREEN };
                    ui.colored_label(mem_color, format!("{:.1}%", self.monitor_memory));
                    ui.end_row();

                    ui.label("Disk:");
                    let disk_color = if self.monitor_disk > 90.0 { egui::Color32::RED }
                        else if self.monitor_disk > 70.0 { egui::Color32::YELLOW }
                        else { egui::Color32::GREEN };
                    ui.colored_label(disk_color, format!("{:.1}%", self.monitor_disk));
                    ui.end_row();

                    ui.label("Load:");
                    ui.label(&self.monitor_load);
                    ui.end_row();

                    ui.label("Net In:");
                    ui.label(&self.monitor_net_in);
                    ui.end_row();

                    ui.label("Net Out:");
                    ui.label(&self.monitor_net_out);
                    ui.end_row();

                    ui.label("Uptime:");
                    ui.label(&self.monitor_uptime);
                    ui.end_row();
                });
        } else {
            ui.centered_and_justified(|ui| {
                ui.label("Monitor not connected");
            });
        }
    }

    /// Render server list panel in split layout
    fn render_serverlist_panel(&mut self, ui: &mut egui::Ui, _panel_id: PanelId, _content: &split_layout::PanelContent, _is_active: bool) {
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("📡 Servers").strong());
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.small_button("×").clicked() {
                    // Close handled by split layout manager
                }
            });
        });
        ui.separator();

        // Search
        ui.horizontal(|ui| {
            ui.label("🔍");
            ui.text_edit_singleline(&mut self.search_query);
        });
        ui.separator();

        // Server list
        egui::ScrollArea::vertical().show(ui, |ui| {
            for server in &self.servers {
                let is_selected = self.selected_server.as_ref() == Some(&server.id);
                let btn = egui::Button::new(format!("{}@{}:{}", server.username, server.host, server.port))
                    .fill(if is_selected { egui::Color32::from_rgb(64, 156, 255) } else { egui::Color32::from_rgb(48, 52, 62) });
                if ui.add(btn).clicked() {
                    self.selected_server = Some(server.id.clone());
                }
            }
        });
    }
}

