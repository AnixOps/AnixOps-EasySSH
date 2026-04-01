#![allow(dead_code)]

//! Port Forwarding ViewModel for Windows UI
//!
//! Provides port forwarding management with visualization,
//! traffic monitoring, and rule templates.

use easyssh_core::port_forward::{
    ForwardRule,
    ForwardType, builtin_templates,
};
use easyssh_core::AppState;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::sync::Mutex;
use tokio::runtime::Runtime;
use tracing::info;

/// ViewModel for port forwarding UI
#[derive(Clone)]
pub struct PortForwardViewModel {
    runtime: Arc<Runtime>,
    app_state: Arc<Mutex<AppState>>,
}

/// DTO for forward rule with UI-friendly fields
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForwardRuleDto {
    pub id: String,
    pub name: String,
    pub server_id: String,
    pub server_name: Option<String>,
    pub forward_type: String,
    pub forward_type_display: String,
    pub local_addr: String,
    pub remote_addr: Option<String>,
    pub enabled: bool,
    pub status: String,
    pub status_display: String,
    pub auto_reconnect: bool,
    pub traffic: TrafficStatsDto,
    pub browser_url: Option<String>,
    pub notes: Option<String>,
    pub created_at_ms: u128,
}

/// DTO for traffic statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TrafficStatsDto {
    pub bytes_sent: u64,
    pub bytes_received: u64,
    pub bytes_sent_formatted: String,
    pub bytes_received_formatted: String,
    pub connections_total: u64,
    pub connections_active: u64,
    pub errors_total: u64,
    pub last_activity_ms: u128,
}

/// DTO for forward rule template
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForwardRuleTemplateDto {
    pub id: String,
    pub name: String,
    pub description: String,
    pub forward_type: String,
    pub local_addr_pattern: String,
    pub remote_addr_pattern: Option<String>,
    pub category: String,
    pub tags: Vec<String>,
}

/// DTO for topology visualization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForwardTopologyDto {
    pub nodes: Vec<TopologyNodeDto>,
    pub edges: Vec<TopologyEdgeDto>,
}

/// DTO for topology node
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopologyNodeDto {
    pub id: String,
    pub label: String,
    pub node_type: String,
    pub address: String,
}

/// DTO for topology edge
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopologyEdgeDto {
    pub from: String,
    pub to: String,
    pub label: String,
    pub edge_type: String,
    pub bytes_sent: u64,
    pub bytes_received: u64,
    pub connections_active: u64,
}

/// Request to create a new forward rule
#[derive(Debug, Clone, Deserialize)]
pub struct CreateForwardRuleRequest {
    pub name: String,
    pub server_id: String,
    pub forward_type: String,
    pub local_addr: String,
    pub remote_host: Option<String>,
    pub remote_port: Option<u16>,
    pub auto_reconnect: bool,
    pub browser_url: Option<String>,
    pub notes: Option<String>,
}

impl PortForwardViewModel {
    /// Create new port forward viewmodel
    pub fn new(app_state: Arc<Mutex<AppState>>) -> anyhow::Result<Self> {
        let runtime = Arc::new(Runtime::new()?);

        // Clone app_state before locking to avoid move after borrow
        let app_state_clone = app_state.clone();

        // Initialize with built-in templates
        let state = app_state.lock().unwrap();
        let rt = runtime.clone();
        let pf_mgr = state.port_forward_manager.clone();

        rt.block_on(async {
            for template in builtin_templates() {
                let mgr = pf_mgr.write().await;
                mgr.add_template(template).await;
            }
        });

        info!("PortForwardViewModel initialized with built-in templates");

        Ok(Self {
            runtime,
            app_state: app_state_clone,
        })
    }

    /// Get all forward rules for a server
    pub fn get_rules_for_server(&self, _server_id: &str) -> Vec<ForwardRuleDto> {
        // For now, return an empty list
        // In a real implementation, this would query from the database
        vec![]
    }

    /// Get all active forward rules
    pub fn get_active_rules(&self) -> Vec<ForwardRuleDto> {
        let rt = self.runtime.clone();
        let state = self.app_state.lock().unwrap();
        let pf_mgr = state.port_forward_manager.clone();

        rt.block_on(async {
            let mgr = pf_mgr.read().await;
            let active = mgr.list_active().await;

            active
                .into_iter()
                .map(|(id, rule, status)| ForwardRuleDto {
                    id,
                    name: rule.name.clone(),
                    server_id: rule.server_id.clone(),
                    server_name: None, // Would be looked up from database
                    forward_type: format!("{:?}", rule.forward_type),
                    forward_type_display: rule.forward_type.to_string(),
                    local_addr: rule.local_addr.clone(),
                    remote_addr: rule.remote_addr.clone(),
                    enabled: rule.enabled,
                    status: format!("{:?}", status),
                    status_display: status.to_string(),
                    auto_reconnect: rule.auto_reconnect,
                    traffic: TrafficStatsDto::default(),
                    browser_url: rule.browser_url.clone(),
                    notes: rule.notes.clone(),
                    created_at_ms: rule.created_at_ms,
                })
                .collect()
        })
    }

    /// Get traffic statistics for a rule
    pub fn get_rule_traffic(&self, rule_id: &str) -> TrafficStatsDto {
        let rt = self.runtime.clone();
        let state = self.app_state.lock().unwrap();
        let pf_mgr = state.port_forward_manager.clone();

        rt.block_on(async {
            let mgr = pf_mgr.read().await;
            match mgr.get_stats(rule_id).await {
                Some(stats) => TrafficStatsDto {
                    bytes_sent: stats.bytes_sent,
                    bytes_received: stats.bytes_received,
                    bytes_sent_formatted: format_bytes(stats.bytes_sent),
                    bytes_received_formatted: format_bytes(stats.bytes_received),
                    connections_total: stats.connections_total,
                    connections_active: stats.connections_active,
                    errors_total: stats.errors_total,
                    last_activity_ms: stats.last_activity_ms,
                },
                None => TrafficStatsDto::default(),
            }
        })
    }

    /// Create a new forward rule (without starting)
    pub fn create_rule(&self, request: CreateForwardRuleRequest) -> anyhow::Result<ForwardRuleDto> {
        let forward_type = match request.forward_type.as_str() {
            "local" => ForwardType::Local,
            "remote" => ForwardType::Remote,
            "dynamic" => ForwardType::Dynamic,
            _ => return Err(anyhow::anyhow!("Invalid forward type: {}", request.forward_type)),
        };

        let rule = match forward_type {
            ForwardType::Local => {
                let remote_host = request.remote_host.ok_or_else(|| {
                    anyhow::anyhow!("Remote host required for local forward")
                })?;
                let remote_port = request.remote_port.ok_or_else(|| {
                    anyhow::anyhow!("Remote port required for local forward")
                })?;
                ForwardRule::new_local(
                    &request.name,
                    &request.server_id,
                    &request.local_addr,
                    &remote_host,
                    remote_port,
                )
            }
            ForwardType::Remote => {
                let remote_addr = request.remote_host.map(|h| {
                    format!("{}:{}", h, request.remote_port.unwrap_or(0))
                });
                let parts: Vec<&str> = request.local_addr.split(':').collect();
                let local_host = parts.get(0).map(|s| s.to_string()).unwrap_or_else(|| "127.0.0.1".to_string());
                let local_port: u16 = parts.get(1)
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(8080);

                ForwardRule::new_remote(
                    &request.name,
                    &request.server_id,
                    &remote_addr.unwrap_or_else(|| "0.0.0.0:0".to_string()),
                    &local_host,
                    local_port,
                )
            }
            ForwardType::Dynamic => {
                ForwardRule::new_dynamic(&request.name, &request.server_id, &request.local_addr)
            }
        };

        // TODO: Save rule to database

        info!("Created forward rule: {} ({})", rule.name, rule.id);

        Ok(ForwardRuleDto {
            id: rule.id.clone(),
            name: rule.name,
            server_id: rule.server_id,
            server_name: None,
            forward_type: format!("{:?}", rule.forward_type),
            forward_type_display: rule.forward_type.to_string(),
            local_addr: rule.local_addr,
            remote_addr: rule.remote_addr,
            enabled: rule.enabled,
            status: "Stopped".to_string(),
            status_display: "Stopped".to_string(),
            auto_reconnect: rule.auto_reconnect,
            traffic: TrafficStatsDto::default(),
            browser_url: request.browser_url,
            notes: request.notes,
            created_at_ms: rule.created_at_ms,
        })
    }

    /// Start a forward rule
    pub fn start_forward(
        &self,
        rule: ForwardRuleDto,
        session_id: &str,
    ) -> anyhow::Result<()> {
        let rt = self.runtime.clone();
        let state = self.app_state.lock().unwrap();
        let pf_mgr = state.port_forward_manager.clone();
        let ssh_mgr = state.ssh_manager.clone();

        rt.block_on(async {
            // Get the SSH session
            let ssh_lock = ssh_mgr.lock().await;
            let session = ssh_lock.get_session_arc(session_id)
                .ok_or_else(|| anyhow::anyhow!("SSH session not found: {}", session_id))?;
            drop(ssh_lock);

            // Reconstruct the ForwardRule from DTO
            let forward_rule = ForwardRule {
                id: rule.id,
                name: rule.name,
                server_id: rule.server_id,
                forward_type: match rule.forward_type.as_str() {
                    "Local" => ForwardType::Local,
                    "Remote" => ForwardType::Remote,
                    "Dynamic" => ForwardType::Dynamic,
                    _ => ForwardType::Local,
                },
                local_addr: rule.local_addr,
                remote_addr: rule.remote_addr,
                enabled: rule.enabled,
                auto_reconnect: rule.auto_reconnect,
                max_reconnect_attempts: 10,
                reconnect_delay_secs: 5,
                jump_chain: vec![],
                browser_url: rule.browser_url,
                template_id: None,
                notes: rule.notes,
                created_at_ms: rule.created_at_ms,
                modified_at_ms: current_time_millis(),
            };

            let mgr = pf_mgr.write().await;
            mgr.start_forward(forward_rule, session_id, session).await
                .map_err(|e| anyhow::anyhow!("Failed to start forward: {}", e))
        })
    }

    /// Stop a forward rule
    pub fn stop_forward(&self, rule_id: &str) -> anyhow::Result<()> {
        let rt = self.runtime.clone();
        let state = self.app_state.lock().unwrap();
        let pf_mgr = state.port_forward_manager.clone();

        rt.block_on(async {
            let mgr = pf_mgr.read().await;
            mgr.stop_forward(rule_id).await
                .map_err(|e| anyhow::anyhow!("Failed to stop forward: {}", e))
        })
    }

    /// Get available rule templates
    pub fn get_templates(&self) -> Vec<ForwardRuleTemplateDto> {
        let rt = self.runtime.clone();
        let state = self.app_state.lock().unwrap();
        let pf_mgr = state.port_forward_manager.clone();

        rt.block_on(async {
            let mgr = pf_mgr.read().await;
            mgr.list_templates()
                .await
                .into_iter()
                .map(|t| ForwardRuleTemplateDto {
                    id: t.id,
                    name: t.name,
                    description: t.description,
                    forward_type: format!("{:?}", t.forward_type),
                    local_addr_pattern: t.local_addr_pattern,
                    remote_addr_pattern: t.remote_addr_pattern,
                    category: t.category,
                    tags: t.tags,
                })
                .collect()
        })
    }

    /// Create a rule from template
    pub fn create_from_template(
        &self,
        template_id: &str,
        server_id: &str,
        custom_name: Option<&str>,
    ) -> anyhow::Result<ForwardRuleDto> {
        let rt = self.runtime.clone();
        let state = self.app_state.lock().unwrap();
        let pf_mgr = state.port_forward_manager.clone();

        rt.block_on(async {
            let mgr = pf_mgr.read().await;
            let rule = mgr.create_from_template(template_id, server_id, custom_name).await
                .map_err(|e| anyhow::anyhow!("Failed to create from template: {}", e))?;

            Ok(ForwardRuleDto {
                id: rule.id,
                name: rule.name,
                server_id: rule.server_id,
                server_name: None,
                forward_type: format!("{:?}", rule.forward_type),
                forward_type_display: rule.forward_type.to_string(),
                local_addr: rule.local_addr,
                remote_addr: rule.remote_addr,
                enabled: rule.enabled,
                status: "Stopped".to_string(),
                status_display: "Stopped".to_string(),
                auto_reconnect: rule.auto_reconnect,
                traffic: TrafficStatsDto::default(),
                browser_url: rule.browser_url,
                notes: rule.notes,
                created_at_ms: rule.created_at_ms,
            })
        })
    }

    /// Get forwarding topology for visualization
    pub fn get_topology(&self) -> ForwardTopologyDto {
        let rt = self.runtime.clone();
        let state = self.app_state.lock().unwrap();
        let pf_mgr = state.port_forward_manager.clone();

        rt.block_on(async {
            let mgr = pf_mgr.read().await;
            let topology = mgr.get_topology().await;

            ForwardTopologyDto {
                nodes: topology.nodes
                    .into_iter()
                    .map(|n| TopologyNodeDto {
                        id: n.id,
                        label: n.label,
                        node_type: format!("{:?}", n.node_type),
                        address: n.address,
                    })
                    .collect(),
                edges: topology.edges
                    .into_iter()
                    .map(|e| TopologyEdgeDto {
                        from: e.from,
                        to: e.to,
                        label: e.label,
                        edge_type: format!("{:?}", e.edge_type),
                        bytes_sent: e.stats.bytes_sent,
                        bytes_received: e.stats.bytes_received,
                        connections_active: e.stats.connections_active,
                    })
                    .collect(),
            }
        })
    }

    /// Open browser with URL
    pub fn open_browser(&self, url: &str) -> anyhow::Result<()> {
        #[cfg(target_os = "windows")]
        {
            std::process::Command::new("cmd")
                .args(["/C", "start", "", url])
                .spawn()
                .map_err(|e| anyhow::anyhow!("Failed to open browser: {}", e))?;
        }
        #[cfg(target_os = "macos")]
        {
            std::process::Command::new("open")
                .arg(url)
                .spawn()
                .map_err(|e| anyhow::anyhow!("Failed to open browser: {}", e))?;
        }
        #[cfg(target_os = "linux")]
        {
            std::process::Command::new("xdg-open")
                .arg(url)
                .spawn()
                .map_err(|e| anyhow::anyhow!("Failed to open browser: {}", e))?;
        }

        info!("Opened browser: {}", url);
        Ok(())
    }

    /// Stop all forwards for a session (call when disconnecting)
    pub fn stop_session_forwards(&self, session_id: &str) {
        let rt = self.runtime.clone();
        let state = self.app_state.lock().unwrap();
        let pf_mgr = state.port_forward_manager.clone();

        rt.block_on(async {
            let mgr = pf_mgr.read().await;
            mgr.stop_session_forwards(session_id).await;
        });

        info!("Stopped all forwards for session: {}", session_id);
    }
}

/// Format bytes to human-readable string
fn format_bytes(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    let mut size = bytes as f64;
    let mut unit_index = 0;

    while size >= 1024.0 && unit_index < UNITS.len() - 1 {
        size /= 1024.0;
        unit_index += 1;
    }

    format!("{:.2} {}", size, UNITS[unit_index])
}

/// Current time in milliseconds
fn current_time_millis() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_bytes() {
        assert_eq!(format_bytes(0), "0.00 B");
        assert_eq!(format_bytes(512), "512.00 B");
        assert_eq!(format_bytes(1024), "1.00 KB");
        assert_eq!(format_bytes(1024 * 1024), "1.00 MB");
        assert_eq!(format_bytes(1024 * 1024 * 1024), "1.00 GB");
    }
}
