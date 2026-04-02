//! Docker client - Main Docker manager

use crate::error::LiteError;
use crate::ssh::SshSessionManager;
use std::collections::HashMap;
use tokio::sync::{mpsc, RwLock};
use tokio::task::JoinHandle;

use super::types::{
    ContainerInfo, ContainerNetworkInfo, ContainerStats, ContainerStatus, DockerConnection,
    DockerEvent, DockerSystemInfo, HostConfig, ImageInfo, IoEntry, MemoryStats, MountPoint,
    NetworkInfo, NetworkSettings, PortMapping, VolumeInfo,
};

/// Exec session
#[derive(Clone)]
struct ExecSession {
    container_id: String,
    command: String,
    tty: bool,
    stdin: bool,
    stdout: bool,
    stderr: bool,
}

/// Docker manager
pub struct DockerManager {
    connections: RwLock<HashMap<String, DockerConnection>>,
    active_logs: RwLock<HashMap<String, JoinHandle<()>>>,
    log_channels: RwLock<HashMap<String, mpsc::UnboundedSender<String>>>,
    exec_sessions: RwLock<HashMap<String, ExecSession>>,
}

impl DockerManager {
    /// Create new Docker manager
    pub fn new() -> Self {
        Self {
            connections: RwLock::new(HashMap::new()),
            active_logs: RwLock::new(HashMap::new()),
            log_channels: RwLock::new(HashMap::new()),
            exec_sessions: RwLock::new(HashMap::new()),
        }
    }

    /// Add Docker connection
    pub async fn add_connection(&self, connection: DockerConnection) -> Result<(), LiteError> {
        let mut connections = self.connections.write().await;
        connections.insert(connection.name.clone(), connection);
        Ok(())
    }

    /// Remove Docker connection
    pub async fn remove_connection(&self, name: &str) -> Result<(), LiteError> {
        let mut connections = self.connections.write().await;
        connections.remove(name);
        Ok(())
    }

    /// List Docker connections
    pub async fn list_connections(&self) -> Vec<DockerConnection> {
        let connections = self.connections.read().await;
        connections.values().cloned().collect()
    }

    /// Parse container JSON from docker ps output
    fn parse_container_json(&self, value: serde_json::Value) -> Result<ContainerInfo, LiteError> {
        Ok(ContainerInfo {
            id: value.get("ID").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            names: value
                .get("Names")
                .and_then(|v| v.as_str())
                .map(|s| s.split(',').map(|n| n.to_string()).collect())
                .unwrap_or_default(),
            image: value
                .get("Image")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            image_id: value
                .get("ImageID")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            command: value
                .get("Command")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            created: value.get("Created").and_then(|v| v.as_i64()).unwrap_or(0),
            status: value
                .get("Status")
                .and_then(|v| v.as_str())
                .and_then(|s| s.parse().ok())
                .unwrap_or(ContainerStatus::Created),
            state: value
                .get("State")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            ports: self.parse_ports_json(value.get("Ports")),
            labels: value
                .get("Labels")
                .and_then(|v| v.as_str())
                .map(|s| {
                    s.split(',')
                        .filter_map(|pair| {
                            let mut parts = pair.splitn(2, '=');
                            let key = parts.next()?;
                            let value = parts.next().unwrap_or("");
                            Some((key.to_string(), value.to_string()))
                        })
                        .collect()
                })
                .unwrap_or_default(),
            size_rw: value.get("SizeRw").and_then(|v| v.as_i64()),
            size_root_fs: value.get("SizeRootFs").and_then(|v| v.as_i64()),
            host_config: HostConfig {
                network_mode: value
                    .get("HostConfig")
                    .and_then(|v| v.get("NetworkMode"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
                cpu_shares: None,
                memory: None,
                memory_swap: None,
                cpu_percent: None,
                cpu_quota: None,
                cpu_period: None,
            },
            network_settings: NetworkSettings {
                networks: HashMap::new(),
                ip_address: String::new(),
                gateway: String::new(),
                mac_address: String::new(),
            },
            mounts: Vec::new(),
        })
    }

    /// Parse ports from JSON
    fn parse_ports_json(&self, value: Option<&serde_json::Value>) -> Vec<PortMapping> {
        value
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|p| {
                        Some(PortMapping {
                            ip: p.get("IP").and_then(|v| v.as_str())?.to_string(),
                            private_port: p.get("PrivatePort").and_then(|v| v.as_u64())? as u16,
                            public_port: p.get("PublicPort").and_then(|v| v.as_u64()).unwrap_or(0)
                                as u16,
                            protocol: p
                                .get("Type")
                                .and_then(|v| v.as_str())
                                .unwrap_or("tcp")
                                .to_string(),
                        })
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Parse ports from string
    fn parse_ports(&self, ports_str: Option<&&str>) -> Vec<PortMapping> {
        ports_str
            .map(|s| {
                s.split(", ")
                    .filter_map(|part| {
                        let mut parts = part.split("->");
                        let public_part = parts.next()?;
                        let private_part = parts.next()?;

                        let public_port = public_part.split(':').last()?.parse::<u16>().ok()?;
                        let private_port = private_part.split('/').next()?.parse::<u16>().ok()?;
                        let protocol = private_part.split('/').nth(1).unwrap_or("tcp");

                        Some(PortMapping {
                            ip: "0.0.0.0".to_string(),
                            private_port,
                            public_port,
                            protocol: protocol.to_string(),
                        })
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Parse status string
    fn parse_status(&self, status: &str) -> ContainerStatus {
        if status.starts_with("Up") {
            ContainerStatus::Running
        } else if status.starts_with("Exited") {
            ContainerStatus::Exited
        } else if status.starts_with("Paused") {
            ContainerStatus::Paused
        } else if status.starts_with("Restarting") {
            ContainerStatus::Restarting
        } else {
            ContainerStatus::Created
        }
    }

    /// Parse image JSON
    fn parse_image_json(&self, value: serde_json::Value) -> Result<ImageInfo, LiteError> {
        Ok(ImageInfo {
            id: value.get("ID").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            repo_tags: value
                .get("RepoTags")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(|s| s.to_string()))
                        .collect()
                })
                .unwrap_or_default(),
            repo_digests: value
                .get("RepoDigests")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(|s| s.to_string()))
                        .collect()
                })
                .unwrap_or_default(),
            parent: value
                .get("Parent")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            comment: value
                .get("Comment")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            created: value
                .get("Created")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            container: value
                .get("Container")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            size: value.get("Size").and_then(|v| v.as_i64()).unwrap_or(0),
            virtual_size: value.get("VirtualSize").and_then(|v| v.as_i64()).unwrap_or(0),
            shared_size: value.get("SharedSize").and_then(|v| v.as_i64()).unwrap_or(0),
            labels: value
                .get("Labels")
                .and_then(|v| v.as_object())
                .map(|obj| {
                    obj.iter()
                        .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
                        .collect()
                })
                .unwrap_or_default(),
        })
    }

    /// Parse network JSON
    fn parse_network_json(&self, value: serde_json::Value) -> Result<NetworkInfo, LiteError> {
        Ok(NetworkInfo {
            id: value.get("Id").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            name: value
                .get("Name")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            driver: value
                .get("Driver")
                .and_then(|v| v.as_str())
                .unwrap_or("bridge")
                .to_string(),
            scope: value
                .get("Scope")
                .and_then(|v| v.as_str())
                .unwrap_or("local")
                .to_string(),
            internal: value.get("Internal").and_then(|v| v.as_bool()).unwrap_or(false),
            enable_ipv6: value.get("EnableIPv6").and_then(|v| v.as_bool()).unwrap_or(false),
            ipam: super::types::IpamConfig {
                driver: "default".to_string(),
                config: Vec::new(),
                options: HashMap::new(),
            },
            labels: HashMap::new(),
            containers: HashMap::new(),
            options: HashMap::new(),
        })
    }

    /// Parse volume JSON
    fn parse_volume_json(&self, value: serde_json::Value) -> Result<VolumeInfo, LiteError> {
        Ok(VolumeInfo {
            name: value.get("Name").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            driver: value
                .get("Driver")
                .and_then(|v| v.as_str())
                .unwrap_or("local")
                .to_string(),
            mountpoint: value
                .get("Mountpoint")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            created_at: value
                .get("CreatedAt")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            status: None,
            labels: HashMap::new(),
            scope: value
                .get("Scope")
                .and_then(|v| v.as_str())
                .unwrap_or("local")
                .to_string(),
            options: None,
            usage_data: None,
        })
    }

    /// Parse size string like "1.23MB" to bytes
    fn parse_size(&self, size_str: &str) -> i64 {
        let size_str = size_str.trim();
        let num: f64 = size_str
            .chars()
            .take_while(|c| c.is_digit(10) || *c == '.')
            .collect::<String>()
            .parse()
            .unwrap_or(0.0);

        let unit: String = size_str.chars().skip_while(|c| c.is_digit(10) || *c == '.').collect();

        let multiplier = match unit.to_lowercase().as_str() {
            "b" => 1.0,
            "kb" | "kib" => 1024.0,
            "mb" | "mib" => 1024.0 * 1024.0,
            "gb" | "gib" => 1024.0 * 1024.0 * 1024.0,
            "tb" | "tib" => 1024.0 * 1024.0 * 1024.0 * 1024.0,
            _ => 1.0,
        };

        (num * multiplier) as i64
    }
}

impl Default for DockerManager {
    fn default() -> Self {
        Self::new()
    }
}
