use crate::error::LiteError;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex as TokioMutex;
use tokio::task::JoinHandle;

/// Remote desktop protocol types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RemoteDesktopProtocol {
    Rdp,
    Vnc,
    SshTunnelRdp,
    SshTunnelVnc,
}

impl RemoteDesktopProtocol {
    pub fn default_port(&self) -> u16 {
        match self {
            RemoteDesktopProtocol::Rdp | RemoteDesktopProtocol::SshTunnelRdp => 3389,
            RemoteDesktopProtocol::Vnc | RemoteDesktopProtocol::SshTunnelVnc => 5900,
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            RemoteDesktopProtocol::Rdp => "RDP",
            RemoteDesktopProtocol::Vnc => "VNC",
            RemoteDesktopProtocol::SshTunnelRdp => "SSH Tunnel RDP",
            RemoteDesktopProtocol::SshTunnelVnc => "SSH Tunnel VNC",
        }
    }

    pub fn requires_ssh_tunnel(&self) -> bool {
        matches!(
            self,
            RemoteDesktopProtocol::SshTunnelRdp | RemoteDesktopProtocol::SshTunnelVnc
        )
    }
}

/// Remote desktop connection settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteDesktopSettings {
    pub protocol: RemoteDesktopProtocol,
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: Option<String>,
    pub domain: Option<String>,
    /// SSH tunnel configuration (if using SSH tunnel protocols)
    pub ssh_tunnel: Option<SshTunnelConfig>,
    /// Display settings
    pub display: DisplaySettings,
    /// Performance settings
    pub performance: PerformanceSettings,
    /// Local resources redirection
    pub local_resources: LocalResourceSettings,
    /// Experience settings
    pub experience: ExperienceSettings,
    /// Gateway settings
    pub gateway: Option<GatewaySettings>,
}

impl Default for RemoteDesktopSettings {
    fn default() -> Self {
        Self {
            protocol: RemoteDesktopProtocol::Rdp,
            host: String::new(),
            port: 3389,
            username: String::new(),
            password: None,
            domain: None,
            ssh_tunnel: None,
            display: DisplaySettings::default(),
            performance: PerformanceSettings::default(),
            local_resources: LocalResourceSettings::default(),
            experience: ExperienceSettings::default(),
            gateway: None,
        }
    }
}

/// SSH tunnel configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SshTunnelConfig {
    pub ssh_host: String,
    pub ssh_port: u16,
    pub ssh_username: String,
    pub ssh_auth_type: String, // "password" or "key"
    pub ssh_password: Option<String>,
    pub ssh_key_path: Option<String>,
    pub remote_host: String, // Usually localhost or 127.0.0.1
    pub remote_port: u16,
    pub local_port: u16, // Local port to bind (0 for auto)
}

/// Display settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DisplaySettings {
    pub width: u32,
    pub height: u32,
    pub bpp: u8, // Bits per pixel: 8, 15, 16, 24, 32
    pub fullscreen: bool,
    pub multi_monitor: bool,
    pub smart_sizing: bool,
    pub dynamic_resolution: bool,
    pub fit_session_to_window: bool,
    pub desktop_scale_factor: u32, // 100-500%
}

impl Default for DisplaySettings {
    fn default() -> Self {
        Self {
            width: 1920,
            height: 1080,
            bpp: 32,
            fullscreen: false,
            multi_monitor: false,
            smart_sizing: true,
            dynamic_resolution: true,
            fit_session_to_window: true,
            desktop_scale_factor: 100,
        }
    }
}

/// Performance settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceSettings {
    pub connection_type: ConnectionType,
    pub disable_wallpaper: bool,
    pub disable_themes: bool,
    pub disable_menu_animations: bool,
    pub disable_full_window_drag: bool,
    pub disable_font_smoothing: bool,
    pub persistent_bitmap_caching: bool,
    pub compression: bool,
}

impl Default for PerformanceSettings {
    fn default() -> Self {
        Self {
            connection_type: ConnectionType::Lan,
            disable_wallpaper: false,
            disable_themes: false,
            disable_menu_animations: false,
            disable_full_window_drag: false,
            disable_font_smoothing: false,
            persistent_bitmap_caching: true,
            compression: true,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum ConnectionType {
    Modem,
    LowSpeedBroadband,
    Satellite,
    HighSpeedBroadband,
    Wan,
    Lan,
}

impl ConnectionType {
    pub fn display_name(&self) -> &'static str {
        match self {
            ConnectionType::Modem => "Modem (56 Kbps)",
            ConnectionType::LowSpeedBroadband => "Low-speed broadband (256 Kbps - 2 Mbps)",
            ConnectionType::Satellite => "Satellite (2 Mbps - 16 Mbps)",
            ConnectionType::HighSpeedBroadband => "High-speed broadband (2 Mbps - 10 Mbps)",
            ConnectionType::Wan => "WAN (10 Mbps+)",
            ConnectionType::Lan => "LAN (10 Mbps+)",
        }
    }
}

/// Local resource settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalResourceSettings {
    pub clipboard: bool,
    pub printer: bool,
    pub smart_cards: bool,
    pub ports: bool,
    pub drives: DriveRedirectionMode,
    pub audio: AudioRedirectionMode,
    pub microphone: bool,
    pub video_capture: bool,
}

impl Default for LocalResourceSettings {
    fn default() -> Self {
        Self {
            clipboard: true,
            printer: false,
            smart_cards: false,
            ports: false,
            drives: DriveRedirectionMode::Disabled,
            audio: AudioRedirectionMode::Client,
            microphone: false,
            video_capture: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DriveRedirectionMode {
    Disabled,
    LocalDrives,
    SpecificDrives(Vec<String>),
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum AudioRedirectionMode {
    Server,
    Client,
    DoNotPlay,
}

/// Experience settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExperienceSettings {
    pub desktop_background: bool,
    pub font_smoothing: bool,
    pub desktop_composition: bool,
    pub show_window_contents: bool,
    pub menu_window_animation: bool,
    pub visual_styles: bool,
    pub reconnect_on_disconnect: bool,
    pub auto_reconnect: bool,
    pub auto_reconnect_max_attempts: u32,
}

impl Default for ExperienceSettings {
    fn default() -> Self {
        Self {
            desktop_background: true,
            font_smoothing: true,
            desktop_composition: true,
            show_window_contents: true,
            menu_window_animation: true,
            visual_styles: true,
            reconnect_on_disconnect: true,
            auto_reconnect: true,
            auto_reconnect_max_attempts: 20,
        }
    }
}

/// Gateway settings for RDS gateway
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GatewaySettings {
    pub server: String,
    pub auth_method: GatewayAuthMethod,
    pub logon_method: GatewayLogonMethod,
    pub use_cached_credentials: bool,
    pub bypass_for_local: bool,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum GatewayAuthMethod {
    Password,
    SmartCard,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum GatewayLogonMethod {
    AllowUserToSelect,
    AskForPassword,
    SmartCard,
}

/// Session recording settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordingSettings {
    pub enabled: bool,
    pub output_path: String,
    pub format: RecordingFormat,
    pub quality: RecordingQuality,
    pub include_audio: bool,
    pub max_file_size_mb: u64,
    pub auto_start: bool,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum RecordingFormat {
    Mkv,
    Mp4,
    Avi,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum RecordingQuality {
    Low,
    Medium,
    High,
    Lossless,
}

/// Remote desktop session metadata
#[derive(Debug, Clone, Serialize)]
pub struct RemoteDesktopSession {
    pub id: String,
    pub protocol: RemoteDesktopProtocol,
    pub host: String,
    pub port: u16,
    pub username: String,
    #[serde(skip)]
    pub connected_at: Instant,
    pub status: SessionStatus,
    #[serde(skip)]
    pub settings: RemoteDesktopSettings,
    pub recording_active: bool,
    pub recording_path: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SessionStatus {
    Connecting,
    Connected,
    Disconnected,
    Error,
    Recording,
}

impl SessionStatus {
    pub fn display_name(&self) -> &'static str {
        match self {
            SessionStatus::Connecting => "Connecting",
            SessionStatus::Connected => "Connected",
            SessionStatus::Disconnected => "Disconnected",
            SessionStatus::Error => "Error",
            SessionStatus::Recording => "Recording",
        }
    }
}

/// Active SSH tunnel for remote desktop
struct ActiveSshTunnel {
    session: Arc<TokioMutex<ssh2::Session>>,
    local_port: u16,
    _handle: JoinHandle<()>,
}

/// Remote desktop connection manager
pub struct RemoteDesktopManager {
    sessions: HashMap<String, RemoteDesktopSession>,
    active_tunnels: HashMap<String, ActiveSshTunnel>,
    recording_sessions: HashMap<String, RecordingState>,
    stop_flags: HashMap<String, Arc<AtomicBool>>,
}

struct RecordingState {
    start_time: Instant,
    output_path: String,
    format: RecordingFormat,
    _handle: JoinHandle<()>,
}

impl RemoteDesktopManager {
    pub fn new() -> Self {
        Self {
            sessions: HashMap::new(),
            active_tunnels: HashMap::new(),
            recording_sessions: HashMap::new(),
            stop_flags: HashMap::new(),
        }
    }

    /// Create a new remote desktop session
    pub fn create_session(
        &mut self,
        settings: RemoteDesktopSettings,
    ) -> Result<RemoteDesktopSession, LiteError> {
        let session_id = uuid::Uuid::new_v4().to_string();

        let session = RemoteDesktopSession {
            id: session_id.clone(),
            protocol: settings.protocol,
            host: settings.host.clone(),
            port: settings.port,
            username: settings.username.clone(),
            connected_at: Instant::now(),
            status: SessionStatus::Connecting,
            settings,
            recording_active: false,
            recording_path: None,
        };

        self.sessions.insert(session_id.clone(), session.clone());
        self.stop_flags
            .insert(session_id, Arc::new(AtomicBool::new(false)));

        Ok(session)
    }

    /// Start SSH tunnel for tunnel-based protocols
    pub async fn start_ssh_tunnel(
        &mut self,
        session_id: &str,
        tunnel_config: &SshTunnelConfig,
    ) -> Result<u16, LiteError> {
        use std::io::{Read, Write};
        use std::net::TcpStream;

        // Create SSH connection
        let addr = format!("{}:{}", tunnel_config.ssh_host, tunnel_config.ssh_port);
        let tcp = TcpStream::connect(&addr).map_err(|e| LiteError::SshConnectionFailed {
            host: tunnel_config.ssh_host.clone(),
            port: tunnel_config.ssh_port,
            message: e.to_string(),
        })?;

        tcp.set_read_timeout(Some(Duration::from_secs(30)))
            .map_err(|e| LiteError::Io(e.to_string()))?;
        tcp.set_write_timeout(Some(Duration::from_secs(30)))
            .map_err(|e| LiteError::Io(e.to_string()))?;

        let mut session = ssh2::Session::new().map_err(|e| LiteError::Ssh(e.to_string()))?;
        session.set_tcp_stream(tcp);
        session
            .handshake()
            .map_err(|e| LiteError::Ssh(format!("Handshake failed: {}", e)))?;

        // Authenticate
        match &tunnel_config.ssh_password {
            Some(pwd) if tunnel_config.ssh_auth_type == "password" => {
                session
                    .userauth_password(&tunnel_config.ssh_username, pwd)
                    .map_err(|_| LiteError::SshAuthFailed {
                        host: tunnel_config.ssh_host.clone(),
                        username: tunnel_config.ssh_username.clone(),
                    })?;
            }
            _ => {
                session
                    .userauth_agent(&tunnel_config.ssh_username)
                    .map_err(|_| LiteError::SshAuthFailed {
                        host: tunnel_config.ssh_host.clone(),
                        username: tunnel_config.ssh_username.clone(),
                    })?;
            }
        }

        if !session.authenticated() {
            return Err(LiteError::SshAuthFailed {
                host: tunnel_config.ssh_host.clone(),
                username: tunnel_config.ssh_username.clone(),
            });
        }

        // Find available local port
        let local_port = if tunnel_config.local_port > 0 {
            tunnel_config.local_port
        } else {
            find_available_port().await?
        };

        let remote_host = tunnel_config.remote_host.clone();
        let remote_port = tunnel_config.remote_port;
        let session_arc = Arc::new(TokioMutex::new(session));
        let session_clone = session_arc.clone();
        let stop_flag = self
            .stop_flags
            .get(session_id)
            .ok_or_else(|| LiteError::SessionNotFound(session_id.to_string()))?
            .clone();

        // Start tunnel forwarding
        let handle = tokio::spawn(async move {
            let listener =
                match tokio::net::TcpListener::bind(format!("127.0.0.1:{}", local_port)).await {
                    Ok(l) => l,
                    Err(e) => {
                        log::error!("Failed to bind local port {}: {}", local_port, e);
                        return;
                    }
                };

            log::info!("SSH tunnel listening on port {}", local_port);

            loop {
                if stop_flag.load(Ordering::Relaxed) {
                    break;
                }

                match tokio::time::timeout(Duration::from_secs(1), listener.accept()).await {
                    Ok(Ok((local_stream, _))) => {
                        let session = session_clone.clone();
                        let remote_host = remote_host.clone();
                        let stop_flag = stop_flag.clone();

                        tokio::spawn(async move {
                            let local_stream = local_stream;

                            // Convert to std TcpStream for blocking I/O
                            let local_stream = match local_stream.into_std() {
                                Ok(s) => s,
                                Err(e) => {
                                    log::error!("Failed to convert to std stream: {}", e);
                                    return;
                                }
                            };
                            // Set non-blocking to false for synchronous operations
                            if let Err(e) = local_stream.set_nonblocking(false) {
                                log::error!("Failed to set blocking mode: {}", e);
                                return;
                            }

                            // Run tunnel in blocking context
                            tokio::task::spawn_blocking(move || {
                                let mut local_stream = local_stream;
                                let session_guard = match session.try_lock() {
                                    Ok(g) => g,
                                    Err(_) => {
                                        log::error!("Failed to acquire session lock");
                                        return;
                                    }
                                };

                                let mut channel = match session_guard.channel_direct_tcpip(
                                    &remote_host,
                                    remote_port,
                                    None,
                                ) {
                                    Ok(ch) => ch,
                                    Err(e) => {
                                        log::error!("Failed to create tunnel channel: {}", e);
                                        return;
                                    }
                                };
                                drop(session_guard); // Release lock

                                // Bidirectional forwarding using blocking I/O
                                let mut local_buf = [0u8; 4096];
                                let mut remote_buf = [0u8; 4096];

                                loop {
                                    if stop_flag.load(Ordering::Relaxed) {
                                        break;
                                    }

                                    // Read from local and write to remote
                                    match std::io::Read::read(&mut local_stream, &mut local_buf) {
                                        Ok(0) => break,
                                        Ok(n) => {
                                            if channel.write(&local_buf[..n]).is_err() {
                                                break;
                                            }
                                        }
                                        Err(_) => break,
                                    }

                                    if stop_flag.load(Ordering::Relaxed) {
                                        break;
                                    }

                                    // Read from remote and write to local
                                    match channel.read(&mut remote_buf) {
                                        Ok(0) => break,
                                        Ok(n) => {
                                            if std::io::Write::write_all(
                                                &mut local_stream,
                                                &remote_buf[..n],
                                            )
                                            .is_err()
                                            {
                                                break;
                                            }
                                        }
                                        Err(_) => break,
                                    }
                                }
                            });
                        });
                    }
                    Ok(Err(e)) => {
                        log::error!("Tunnel accept error: {}", e);
                        break;
                    }
                    Err(_) => continue, // Timeout, check stop flag
                }
            }

            log::info!("SSH tunnel on port {} stopped", local_port);
        });

        let tunnel = ActiveSshTunnel {
            session: session_arc,
            local_port,
            _handle: handle,
        };

        self.active_tunnels.insert(session_id.to_string(), tunnel);

        Ok(local_port)
    }

    /// Get session by ID
    pub fn get_session(&self, session_id: &str) -> Option<&RemoteDesktopSession> {
        self.sessions.get(session_id)
    }

    /// Get mutable session
    pub fn get_session_mut(&mut self, session_id: &str) -> Option<&mut RemoteDesktopSession> {
        self.sessions.get_mut(session_id)
    }

    /// Update session status
    pub fn update_status(&mut self, session_id: &str, status: SessionStatus) {
        if let Some(session) = self.sessions.get_mut(session_id) {
            session.status = status;
        }
    }

    /// List all sessions
    pub fn list_sessions(&self) -> Vec<&RemoteDesktopSession> {
        self.sessions.values().collect()
    }

    /// Disconnect session
    pub async fn disconnect(&mut self, session_id: &str) -> Result<(), LiteError> {
        // Stop recording if active
        if let Some(session) = self.sessions.get(session_id) {
            if session.recording_active {
                self.stop_recording(session_id).await?;
            }
        }

        // Stop SSH tunnel if exists
        if let Some(_tunnel) = self.active_tunnels.remove(session_id) {
            // Stop flag is already checked by tunnel loop
            log::info!("Stopped SSH tunnel for session {}", session_id);
        }

        // Set stop flag
        if let Some(stop_flag) = self.stop_flags.get(session_id) {
            stop_flag.store(true, Ordering::Relaxed);
        }

        // Update status
        if let Some(session) = self.sessions.get_mut(session_id) {
            session.status = SessionStatus::Disconnected;
        }

        log::info!("Remote desktop session {} disconnected", session_id);
        Ok(())
    }

    /// Remove session
    pub async fn remove_session(&mut self, session_id: &str) -> Result<(), LiteError> {
        self.disconnect(session_id).await?;
        self.sessions.remove(session_id);
        self.stop_flags.remove(session_id);
        Ok(())
    }

    /// Start session recording
    pub async fn start_recording(
        &mut self,
        session_id: &str,
        settings: RecordingSettings,
    ) -> Result<String, LiteError> {
        if !self.sessions.contains_key(session_id) {
            return Err(LiteError::SessionNotFound(session_id.to_string()));
        }

        let output_path = if settings.output_path.is_empty() {
            let default_dir = dirs::video_dir()
                .or_else(|| dirs::home_dir().map(|h| h.join("Videos")))
                .unwrap_or_else(|| std::env::temp_dir());

            let filename = format!(
                "easyssh_recording_{}_{}.{}",
                session_id,
                chrono::Local::now().format("%Y%m%d_%H%M%S"),
                match settings.format {
                    RecordingFormat::Mkv => "mkv",
                    RecordingFormat::Mp4 => "mp4",
                    RecordingFormat::Avi => "avi",
                }
            );

            default_dir.join(filename).to_string_lossy().to_string()
        } else {
            settings.output_path.clone()
        };

        // Ensure directory exists
        if let Some(parent) = std::path::Path::new(&output_path).parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                LiteError::Io(format!("Failed to create recording directory: {}", e))
            })?;
        }

        let stop_flag = self
            .stop_flags
            .get(session_id)
            .ok_or_else(|| LiteError::SessionNotFound(session_id.to_string()))?
            .clone();

        let path_clone = output_path.clone();
        let handle = tokio::spawn(async move {
            // Recording implementation would integrate with screen capture APIs
            // For now, we create a placeholder that monitors the session
            log::info!("Recording started: {}", path_clone);

            loop {
                if stop_flag.load(Ordering::Relaxed) {
                    break;
                }
                tokio::time::sleep(Duration::from_secs(1)).await;
            }

            log::info!("Recording stopped: {}", path_clone);
        });

        let recording = RecordingState {
            start_time: Instant::now(),
            output_path: output_path.clone(),
            format: settings.format,
            _handle: handle,
        };

        self.recording_sessions
            .insert(session_id.to_string(), recording);

        // Update session
        if let Some(session) = self.sessions.get_mut(session_id) {
            session.recording_active = true;
            session.recording_path = Some(output_path.clone());
            session.status = SessionStatus::Recording;
        }

        Ok(output_path)
    }

    /// Stop session recording
    pub async fn stop_recording(&mut self, session_id: &str) -> Result<String, LiteError> {
        let recording = self
            .recording_sessions
            .remove(session_id)
            .ok_or_else(|| LiteError::RecordingError("No active recording".to_string()))?;

        // Update session
        if let Some(session) = self.sessions.get_mut(session_id) {
            session.recording_active = false;
            let path = session.recording_path.clone();
            session.recording_path = None;
            session.status = SessionStatus::Connected;

            log::info!(
                "Recording stopped. Duration: {:?}, Path: {}",
                recording.start_time.elapsed(),
                recording.output_path
            );

            Ok(path.unwrap_or(recording.output_path))
        } else {
            Ok(recording.output_path)
        }
    }

    /// Check if recording is active
    pub fn is_recording(&self, session_id: &str) -> bool {
        self.recording_sessions.contains_key(session_id)
    }

    /// Get recording duration
    pub fn get_recording_duration(&self, session_id: &str) -> Option<Duration> {
        self.recording_sessions
            .get(session_id)
            .map(|r| r.start_time.elapsed())
    }

    /// Generate FreeRDP command line arguments
    #[cfg(any(target_os = "windows", target_os = "linux", target_os = "macos"))]
    pub fn generate_freerdp_args(&self, session_id: &str) -> Result<Vec<String>, LiteError> {
        let session = self
            .sessions
            .get(session_id)
            .ok_or_else(|| LiteError::SessionNotFound(session_id.to_string()))?;

        let settings = &session.settings;
        let mut args = vec![];

        // Host and port
        let connect_host = if let Some(tunnel) = self.active_tunnels.get(session_id) {
            format!("127.0.0.1:{}", tunnel.local_port)
        } else {
            format!("{}:{}", settings.host, settings.port)
        };
        args.push(format!("/v:{}", connect_host));

        // Credentials
        args.push(format!("/u:{}", settings.username));
        if let Some(domain) = &settings.domain {
            args.push(format!("/d:{}", domain));
        }
        if let Some(password) = &settings.password {
            // Note: In production, use credential manager instead
            args.push(format!("/p:{}", password));
        }

        // Display settings
        let display = &settings.display;
        if display.fullscreen {
            args.push("/f".to_string());
        } else {
            args.push(format!("/w:{}", display.width));
            args.push(format!("/h:{}", display.height));
        }

        if display.multi_monitor {
            args.push("/multimon".to_string());
        }

        if display.dynamic_resolution {
            args.push("/dynamic-resolution".to_string());
        }

        if display.smart_sizing {
            args.push("/smart-sizing".to_string());
        }

        // Performance settings
        let perf = &settings.performance;
        match perf.connection_type {
            ConnectionType::Modem => args.push("/network:modem".to_string()),
            ConnectionType::LowSpeedBroadband => args.push("/network:broadband".to_string()),
            ConnectionType::Satellite => args.push("/network:satellite".to_string()),
            ConnectionType::HighSpeedBroadband => args.push("/network:broadband-high".to_string()),
            ConnectionType::Wan => args.push("/network:wan".to_string()),
            ConnectionType::Lan => args.push("/network:lan".to_string()),
        }

        if perf.disable_wallpaper {
            args.push("-wallpaper".to_string());
        }
        if perf.disable_themes {
            args.push("-themes".to_string());
        }
        if perf.disable_menu_animations {
            args.push("-menu-anims".to_string());
        }
        if perf.compression {
            args.push("/compression".to_string());
        }

        // Local resources
        let local_res = &settings.local_resources;
        if local_res.clipboard {
            args.push("+clipboard".to_string());
        } else {
            args.push("-clipboard".to_string());
        }

        if local_res.printer {
            args.push("/printer".to_string());
        }

        match local_res.drives {
            DriveRedirectionMode::LocalDrives => {
                args.push("/drive:*,\\tsclient\\*".to_string());
            }
            DriveRedirectionMode::SpecificDrives(ref drives) => {
                for drive in drives {
                    args.push(format!("/drive:{},\\tsclient\\{}", drive, drive));
                }
            }
            DriveRedirectionMode::Disabled => {}
        }

        // Audio
        match local_res.audio {
            AudioRedirectionMode::Server => args.push("/audio-mode:0".to_string()),
            AudioRedirectionMode::Client => args.push("/audio-mode:1".to_string()),
            AudioRedirectionMode::DoNotPlay => args.push("/audio-mode:2".to_string()),
        }

        if local_res.microphone {
            args.push("/microphone".to_string());
        }

        // Experience settings
        let exp = &settings.experience;
        if exp.auto_reconnect {
            args.push(format!(
                "/auto-reconnect-max-attempts:{}",
                exp.auto_reconnect_max_attempts
            ));
        }

        // Gateway
        if let Some(gateway) = &settings.gateway {
            args.push(format!("/g:{}", gateway.server));
            match gateway.auth_method {
                GatewayAuthMethod::Password => args.push("/gt:password".to_string()),
                GatewayAuthMethod::SmartCard => args.push("/gt:smartcard".to_string()),
            }
        }

        // Bitmap caching
        if perf.persistent_bitmap_caching {
            args.push("/bpp:32".to_string());
            args.push("/cache:bitmap".to_string());
        } else {
            args.push(format!("/bpp:{}", display.bpp));
        }

        Ok(args)
    }

    /// Generate TigerVNC command line arguments
    pub fn generate_vnc_args(&self, session_id: &str) -> Result<Vec<String>, LiteError> {
        let session = self
            .sessions
            .get(session_id)
            .ok_or_else(|| LiteError::SessionNotFound(session_id.to_string()))?;

        let settings = &session.settings;
        let mut args = vec![];

        // Host and port
        let connect_host = if let Some(tunnel) = self.active_tunnels.get(session_id) {
            format!("127.0.0.1:{}", tunnel.local_port)
        } else {
            format!("{}:{}", settings.host, settings.port)
        };
        args.push(connect_host);

        // Quality and compression settings
        match settings.performance.connection_type {
            ConnectionType::Modem | ConnectionType::LowSpeedBroadband => {
                args.push("-PreferredEncoding=ZRLE".to_string());
                args.push("-CompressLevel=9".to_string());
                args.push("-QualityLevel=3".to_string());
            }
            ConnectionType::Satellite => {
                args.push("-PreferredEncoding=ZRLE".to_string());
                args.push("-CompressLevel=6".to_string());
                args.push("-QualityLevel=6".to_string());
            }
            _ => {
                args.push("-PreferredEncoding=Tight".to_string());
                args.push("-CompressLevel=2".to_string());
                args.push("-QualityLevel=9".to_string());
            }
        }

        // Fullscreen
        if settings.display.fullscreen {
            args.push("-FullScreen".to_string());
        }

        // Clipboard
        if settings.local_resources.clipboard {
            args.push("-SendClipboard".to_string());
            args.push("-AcceptClipboard".to_string());
        }

        Ok(args)
    }

    /// Cleanup expired sessions
    pub fn cleanup(&mut self) {
        let to_remove: Vec<String> = self
            .sessions
            .iter()
            .filter(|(_, s)| s.status == SessionStatus::Disconnected)
            .map(|(id, _)| id.clone())
            .collect();

        for id in to_remove {
            self.sessions.remove(&id);
            self.stop_flags.remove(&id);
            self.active_tunnels.remove(&id);
            self.recording_sessions.remove(&id);
        }
    }

    /// Get session count
    pub fn session_count(&self) -> usize {
        self.sessions.len()
    }

    /// Get active session count
    pub fn active_session_count(&self) -> usize {
        self.sessions
            .values()
            .filter(|s| {
                s.status == SessionStatus::Connected || s.status == SessionStatus::Recording
            })
            .count()
    }
}

impl Default for RemoteDesktopManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Find an available local port
async fn find_available_port() -> Result<u16, LiteError> {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .map_err(|e| LiteError::Io(format!("Failed to find available port: {}", e)))?;

    let addr = listener
        .local_addr()
        .map_err(|e| LiteError::Io(format!("Failed to get local address: {}", e)))?;

    // Drop the listener so the port becomes available
    drop(listener);

    // Small delay to ensure port is released
    tokio::time::sleep(Duration::from_millis(100)).await;

    Ok(addr.port())
}

/// RDP connection via Windows RDP client (mstsc)
#[cfg(target_os = "windows")]
pub fn generate_mstsc_rdp_file(settings: &RemoteDesktopSettings) -> Result<String, LiteError> {
    let mut rdp_content = String::new();

    rdp_content.push_str(&format!(
        "full address:s:{}:{}\n",
        settings.host, settings.port
    ));
    rdp_content.push_str(&format!("username:s:{}\n", settings.username));

    if let Some(domain) = &settings.domain {
        rdp_content.push_str(&format!("domain:s:{}\n", domain));
    }

    let display = &settings.display;
    if display.fullscreen {
        rdp_content.push_str("screen mode id:i:2\n");
    } else {
        rdp_content.push_str("screen mode id:i:1\n");
        rdp_content.push_str(&format!("desktopwidth:i:{}\n", display.width));
        rdp_content.push_str(&format!("desktopheight:i:{}\n", display.height));
    }

    if display.multi_monitor {
        rdp_content.push_str("use multimon:i:1\n");
        rdp_content.push_str(&format!("selectedmonitors:s:{}\n", "0,1"));
    }

    if display.smart_sizing {
        rdp_content.push_str("smart sizing:i:1\n");
    }

    if display.dynamic_resolution {
        rdp_content.push_str("dynamic resolution:i:1\n");
    }

    rdp_content.push_str(&format!(
        "desktopscalefactor:i:{}\n",
        display.desktop_scale_factor
    ));

    // Performance settings
    let perf = &settings.performance;
    rdp_content.push_str(&format!(
        "connection type:i:{}\n",
        match perf.connection_type {
            ConnectionType::Modem => 1,
            ConnectionType::LowSpeedBroadband => 2,
            ConnectionType::Satellite => 3,
            ConnectionType::HighSpeedBroadband => 4,
            ConnectionType::Wan => 5,
            ConnectionType::Lan => 6,
        }
    ));

    if perf.disable_wallpaper {
        rdp_content.push_str("disable wallpaper:i:1\n");
    }
    if perf.disable_themes {
        rdp_content.push_str("disable themes:i:1\n");
    }
    if perf.disable_menu_animations {
        rdp_content.push_str("disable menu anims:i:1\n");
    }
    if perf.disable_full_window_drag {
        rdp_content.push_str("disable full window drag:i:1\n");
    }
    if perf.disable_font_smoothing {
        rdp_content.push_str("disable font smoothing:i:1\n");
    }

    // Local resources
    let local_res = &settings.local_resources;

    // Clipboard
    rdp_content.push_str(&format!(
        "redirectclipboard:i:{}\n",
        if local_res.clipboard { 1 } else { 0 }
    ));

    // Printers
    rdp_content.push_str(&format!(
        "redirectprinters:i:{}\n",
        if local_res.printer { 1 } else { 0 }
    ));

    // Smart cards
    rdp_content.push_str(&format!(
        "redirectsmartcards:i:{}\n",
        if local_res.smart_cards { 1 } else { 0 }
    ));

    // COM ports
    rdp_content.push_str(&format!(
        "redirectcomports:i:{}\n",
        if local_res.ports { 1 } else { 0 }
    ));

    // Drives
    match local_res.drives {
        DriveRedirectionMode::Disabled => {
            rdp_content.push_str("redirectdrives:i:0\n");
        }
        DriveRedirectionMode::LocalDrives => {
            rdp_content.push_str("redirectdrives:i:1\n");
        }
        DriveRedirectionMode::SpecificDrives(ref drives) => {
            rdp_content.push_str("redirectdrives:i:1\n");
            rdp_content.push_str(&format!("drivestoredirect:s:{}\n", drives.join(";")));
        }
    }

    // Audio
    rdp_content.push_str(&format!(
        "audiomode:i:{}\n",
        match local_res.audio {
            AudioRedirectionMode::Server => 0,
            AudioRedirectionMode::Client => 1,
            AudioRedirectionMode::DoNotPlay => 2,
        }
    ));

    rdp_content.push_str(&format!(
        "audiocapturemode:i:{}\n",
        if local_res.microphone { 1 } else { 0 }
    ));

    // Video capture
    if local_res.video_capture {
        rdp_content.push_str("camerastoredirect:s:*\n");
    }

    // Experience
    let exp = &settings.experience;
    rdp_content.push_str(&format!(
        "allow desktop composition:i:{}\n",
        if exp.desktop_composition { 1 } else { 0 }
    ));
    rdp_content.push_str(&format!(
        "allow font smoothing:i:{}\n",
        if exp.font_smoothing { 1 } else { 0 }
    ));
    rdp_content.push_str(&format!(
        "disable full window drag:i:{}\n",
        if exp.show_window_contents { 0 } else { 1 }
    ));
    rdp_content.push_str(&format!(
        "disable menu anims:i:{}\n",
        if exp.menu_window_animation { 0 } else { 1 }
    ));
    rdp_content.push_str(&format!(
        "disable themes:i:{}\n",
        if exp.visual_styles { 0 } else { 1 }
    ));
    rdp_content.push_str(&format!(
        "disable wallpaper:i:{}\n",
        if exp.desktop_background { 0 } else { 1 }
    ));

    rdp_content.push_str(&format!(
        "autoreconnection enabled:i:{}\n",
        if exp.auto_reconnect { 1 } else { 0 }
    ));
    rdp_content.push_str(&format!(
        "autoreconnect max retries:i:{}\n",
        exp.auto_reconnect_max_attempts
    ));

    // Gateway
    if let Some(gateway) = &settings.gateway {
        rdp_content.push_str(&format!("gatewayhostname:s:{}\n", gateway.server));
        rdp_content.push_str(&format!(
            "gatewayusagemethod:i:{}\n",
            if gateway.bypass_for_local { 2 } else { 1 }
        ));
        rdp_content.push_str(&format!(
            "gatewaycredentialssource:i:{}\n",
            if gateway.use_cached_credentials { 1 } else { 0 }
        ));
        rdp_content.push_str(&format!(
            "gatewayprofileusagemethod:i:{}\n",
            match gateway.logon_method {
                GatewayLogonMethod::AllowUserToSelect => 0,
                GatewayLogonMethod::AskForPassword => 1,
                GatewayLogonMethod::SmartCard => 2,
            }
        ));

        match gateway.auth_method {
            GatewayAuthMethod::Password => {
                rdp_content.push_str("gatewayauthtype:i:0\n");
            }
            GatewayAuthMethod::SmartCard => {
                rdp_content.push_str("gatewayauthtype:i:1\n");
            }
        }
    }

    // Bitmap caching
    if perf.persistent_bitmap_caching {
        rdp_content.push_str("bitmapcachepersistenable:i:1\n");
        rdp_content.push_str(&format!("session bpp:i:{}\n", 32));
    } else {
        rdp_content.push_str(&format!("session bpp:i:{}\n", display.bpp));
    }

    rdp_content.push_str("use redirection server name:i:0\n");
    rdp_content.push_str("redirectlocation:i:0\n");

    Ok(rdp_content)
}

/// Launch RDP connection using Windows mstsc
#[cfg(target_os = "windows")]
pub async fn launch_windows_rdp(settings: &RemoteDesktopSettings) -> Result<(), LiteError> {
    use std::process::Command;

    let rdp_content = generate_mstsc_rdp_file(settings)?;

    // Create temporary RDP file
    let temp_dir = std::env::temp_dir();
    let rdp_file = temp_dir.join(format!("easyssh_{}.rdp", uuid::Uuid::new_v4()));

    tokio::fs::write(&rdp_file, rdp_content)
        .await
        .map_err(|e| LiteError::Io(format!("Failed to write RDP file: {}", e)))?;

    // Launch mstsc
    let mut cmd = Command::new("mstsc");
    cmd.arg(&rdp_file);

    // Add password if available (through command line is not secure, use credential manager in production)
    if settings.password.is_some() {
        // Note: mstsc doesn't support password via command line for security reasons
        // We would need to use the Windows Credential Manager or WTS APIs
        log::warn!("Password-based RDP requires manual entry or credential manager integration");
    }

    std::thread::spawn(move || {
        let _ = cmd.spawn();

        // Clean up temp file after a delay
        std::thread::sleep(Duration::from_secs(30));
        let _ = std::fs::remove_file(&rdp_file);
    });

    Ok(())
}

/// Launch FreeRDP connection
pub async fn launch_freerdp(
    session_id: &str,
    manager: &RemoteDesktopManager,
) -> Result<(), LiteError> {
    use std::process::Command;

    let args = manager.generate_freerdp_args(session_id)?;

    let mut cmd = Command::new("xfreerdp3");
    cmd.args(&args);

    std::thread::spawn(move || {
        let _ = cmd.spawn();
    });

    Ok(())
}

/// Launch TigerVNC connection
pub async fn launch_vnc(session_id: &str, manager: &RemoteDesktopManager) -> Result<(), LiteError> {
    use std::process::Command;

    let args = manager.generate_vnc_args(session_id)?;

    let mut cmd = Command::new("vncviewer");
    cmd.args(&args);

    std::thread::spawn(move || {
        let _ = cmd.spawn();
    });

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_remote_desktop_protocol_default_port() {
        assert_eq!(RemoteDesktopProtocol::Rdp.default_port(), 3389);
        assert_eq!(RemoteDesktopProtocol::Vnc.default_port(), 5900);
        assert_eq!(RemoteDesktopProtocol::SshTunnelRdp.default_port(), 3389);
        assert_eq!(RemoteDesktopProtocol::SshTunnelVnc.default_port(), 5900);
    }

    #[test]
    fn test_remote_desktop_protocol_requires_ssh_tunnel() {
        assert!(!RemoteDesktopProtocol::Rdp.requires_ssh_tunnel());
        assert!(!RemoteDesktopProtocol::Vnc.requires_ssh_tunnel());
        assert!(RemoteDesktopProtocol::SshTunnelRdp.requires_ssh_tunnel());
        assert!(RemoteDesktopProtocol::SshTunnelVnc.requires_ssh_tunnel());
    }

    #[test]
    fn test_display_settings_default() {
        let settings = DisplaySettings::default();
        assert_eq!(settings.width, 1920);
        assert_eq!(settings.height, 1080);
        assert_eq!(settings.bpp, 32);
        assert!(!settings.fullscreen);
        assert!(settings.smart_sizing);
    }

    #[test]
    fn test_performance_settings_default() {
        let settings = PerformanceSettings::default();
        assert!(settings.persistent_bitmap_caching);
        assert!(settings.compression);
    }

    #[test]
    fn test_local_resource_settings_default() {
        let settings = LocalResourceSettings::default();
        assert!(settings.clipboard);
        assert!(!settings.printer);
        assert!(!settings.microphone);
    }

    #[test]
    fn test_session_status_display_name() {
        assert_eq!(SessionStatus::Connecting.display_name(), "Connecting");
        assert_eq!(SessionStatus::Connected.display_name(), "Connected");
        assert_eq!(SessionStatus::Disconnected.display_name(), "Disconnected");
    }

    #[test]
    fn test_manager_create_session() {
        let mut manager = RemoteDesktopManager::new();
        let settings = RemoteDesktopSettings::default();

        let session = manager.create_session(settings).unwrap();
        assert_eq!(session.protocol, RemoteDesktopProtocol::Rdp);
        assert_eq!(session.status, SessionStatus::Connecting);

        assert_eq!(manager.session_count(), 1);
    }

    #[test]
    fn test_manager_list_sessions() {
        let mut manager = RemoteDesktopManager::new();
        let settings = RemoteDesktopSettings::default();

        manager.create_session(settings.clone()).unwrap();
        manager.create_session(settings).unwrap();

        let sessions = manager.list_sessions();
        assert_eq!(sessions.len(), 2);
    }

    #[test]
    fn test_manager_get_session() {
        let mut manager = RemoteDesktopManager::new();
        let settings = RemoteDesktopSettings::default();

        let session = manager.create_session(settings).unwrap();
        let id = session.id.clone();

        assert!(manager.get_session(&id).is_some());
        assert!(manager.get_session("nonexistent").is_none());
    }

    #[test]
    fn test_connection_type_display_name() {
        assert!(ConnectionType::Lan.display_name().contains("LAN"));
        assert!(ConnectionType::Wan.display_name().contains("WAN"));
        assert!(ConnectionType::Modem.display_name().contains("Modem"));
    }

    #[test]
    fn test_recording_format_variants() {
        let formats = vec![
            RecordingFormat::Mkv,
            RecordingFormat::Mp4,
            RecordingFormat::Avi,
        ];

        for format in formats {
            let settings = RecordingSettings {
                enabled: true,
                output_path: "/tmp/test".to_string(),
                format,
                quality: RecordingQuality::High,
                include_audio: true,
                max_file_size_mb: 1024,
                auto_start: false,
            };
            assert!(settings.enabled);
        }
    }

    #[test]
    fn test_gateway_auth_methods() {
        let _ = GatewayAuthMethod::Password;
        let _ = GatewayAuthMethod::SmartCard;
    }

    #[test]
    fn test_audio_redirection_modes() {
        assert!(matches!(
            AudioRedirectionMode::Client,
            AudioRedirectionMode::Client
        ));
        assert!(matches!(
            AudioRedirectionMode::Server,
            AudioRedirectionMode::Server
        ));
    }
}
