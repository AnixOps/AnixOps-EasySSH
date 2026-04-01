//! Enhanced Application State for Linux GTK4 with Connection Pool

use gtk4::prelude::*;
use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::Mutex;
use std::time::{Duration, Instant};
use tokio::runtime::Runtime;
use tokio::sync::mpsc::UnboundedReceiver;

use crate::models::{Server, ServerGroup};

/// Session with enhanced tracking
pub struct EnhancedActiveSession {
    pub session_id: String,
    pub server: Server,
    pub receiver: Option<UnboundedReceiver<String>>,
    pub start_time: Instant,
    pub terminal_content: String,
    pub compressed_size: usize,
    pub original_size: usize,
}

/// Enhanced view model with connection pool
pub struct EnhancedAppViewModel {
    core_state: Arc<Mutex<easyssh_core::AppState>>,
    runtime: Arc<Runtime>,
    enhanced_manager: Arc<Mutex<easyssh_core::EnhancedSshManager>>,
    session_content_cache: Arc<Mutex<HashMap<String, String>>>,
}

impl EnhancedAppViewModel {
    pub fn new() -> anyhow::Result<Self> {
        let core_state = Arc::new(Mutex::new(easyssh_core::AppState::new()));

        let runtime = Arc::new(
            tokio::runtime::Builder::new_multi_thread()
                .worker_threads(8) // More threads for connection pool
                .thread_name("easyssh-pool")
                .enable_all()
                .build()
                .map_err(|e| anyhow::anyhow!("Failed to create Tokio runtime: {}", e))?
        );

        // Create enhanced manager with optimized settings
        let enhanced_manager = Arc::new(Mutex::new(
            easyssh_core::EnhancedSshManagerBuilder::new()
                .max_connections_per_minute(60)
                .max_stored_sessions(200)
                .health_check_interval(30)
                .reconnect_max_attempts(5)
                .max_global_connections(100)
                .build()
        ));

        let session_content_cache = Arc::new(Mutex::new(HashMap::new()));

        Ok(Self {
            core_state,
            runtime,
            enhanced_manager,
            session_content_cache,
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
        easyssh_core::get_groups(&state)
            .map_err(|e| anyhow::anyhow!("Failed to get groups: {}", e))
    }

    pub fn add_server(&self, name: &str, host: &str, port: i64, username: &str, auth_type: &str) -> anyhow::Result<()> {
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
        easyssh_core::keychain::get_password(server_id).ok().flatten()
    }

    pub fn save_password(&self, server_id: &str, password: &str) -> anyhow::Result<()> {
        easyssh_core::keychain::store_password(server_id, password)
            .map_err(|e| anyhow::anyhow!("Failed to save password: {}", e))
    }

    /// Connect with enhanced connection pool
    pub fn connect(&self, session_id: &str, host: &str, port: i64, username: &str, password: Option<&str>) -> anyhow::Result<()> {
        let rt = self.runtime.clone();
        let manager = self.enhanced_manager.clone();
        let sid = session_id.to_string();
        let h = host.to_string();
        let u = username.to_string();
        let p = password.map(|s| s.to_string());

        rt.block_on(async move {
            let mgr = manager.lock().unwrap();
            mgr.connect(&sid, &h, port as u16, &u, p.as_deref()).await
                .map_err(|e| anyhow::anyhow!("SSH connection failed: {}", e))
        })
    }

    /// Execute with auto-reconnect
    pub fn execute_with_reconnect(&self, session_id: &str, command: &str) -> anyhow::Result<String> {
        let rt = self.runtime.clone();
        let manager = self.enhanced_manager.clone();
        let sid = session_id.to_string();
        let cmd = command.to_string();

        rt.block_on(async move {
            let mgr = manager.lock().unwrap();
            mgr.execute_with_auto_reconnect(&sid, &cmd).await
                .map_err(|e| anyhow::anyhow!("Command execution failed: {}", e))
        })
    }

    /// Execute stream with enhanced manager
    pub fn execute_stream(&self, session_id: &str, command: &str) -> anyhow::Result<UnboundedReceiver<String>> {
        // Use base manager for streaming
        let rt = self.runtime.clone();
        let manager = self.enhanced_manager.clone();
        let sid = session_id.to_string();
        let cmd = command.to_string();

        rt.block_on(async move {
            // Get underlying session and create stream
            let mgr = manager.lock().unwrap();
            // This would need to be implemented in EnhancedSshManager
            // For now, return an error
            Err(anyhow::anyhow!("Stream execution not yet implemented in enhanced manager"))
        })
    }

    /// Store session content with compression
    pub fn store_session_content(&self, session_id: &str, server_key: &str, content: &str) -> anyhow::Result<()> {
        let rt = self.runtime.clone();
        let manager = self.enhanced_manager.clone();
        let sid = session_id.to_string();
        let sk = server_key.to_string();
        let c = content.to_string();

        rt.block_on(async move {
            let mgr = manager.lock().unwrap();
            mgr.store_session_content(&sid, &sk, &c).await
                .map_err(|e| anyhow::anyhow!("Failed to store session content: {}", e))
        })
    }

    /// Retrieve compressed session content
    pub fn retrieve_session_content(&self, session_id: &str) -> anyhow::Result<Option<String>> {
        let rt = self.runtime.clone();
        let manager = self.enhanced_manager.clone();
        let sid = session_id.to_string();

        rt.block_on(async move {
            let mgr = manager.lock().unwrap();
            mgr.retrieve_session_content(&sid).await
                .map_err(|e| anyhow::anyhow!("Failed to retrieve session content: {}", e))
        })
    }

    /// Get connection state
    pub fn get_connection_state(&self, session_id: &str) -> easyssh_core::EnhancedConnectionState {
        let rt = self.runtime.clone();
        let manager = self.enhanced_manager.clone();
        let sid = session_id.to_string();

        rt.block_on(async move {
            let mgr = manager.lock().unwrap();
            mgr.get_connection_state(&sid).await
        })
    }

    /// Get enhanced pool statistics
    pub fn get_pool_stats(&self) -> easyssh_core::EnhancedPoolStats {
        let rt = self.runtime.clone();
        let manager = self.enhanced_manager.clone();

        rt.block_on(async move {
            let mgr = manager.lock().unwrap();
            mgr.get_stats().await
        })
    }

    pub fn disconnect(&self, session_id: &str) -> anyhow::Result<()> {
        let rt = self.runtime.clone();
        let manager = self.enhanced_manager.clone();
        let sid = session_id.to_string();

        rt.block_on(async move {
            let mgr = manager.lock().unwrap();
            mgr.disconnect(&sid).await
                .map_err(|e| anyhow::anyhow!("Disconnect failed: {}", e))
        })
    }

    /// Cleanup all resources
    pub fn shutdown(&self) {
        tracing::info!("Shutting down EnhancedAppViewModel...");

        let rt = self.runtime.clone();
        let manager = self.enhanced_manager.clone();

        rt.block_on(async move {
            let mgr = manager.lock().unwrap();
            mgr.shutdown().await;
        });

        tracing::info!("EnhancedAppViewModel shutdown complete");
    }

    /// Get all session states for monitoring
    pub fn get_all_session_states(&self) -> Vec<(String, easyssh_core::EnhancedConnectionState)> {
        let rt = self.runtime.clone();
        let manager = self.enhanced_manager.clone();

        rt.block_on(async move {
            let mgr = manager.lock().unwrap();
            mgr.list_session_states().await
        })
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
