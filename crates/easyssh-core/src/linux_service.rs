//! Linux Service Integration for systemd
//!
//! Features:
//! - systemd service notification (sd_notify)
//! - Daemon mode for headless operation
//! - Background service management
//! - D-Bus integration for service control

use std::env;
use std::fs;
use std::os::unix::ffi::OsStrExt;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, Mutex};
use tokio::task::JoinHandle;

/// Service state for systemd notification
#[derive(Debug, Clone, PartialEq)]
pub enum ServiceState {
    Starting,
    Ready,
    Reloading,
    Stopping,
    Status(String),
    Watchdog,
}

impl ServiceState {
    fn to_notify_string(&self) -> String {
        match self {
            ServiceState::Starting => "READY=1\n".to_string(),
            ServiceState::Ready => "READY=1\n".to_string(),
            ServiceState::Reloading => "RELOADING=1\n".to_string(),
            ServiceState::Stopping => "STOPPING=1\n".to_string(),
            ServiceState::Status(msg) => format!("STATUS={}\n", msg),
            ServiceState::Watchdog => "WATCHDOG=1\n".to_string(),
        }
    }
}

/// systemd notification handle
pub struct SystemdNotifier {
    socket_path: Option<PathBuf>,
    watchdog_enabled: bool,
    watchdog_usec: u64,
    last_watchdog: Arc<Mutex<std::time::Instant>>,
    watchdog_handle: Option<JoinHandle<()>>,
}

impl SystemdNotifier {
    /// Create new notifier, detecting systemd environment
    pub fn new() -> Self {
        let socket_path = env::var("NOTIFY_SOCKET").ok().map(|s| {
            // Handle '@' prefix for abstract namespace sockets
            if s.starts_with('@') {
                PathBuf::from(format!("\0{}", &s[1..]))
            } else {
                PathBuf::from(s)
            }
        });

        let watchdog_usec = env::var("WATCHDOG_USEC")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);

        let watchdog_enabled = socket_path.is_some() && watchdog_usec > 0;

        Self {
            socket_path,
            watchdog_enabled,
            watchdog_usec,
            last_watchdog: Arc::new(Mutex::new(std::time::Instant::now())),
            watchdog_handle: None,
        }
    }

    /// Check if running under systemd
    pub fn is_systemd_service(&self) -> bool {
        self.socket_path.is_some()
    }

    /// Notify systemd of service state
    pub fn notify(&self, state: ServiceState) -> std::io::Result<()> {
        if let Some(ref socket_path) = self.socket_path {
            use std::os::unix::net::UnixDatagram;

            let msg = state.to_notify_string();
            let sock = UnixDatagram::unbound()?;

            // Handle abstract namespace socket (starts with null byte)
            let addr_bytes = socket_path.as_os_str().as_encoded_bytes();
            if addr_bytes.starts_with(&[0]) {
                // For abstract Unix sockets, we need to use the path directly with a null prefix
                // The standard library doesn't have from_abstract_name, so we use the path directly
                let abstract_name = std::ffi::OsStr::from_bytes(&addr_bytes[1..]);
                sock.send_to(msg.as_bytes(), abstract_name)?;
            } else {
                sock.send_to(msg.as_bytes(), socket_path)?;
            }

            tracing::debug!("systemd notify: {:?}", state);
        }
        Ok(())
    }

    /// Start watchdog keepalive
    pub fn start_watchdog(&mut self) {
        if !self.watchdog_enabled {
            return;
        }

        let interval_us = self.watchdog_usec / 2; // Send at half the timeout
        let interval = Duration::from_micros(interval_us);
        let last_watchdog = self.last_watchdog.clone();
        let notify_socket = self.socket_path.clone();

        let handle = tokio::spawn(async move {
            let mut ticker = tokio::time::interval(interval);

            loop {
                ticker.tick().await;

                if let Some(ref socket_path) = notify_socket {
                    use tokio::net::UnixDatagram;

                    if let Ok(sock) = UnixDatagram::unbound() {
                        let msg = "WATCHDOG=1\n";

                        let _ = sock.send_to(msg.as_bytes(), socket_path).await;

                        let mut last = last_watchdog.lock().await;
                        *last = std::time::Instant::now();
                    }
                }
            }
        });

        self.watchdog_handle = Some(handle);
    }

    /// Stop watchdog
    pub fn stop_watchdog(&mut self) {
        if let Some(handle) = self.watchdog_handle.take() {
            handle.abort();
        }
    }

    /// Get main pid notification
    pub fn notify_mainpid(&self) -> std::io::Result<()> {
        let pid = std::process::id();
        self.notify(ServiceState::Status(format!("MAINPID={}", pid)))
    }

    /// Notify systemd that service is ready
    pub fn ready(&self) -> std::io::Result<()> {
        self.notify(ServiceState::Ready)
    }

    /// Notify systemd that service is stopping
    pub fn stopping(&self) -> std::io::Result<()> {
        self.notify(ServiceState::Stopping)
    }

    /// Set service status message
    pub fn set_status(&self, msg: impl Into<String>) -> std::io::Result<()> {
        self.notify(ServiceState::Status(msg.into()))
    }
}

impl Drop for SystemdNotifier {
    fn drop(&mut self) {
        self.stop_watchdog();
    }
}

/// Daemon mode configuration
#[derive(Clone, Debug)]
pub struct DaemonConfig {
    /// Run as daemon (detached from terminal)
    pub daemonize: bool,
    /// Working directory for daemon
    pub working_dir: PathBuf,
    /// PID file path
    pub pid_file: Option<PathBuf>,
    /// Log file path
    pub log_file: Option<PathBuf>,
    /// User to run as (if started as root)
    pub user: Option<String>,
    /// Group to run as (if started as root)
    pub group: Option<String>,
}

impl Default for DaemonConfig {
    fn default() -> Self {
        Self {
            daemonize: false,
            working_dir: PathBuf::from("/"),
            pid_file: Some(PathBuf::from("/run/easyssh/easyssh.pid")),
            log_file: Some(PathBuf::from("/var/log/easyssh/daemon.log")),
            user: None,
            group: None,
        }
    }
}

/// Service manager for background operation
pub struct LinuxServiceManager {
    notifier: SystemdNotifier,
    config: DaemonConfig,
    shutdown_tx: Option<mpsc::Sender<()>>,
    shutdown_rx: Option<mpsc::Receiver<()>>,
}

impl LinuxServiceManager {
    /// Create new service manager
    pub fn new(config: DaemonConfig) -> Self {
        let (tx, rx) = mpsc::channel(1);
        Self {
            notifier: SystemdNotifier::new(),
            config,
            shutdown_tx: Some(tx),
            shutdown_rx: Some(rx),
        }
    }

    /// Check if running as systemd service
    pub fn is_systemd_service(&self) -> bool {
        self.notifier.is_systemd_service()
    }

    /// Initialize service (create directories, set permissions)
    pub fn initialize(&self) -> std::io::Result<()> {
        // Create PID directory
        if let Some(ref pid_path) = self.config.pid_file {
            if let Some(parent) = pid_path.parent() {
                fs::create_dir_all(parent)?;
            }
        }

        // Create log directory
        if let Some(ref log_path) = self.config.log_file {
            if let Some(parent) = log_path.parent() {
                fs::create_dir_all(parent)?;
            }
        }

        // Write PID file
        if let Some(ref pid_path) = self.config.pid_file {
            let pid = std::process::id().to_string();
            fs::write(pid_path, pid)?;
        }

        Ok(())
    }

    /// Notify systemd that service is ready
    pub fn notify_ready(&self) -> std::io::Result<()> {
        self.notifier.ready()
    }

    /// Notify systemd of status
    pub fn notify_status(&self, status: &'static str) -> std::io::Result<()> {
        self.notifier.set_status(status)
    }

    /// Start watchdog keepalive
    pub fn start_watchdog(&mut self) {
        self.notifier.start_watchdog();
    }

    /// Run the service main loop
    pub async fn run<F, Fut>(mut self, service_fn: F) -> std::io::Result<()>
    where
        F: FnOnce(mpsc::Receiver<()>) -> Fut,
        Fut: std::future::Future<Output = ()>,
    {
        // Initialize
        self.initialize()?;

        // Notify systemd we're starting
        self.notifier
            .set_status("Initializing EasySSH service...")?;

        // Start watchdog if enabled
        if self.notifier.watchdog_enabled {
            self.start_watchdog();
        }

        // Get shutdown channel
        let shutdown_rx = self.shutdown_rx.take().unwrap();

        // Setup signal handlers
        self.setup_signal_handlers();

        // Notify ready
        self.notify_ready()?;
        tracing::info!("EasySSH service ready");

        // Run service
        service_fn(shutdown_rx).await;

        // Cleanup
        self.shutdown().await;

        Ok(())
    }

    /// Setup signal handlers for graceful shutdown
    fn setup_signal_handlers(&self) {
        let shutdown_tx = self.shutdown_tx.clone();

        tokio::spawn(async move {
            let mut sigterm =
                tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
                    .expect("Failed to create SIGTERM handler");

            let mut sigint =
                tokio::signal::unix::signal(tokio::signal::unix::SignalKind::interrupt())
                    .expect("Failed to create SIGINT handler");

            tokio::select! {
                _ = sigterm.recv() => {
                    tracing::info!("Received SIGTERM, shutting down...");
                }
                _ = sigint.recv() => {
                    tracing::info!("Received SIGINT, shutting down...");
                }
            }

            if let Some(tx) = shutdown_tx {
                let _ = tx.send(()).await;
            }
        });
    }

    /// Graceful shutdown
    async fn shutdown(&self) {
        tracing::info!("Service shutting down...");

        // Notify systemd
        let _ = self.notifier.stopping();

        // Remove PID file
        if let Some(ref pid_path) = self.config.pid_file {
            let _ = fs::remove_file(pid_path);
        }

        tracing::info!("Service shutdown complete");
    }
}

/// Generate systemd service file content
pub fn generate_systemd_service() -> String {
    r#"[Unit]
Description=EasySSH SSH Client Service
Documentation=https://easyssh.io/docs
After=network.target

[Service]
Type=notify
ExecStart=/usr/bin/easyssh --daemon
ExecReload=/bin/kill -HUP $MAINPID
KillMode=mixed
KillSignal=SIGTERM
Restart=on-failure
RestartSec=5
WatchdogSec=30

# Security hardening
NoNewPrivileges=true
PrivateTmp=true
ProtectSystem=strict
ProtectHome=true
ReadWritePaths=/var/lib/easyssh /var/log/easyssh /run/easyssh

# Resource limits
LimitNOFILE=65536
LimitNPROC=4096

# Environment
Environment="RUST_LOG=info"
Environment="EASYSSH_CONFIG_DIR=/var/lib/easyssh"

[Install]
WantedBy=multi-user.target
"#
    .to_string()
}

/// Install systemd service file
pub fn install_systemd_service() -> std::io::Result<()> {
    let service_content = generate_systemd_service();
    let service_path = PathBuf::from("/etc/systemd/system/easyssh.service");

    fs::write(&service_path, service_content)?;
    tracing::info!("Installed systemd service to {:?}", service_path);

    Ok(())
}

/// Generate D-Bus service configuration
pub fn generate_dbus_config() -> String {
    r#"<!DOCTYPE busconfig PUBLIC "-//freedesktop//DTD D-BUS Bus Configuration 1.0//EN"
  "http://www.freedesktop.org/standards/dbus/1.0/busconfig.dtd">
<busconfig>
  <policy user="root">
    <allow own="com.easyssh.Service"/>
    <allow send_destination="com.easyssh.Service"/>
    <allow receive_sender="com.easyssh.Service"/>
  </policy>

  <policy context="default">
    <allow send_destination="com.easyssh.Service"/>
    <allow receive_sender="com.easyssh.Service"/>
  </policy>
</busconfig>
"#
    .to_string()
}

/// Service runtime information
#[derive(Debug, Clone)]
pub struct ServiceInfo {
    pub start_time: std::time::Instant,
    pub systemd_mode: bool,
    pub watchdog_enabled: bool,
    pub pid: u32,
}

impl ServiceInfo {
    pub fn new(systemd_mode: bool, watchdog_enabled: bool) -> Self {
        Self {
            start_time: std::time::Instant::now(),
            systemd_mode,
            watchdog_enabled,
            pid: std::process::id(),
        }
    }

    pub fn uptime_secs(&self) -> u64 {
        self.start_time.elapsed().as_secs()
    }
}
