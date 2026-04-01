#![allow(dead_code)]

use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::mpsc as std_mpsc;
use std::time::{Duration, Instant};
use tokio::sync::mpsc;
use tracing::{error, info, debug};
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
mod theme_system;
mod port_forward_dialog;
mod pages;

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

#[cfg(feature = "workflow")]
use workflow_editor::{WorkflowEditor, ScriptLibraryBrowser};
#[cfg(feature = "workflow")]
use macro_recorder_ui::MacroRecorderPanel;
#[cfg(feature = "workflow")]
use scheduled_tasks_ui::ScheduledTasksPanel;
#[cfg(feature = "workflow")]
use batch_results_ui::BatchExecutionResultsPanel;
#[cfg(feature = "workflow")]
use workflow_panel::WorkflowPanel;

#[cfg(feature = "ai-terminal")]
use ai_terminal_ui::AiTerminalUi;

use design::{DesignTheme, AccessibilitySettings};
use apple_design::{
    AppleButton, AppleCard, AppleShadows, AppleSpinner, AppleTypography,
    ButtonSize, ButtonStyle, EmptyState, ErrorState, LucideIcons,
    MicrointeractionState, Motion, Divider, EasingFunction, AnimationState,
};
use viewmodels::{AppViewModel, GroupViewModel, ServerViewModel};
use ws_server::{update_ui_debug, WsControlServer};
use search::{GlobalSearchEngine, FilterCriteria, ConnectionStatusFilter, QuickAction};
use snippets::{SnippetCategory, SnippetInputDialog, SnippetManager, Snippet};
use layout_manager::{SplitLayoutManager, LayoutPreset};
use terminal::{WebGlTerminalManager, EguiWebGlTerminal, StreamingProcessor, RenderStats};
use sftp_file_manager::SftpFileManager;
use notifications::NotificationManager;
use notification_panel::{NotificationPanel, NotificationSettingsPanel};
use split_layout::{PanelContent, PanelId, PanelType, SplitLayout, DropTarget, LayoutPresets};
use hotkeys::{HotkeyManager, CommandPalette, HotkeySettingsUI, HotkeyAction, KeyBinding, Key, Command};
use performance::{global_monitor, PerformanceMonitor, global_render_optimizer, PerformanceOptimizer, init as init_performance};
use settings::{SettingsPanel, SettingsTab};

#[cfg(feature = "remote-desktop")]
use remote_desktop_ui::{RemoteDesktopManagerUI, render_connections_list, render_active_sessions, render_connection_dialog, render_remote_desktop_panel};
#[cfg(feature = "remote-desktop")]
use embedded_rdp::{RemoteDesktopViewer, RemoteDesktopViewerManager, ConnectionSettings, RemoteDesktopType};

// Version-specific features
#[cfg(feature = "lite")]
mod lite_features;

#[cfg(feature = "enterprise")]
mod enterprise_vault;
#[cfg(feature = "enterprise")]
use enterprise_vault::{EnterpriseVaultUI, VaultSecret, VaultAuditLog};

use pages::{AppPage, render_main_page, render_add_server_dialog, render_connect_dialog};

const DEV_TOOLS_HTML: &str = include_str!("../dev-tools.html");

fn main() -> eframe::Result {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter("info,easyssh=debug")
        .init();

    info!("Starting EasySSH for Windows");

    let rt = tokio::runtime::Runtime::new().expect("Failed to create Tokio runtime");

    // Start WebSocket debug API server if debug feature enabled
    #[cfg(feature = "debug")]
    {
        let ws_view_model = Arc::new(Mutex::new(AppViewModel::new().expect("Failed to init")));
        let ws_vm_clone = ws_view_model.clone();
        rt.spawn(async move {
            let ws_server = WsControlServer::new(8765, ws_vm_clone);
            if let Err(e) = ws_server.start().await {
                error!("WebSocket server error: {}", e);
            }
        });

        // Start embedded dev-tools HTTP server
        std::thread::spawn(|| {
            if let Ok(server) = tiny_http::Server::http("0.0.0.0:8766") {
                for request in server.incoming_requests() {
                    let url = request.url().to_string();
                    let response = match url.as_str() {
                        "/" | "/dev-tools.html" | "/index.html" => {
                            tiny_http::Response::from_string(DEV_TOOLS_HTML).with_header(
                                tiny_http::Header::from_bytes(
                                    &b"Content-Type"[..],
                                    &b"text/html; charset=utf-8"[..],
                                )
                                .unwrap(),
                            )
                        }
                        _ => tiny_http::Response::from_string("404 Not Found").with_status_code(404),
                    };
                    let _ = request.respond(response);
                }
            }
        });

        let _ = webbrowser::open("http://localhost:8766/dev-tools.html");
        info!("Debug mode: WebSocket API: ws://localhost:8765 | DevTools: http://localhost:8766");
    }

    // Determine version from binary name or feature flags
    let version_name = get_version_name();
    info!("Running version: {}", version_name);

    let viewport_size = match version_name.as_str() {
        "EasySSH-Lite" => [800.0, 600.0],
        "EasySSH-Pro" => [1600.0, 1000.0],
        _ => [1200.0, 800.0], // Standard
    };

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size(viewport_size)
            .with_min_inner_size([600.0, 400.0]),
        ..Default::default()
    };

    eframe::run_native(
        &version_name,
        options,
        Box::new(|cc| Ok(Box::new(EasySSHApp::new(cc, rt)))),
    )
}

fn get_version_name() -> String {
    // Check binary name from environment or features
    if cfg!(feature = "pro") {
        "EasySSH-Pro".to_string()
    } else if cfg!(feature = "lite") {
        "EasySSH-Lite".to_string()
    } else {
        "EasySSH".to_string() // Standard
    }
}

// ... rest of the implementation continues from main.rs ...
// (SessionTab, EasySSHApp struct, impl blocks, etc.)

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

struct EasySSHApp {
    view_model: Arc<Mutex<AppViewModel>>,
    runtime: tokio::runtime::Runtime,
    // ... rest of fields from main.rs ...
}

impl EasySSHApp {
    fn new(cc: &eframe::CreationContext<'_>, runtime: tokio::runtime::Runtime) -> Self {
        // Initialize with runtime for async operations
        Self {
            view_model: Arc::new(Mutex::new(AppViewModel::new().expect("Failed to init"))),
            runtime,
            // ... initialize other fields ...
        }
    }
}

// Include the full implementation from main.rs
// This is a stub - the actual implementation would include all methods
