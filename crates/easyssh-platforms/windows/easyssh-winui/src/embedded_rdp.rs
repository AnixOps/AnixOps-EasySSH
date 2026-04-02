//! Embedded Remote Desktop Viewer for Windows
//!
//! Supports launching external RDP/VNC clients as fallback for embedded view.

use eframe::egui;
use std::process::Command;

/// Remote desktop viewer control
pub struct RemoteDesktopViewer {
    settings: ConnectionSettings,
    viewer_type: RemoteDesktopType,
    state: ViewerState,
    error_message: Option<String>,
}

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
    ClientNotFound(String),
}

impl std::fmt::Display for RemoteDesktopError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RemoteDesktopError::Connection(s) => write!(f, "Connection error: {}", s),
            RemoteDesktopError::Authentication(s) => write!(f, "Authentication error: {}", s),
            RemoteDesktopError::WindowCreation(s) => write!(f, "Window creation error: {}", s),
            RemoteDesktopError::NotSupported => write!(f, "RDP not supported in this build"),
            RemoteDesktopError::ClientNotFound(s) => write!(f, "External client not found: {}", s),
        }
    }
}

impl std::error::Error for RemoteDesktopError {}

impl RemoteDesktopViewer {
    pub fn new(
        _parent: egui::Rect,
        settings: ConnectionSettings,
        viewer_type: RemoteDesktopType,
    ) -> Result<Self, RemoteDesktopError> {
        Ok(Self {
            settings,
            viewer_type,
            state: ViewerState::Disconnected,
            error_message: None,
        })
    }

    /// Connect using external client (mstsc for RDP, vncviewer for VNC)
    pub fn connect(&mut self) -> Result<(), RemoteDesktopError> {
        match self.viewer_type {
            RemoteDesktopType::Rdp => self.launch_mstsc(),
            RemoteDesktopType::Vnc => self.launch_vncviewer(),
            RemoteDesktopType::SshX11 => Err(RemoteDesktopError::NotSupported),
        }
    }

    /// Launch Windows built-in RDP client (mstsc.exe)
    fn launch_mstsc(&mut self) -> Result<(), RemoteDesktopError> {
        let mstsc_path = "C:\\Windows\\System32\\mstsc.exe";

        // Check if mstsc exists
        if !std::path::Path::new(mstsc_path).exists() {
            return Err(RemoteDesktopError::ClientNotFound(
                "mstsc.exe not found. Please ensure Remote Desktop is enabled.".to_string(),
            ));
        }

        // Create temporary .rdp file for connection settings
        let rdp_file = self.create_rdp_file()?;

        // Launch mstsc with the RDP file
        let _result = Command::new(mstsc_path)
            .arg(&rdp_file)
            .spawn()
            .map_err(|e| RemoteDesktopError::Connection(format!("Failed to start mstsc: {}", e)))?;

        self.state = ViewerState::Connected;
        println!(
            "Launched mstsc for RDP connection to {}:{}",
            self.settings.host, self.settings.port
        );

        Ok(())
    }

    /// Create temporary .rdp file with connection settings
    fn create_rdp_file(&self) -> Result<String, RemoteDesktopError> {
        use std::io::Write;

        let rdp_content = format!(
            "screen mode id:i:1\n\
use multimon:i:0\n\
desktopwidth:i:1920\n\
desktopheight:i:1080\n\
session bpp:i:32\n\
compression:i:1\n\
keyboardhook:i:2\n\
audiocapturemode:i:0\n\
videoplaybackmode:i:0\n\
connection type:i:7\n\
networkautodetect:i:1\n\
bandwidthautodetect:i:1\n\
enablecompression:i:1\n\
username:s:{}\n\
prompt for credentials:i:0\n\
negotiate security layer:i:1\n\
remoteapplicationmode:i:0\n\
gatewayusagemethod:i:0\n\
gatewayprofileusagemethod:i:1\n\
gatewaycredentialssource:i:0\n\
full address:s:{}:{}\n",
            self.settings.username, self.settings.host, self.settings.port
        );

        // Write to temp file
        let temp_dir = std::env::temp_dir();
        let rdp_file = temp_dir.join(format!("easyssh_rdp_{}.rdp", uuid::Uuid::new_v4()));

        let mut file = std::fs::File::create(&rdp_file).map_err(|e| {
            RemoteDesktopError::Connection(format!("Failed to create RDP file: {}", e))
        })?;

        file.write_all(rdp_content.as_bytes()).map_err(|e| {
            RemoteDesktopError::Connection(format!("Failed to write RDP file: {}", e))
        })?;

        Ok(rdp_file.to_string_lossy().to_string())
    }

    /// Launch VNC viewer (TightVNC/RealVNC/TigerVNC)
    fn launch_vncviewer(&mut self) -> Result<(), RemoteDesktopError> {
        // Try common VNC viewer locations
        let vnc_paths = [
            "C:\\Program Files\\TightVNC\\tvnviewer.exe",
            "C:\\Program Files (x86)\\TightVNC\\tvnviewer.exe",
            "C:\\Program Files\\RealVNC\\VNC Viewer\\vncviewer.exe",
            "C:\\Program Files (x86)\\RealVNC\\VNC Viewer\\vncviewer.exe",
            "C:\\Program Files\\TigerVNC\\vncviewer.exe",
        ];

        let vnc_path = vnc_paths
            .iter()
            .find(|p| std::path::Path::new(p).exists())
            .ok_or_else(|| {
                RemoteDesktopError::ClientNotFound(
                    "VNC viewer not found. Please install TightVNC, RealVNC, or TigerVNC."
                        .to_string(),
                )
            })?;

        let addr = format!("{}:{}", self.settings.host, self.settings.port);

        // Launch VNC viewer
        let _result = Command::new(vnc_path).arg(&addr).spawn().map_err(|e| {
            RemoteDesktopError::Connection(format!("Failed to start VNC viewer: {}", e))
        })?;

        self.state = ViewerState::Connected;
        println!("Launched VNC viewer for connection to {}", addr);

        Ok(())
    }

    pub fn disconnect(&mut self) {
        self.state = ViewerState::Disconnected;
    }

    pub fn state(&self) -> ViewerState {
        self.state
    }

    pub fn render(&mut self, ui: &mut egui::Ui) {
        match self.state {
            ViewerState::Connected => {
                ui.label("✅ External remote desktop client launched successfully.");
                ui.label(format!(
                    "Connected to {}:{}",
                    self.settings.host, self.settings.port
                ));
                ui.separator();
                ui.label("Note: The remote desktop is running in a separate window.");
                if ui.button("Disconnect").clicked() {
                    self.disconnect();
                }
            }
            ViewerState::Connecting => {
                ui.label("🔄 Launching external remote desktop client...");
            }
            ViewerState::Error => {
                ui.label("❌ Failed to launch remote desktop client.");
                if let Some(ref err) = self.error_message {
                    ui.label(format!("Error: {}", err));
                }
                if ui.button("Retry").clicked() {
                    self.state = ViewerState::Disconnected;
                }
            }
            ViewerState::Disconnected => {
                ui.label("Remote Desktop connection is ready to start.");
                if ui.button("Connect").clicked() {
                    if let Err(e) = self.connect() {
                        self.error_message = Some(e.to_string());
                        self.state = ViewerState::Error;
                    }
                }
            }
        }
    }

    pub fn resize(&mut self, _width: u32, _height: u32) {
        // External client handles its own window sizing
    }

    pub fn handle_input(&mut self, _event: &egui::Event) {
        // External client handles its own input
    }

    pub fn take_screenshot(&self) -> Option<Vec<u8>> {
        None // External client screenshots not supported
    }

    pub fn start_recording(&mut self, _path: std::path::PathBuf) -> Result<(), RemoteDesktopError> {
        Err(RemoteDesktopError::NotSupported)
    }

    pub fn stop_recording(&mut self) {
        // Not supported for external clients
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
        ui.label("RDP/VNC support uses external client.");
        ui.label("Connect to launch your default RDP/VNC viewer.");
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
pub struct RemoteDesktopViewerManager {
    viewers: std::collections::HashMap<String, RemoteDesktopViewer>,
}

impl RemoteDesktopViewerManager {
    pub fn new() -> Self {
        Self {
            viewers: std::collections::HashMap::new(),
        }
    }

    pub fn add_viewer(&mut self, viewer: RemoteDesktopViewer) -> String {
        let id = format!("rdp_{}", uuid::Uuid::new_v4());
        self.viewers.insert(id.clone(), viewer);
        id
    }

    pub fn remove_viewer(&mut self, id: &str) {
        self.viewers.remove(id);
    }

    pub fn get_viewer(&mut self, id: &str) -> Option<&mut RemoteDesktopViewer> {
        self.viewers.get_mut(id)
    }

    pub fn render_viewer(&mut self, id: &str, ui: &mut egui::Ui) {
        if let Some(viewer) = self.viewers.get_mut(id) {
            viewer.render(ui);
        } else {
            ui.label("Viewer not found");
        }
    }

    pub fn has_viewers(&self) -> bool {
        !self.viewers.is_empty()
    }

    pub fn viewer_ids(&self) -> Vec<String> {
        self.viewers.keys().cloned().collect()
    }
}
