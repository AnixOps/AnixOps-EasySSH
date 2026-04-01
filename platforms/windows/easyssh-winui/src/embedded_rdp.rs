#![allow(dead_code)]

//! Embedded Remote Desktop Viewer for Windows (Stub)
//!
//! This is a stub implementation. Full RDP support requires additional Windows API setup.

use eframe::egui;

/// Remote desktop viewer control (stub)
pub struct RemoteDesktopViewer;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RemoteDesktopType {
    Rdp,
    Vnc,
    SshX11,
}

#[derive(Debug, Clone)]
pub struct ConnectionSettings {
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: String,
    pub domain: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewerState {
    Disconnected,
    Connecting,
    Connected,
    Error,
}

#[derive(Debug, Clone)]
pub enum RemoteDesktopError {
    Connection(String),
    Authentication(String),
    WindowCreation(String),
    NotSupported,
}

impl std::fmt::Display for RemoteDesktopError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RemoteDesktopError::Connection(s) => write!(f, "Connection error: {}", s),
            RemoteDesktopError::Authentication(s) => write!(f, "Authentication error: {}", s),
            RemoteDesktopError::WindowCreation(s) => write!(f, "Window creation error: {}", s),
            RemoteDesktopError::NotSupported => write!(f, "RDP not supported in this build"),
        }
    }
}

impl std::error::Error for RemoteDesktopError {}

impl RemoteDesktopViewer {
    pub fn new(
        _parent: egui::Rect,
        _settings: ConnectionSettings,
        _type: RemoteDesktopType,
    ) -> Result<Self, RemoteDesktopError> {
        Err(RemoteDesktopError::NotSupported)
    }

    pub fn connect(&mut self) -> Result<(), RemoteDesktopError> {
        Err(RemoteDesktopError::NotSupported)
    }

    pub fn disconnect(&mut self) {
        // Stub
    }

    pub fn state(&self) -> ViewerState {
        ViewerState::Disconnected
    }

    pub fn render(&mut self, ui: &mut egui::Ui) {
        ui.label("Remote Desktop support is not available in this build.");
        ui.label("Please use an external RDP client.");
    }

    pub fn resize(&mut self, _width: u32, _height: u32) {
        // Stub
    }

    pub fn handle_input(&mut self, _event: &egui::Event) {
        // Stub
    }

    pub fn take_screenshot(&self) -> Option<Vec<u8>> {
        None
    }

    pub fn start_recording(&mut self, _path: std::path::PathBuf) -> Result<(), RemoteDesktopError> {
        Err(RemoteDesktopError::NotSupported)
    }

    pub fn stop_recording(&mut self) {
        // Stub
    }
}

/// Remote Desktop Manager UI
#[derive(Default)]
pub struct RemoteDesktopManagerUI;

impl RemoteDesktopManagerUI {
    pub fn new() -> Self {
        Self
    }

    pub fn render(&mut self, ui: &mut egui::Ui) {
        ui.heading("Remote Desktop");
        ui.label("RDP/VNC support is not available in this build.");
        ui.label("Please use an external client to connect to remote desktops.");
    }

    pub fn open_connection_dialog(&mut self) {}

    pub fn has_active_connections(&self) -> bool {
        false
    }
}

/// Recording state
#[derive(Debug, Clone)]
pub struct RecordingState;

/// Remote desktop viewer manager
#[derive(Default)]
pub struct RemoteDesktopViewerManager;

impl RemoteDesktopViewerManager {
    pub fn new() -> Self {
        Self
    }

    pub fn add_viewer(&mut self, _viewer: RemoteDesktopViewer) -> String {
        "stub".to_string()
    }

    pub fn remove_viewer(&mut self, _id: &str) {}

    pub fn get_viewer(&mut self, _id: &str) -> Option<&mut RemoteDesktopViewer> {
        None
    }

    pub fn render_viewer(&mut self, _id: &str, _ui: &mut egui::Ui) {
        // Stub
    }
}
