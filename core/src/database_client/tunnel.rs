//! SSH tunnel support for database connections

use crate::database_client::DatabaseError;
use crate::ssh::SshSessionManager;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{Mutex, RwLock};

/// SSH tunnel configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TunnelConfig {
    pub id: String,
    pub name: String,
    pub ssh_server_id: String,
    pub ssh_host: String,
    pub ssh_port: u16,
    pub ssh_username: String,
    pub local_bind_host: String,
    pub local_bind_port: u16,
    pub remote_host: String,
    pub remote_port: u16,
    pub use_key_auth: bool,
    pub ssh_key_path: Option<String>,
    pub ssh_password: Option<String>,
    pub keep_alive_interval_secs: u64,
    pub auto_reconnect: bool,
}

impl TunnelConfig {
    pub fn new(name: String, ssh_server_id: String) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name,
            ssh_server_id,
            ssh_host: String::new(),
            ssh_port: 22,
            ssh_username: String::new(),
            local_bind_host: "127.0.0.1".to_string(),
            local_bind_port: 0, // Random port
            remote_host: "localhost".to_string(),
            remote_port: 3306,
            use_key_auth: true,
            ssh_key_path: None,
            ssh_password: None,
            keep_alive_interval_secs: 30,
            auto_reconnect: true,
        }
    }

    pub fn local_address(&self) -> String {
        format!("{}:{}", self.local_bind_host, self.local_bind_port)
    }

    pub fn remote_address(&self) -> String {
        format!("{}:{}", self.remote_host, self.remote_port)
    }
}

/// Tunnel state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TunnelState {
    Disconnected,
    Connecting,
    Connected,
    Error,
    Reconnecting,
}

/// Active tunnel information
#[derive(Debug, Clone)]
pub struct ActiveTunnel {
    pub config: TunnelConfig,
    pub state: TunnelState,
    pub local_addr: SocketAddr,
    pub connected_at: Option<chrono::DateTime<chrono::Utc>>,
    pub last_activity: Option<chrono::DateTime<chrono::Utc>>,
    pub bytes_transferred: u64,
    pub error_count: u32,
    pub reconnect_count: u32,
}

/// Tunnel manager
pub struct TunnelManager {
    tunnels: Arc<RwLock<HashMap<String, ActiveTunnel>>>,
    ssh_manager: Arc<Mutex<SshSessionManager>>,
    listeners: Arc<RwLock<HashMap<String, TcpListener>>>,
}

impl TunnelManager {
    pub fn new() -> Self {
        Self {
            tunnels: Arc::new(RwLock::new(HashMap::new())),
            ssh_manager: Arc::new(Mutex::new(SshSessionManager::new())),
            listeners: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Create and start a tunnel
    pub async fn create_tunnel(&self, config: TunnelConfig) -> Result<SocketAddr, DatabaseError> {
        // Check if already exists
        {
            let tunnels = self.tunnels.read().await;
            if let Some(existing) = tunnels.get(&config.id) {
                if existing.state == TunnelState::Connected {
                    return Ok(existing.local_addr);
                }
            }
        }

        // Bind local port
        let bind_addr = if config.local_bind_port == 0 {
            format!("{}:0", config.local_bind_host)
        } else {
            config.local_address()
        };

        let listener = TcpListener::bind(&bind_addr)
            .await
            .map_err(|e| DatabaseError::TunnelError(format!("Failed to bind: {}", e)))?;

        let local_addr = listener
            .local_addr()
            .map_err(|e| DatabaseError::TunnelError(e.to_string()))?;

        // Store listener
        {
            let mut listeners = self.listeners.write().await;
            listeners.insert(config.id.clone(), listener);
        }

        // Create active tunnel entry
        let tunnel = ActiveTunnel {
            config: config.clone(),
            state: TunnelState::Connecting,
            local_addr,
            connected_at: None,
            last_activity: None,
            bytes_transferred: 0,
            error_count: 0,
            reconnect_count: 0,
        };

        {
            let mut tunnels = self.tunnels.write().await;
            tunnels.insert(config.id.clone(), tunnel);
        }

        // Start tunnel in background
        let tunnels = self.tunnels.clone();
        let ssh_manager = self.ssh_manager.clone();
        let config_clone = config.clone();

        tokio::spawn(async move {
            Self::run_tunnel(tunnels, ssh_manager, config_clone, local_addr).await;
        });

        // Wait for connection
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

        let tunnels = self.tunnels.read().await;
        if let Some(tunnel) = tunnels.get(&config.id) {
            if tunnel.state == TunnelState::Connected {
                Ok(local_addr)
            } else {
                Err(DatabaseError::TunnelError(
                    "Failed to establish tunnel".to_string(),
                ))
            }
        } else {
            Err(DatabaseError::TunnelError("Tunnel not found".to_string()))
        }
    }

    async fn run_tunnel(
        tunnels: Arc<RwLock<HashMap<String, ActiveTunnel>>>,
        _ssh_manager: Arc<Mutex<SshSessionManager>>,
        config: TunnelConfig,
        local_addr: SocketAddr,
    ) {
        // This is a simplified implementation
        // In reality, this would establish SSH connection and forward ports

        // Update state
        {
            let mut tunnels = tunnels.write().await;
            if let Some(tunnel) = tunnels.get_mut(&config.id) {
                tunnel.state = TunnelState::Connected;
                tunnel.connected_at = Some(chrono::Utc::now());
            }
        }

        // Keep tunnel alive
        loop {
            tokio::time::sleep(tokio::time::Duration::from_secs(
                config.keep_alive_interval_secs,
            ))
            .await;

            // Check if should continue
            let should_continue = {
                let tunnels = tunnels.read().await;
                tunnels
                    .get(&config.id)
                    .map(|t| t.state == TunnelState::Connected)
                    .unwrap_or(false)
            };

            if !should_continue {
                break;
            }

            // Update activity
            {
                let mut tunnels = tunnels.write().await;
                if let Some(tunnel) = tunnels.get_mut(&config.id) {
                    tunnel.last_activity = Some(chrono::Utc::now());
                }
            }
        }
    }

    /// Close a tunnel
    pub async fn close_tunnel(&self, tunnel_id: &str) -> Result<(), DatabaseError> {
        // Update state
        {
            let mut tunnels = self.tunnels.write().await;
            if let Some(tunnel) = tunnels.get_mut(tunnel_id) {
                tunnel.state = TunnelState::Disconnected;
            }
        }

        // Remove listener
        {
            let mut listeners = self.listeners.write().await;
            listeners.remove(tunnel_id);
        }

        // Remove from active tunnels
        {
            let mut tunnels = self.tunnels.write().await;
            tunnels.remove(tunnel_id);
        }

        Ok(())
    }

    /// Get tunnel status
    pub async fn get_tunnel_status(&self, tunnel_id: &str) -> Option<ActiveTunnel> {
        let tunnels = self.tunnels.read().await;
        tunnels.get(tunnel_id).cloned()
    }

    /// List all active tunnels
    pub async fn list_tunnels(&self) -> Vec<ActiveTunnel> {
        let tunnels = self.tunnels.read().await;
        tunnels.values().cloned().collect()
    }

    /// Test tunnel connectivity
    pub async fn test_tunnel(&self, tunnel_id: &str) -> Result<bool, DatabaseError> {
        let tunnels = self.tunnels.read().await;

        if let Some(tunnel) = tunnels.get(tunnel_id) {
            // Try to connect to local port
            match tokio::time::timeout(
                tokio::time::Duration::from_secs(5),
                TcpStream::connect(tunnel.local_addr),
            )
            .await
            {
                Ok(Ok(_)) => Ok(true),
                _ => Ok(false),
            }
        } else {
            Err(DatabaseError::TunnelError("Tunnel not found".to_string()))
        }
    }

    /// Get tunnel statistics
    pub async fn get_statistics(&self, tunnel_id: &str) -> Option<TunnelStatistics> {
        let tunnels = self.tunnels.read().await;

        tunnels.get(tunnel_id).map(|t| {
            let uptime_secs = t
                .connected_at
                .map(|start| {
                    chrono::Utc::now()
                        .signed_duration_since(start)
                        .num_seconds() as u64
                })
                .unwrap_or(0);

            TunnelStatistics {
                uptime_secs,
                bytes_transferred: t.bytes_transferred,
                connection_count: t.reconnect_count + 1,
                error_count: t.error_count,
                avg_throughput_bps: if uptime_secs > 0 {
                    t.bytes_transferred / uptime_secs * 8
                } else {
                    0
                },
            }
        })
    }

    /// Auto-discover tunnel configuration for a database
    pub async fn auto_configure(
        &self,
        ssh_server_id: &str,
        db_host: &str,
        db_port: u16,
    ) -> Result<TunnelConfig, DatabaseError> {
        let mut config = TunnelConfig::new(
            format!("tunnel_{}_{}", ssh_server_id, db_port),
            ssh_server_id.to_string(),
        );

        config.remote_host = db_host.to_string();
        config.remote_port = db_port;

        // Find an available local port
        let test_listener = TcpListener::bind("127.0.0.1:0")
            .await
            .map_err(|e| DatabaseError::TunnelError(e.to_string()))?;
        let local_port = test_listener
            .local_addr()
            .map_err(|e| DatabaseError::TunnelError(e.to_string()))?
            .port();
        drop(test_listener);

        config.local_bind_port = local_port;

        Ok(config)
    }
}

impl Default for TunnelManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Tunnel statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TunnelStatistics {
    pub uptime_secs: u64,
    pub bytes_transferred: u64,
    pub connection_count: u32,
    pub error_count: u32,
    pub avg_throughput_bps: u64,
}

/// Tunnel health check
pub struct TunnelHealthChecker {
    check_interval_secs: u64,
}

impl TunnelHealthChecker {
    pub fn new() -> Self {
        Self {
            check_interval_secs: 30,
        }
    }

    pub fn with_interval(mut self, secs: u64) -> Self {
        self.check_interval_secs = secs;
        self
    }

    pub async fn start_monitoring(&self, manager: Arc<TunnelManager>) {
        let interval = tokio::time::Duration::from_secs(self.check_interval_secs);

        loop {
            tokio::time::sleep(interval).await;

            let tunnels = manager.list_tunnels().await;

            for tunnel in tunnels {
                if tunnel.state == TunnelState::Connected {
                    match manager.test_tunnel(&tunnel.config.id).await {
                        Ok(true) => {}
                        _ => {
                            // Tunnel unhealthy - trigger reconnect if enabled
                            if tunnel.config.auto_reconnect {
                                // Would trigger reconnect here
                            }
                        }
                    }
                }
            }
        }
    }
}

impl Default for TunnelHealthChecker {
    fn default() -> Self {
        Self::new()
    }
}
