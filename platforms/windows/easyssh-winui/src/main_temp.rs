#![allow(dead_code)]

use eframe::egui;
use std::collections::{HashMap, HashSet};
use std::sync::mpsc as std_mpsc;
use std::sync::Arc;
use std::sync::Mutex;
use std::time::Instant;
use tokio::sync::mpsc;
use tracing::{error, info};
use uuid::Uuid;

#[cfg(feature = "code-editor")]
mod code_editor;
#[cfg(feature = "code-editor")]
use code_editor::{CodeEditor, FileInfo};

mod app_settings;
mod apple_design;
mod bridge;
mod cloud_sync_ui;
mod connection_pool_ui;
mod design;
mod file_icons;
mod file_preview;
mod file_preview_ui;
mod hotkey_helpers;
mod hotkeys;
mod key_manager_ui;
mod layout_manager;
mod notification_panel;
mod notifications;
mod pages;
mod performance;
mod performance_panel;
mod port_forward_dialog;
mod proxy_jump_ui;
mod search;
#[cfg(feature = "search-enhanced")]
mod search_ui;
mod session_manager_ui;
mod settings;
mod sftp_file_manager;
mod snippets;
mod snippets_ui;
#[cfg(feature = "backup")]
mod backup_restore_ui;
mod split_layout;
mod startup;
mod terminal;
mod terminal_search;
mod theme_system;
mod transfer_queue;

#[cfg(feature = "team")]
mod team_ui;

#[cfg(feature = "docker")]
mod docker_ui;
#[cfg(feature = "kubernetes")]
mod kubernetes_ui;

mod user_experience;
mod viewmodels;
mod ws_server;

#[cfg(feature = "file-preview")]
use file_preview_ui::{render_file_preview_panel, FilePreviewPanel};

#[cfg(feature = "database-client")]
mod database_client_ui;
#[cfg(feature = "database-client")]
use database_client_ui::{render_add_connection_dialog, render_database_client_panel, render_query_history_panel, DatabaseClientPanel};

use startup::{global_profiler, DeferredInitQueue};

#[cfg(feature = "remote-desktop")]
mod embedded_rdp;
#[cfg(feature = "remote-desktop")]
mod remote_desktop_ui;

#[cfg(feature = "workflow")]
mod batch_results_ui;
#[cfg(feature = "macro-recorder")]
mod macro_recorder_ui;
#[cfg(feature = "workflow")]
mod scheduled_tasks_ui;

#[cfg(feature = "audit")]
mod audit_log_ui;
#[cfg(feature = "workflow")]
mod workflow_editor;
#[cfg(feature = "workflow")]
mod workflow_panel;

#[cfg(feature = "sso")]
mod sso_login_ui;

#[cfg(feature = "ai-terminal")]
mod ai_terminal;
#[cfg(feature = "ai-terminal")]
mod ai_terminal_ui;

#[cfg(feature = "ai-terminal")]
use ai_terminal_ui::AiTerminalUi;

#[cfg(feature = "macro-recorder")]
use macro_recorder_ui::{MacroRecorderPanel, MacroRecorderResponse};

#[cfg(feature = "audit")]
use audit_log_ui::{AuditLogPanel, AuditLogWindow};

// New feature modules
use cloud_sync_ui::{CloudSyncUI, SyncStatistics};
use connection_pool_ui::{render_pool_status_widget, ConnectionPoolManagerUI};
use key_manager_ui::KeyManagerUI;
use proxy_jump_ui::{ProxyJumpUI, ServerInfo as ProxyServerInfo};
use session_manager_ui::{SessionInfo, SessionManagerUI};

use apple_design::MicrointeractionState;
use design::{AccessibilitySettings, DesignTheme};
use hotkeys::{CommandPalette, HotkeyManager, HotkeySettingsUI};
use layout_manager::SplitLayoutManager;
use notification_panel::{NotificationPanel, NotificationSettingsPanel};
use notifications::NotificationManager;
use performance_panel::PerformancePanel;
use search::{ConnectionStatusFilter, FilterCriteria, GlobalSearchEngine, QuickAction};
use settings::SettingsPanel;
use snippets::{SnippetCategory, SnippetInputDialog, SnippetManager};
use split_layout::{DropTarget, PanelId, PanelType};
use terminal::{RenderStats, WebGlTerminalManager};
use viewmodels::{AppViewModel, GroupViewModel, ServerViewModel};
use ws_server::{update_ui_debug, WsControlServer};

#[cfg(feature = "remote-desktop")]
use embedded_rdp::RemoteDesktopViewerManager;
#[cfg(feature = "remote-desktop")]
use remote_desktop_ui::{render_remote_desktop_panel, RemoteDesktopManagerUI};

use app_settings::SettingsManager;
use port_forward_dialog::PortForwardDialog;
use theme_system::{ThemeEditor, ThemeGallery, ThemeManager};
use user_experience::{
    LoadingOperation, OnboardingAction, OnboardingWizard, QuickTip, ToastNotification, UXManager,
};

#[cfg(feature = "team")]
use team_ui::{render_team_panel, TeamManagerUI};

#[cfg(feature = "audit")]
fn main() -> eframe::Result {
