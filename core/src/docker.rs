#![allow(dead_code)]

use crate::error::LiteError;
use crate::ssh::SshSessionManager;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::Read;
use tokio::sync::{mpsc, RwLock};
use tokio::task::JoinHandle;
use std::time::Duration;

/// Docker container status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ContainerStatus {
    Created,
    Running,
    Paused,
    Restarting,
    Removing,
    Exited,
    Dead,
}

impl ContainerStatus {
    pub fn is_running(&self) -> bool {
        matches!(self, Self::Running | Self::Restarting)
    }

    pub fn can_start(&self) -> bool {
        matches!(self, Self::Created | Self::Exited | Self::Dead)
    }

    pub fn can_stop(&self) -> bool {
        matches!(self, Self::Running | Self::Restarting | Self::Paused)
    }

    pub fn can_restart(&self) -> bool {
        matches!(self, Self::Running | Self::Exited | Self::Paused | Self::Dead)
    }

    pub fn can_pause(&self) -> bool {
        matches!(self, Self::Running)
    }

    pub fn can_unpause(&self) -> bool {
        matches!(self, Self::Paused)
    }
}

impl std::fmt::Display for ContainerStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Created => write!(f, "created"),
            Self::Running => write!(f, "running"),
            Self::Paused => write!(f, "paused"),
            Self::Restarting => write!(f, "restarting"),
            Self::Removing => write!(f, "removing"),
            Self::Exited => write!(f, "exited"),
            Self::Dead => write!(f, "dead"),
        }
    }
}

impl std::str::FromStr for ContainerStatus {
    type Err = LiteError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "created" => Ok(Self::Created),
            "running" => Ok(Self::Running),
            "paused" => Ok(Self::Paused),
            "restarting" => Ok(Self::Restarting),
            "removing" => Ok(Self::Removing),
            "exited" => Ok(Self::Exited),
            "dead" => Ok(Self::Dead),
            _ => Err(LiteError::Docker(format!("Unknown container status: {}", s))),
        }
    }
}

/// Docker container information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerInfo {
    pub id: String,
    pub names: Vec<String>,
    pub image: String,
    pub image_id: String,
    pub command: String,
    pub created: i64,
    pub status: ContainerStatus,
    pub state: String,
    pub ports: Vec<PortMapping>,
    pub labels: HashMap<String, String>,
    pub size_rw: Option<i64>,
    pub size_root_fs: Option<i64>,
    pub host_config: HostConfig,
    pub network_settings: NetworkSettings,
    pub mounts: Vec<MountPoint>,
}

/// Port mapping
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortMapping {
    pub ip: String,
    pub private_port: u16,
    pub public_port: u16,
    pub protocol: String,
}

/// Host configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HostConfig {
    pub network_mode: String,
    pub cpu_shares: Option<i64>,
    pub memory: Option<i64>,
    pub memory_swap: Option<i64>,
    pub cpu_percent: Option<i64>,
    pub cpu_quota: Option<i64>,
    pub cpu_period: Option<i64>,
}

/// Network settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkSettings {
    pub networks: HashMap<String, ContainerNetworkInfo>,
    pub ip_address: String,
    pub gateway: String,
    pub mac_address: String,
}

/// Container network information (for container's network attachment)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerNetworkInfo {
    pub network_id: String,
    pub endpoint_id: String,
    pub gateway: String,
    pub ip_address: String,
    pub ip_prefix_len: i32,
    pub mac_address: String,
}

/// Mount point
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MountPoint {
    pub mount_type: String,
    pub name: Option<String>,
    pub source: String,
    pub destination: String,
    pub driver: Option<String>,
    pub mode: String,
    pub rw: bool,
    pub propagation: String,
}

/// Docker image information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageInfo {
    pub id: String,
    pub repo_tags: Vec<String>,
    pub repo_digests: Vec<String>,
    pub parent: String,
    pub comment: String,
    pub created: String,
    pub container: String,
    pub size: i64,
    pub virtual_size: i64,
    pub shared_size: i64,
    pub labels: HashMap<String, String>,
}

/// Docker network information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkInfo {
    pub id: String,
    pub name: String,
    pub driver: String,
    pub scope: String,
    pub internal: bool,
    pub enable_ipv6: bool,
    pub ipam: IpamConfig,
    pub labels: HashMap<String, String>,
    pub containers: HashMap<String, NetworkContainer>,
    pub options: HashMap<String, String>,
}

/// IPAM configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IpamConfig {
    pub driver: String,
    pub config: Vec<IpamSubnetConfig>,
    pub options: HashMap<String, String>,
}

/// IPAM subnet configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IpamSubnetConfig {
    pub subnet: String,
    pub gateway: String,
    pub ip_range: Option<String>,
    pub auxiliary_addresses: HashMap<String, String>,
}

/// Network container attachment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkContainer {
    pub name: String,
    pub endpoint_id: String,
    pub mac_address: String,
    pub ipv4_address: String,
    pub ipv6_address: String,
}

/// Docker volume information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VolumeInfo {
    pub name: String,
    pub driver: String,
    pub mountpoint: String,
    pub created_at: String,
    pub status: Option<HashMap<String, serde_json::Value>>,
    pub labels: HashMap<String, String>,
    pub scope: String,
    pub options: Option<HashMap<String, String>>,
    pub usage_data: Option<VolumeUsageData>,
}

/// Volume usage data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VolumeUsageData {
    pub size: i64,
    pub ref_count: i32,
}

/// Container resource stats
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerStats {
    pub read: String,
    pub preread: String,
    pub pids_stats: PidsStats,
    pub memory_stats: MemoryStats,
    pub cpu_stats: CpuStats,
    pub io_stats: IoStats,
    pub network_stats: NetworkStats,
}

/// PIDs stats
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PidsStats {
    pub current: Option<i64>,
    pub limit: Option<i64>,
}

/// Memory stats
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryStats {
    pub usage: i64,
    pub stats: HashMap<String, i64>,
    pub limit: i64,
}

/// CPU stats
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CpuStats {
    pub total_usage: i64,
    pub percpu_usage: Option<Vec<i64>>,
    pub usage_in_kernelmode: i64,
    pub usage_in_usermode: i64,
    pub system_cpu_usage: Option<i64>,
    pub online_cpus: i64,
    pub throttling_data: ThrottlingData,
}

/// Throttling data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThrottlingData {
    pub periods: i64,
    pub throttled_periods: i64,
    pub throttled_time: i64,
}

/// I/O stats
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IoStats {
    pub io_service_bytes_recursive: Vec<IoEntry>,
    pub io_serviced_recursive: Vec<IoEntry>,
}

/// I/O entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IoEntry {
    pub major: i64,
    pub minor: i64,
    pub op: String,
    pub value: i64,
}

/// Network stats
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkStats {
    pub rx_bytes: i64,
    pub rx_packets: i64,
    pub rx_errors: i64,
    pub rx_dropped: i64,
    pub tx_bytes: i64,
    pub tx_packets: i64,
    pub tx_errors: i64,
    pub tx_dropped: i64,
}

/// Docker compose project
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComposeProject {
    pub name: String,
    pub status: String,
    pub config_files: Vec<String>,
    pub services: Vec<ComposeService>,
}

/// Compose service
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComposeService {
    pub name: String,
    pub image: String,
    pub state: String,
    pub replicas: i32,
    pub ports: Vec<PortMapping>,
    pub health: Option<String>,
}

/// Docker registry configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryConfig {
    pub name: String,
    pub url: String,
    pub username: Option<String>,
    pub password: Option<String>,
    pub insecure: bool,
    pub ca_cert: Option<String>,
}

/// Docker connection info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DockerConnection {
    pub name: String,
    pub host_type: DockerHostType,
    pub endpoint: String,
    pub ssh_session_id: Option<String>,
    pub tls_config: Option<DockerTlsConfig>,
}

/// Docker host type
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DockerHostType {
    Local,
    RemoteSsh,
    RemoteTcp,
    RemoteTls,
}

/// Docker TLS configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DockerTlsConfig {
    pub ca_cert: String,
    pub client_cert: String,
    pub client_key: String,
}

/// Container log stream
pub struct LogStream {
    pub container_id: String,
    pub follow: bool,
    pub timestamps: bool,
    pub since: Option<i64>,
    pub until: Option<i64>,
    pub tail: Option<i64>,
    pub stdout: bool,
    pub stderr: bool,
}

/// Docker manager for container management
pub struct DockerManager {
    connections: RwLock<HashMap<String, DockerConnection>>,
    active_logs: RwLock<HashMap<String, JoinHandle<()>>>,
    log_channels: RwLock<HashMap<String, mpsc::UnboundedSender<String>>>,
    exec_sessions: RwLock<HashMap<String, ExecSession>>,
}

/// Docker exec session
#[derive(Clone)]
struct ExecSession {
    container_id: String,
    command: String,
    tty: bool,
    stdin: bool,
    stdout: bool,
    stderr: bool,
}

impl DockerManager {
    pub fn new() -> Self {
        Self {
            connections: RwLock::new(HashMap::new()),
            active_logs: RwLock::new(HashMap::new()),
            log_channels: RwLock::new(HashMap::new()),
            exec_sessions: RwLock::new(HashMap::new()),
        }
    }

    /// Add a Docker connection
    pub async fn add_connection(&self, connection: DockerConnection) -> Result<(), LiteError> {
        let mut connections = self.connections.write().await;
        connections.insert(connection.name.clone(), connection);
        Ok(())
    }

    /// Remove a Docker connection
    pub async fn remove_connection(&self, name: &str) -> Result<(), LiteError> {
        let mut connections = self.connections.write().await;
        connections.remove(name);
        Ok(())
    }

    /// List all Docker connections
    pub async fn list_connections(&self) -> Vec<DockerConnection> {
        let connections = self.connections.read().await;
        connections.values().cloned().collect()
    }

    /// Get Docker connection by name
    pub async fn get_connection(&self, name: &str) -> Option<DockerConnection> {
        let connections = self.connections.read().await;
        connections.get(name).cloned()
    }

    /// List containers via SSH
    pub async fn list_containers(
        &self,
        ssh_manager: &SshSessionManager,
        ssh_session_id: &str,
        all: bool,
    ) -> Result<Vec<ContainerInfo>, LiteError> {
        let all_flag = if all { "-a" } else { "" };
        let cmd = format!(
            "docker ps {} --format '{{{{json .}}}}' 2>/dev/null || docker ps {} --format '{{{{.ID}}}}|{{{{.Names}}}}|{{{{.Image}}}}|{{{{.Status}}}}|{{{{.Ports}}}}'",
            all_flag, all_flag
        );

        let output = ssh_manager.execute_via_sftp(ssh_session_id, &cmd).await?;

        let mut containers = Vec::new();
        for line in output.lines().filter(|l| !l.is_empty()) {
            if let Ok(info) = serde_json::from_str::<serde_json::Value>(line) {
                containers.push(self.parse_container_json(info)?);
            } else {
                // Fallback to simple format parsing
                let parts: Vec<&str> = line.split('|').collect();
                if parts.len() >= 4 {
                    containers.push(ContainerInfo {
                        id: parts[0].to_string(),
                        names: vec![parts[1].to_string()],
                        image: parts[2].to_string(),
                        image_id: String::new(),
                        command: String::new(),
                        created: 0,
                        status: self.parse_status(parts[3]),
                        state: parts[3].to_string(),
                        ports: self.parse_ports(parts.get(4).unwrap_or(&"")),
                        labels: HashMap::new(),
                        size_rw: None,
                        size_root_fs: None,
                        host_config: HostConfig {
                            network_mode: String::new(),
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
                    });
                }
            }
        }

        Ok(containers)
    }

    /// Start container
    pub async fn start_container(
        &self,
        ssh_manager: &SshSessionManager,
        ssh_session_id: &str,
        container_id: &str,
    ) -> Result<(), LiteError> {
        let cmd = format!("docker start {}", container_id);
        let output = ssh_manager.execute_via_sftp(ssh_session_id, &cmd).await?;

        if output.trim() == container_id || output.trim() == container_id[..12].to_string() {
            Ok(())
        } else {
            Err(LiteError::Docker(format!("Failed to start container: {}", output)))
        }
    }

    /// Stop container
    pub async fn stop_container(
        &self,
        ssh_manager: &SshSessionManager,
        ssh_session_id: &str,
        container_id: &str,
        timeout: Option<u32>,
    ) -> Result<(), LiteError> {
        let timeout_flag = timeout.map(|t| format!(" -t {}", t)).unwrap_or_default();
        let cmd = format!("docker stop{}{} {}", timeout_flag, if timeout_flag.is_empty() { "" } else { "" }, container_id);
        let output = ssh_manager.execute_via_sftp(ssh_session_id, &cmd).await?;

        if output.trim() == container_id || output.trim() == container_id[..12].to_string() {
            Ok(())
        } else {
            Err(LiteError::Docker(format!("Failed to stop container: {}", output)))
        }
    }

    /// Restart container
    pub async fn restart_container(
        &self,
        ssh_manager: &SshSessionManager,
        ssh_session_id: &str,
        container_id: &str,
        timeout: Option<u32>,
    ) -> Result<(), LiteError> {
        let timeout_flag = timeout.map(|t| format!(" -t {}", t)).unwrap_or_default();
        let cmd = format!("docker restart{} {}", timeout_flag, container_id);
        let output = ssh_manager.execute_via_sftp(ssh_session_id, &cmd).await?;

        if output.trim() == container_id || output.trim() == container_id[..12].to_string() {
            Ok(())
        } else {
            Err(LiteError::Docker(format!("Failed to restart container: {}", output)))
        }
    }

    /// Pause container
    pub async fn pause_container(
        &self,
        ssh_manager: &SshSessionManager,
        ssh_session_id: &str,
        container_id: &str,
    ) -> Result<(), LiteError> {
        let cmd = format!("docker pause {}", container_id);
        let output = ssh_manager.execute_via_sftp(ssh_session_id, &cmd).await?;

        if output.trim() == container_id || output.trim() == container_id[..12].to_string() {
            Ok(())
        } else {
            Err(LiteError::Docker(format!("Failed to pause container: {}", output)))
        }
    }

    /// Unpause container
    pub async fn unpause_container(
        &self,
        ssh_manager: &SshSessionManager,
        ssh_session_id: &str,
        container_id: &str,
    ) -> Result<(), LiteError> {
        let cmd = format!("docker unpause {}", container_id);
        let output = ssh_manager.execute_via_sftp(ssh_session_id, &cmd).await?;

        if output.trim() == container_id || output.trim() == container_id[..12].to_string() {
            Ok(())
        } else {
            Err(LiteError::Docker(format!("Failed to unpause container: {}", output)))
        }
    }

    /// Kill container
    pub async fn kill_container(
        &self,
        ssh_manager: &SshSessionManager,
        ssh_session_id: &str,
        container_id: &str,
        signal: Option<&str>,
    ) -> Result<(), LiteError> {
        let signal_flag = signal.map(|s| format!(" -s {}", s)).unwrap_or_default();
        let cmd = format!("docker kill{} {}", signal_flag, container_id);
        let output = ssh_manager.execute_via_sftp(ssh_session_id, &cmd).await?;

        if output.trim() == container_id || output.trim() == container_id[..12].to_string() {
            Ok(())
        } else {
            Err(LiteError::Docker(format!("Failed to kill container: {}", output)))
        }
    }

    /// Remove container
    pub async fn remove_container(
        &self,
        ssh_manager: &SshSessionManager,
        ssh_session_id: &str,
        container_id: &str,
        force: bool,
        volumes: bool,
    ) -> Result<(), LiteError> {
        let force_flag = if force { " -f" } else { "" };
        let volumes_flag = if volumes { " -v" } else { "" };
        let cmd = format!("docker rm{}{} {}", force_flag, volumes_flag, container_id);
        let output = ssh_manager.execute_via_sftp(ssh_session_id, &cmd).await?;

        if output.trim().contains(&container_id[..12]) {
            Ok(())
        } else {
            Err(LiteError::Docker(format!("Failed to remove container: {}", output)))
        }
    }

    /// Create container
    pub async fn create_container(
        &self,
        ssh_manager: &SshSessionManager,
        ssh_session_id: &str,
        name: Option<&str>,
        image: &str,
        command: Option<&str>,
        ports: &[(u16, u16, &str)], // (host, container, protocol)
        volumes: &[(&str, &str)], // (host, container)
        env: &[(&str, &str)],
        network: Option<&str>,
        restart: Option<&str>,
        labels: &[(&str, &str)],
    ) -> Result<String, LiteError> {
        let name_flag = name.map(|n| format!(" --name {}", n)).unwrap_or_default();
        let network_flag = network.map(|n| format!(" --network {}", n)).unwrap_or_default();
        let restart_flag = restart.map(|r| format!(" --restart {}", r)).unwrap_or_default();

        let mut ports_flags = String::new();
        for (host, container, proto) in ports {
            ports_flags.push_str(&format!(" -p {}:{}/{}", host, container, proto));
        }

        let mut volumes_flags = String::new();
        for (host, container) in volumes {
            volumes_flags.push_str(&format!(" -v {}:{}", host, container));
        }

        let mut env_flags = String::new();
        for (key, value) in env {
            env_flags.push_str(&format!(" -e {}='{}'", key, value.replace("'", "'\\''")));
        }

        let mut labels_flags = String::new();
        for (key, value) in labels {
            labels_flags.push_str(&format!(" -l {}={}", key, value));
        }

        let cmd_flag = command.map(|c| format!(" {}", c)).unwrap_or_default();

        let cmd = format!(
            "docker create{}{}{}{}{}{}{}{} {}{}",
            name_flag, ports_flags, volumes_flags, env_flags,
            network_flag, restart_flag, labels_flags, image, cmd_flag, ""
        );

        let output = ssh_manager.execute_via_sftp(ssh_session_id, &cmd).await?;
        let container_id = output.trim();

        if container_id.len() == 64 && container_id.chars().all(|c| c.is_ascii_hexdigit()) {
            Ok(container_id.to_string())
        } else {
            Err(LiteError::Docker(format!("Failed to create container: {}", output)))
        }
    }

    /// List images
    pub async fn list_images(
        &self,
        ssh_manager: &SshSessionManager,
        ssh_session_id: &str,
        all: bool,
        dangling: bool,
    ) -> Result<Vec<ImageInfo>, LiteError> {
        let all_flag = if all { " -a" } else { "" };
        let filter_flag = if dangling { " --filter dangling=true" } else { "" };

        let cmd = format!(
            "docker images{} --format '{{{{json .}}}}' 2>/dev/null || docker images{}{} --format '{{{{.ID}}}}|{{{{.Repository}}}}|{{{{.Tag}}}}|{{{{.Size}}}}|{{{{.CreatedAt}}}}'",
            all_flag, all_flag, filter_flag
        );

        let output = ssh_manager.execute_via_sftp(ssh_session_id, &cmd).await?;

        let mut images = Vec::new();
        for line in output.lines().filter(|l| !l.is_empty()) {
            if let Ok(info) = serde_json::from_str::<serde_json::Value>(line) {
                images.push(self.parse_image_json(info)?);
            } else {
                let parts: Vec<&str> = line.split('|').collect();
                if parts.len() >= 4 {
                    images.push(ImageInfo {
                        id: parts[0].to_string(),
                        repo_tags: vec![format!("{}:{}", parts[1], parts[2])],
                        repo_digests: Vec::new(),
                        parent: String::new(),
                        comment: String::new(),
                        created: parts.get(4).unwrap_or(&"").to_string(),
                        container: String::new(),
                        size: self.parse_size(parts[3]),
                        virtual_size: 0,
                        shared_size: 0,
                        labels: HashMap::new(),
                    });
                }
            }
        }

        Ok(images)
    }

    /// Pull image
    pub async fn pull_image(
        &self,
        ssh_manager: &SshSessionManager,
        ssh_session_id: &str,
        image: &str,
        tag: Option<&str>,
        registry: Option<&str>,
    ) -> Result<(), LiteError> {
        let full_image = if let Some(reg) = registry {
            format!("{}/{}", reg, image)
        } else {
            image.to_string()
        };

        let full_image = if let Some(t) = tag {
            format!("{}:{}", full_image, t)
        } else {
            full_image
        };

        let cmd = format!("docker pull {}", full_image);
        let output = ssh_manager.execute_via_sftp(ssh_session_id, &cmd).await?;

        if output.contains("Downloaded") || output.contains("up to date") || output.contains("Already exists") {
            Ok(())
        } else {
            Err(LiteError::Docker(format!("Failed to pull image: {}", output)))
        }
    }

    /// Remove image
    pub async fn remove_image(
        &self,
        ssh_manager: &SshSessionManager,
        ssh_session_id: &str,
        image_id: &str,
        force: bool,
    ) -> Result<(), LiteError> {
        let force_flag = if force { " -f" } else { "" };
        let cmd = format!("docker rmi{} {}", force_flag, image_id);
        let output = ssh_manager.execute_via_sftp(ssh_session_id, &cmd).await?;

        if output.contains("Deleted") || output.contains("Untagged") {
            Ok(())
        } else {
            Err(LiteError::Docker(format!("Failed to remove image: {}", output)))
        }
    }

    /// Tag image
    pub async fn tag_image(
        &self,
        ssh_manager: &SshSessionManager,
        ssh_session_id: &str,
        source: &str,
        target: &str,
    ) -> Result<(), LiteError> {
        let cmd = format!("docker tag {} {}", source, target);
        let output = ssh_manager.execute_via_sftp(ssh_session_id, &cmd).await?;

        if output.trim().is_empty() {
            Ok(())
        } else {
            Err(LiteError::Docker(format!("Failed to tag image: {}", output)))
        }
    }

    /// Push image
    pub async fn push_image(
        &self,
        ssh_manager: &SshSessionManager,
        ssh_session_id: &str,
        image: &str,
    ) -> Result<(), LiteError> {
        let cmd = format!("docker push {}", image);
        let output = ssh_manager.execute_via_sftp(ssh_session_id, &cmd).await?;

        if output.contains("pushed") || output.contains("Layer already exists") {
            Ok(())
        } else {
            Err(LiteError::Docker(format!("Failed to push image: {}", output)))
        }
    }

    /// List networks
    pub async fn list_networks(
        &self,
        ssh_manager: &SshSessionManager,
        ssh_session_id: &str,
    ) -> Result<Vec<NetworkInfo>, LiteError> {
        let cmd = format!(
            "docker network ls --format '{{{{json .}}}}' 2>/dev/null || docker network ls --format '{{{{.ID}}}}|{{{{.Name}}}}|{{{{.Driver}}}}|{{{{.Scope}}}}'"
        );

        let output = ssh_manager.execute_via_sftp(ssh_session_id, &cmd).await?;

        let mut networks = Vec::new();
        for line in output.lines().filter(|l| !l.is_empty()) {
            if let Ok(info) = serde_json::from_str::<serde_json::Value>(line) {
                networks.push(self.parse_network_json(info)?);
            } else {
                let parts: Vec<&str> = line.split('|').collect();
                if parts.len() >= 4 {
                    networks.push(NetworkInfo {
                        id: parts[0].to_string(),
                        name: parts[1].to_string(),
                        driver: parts[2].to_string(),
                        scope: parts[3].to_string(),
                        internal: false,
                        enable_ipv6: false,
                        ipam: IpamConfig {
                            driver: "default".to_string(),
                            config: Vec::new(),
                            options: HashMap::new(),
                        },
                        labels: HashMap::new(),
                        containers: HashMap::new(),
                        options: HashMap::new(),
                    });
                }
            }
        }

        Ok(networks)
    }

    /// Create network
    pub async fn create_network(
        &self,
        ssh_manager: &SshSessionManager,
        ssh_session_id: &str,
        name: &str,
        driver: &str,
        subnet: Option<&str>,
        gateway: Option<&str>,
        internal: bool,
        ipv6: bool,
    ) -> Result<String, LiteError> {
        let internal_flag = if internal { " --internal" } else { "" };
        let ipv6_flag = if ipv6 { " --ipv6" } else { "" };
        let subnet_flag = subnet.map(|s| format!(" --subnet {}", s)).unwrap_or_default();
        let gateway_flag = gateway.map(|g| format!(" --gateway {}", g)).unwrap_or_default();

        let cmd = format!(
            "docker network create{}{}{}{} --driver {} {}",
            internal_flag, ipv6_flag, subnet_flag, gateway_flag, driver, name
        );

        let output = ssh_manager.execute_via_sftp(ssh_session_id, &cmd).await?;
        let network_id = output.trim();

        if network_id.len() >= 12 && network_id.chars().all(|c| c.is_ascii_hexdigit()) {
            Ok(network_id.to_string())
        } else {
            Err(LiteError::Docker(format!("Failed to create network: {}", output)))
        }
    }

    /// Remove network
    pub async fn remove_network(
        &self,
        ssh_manager: &SshSessionManager,
        ssh_session_id: &str,
        network_id: &str,
    ) -> Result<(), LiteError> {
        let cmd = format!("docker network rm {}", network_id);
        let output = ssh_manager.execute_via_sftp(ssh_session_id, &cmd).await?;

        if output.trim() == network_id || output.trim().contains(&network_id[..12]) {
            Ok(())
        } else {
            Err(LiteError::Docker(format!("Failed to remove network: {}", output)))
        }
    }

    /// Inspect network
    pub async fn inspect_network(
        &self,
        ssh_manager: &SshSessionManager,
        ssh_session_id: &str,
        network_id: &str,
    ) -> Result<NetworkInfo, LiteError> {
        let cmd = format!("docker network inspect {}", network_id);
        let output = ssh_manager.execute_via_sftp(ssh_session_id, &cmd).await?;

        if let Ok(info) = serde_json::from_str::<serde_json::Value>(&output) {
            if let Some(arr) = info.as_array() {
                if let Some(first) = arr.first() {
                    return self.parse_network_inspect_json(first.clone());
                }
            }
        }

        Err(LiteError::Docker(format!("Failed to inspect network: {}", output)))
    }

    /// List volumes
    pub async fn list_volumes(
        &self,
        ssh_manager: &SshSessionManager,
        ssh_session_id: &str,
    ) -> Result<Vec<VolumeInfo>, LiteError> {
        let cmd = format!(
            "docker volume ls --format '{{{{json .}}}}' 2>/dev/null || docker volume ls --format '{{{{.Name}}}}|{{{{.Driver}}}}|{{{{.Scope}}}}'"
        );

        let output = ssh_manager.execute_via_sftp(ssh_session_id, &cmd).await?;

        let mut volumes = Vec::new();
        for line in output.lines().filter(|l| !l.is_empty()) {
            if let Ok(info) = serde_json::from_str::<serde_json::Value>(line) {
                volumes.push(self.parse_volume_json(info)?);
            } else {
                let parts: Vec<&str> = line.split('|').collect();
                if parts.len() >= 3 {
                    volumes.push(VolumeInfo {
                        name: parts[0].to_string(),
                        driver: parts[1].to_string(),
                        mountpoint: String::new(),
                        created_at: String::new(),
                        status: None,
                        labels: HashMap::new(),
                        scope: parts[2].to_string(),
                        options: None,
                        usage_data: None,
                    });
                }
            }
        }

        Ok(volumes)
    }

    /// Create volume
    pub async fn create_volume(
        &self,
        ssh_manager: &SshSessionManager,
        ssh_session_id: &str,
        name: &str,
        driver: &str,
        driver_opts: &[(&str, &str)],
    ) -> Result<VolumeInfo, LiteError> {
        let mut opts_flags = String::new();
        for (key, value) in driver_opts {
            opts_flags.push_str(&format!(" -o {}={}", key, value));
        }

        let cmd = format!("docker volume create{} --driver {} {}", opts_flags, driver, name);
        let output = ssh_manager.execute_via_sftp(ssh_session_id, &cmd).await?;

        let vol_name = output.trim();
        if !vol_name.is_empty() {
            // Get volume details
            let inspect_cmd = format!("docker volume inspect {}", vol_name);
            let inspect_output = ssh_manager.execute_via_sftp(ssh_session_id, &inspect_cmd).await?;

            if let Ok(info) = serde_json::from_str::<serde_json::Value>(&inspect_output) {
                if let Some(arr) = info.as_array() {
                    if let Some(first) = arr.first() {
                        return self.parse_volume_json(first.clone());
                    }
                }
            }
        }

        Err(LiteError::Docker(format!("Failed to create volume: {}", output)))
    }

    /// Remove volume
    pub async fn remove_volume(
        &self,
        ssh_manager: &SshSessionManager,
        ssh_session_id: &str,
        volume_name: &str,
        force: bool,
    ) -> Result<(), LiteError> {
        let force_flag = if force { " -f" } else { "" };
        let cmd = format!("docker volume rm{} {}", force_flag, volume_name);
        let output = ssh_manager.execute_via_sftp(ssh_session_id, &cmd).await?;

        if output.trim() == volume_name {
            Ok(())
        } else {
            Err(LiteError::Docker(format!("Failed to remove volume: {}", output)))
        }
    }

    /// Prune volumes
    pub async fn prune_volumes(
        &self,
        ssh_manager: &SshSessionManager,
        ssh_session_id: &str,
    ) -> Result<String, LiteError> {
        let cmd = "docker volume prune -f".to_string();
        let output = ssh_manager.execute_via_sftp(ssh_session_id, &cmd).await?;
        Ok(output)
    }

    /// Start streaming container logs
    pub async fn stream_logs(
        &self,
        ssh_manager: &SshSessionManager,
        ssh_session_id: &str,
        container_id: &str,
        follow: bool,
        tail: Option<i64>,
    ) -> Result<mpsc::UnboundedReceiver<String>, LiteError> {
        let (tx, rx) = mpsc::unbounded_channel();
        let tx_for_insert = tx.clone();

        let follow_flag = if follow { " -f" } else { "" };
        let tail_flag = tail.map(|t| format!(" --tail {}", t)).unwrap_or_default();
        let cmd = format!("docker logs{}{} {}", follow_flag, tail_flag, container_id);

        let session_arc = ssh_manager.get_sftp_session_arc(ssh_session_id)
            .ok_or_else(|| LiteError::SshSessionNotFound(ssh_session_id.to_string()))?;

        let container_id = container_id.to_string();
        let session_id = ssh_session_id.to_string();
        let container_id_for_async = container_id.clone();
        let _session_id_for_async = session_id.clone();
        let container_id_for_maps = container_id.clone();
        let session_id_for_maps = session_id.clone();

        let handle = tokio::spawn(async move {
            let tx_clone = tx.clone();
            let _container_id_inner = container_id_for_async.clone();
            let _result = tokio::task::spawn_blocking(move || {
                let session_guard = session_arc.blocking_lock();

                let mut channel = match session_guard.channel_session() {
                    Ok(ch) => ch,
                    Err(_) => return,
                };

                if channel.exec(&cmd).is_err() {
                    return;
                }

                let mut buf = [0u8; 4096];
                loop {
                    match channel.read(&mut buf) {
                        Ok(0) => {
                            if channel.eof() {
                                break;
                            }
                            std::thread::sleep(Duration::from_millis(10));
                        }
                        Ok(n) => {
                            let text = String::from_utf8_lossy(&buf[..n]);
                            if tx_clone.send(text.to_string()).is_err() {
                                break;
                            }
                        }
                        Err(_) => break,
                    }
                }
                let _ = channel.wait_close();
            }).await;

            log::info!("Log stream ended for container {}", container_id_for_async);
        });

        let mut active_logs = self.active_logs.write().await;
        active_logs.insert(format!("{}_{}", session_id_for_maps, container_id_for_maps), handle);

        let mut log_channels = self.log_channels.write().await;
        log_channels.insert(format!("{}_{}", session_id_for_maps, container_id_for_maps), tx_for_insert);

        Ok(rx)
    }

    /// Stop streaming logs
    pub async fn stop_log_stream(&self, session_id: &str, container_id: &str) -> Result<(), LiteError> {
        let key = format!("{}_{}", session_id, container_id);

        let mut active_logs = self.active_logs.write().await;
        if let Some(handle) = active_logs.remove(&key) {
            handle.abort();
        }

        let mut log_channels = self.log_channels.write().await;
        log_channels.remove(&key);

        Ok(())
    }

    /// Build image from Dockerfile
    pub async fn build_image(
        &self,
        ssh_manager: &SshSessionManager,
        ssh_session_id: &str,
        context_path: &str,
        dockerfile_path: Option<&str>,
        tag: Option<&str>,
        build_args: &[(&str, &str)],
        no_cache: bool,
    ) -> Result<String, LiteError> {
        let dockerfile_flag = dockerfile_path.map(|d| format!(" -f {}", d)).unwrap_or_default();
        let tag_flag = tag.map(|t| format!(" -t {}", t)).unwrap_or_default();
        let no_cache_flag = if no_cache { " --no-cache" } else { "" };

        let mut build_args_flags = String::new();
        for (key, value) in build_args {
            build_args_flags.push_str(&format!(" --build-arg {}='{}'", key, value.replace("'", "'\\''")));
        }

        let cmd = format!(
            "cd {} && docker build{} {} .{}{}",
            context_path, dockerfile_flag, tag_flag, no_cache_flag, build_args_flags
        );

        let output = ssh_manager.execute_via_sftp(ssh_session_id, &cmd).await?;

        // Parse build output to find image ID
        let image_id = output.lines()
            .filter(|line| line.contains("Successfully built "))
            .last()
            .and_then(|line| line.split("Successfully built ").nth(1))
            .map(|s| s.trim().to_string());

        match image_id {
            Some(id) => Ok(id),
            None => {
                if output.contains("error") || output.contains("Error") {
                    Err(LiteError::Docker(format!("Build failed: {}", output)))
                } else {
                    // Try to find image ID from 'writing image' line
                    let img_id = output.lines()
                        .filter(|line| line.contains("writing image "))
                        .last()
                        .and_then(|line| {
                            let start = line.find("sha256:")?;
                            let end = line[start..].find(' ').unwrap_or(line[start..].len());
                            Some(line[start..start + end].to_string())
                        });

                    match img_id {
                        Some(id) => Ok(id),
                        None => Err(LiteError::Docker(format!("Build output unclear: {}", output)))
                    }
                }
            }
        }
    }

    /// Build image with streaming output
    pub async fn build_image_stream(
        &self,
        ssh_manager: &SshSessionManager,
        ssh_session_id: &str,
        context_path: &str,
        dockerfile_path: Option<&str>,
        tag: Option<&str>,
        build_args: &[(&str, &str)],
        no_cache: bool,
    ) -> Result<mpsc::UnboundedReceiver<String>, LiteError> {
        let (tx, rx) = mpsc::unbounded_channel();

        let dockerfile_flag = dockerfile_path.map(|d| format!(" -f {}", d)).unwrap_or_default();
        let tag_flag = tag.map(|t| format!(" -t {}", t)).unwrap_or_default();
        let no_cache_flag = if no_cache { " --no-cache" } else { "" };

        let mut build_args_flags = String::new();
        for (key, value) in build_args {
            build_args_flags.push_str(&format!(" --build-arg {}='{}'", key, value.replace("'", "'\\''")));
        }

        let cmd = format!(
            "cd {} && docker build{} {} . --progress=plain{}{}",
            context_path, dockerfile_flag, tag_flag, no_cache_flag, build_args_flags
        );

        let session_arc = ssh_manager.get_sftp_session_arc(ssh_session_id)
            .ok_or_else(|| LiteError::SshSessionNotFound(ssh_session_id.to_string()))?;

        let handle = tokio::spawn(async move {
            let _result = tokio::task::spawn_blocking(move || {
                let session_guard = session_arc.blocking_lock();

                let mut channel = match session_guard.channel_session() {
                    Ok(ch) => ch,
                    Err(_) => return,
                };

                if channel.exec(&cmd).is_err() {
                    return;
                }

                let mut buf = [0u8; 4096];
                loop {
                    match channel.read(&mut buf) {
                        Ok(0) => {
                            if channel.eof() {
                                break;
                            }
                            std::thread::sleep(Duration::from_millis(10));
                        }
                        Ok(n) => {
                            let text = String::from_utf8_lossy(&buf[..n]);
                            if tx.send(text.to_string()).is_err() {
                                break;
                            }
                        }
                        Err(_) => break,
                    }
                }
                let _ = channel.wait_close();
            }).await;

            log::info!("Build stream ended");
        });

        // Store handle for potential cancellation
        let build_key = format!("build_{}_{}", ssh_session_id, context_path.replace('/', "_"));
        let mut active_logs = self.active_logs.write().await;
        active_logs.insert(build_key.clone(), handle);

        Ok(rx)
    }

    /// Stream container stats in real-time
    pub async fn stream_stats(
        &self,
        ssh_manager: &SshSessionManager,
        ssh_session_id: &str,
        container_id: &str,
    ) -> Result<mpsc::UnboundedReceiver<ContainerStats>, LiteError> {
        let (tx, rx) = mpsc::unbounded_channel();

        let cmd = format!("docker stats {} --format '{{{{json .}}}}'", container_id);

        let session_arc = ssh_manager.get_sftp_session_arc(ssh_session_id)
            .ok_or_else(|| LiteError::SshSessionNotFound(ssh_session_id.to_string()))?;

        let container_id = container_id.to_string();
        let container_id_for_async = container_id.clone();

        let handle = tokio::spawn(async move {
            let tx_clone = tx.clone();
            let _result = tokio::task::spawn_blocking(move || {
                let session_guard = session_arc.blocking_lock();

                let mut channel = match session_guard.channel_session() {
                    Ok(ch) => ch,
                    Err(_) => return,
                };

                if channel.exec(&cmd).is_err() {
                    return;
                }

                let mut buf = [0u8; 8192];
                let mut line_buffer = String::new();

                loop {
                    match channel.read(&mut buf) {
                        Ok(0) => {
                            if channel.eof() {
                                break;
                            }
                            std::thread::sleep(Duration::from_millis(100));
                        }
                        Ok(n) => {
                            let text = String::from_utf8_lossy(&buf[..n]);
                            line_buffer.push_str(&text);

                            // Process complete lines
                            while let Some(pos) = line_buffer.find('\n') {
                                let line = line_buffer[..pos].to_string();
                                line_buffer = line_buffer[pos + 1..].to_string();

                                if let Ok(stats) = serde_json::from_str::<ContainerStats>(&line) {
                                    if tx_clone.send(stats).is_err() {
                                        return;
                                    }
                                }
                            }
                        }
                        Err(_) => break,
                    }
                }
                let _ = channel.wait_close();
            }).await;

            log::info!("Stats stream ended for container {}", container_id_for_async);
        });

        let stats_key = format!("stats_{}_{}", ssh_session_id, container_id);
        let mut active_logs = self.active_logs.write().await;
        active_logs.insert(stats_key, handle);

        Ok(rx)
    }

    /// Stop stats stream
    pub async fn stop_stats_stream(&self, session_id: &str, container_id: &str) -> Result<(), LiteError> {
        let key = format!("stats_{}_{}", session_id, container_id);

        let mut active_logs = self.active_logs.write().await;
        if let Some(handle) = active_logs.remove(&key) {
            handle.abort();
        }

        Ok(())
    }

    /// Export container to tar archive
    pub async fn export_container(
        &self,
        ssh_manager: &SshSessionManager,
        ssh_session_id: &str,
        container_id: &str,
        output_path: &str,
    ) -> Result<(), LiteError> {
        let cmd = format!("docker export {} > {}", container_id, output_path);
        let output = ssh_manager.execute_via_sftp(ssh_session_id, &cmd).await?;

        if output.trim().is_empty() {
            Ok(())
        } else {
            Err(LiteError::Docker(format!("Failed to export container: {}", output)))
        }
    }

    /// Import container from tar archive
    pub async fn import_image(
        &self,
        ssh_manager: &SshSessionManager,
        ssh_session_id: &str,
        input_path: &str,
        repository: Option<&str>,
        tag: Option<&str>,
    ) -> Result<String, LiteError> {
        let repo_flag = repository.map(|r| format!("- {} ", r)).unwrap_or_default();
        let tag_flag = tag.map(|t| format!("{}:", t)).unwrap_or_default();

        let cmd = format!("cat {} | docker import {}{}-", input_path, repo_flag, tag_flag);
        let output = ssh_manager.execute_via_sftp(ssh_session_id, &cmd).await?;

        let image_id = output.trim();
        if !image_id.is_empty() && image_id.starts_with("sha256:") {
            Ok(image_id.to_string())
        } else {
            Err(LiteError::Docker(format!("Failed to import image: {}", output)))
        }
    }

    /// Save image to tar archive
    pub async fn save_image(
        &self,
        ssh_manager: &SshSessionManager,
        ssh_session_id: &str,
        image: &str,
        output_path: &str,
    ) -> Result<(), LiteError> {
        let cmd = format!("docker save {} -o {}", image, output_path);
        let output = ssh_manager.execute_via_sftp(ssh_session_id, &cmd).await?;

        if output.trim().is_empty() {
            Ok(())
        } else {
            Err(LiteError::Docker(format!("Failed to save image: {}", output)))
        }
    }

    /// Load image from tar archive
    pub async fn load_image(
        &self,
        ssh_manager: &SshSessionManager,
        ssh_session_id: &str,
        input_path: &str,
    ) -> Result<String, LiteError> {
        let cmd = format!("docker load -i {}", input_path);
        let output = ssh_manager.execute_via_sftp(ssh_session_id, &cmd).await?;

        // Parse output to find loaded image
        let image = output.lines()
            .filter(|line| line.contains("Loaded image: "))
            .last()
            .and_then(|line| line.split("Loaded image: ").nth(1))
            .map(|s| s.trim().to_string());

        match image {
            Some(img) => Ok(img),
            None => {
                if output.contains("Loaded image") {
                    Ok(output.trim().to_string())
                } else {
                    Err(LiteError::Docker(format!("Failed to load image: {}", output)))
                }
            }
        }
    }

    /// Copy files from container to host
    pub async fn copy_from_container(
        &self,
        ssh_manager: &SshSessionManager,
        ssh_session_id: &str,
        container_id: &str,
        container_path: &str,
        host_path: &str,
    ) -> Result<(), LiteError> {
        let cmd = format!("docker cp {}:{} {}", container_id, container_path, host_path);
        let output = ssh_manager.execute_via_sftp(ssh_session_id, &cmd).await?;

        if output.trim().is_empty() {
            Ok(())
        } else {
            Err(LiteError::Docker(format!("Failed to copy from container: {}", output)))
        }
    }

    /// Copy files from host to container
    pub async fn copy_to_container(
        &self,
        ssh_manager: &SshSessionManager,
        ssh_session_id: &str,
        host_path: &str,
        container_id: &str,
        container_path: &str,
    ) -> Result<(), LiteError> {
        let cmd = format!("docker cp {} {}:{}", host_path, container_id, container_path);
        let output = ssh_manager.execute_via_sftp(ssh_session_id, &cmd).await?;

        if output.trim().is_empty() {
            Ok(())
        } else {
            Err(LiteError::Docker(format!("Failed to copy to container: {}", output)))
        }
    }

    /// Get Docker system information
    pub async fn get_system_info(
        &self,
        ssh_manager: &SshSessionManager,
        ssh_session_id: &str,
    ) -> Result<DockerSystemInfo, LiteError> {
        let cmd = "docker system info --format '{{json .}}'".to_string();
        let output = ssh_manager.execute_via_sftp(ssh_session_id, &cmd).await?;

        serde_json::from_str(&output)
            .map_err(|e| LiteError::Docker(format!("Failed to parse system info: {}", e)))
    }

    /// Stream Docker events
    pub async fn stream_events(
        &self,
        ssh_manager: &SshSessionManager,
        ssh_session_id: &str,
        since: Option<i64>,
        until: Option<i64>,
        filters: &[(&str, &str)],
    ) -> Result<mpsc::UnboundedReceiver<DockerEvent>, LiteError> {
        let (tx, rx) = mpsc::unbounded_channel();

        let since_flag = since.map(|s| format!(" --since {}", s)).unwrap_or_default();
        let until_flag = until.map(|u| format!(" --until {}", u)).unwrap_or_default();

        let mut filter_flags = String::new();
        for (key, value) in filters {
            filter_flags.push_str(&format!(" --filter '{}={}'", key, value));
        }

        let cmd = format!("docker events{} {}{} --format '{{{{json .}}}}'", since_flag, until_flag, filter_flags);

        let session_arc = ssh_manager.get_sftp_session_arc(ssh_session_id)
            .ok_or_else(|| LiteError::SshSessionNotFound(ssh_session_id.to_string()))?;

        let handle = tokio::spawn(async move {
            let _result = tokio::task::spawn_blocking(move || {
                let session_guard = session_arc.blocking_lock();

                let mut channel = match session_guard.channel_session() {
                    Ok(ch) => ch,
                    Err(_) => return,
                };

                if channel.exec(&cmd).is_err() {
                    return;
                }

                let mut buf = [0u8; 4096];
                let mut line_buffer = String::new();

                loop {
                    match channel.read(&mut buf) {
                        Ok(0) => {
                            if channel.eof() {
                                break;
                            }
                            std::thread::sleep(Duration::from_millis(100));
                        }
                        Ok(n) => {
                            let text = String::from_utf8_lossy(&buf[..n]);
                            line_buffer.push_str(&text);

                            // Process complete lines
                            while let Some(pos) = line_buffer.find('\n') {
                                let line = line_buffer[..pos].to_string();
                                line_buffer = line_buffer[pos + 1..].to_string();

                                if let Ok(event) = serde_json::from_str::<DockerEvent>(&line) {
                                    if tx.send(event).is_err() {
                                        return;
                                    }
                                }
                            }
                        }
                        Err(_) => break,
                    }
                }
                let _ = channel.wait_close();
            }).await;

            log::info!("Events stream ended");
        });

        let events_key = format!("events_{}", ssh_session_id);
        let mut active_logs = self.active_logs.write().await;
        active_logs.insert(events_key, handle);

        Ok(rx)
    }

    /// Stop events stream
    pub async fn stop_events_stream(&self, session_id: &str) -> Result<(), LiteError> {
        let key = format!("events_{}", session_id);

        let mut active_logs = self.active_logs.write().await;
        if let Some(handle) = active_logs.remove(&key) {
            handle.abort();
        }

        Ok(())
    }

    /// Prune containers, images, networks, or volumes
    pub async fn prune(
        &self,
        ssh_manager: &SshSessionManager,
        ssh_session_id: &str,
        resource_type: &str, // "containers", "images", "networks", "volumes", "system", "build"
        all: bool,
        filters: &[(&str, &str)],
    ) -> Result<String, LiteError> {
        let all_flag = if all && resource_type == "containers" { " -a" } else { "" };

        let mut filter_flags = String::new();
        for (key, value) in filters {
            filter_flags.push_str(&format!(" --filter '{}={}'", key, value));
        }

        let cmd = format!("docker {} prune -f{}{}", resource_type, all_flag, filter_flags);
        let output = ssh_manager.execute_via_sftp(ssh_session_id, &cmd).await?;

        Ok(output)
    }

    /// Inspect container
    pub async fn inspect_container(
        &self,
        ssh_manager: &SshSessionManager,
        ssh_session_id: &str,
        container_id: &str,
    ) -> Result<ContainerInfo, LiteError> {
        let cmd = format!("docker inspect {}", container_id);
        let output = ssh_manager.execute_via_sftp(ssh_session_id, &cmd).await?;

        if let Ok(info) = serde_json::from_str::<serde_json::Value>(&output) {
            if let Some(arr) = info.as_array() {
                if let Some(first) = arr.first() {
                    return self.parse_container_json(first.clone());
                }
            }
        }

        Err(LiteError::Docker(format!("Failed to inspect container: {}", output)))
    }

    /// Inspect image
    pub async fn inspect_image(
        &self,
        ssh_manager: &SshSessionManager,
        ssh_session_id: &str,
        image_id: &str,
    ) -> Result<ImageInfo, LiteError> {
        let cmd = format!("docker image inspect {}", image_id);
        let output = ssh_manager.execute_via_sftp(ssh_session_id, &cmd).await?;

        if let Ok(info) = serde_json::from_str::<serde_json::Value>(&output) {
            if let Some(arr) = info.as_array() {
                if let Some(first) = arr.first() {
                    return self.parse_image_json(first.clone());
                }
            }
        }

        Err(LiteError::Docker(format!("Failed to inspect image: {}", output)))
    }

    /// Get container processes (top)
    pub async fn top(
        &self,
        ssh_manager: &SshSessionManager,
        ssh_session_id: &str,
        container_id: &str,
    ) -> Result<String, LiteError> {
        let cmd = format!("docker top {}", container_id);
        let output = ssh_manager.execute_via_sftp(ssh_session_id, &cmd).await?;
        Ok(output)
    }

    /// Wait for container to finish and return exit code
    pub async fn wait(
        &self,
        ssh_manager: &SshSessionManager,
        ssh_session_id: &str,
        container_id: &str,
    ) -> Result<i32, LiteError> {
        let cmd = format!("docker wait {}", container_id);
        let output = ssh_manager.execute_via_sftp(ssh_session_id, &cmd).await?;

        output.trim().parse::<i32>()
            .map_err(|_| LiteError::Docker(format!("Failed to parse exit code: {}", output)))
    }

    /// Rename container
    pub async fn rename_container(
        &self,
        ssh_manager: &SshSessionManager,
        ssh_session_id: &str,
        container_id: &str,
        new_name: &str,
    ) -> Result<(), LiteError> {
        let cmd = format!("docker rename {} {}", container_id, new_name);
        let output = ssh_manager.execute_via_sftp(ssh_session_id, &cmd).await?;

        if output.trim().is_empty() {
            Ok(())
        } else {
            Err(LiteError::Docker(format!("Failed to rename container: {}", output)))
        }
    }

    /// Update container resources
    pub async fn update_container(
        &self,
        ssh_manager: &SshSessionManager,
        ssh_session_id: &str,
        container_id: &str,
        cpu_shares: Option<i64>,
        memory: Option<i64>,
        memory_swap: Option<i64>,
        cpu_period: Option<i64>,
        cpu_quota: Option<i64>,
        restart_policy: Option<&str>,
    ) -> Result<(), LiteError> {
        let mut flags = String::new();

        if let Some(shares) = cpu_shares {
            flags.push_str(&format!(" --cpu-shares {}", shares));
        }
        if let Some(mem) = memory {
            flags.push_str(&format!(" --memory {}", mem));
        }
        if let Some(swap) = memory_swap {
            flags.push_str(&format!(" --memory-swap {}", swap));
        }
        if let Some(period) = cpu_period {
            flags.push_str(&format!(" --cpu-period {}", period));
        }
        if let Some(quota) = cpu_quota {
            flags.push_str(&format!(" --cpu-quota {}", quota));
        }
        if let Some(policy) = restart_policy {
            flags.push_str(&format!(" --restart {}", policy));
        }

        let cmd = format!("docker update{} {}", flags, container_id);
        let output = ssh_manager.execute_via_sftp(ssh_session_id, &cmd).await?;

        if output.trim().contains(&container_id[..12]) || output.trim().is_empty() {
            Ok(())
        } else {
            Err(LiteError::Docker(format!("Failed to update container: {}", output)))
        }
    }

    /// Run container (create + start combined)
    pub async fn run_container(
        &self,
        ssh_manager: &SshSessionManager,
        ssh_session_id: &str,
        name: Option<&str>,
        image: &str,
        command: Option<&str>,
        ports: &[(u16, u16, &str)],
        volumes: &[(&str, &str)],
        env: &[(&str, &str)],
        network: Option<&str>,
        restart: Option<&str>,
        labels: &[(&str, &str)],
        detach: bool,
        auto_remove: bool,
    ) -> Result<String, LiteError> {
        let name_flag = name.map(|n| format!(" --name {}", n)).unwrap_or_default();
        let network_flag = network.map(|n| format!(" --network {}", n)).unwrap_or_default();
        let restart_flag = restart.map(|r| format!(" --restart {}", r)).unwrap_or_default();
        let detach_flag = if detach { " -d" } else { "" };
        let rm_flag = if auto_remove { " --rm" } else { "" };

        let mut ports_flags = String::new();
        for (host, container, proto) in ports {
            ports_flags.push_str(&format!(" -p {}:{}/{}", host, container, proto));
        }

        let mut volumes_flags = String::new();
        for (host, container) in volumes {
            volumes_flags.push_str(&format!(" -v {}:{}", host, container));
        }

        let mut env_flags = String::new();
        for (key, value) in env {
            env_flags.push_str(&format!(" -e {}='{}'", key, value.replace("'", "'\\''")));
        }

        let mut labels_flags = String::new();
        for (key, value) in labels {
            labels_flags.push_str(&format!(" -l {}={}", key, value));
        }

        let cmd_flag = command.map(|c| format!(" {}", c)).unwrap_or_default();

        let cmd = format!(
            "docker run{}{}{}{}{}{}{}{}{} {}{}",
            name_flag, ports_flags, volumes_flags, env_flags,
            network_flag, restart_flag, detach_flag, rm_flag, labels_flags,
            image, cmd_flag
        );

        let output = ssh_manager.execute_via_sftp(ssh_session_id, &cmd).await?;
        let result = output.trim();

        if detach {
            // In detached mode, container ID is returned
            if result.len() == 64 && result.chars().all(|c| c.is_ascii_hexdigit()) {
                Ok(result.to_string())
            } else {
                Err(LiteError::Docker(format!("Failed to run container: {}", output)))
            }
        } else {
            // In foreground mode, output is command output
            Ok(result.to_string())
        }
    }

    /// Get disk usage
    pub async fn get_disk_usage(
        &self,
        ssh_manager: &SshSessionManager,
        ssh_session_id: &str,
    ) -> Result<String, LiteError> {
        let cmd = "docker system df -v".to_string();
        let output = ssh_manager.execute_via_sftp(ssh_session_id, &cmd).await?;
        Ok(output)
    }

    /// Clean up unused data
    pub async fn system_prune(
        &self,
        ssh_manager: &SshSessionManager,
        ssh_session_id: &str,
        all: bool,
        volumes: bool,
    ) -> Result<String, LiteError> {
        let all_flag = if all { " -a" } else { "" };
        let volumes_flag = if volumes { " --volumes" } else { "" };

        let cmd = format!("docker system prune -f{}{}", all_flag, volumes_flag);
        let output = ssh_manager.execute_via_sftp(ssh_session_id, &cmd).await?;
        Ok(output)
    }

    /// Execute command in container (docker exec)
    pub async fn exec_in_container(
        &self,
        ssh_manager: &SshSessionManager,
        ssh_session_id: &str,
        container_id: &str,
        command: &str,
        tty: bool,
        interactive: bool,
        working_dir: Option<&str>,
        env: &[(&str, &str)],
    ) -> Result<String, LiteError> {
        let tty_flag = if tty { " -t" } else { "" };
        let interactive_flag = if interactive { " -i" } else { "" };
        let workdir_flag = working_dir.map(|w| format!(" -w {}", w)).unwrap_or_default();

        let mut env_flags = String::new();
        for (key, value) in env {
            env_flags.push_str(&format!(" -e {}='{}'", key, value.replace("'", "'\\''")));
        }

        let cmd = format!(
            "docker exec{}{}{}{} {} {}",
            interactive_flag, tty_flag, workdir_flag, env_flags, container_id, command
        );

        let output = ssh_manager.execute_via_sftp(ssh_session_id, &cmd).await?;
        Ok(output)
    }

    /// Start interactive exec session
    pub async fn start_exec_session(
        &self,
        ssh_manager: &SshSessionManager,
        ssh_session_id: &str,
        container_id: &str,
        command: &str,
    ) -> Result<String, LiteError> {
        // Create exec instance
        let create_cmd = format!(
            "docker exec -i {} sh -c 'echo {}'",
            container_id,
            base64::encode(command)
        );
        let _ = create_cmd;

        let output = ssh_manager.execute_via_sftp(ssh_session_id, &create_cmd).await?;
        let exec_id = output.trim().to_string();

        self.exec_sessions.write().await.insert(
            exec_id.clone(),
            ExecSession {
                container_id: container_id.to_string(),
                command: command.to_string(),
                tty: true,
                stdin: true,
                stdout: true,
                stderr: true,
            },
        );

        Ok(exec_id)
    }

    /// Get container stats
    pub async fn get_container_stats(
        &self,
        ssh_manager: &SshSessionManager,
        ssh_session_id: &str,
        container_id: &str,
    ) -> Result<ContainerStats, LiteError> {
        let cmd = format!("docker stats {} --no-stream --format '{{{{json .}}}}' 2>/dev/null || docker stats {} --no-stream", container_id, container_id);
        let output = ssh_manager.execute_via_sftp(ssh_session_id, &cmd).await?;

        // Try JSON format first
        if let Ok(stats) = serde_json::from_str::<ContainerStats>(&output) {
            return Ok(stats);
        }

        // Fallback to manual parsing of the table format
        self.parse_stats_table(&output)
    }

    /// Get Compose projects
    pub async fn list_compose_projects(
        &self,
        ssh_manager: &SshSessionManager,
        ssh_session_id: &str,
    ) -> Result<Vec<ComposeProject>, LiteError> {
        let cmd = "docker compose ls --all --format json 2>/dev/null || docker-compose ls --all --format json".to_string();
        let output = ssh_manager.execute_via_sftp(ssh_session_id, &cmd).await?;

        let mut projects = Vec::new();
        for line in output.lines().filter(|l| !l.is_empty()) {
            if let Ok(info) = serde_json::from_str::<serde_json::Value>(line) {
                projects.push(self.parse_compose_project_json(info)?);
            }
        }

        Ok(projects)
    }

    /// Start Compose project
    pub async fn compose_up(
        &self,
        ssh_manager: &SshSessionManager,
        ssh_session_id: &str,
        project_dir: &str,
        services: Option<&[String]>,
        detached: bool,
        build: bool,
    ) -> Result<String, LiteError> {
        let detached_flag = if detached { " -d" } else { "" };
        let build_flag = if build { " --build" } else { "" };
        let services_flag = services.map(|s| format!(" {}", s.join(" "))).unwrap_or_default();

        let cmd = format!(
            "cd {} && docker compose up{}{}{} 2>&1 || cd {} && docker-compose up{}{}{} 2>&1",
            project_dir, detached_flag, build_flag, services_flag,
            project_dir, detached_flag, build_flag, services_flag
        );

        let output = ssh_manager.execute_via_sftp(ssh_session_id, &cmd).await?;
        Ok(output)
    }

    /// Stop Compose project
    pub async fn compose_down(
        &self,
        ssh_manager: &SshSessionManager,
        ssh_session_id: &str,
        project_dir: &str,
        remove_volumes: bool,
        remove_images: bool,
    ) -> Result<String, LiteError> {
        let volumes_flag = if remove_volumes { " -v" } else { "" };
        let images_flag = if remove_images { " --rmi all" } else { "" };

        let cmd = format!(
            "cd {} && docker compose down{}{} 2>&1 || cd {} && docker-compose down{}{} 2>&1",
            project_dir, volumes_flag, images_flag,
            project_dir, volumes_flag, images_flag
        );

        let output = ssh_manager.execute_via_sftp(ssh_session_id, &cmd).await?;
        Ok(output)
    }

    /// Parse docker-compose file and return services
    pub async fn parse_compose_file(
        &self,
        ssh_manager: &SshSessionManager,
        ssh_session_id: &str,
        file_path: &str,
    ) -> Result<serde_json::Value, LiteError> {
        let cmd = format!("cat {} | docker compose -f - config 2>/dev/null || cat {} | docker-compose -f - config", file_path, file_path);
        let output = ssh_manager.execute_via_sftp(ssh_session_id, &cmd).await?;

        serde_yaml::from_str(&output)
            .map_err(|e| LiteError::Docker(format!("Failed to parse compose file: {}", e)))
    }

    /// Validate compose file
    pub async fn validate_compose_file(
        &self,
        ssh_manager: &SshSessionManager,
        ssh_session_id: &str,
        file_path: &str,
    ) -> Result<(), LiteError> {
        let cmd = format!(
            "docker compose -f {} config > /dev/null 2>&1 || docker-compose -f {} config > /dev/null 2>&1",
            file_path, file_path
        );
        let output = ssh_manager.execute_via_sftp(ssh_session_id, &cmd).await?;

        if output.trim().is_empty() {
            Ok(())
        } else {
            Err(LiteError::Docker(format!("Compose file validation failed: {}", output)))
        }
    }

    // Helper methods for parsing

    fn parse_container_json(&self, json: serde_json::Value) -> Result<ContainerInfo, LiteError> {
        Ok(ContainerInfo {
            id: json["Id"].as_str().unwrap_or("").to_string(),
            names: json["Names"].as_array()
                .map(|a| a.iter().map(|v| v.as_str().unwrap_or("").to_string()).collect())
                .unwrap_or_default(),
            image: json["Image"].as_str().unwrap_or("").to_string(),
            image_id: json["ImageID"].as_str().unwrap_or("").to_string(),
            command: json["Command"].as_str().unwrap_or("").to_string(),
            created: json["Created"].as_i64().unwrap_or(0),
            status: self.parse_status(json["State"].as_str().unwrap_or("")),
            state: json["State"].as_str().unwrap_or("").to_string(),
            ports: self.parse_ports_json(&json["Ports"]),
            labels: self.parse_labels_json(&json["Labels"]),
            size_rw: json["SizeRw"].as_i64(),
            size_root_fs: json["SizeRootFs"].as_i64(),
            host_config: HostConfig {
                network_mode: json["HostConfig"]["NetworkMode"].as_str().unwrap_or("").to_string(),
                cpu_shares: json["HostConfig"]["CpuShares"].as_i64(),
                memory: json["HostConfig"]["Memory"].as_i64(),
                memory_swap: json["HostConfig"]["MemorySwap"].as_i64(),
                cpu_percent: json["HostConfig"]["CpuPercent"].as_i64(),
                cpu_quota: json["HostConfig"]["CpuQuota"].as_i64(),
                cpu_period: json["HostConfig"]["CpuPeriod"].as_i64(),
            },
            network_settings: self.parse_network_settings_json(&json["NetworkSettings"]),
            mounts: self.parse_mounts_json(&json["Mounts"]),
        })
    }

    fn parse_status(&self, status: &str) -> ContainerStatus {
        status.parse().unwrap_or(ContainerStatus::Dead)
    }

    fn parse_ports(&self, ports_str: &&str) -> Vec<PortMapping> {
        let mut ports = Vec::new();
        for part in ports_str.split(", ") {
            if let Some(idx) = part.find("->") {
                let public = &part[..idx];
                let private = &part[idx + 2..];

                let (ip, pub_port) = if let Some(colon) = public.rfind(':') {
                    (&public[..colon], &public[colon + 1..])
                } else {
                    ("0.0.0.0", public)
                };

                if let (Ok(pub_port), Ok(priv_port)) = (pub_port.parse::<u16>(),
                    private.split('/').next().unwrap_or("0").parse::<u16>()) {
                    ports.push(PortMapping {
                        ip: ip.to_string(),
                        private_port: priv_port,
                        public_port: pub_port,
                        protocol: private.split('/').nth(1).unwrap_or("tcp").to_string(),
                    });
                }
            }
        }
        ports
    }

    fn parse_ports_json(&self, ports: &serde_json::Value) -> Vec<PortMapping> {
        ports.as_array()
            .map(|a| a.iter().filter_map(|v| {
                Some(PortMapping {
                    ip: v["IP"].as_str()?.to_string(),
                    private_port: v["PrivatePort"].as_u64()? as u16,
                    public_port: v["PublicPort"].as_u64()? as u16,
                    protocol: v["Type"].as_str()?.to_string(),
                })
            }).collect())
            .unwrap_or_default()
    }

    fn parse_labels_json(&self, labels: &serde_json::Value) -> HashMap<String, String> {
        labels.as_object()
            .map(|o| o.iter().map(|(k, v)| (k.clone(), v.as_str().unwrap_or("").to_string())).collect())
            .unwrap_or_default()
    }

    fn parse_network_settings_json(&self, settings: &serde_json::Value) -> NetworkSettings {
        let networks = settings["Networks"].as_object()
            .map(|o| o.iter().map(|(k, v)| {
                (k.clone(), ContainerNetworkInfo {
                    network_id: v["NetworkID"].as_str().unwrap_or("").to_string(),
                    endpoint_id: v["EndpointID"].as_str().unwrap_or("").to_string(),
                    gateway: v["Gateway"].as_str().unwrap_or("").to_string(),
                    ip_address: v["IPAddress"].as_str().unwrap_or("").to_string(),
                    ip_prefix_len: v["IPPrefixLen"].as_i64().unwrap_or(0) as i32,
                    mac_address: v["MacAddress"].as_str().unwrap_or("").to_string(),
                })
            }).collect())
            .unwrap_or_default();

        NetworkSettings {
            networks,
            ip_address: settings["IPAddress"].as_str().unwrap_or("").to_string(),
            gateway: settings["Gateway"].as_str().unwrap_or("").to_string(),
            mac_address: settings["MacAddress"].as_str().unwrap_or("").to_string(),
        }
    }

    fn parse_mounts_json(&self, mounts: &serde_json::Value) -> Vec<MountPoint> {
        mounts.as_array()
            .map(|a| a.iter().filter_map(|v| {
                Some(MountPoint {
                    mount_type: v["Type"].as_str()?.to_string(),
                    name: v["Name"].as_str().map(|s| s.to_string()),
                    source: v["Source"].as_str()?.to_string(),
                    destination: v["Destination"].as_str()?.to_string(),
                    driver: v["Driver"].as_str().map(|s| s.to_string()),
                    mode: v["Mode"].as_str().unwrap_or("").to_string(),
                    rw: v["RW"].as_bool().unwrap_or(false),
                    propagation: v["Propagation"].as_str().unwrap_or("").to_string(),
                })
            }).collect())
            .unwrap_or_default()
    }

    fn parse_image_json(&self, json: serde_json::Value) -> Result<ImageInfo, LiteError> {
        Ok(ImageInfo {
            id: json["Id"].as_str().unwrap_or("").to_string(),
            repo_tags: json["RepoTags"].as_array()
                .map(|a| a.iter().map(|v| v.as_str().unwrap_or("").to_string()).collect())
                .unwrap_or_default(),
            repo_digests: json["RepoDigests"].as_array()
                .map(|a| a.iter().map(|v| v.as_str().unwrap_or("").to_string()).collect())
                .unwrap_or_default(),
            parent: json["Parent"].as_str().unwrap_or("").to_string(),
            comment: json["Comment"].as_str().unwrap_or("").to_string(),
            created: json["Created"].as_str().unwrap_or("").to_string(),
            container: json["Container"].as_str().unwrap_or("").to_string(),
            size: json["Size"].as_i64().unwrap_or(0),
            virtual_size: json["VirtualSize"].as_i64().unwrap_or(0),
            shared_size: json["SharedSize"].as_i64().unwrap_or(0),
            labels: self.parse_labels_json(&json["Config"]["Labels"]),
        })
    }

    fn parse_network_json(&self, json: serde_json::Value) -> Result<NetworkInfo, LiteError> {
        Ok(NetworkInfo {
            id: json["Id"].as_str().unwrap_or("").to_string(),
            name: json["Name"].as_str().unwrap_or("").to_string(),
            driver: json["Driver"].as_str().unwrap_or("").to_string(),
            scope: json["Scope"].as_str().unwrap_or("").to_string(),
            internal: json["Internal"].as_bool().unwrap_or(false),
            enable_ipv6: json["EnableIPv6"].as_bool().unwrap_or(false),
            ipam: IpamConfig {
                driver: json["IPAM"]["Driver"].as_str().unwrap_or("default").to_string(),
                config: json["IPAM"]["Config"].as_array()
                    .map(|a| a.iter().filter_map(|v| {
                        Some(IpamSubnetConfig {
                            subnet: v["Subnet"].as_str()?.to_string(),
                            gateway: v["Gateway"].as_str()?.to_string(),
                            ip_range: v["IPRange"].as_str().map(|s| s.to_string()),
                            auxiliary_addresses: v["AuxiliaryAddresses"].as_object()
                                .map(|o| o.iter().map(|(k, v)| (k.clone(), v.as_str().unwrap_or("").to_string())).collect())
                                .unwrap_or_default(),
                        })
                    }).collect())
                    .unwrap_or_default(),
                options: self.parse_labels_json(&json["IPAM"]["Options"]),
            },
            labels: self.parse_labels_json(&json["Labels"]),
            containers: self.parse_network_containers_json(&json["Containers"]),
            options: self.parse_labels_json(&json["Options"]),
        })
    }

    fn parse_network_inspect_json(&self, json: serde_json::Value) -> Result<NetworkInfo, LiteError> {
        self.parse_network_json(json)
    }

    fn parse_network_containers_json(&self, containers: &serde_json::Value) -> HashMap<String, NetworkContainer> {
        containers.as_object()
            .map(|o| o.iter().filter_map(|(k, v)| {
                Some((k.clone(), NetworkContainer {
                    name: v["Name"].as_str()?.to_string(),
                    endpoint_id: v["EndpointID"].as_str()?.to_string(),
                    mac_address: v["MacAddress"].as_str()?.to_string(),
                    ipv4_address: v["IPv4Address"].as_str()?.to_string(),
                    ipv6_address: v["IPv6Address"].as_str()?.to_string(),
                }))
            }).collect())
            .unwrap_or_default()
    }

    fn parse_volume_json(&self, json: serde_json::Value) -> Result<VolumeInfo, LiteError> {
        Ok(VolumeInfo {
            name: json["Name"].as_str().unwrap_or("").to_string(),
            driver: json["Driver"].as_str().unwrap_or("local").to_string(),
            mountpoint: json["Mountpoint"].as_str().unwrap_or("").to_string(),
            created_at: json["CreatedAt"].as_str().unwrap_or("").to_string(),
            status: json["Status"].as_object()
                .map(|o| o.iter().map(|(k, v)| (k.clone(), v.clone())).collect()),
            labels: self.parse_labels_json(&json["Labels"]),
            scope: json["Scope"].as_str().unwrap_or("local").to_string(),
            options: json["Options"].as_object()
                .map(|o| o.iter().map(|(k, v)| (k.clone(), v.as_str().unwrap_or("").to_string())).collect()),
            usage_data: json["UsageData"].as_object().map(|o| VolumeUsageData {
                size: o["Size"].as_i64().unwrap_or(0),
                ref_count: o["RefCount"].as_i64().unwrap_or(0) as i32,
            }),
        })
    }

    fn parse_size(&self, size_str: &str) -> i64 {
        let parts: Vec<&str> = size_str.split_whitespace().collect();
        if parts.len() >= 2 {
            let num = parts[0].parse::<f64>().unwrap_or(0.0);
            let unit = parts[1];
            let multiplier = match unit {
                "B" => 1.0,
                "KB" => 1024.0,
                "MB" => 1024.0 * 1024.0,
                "GB" => 1024.0 * 1024.0 * 1024.0,
                "TB" => 1024.0 * 1024.0 * 1024.0 * 1024.0,
                _ => 1.0,
            };
            (num * multiplier) as i64
        } else {
            0
        }
    }

    fn parse_stats_table(&self, output: &str) -> Result<ContainerStats, LiteError> {
        // Parse the default docker stats table format
        // CONTAINER ID   NAME   CPU %   MEM USAGE / LIMIT   MEM %   NET I/O   BLOCK I/O   PIDS
        let lines: Vec<&str> = output.lines().collect();
        if lines.len() < 2 {
            return Err(LiteError::Docker("Invalid stats output".to_string()));
        }

        let data_line = lines[1];
        let parts: Vec<&str> = data_line.split_whitespace().collect();

        Ok(ContainerStats {
            read: String::new(),
            preread: String::new(),
            pids_stats: PidsStats {
                current: parts.last().and_then(|p| p.parse().ok()),
                limit: None,
            },
            memory_stats: MemoryStats {
                usage: 0,
                stats: HashMap::new(),
                limit: 0,
            },
            cpu_stats: CpuStats {
                total_usage: 0,
                percpu_usage: None,
                usage_in_kernelmode: 0,
                usage_in_usermode: 0,
                system_cpu_usage: None,
                online_cpus: 0,
                throttling_data: ThrottlingData {
                    periods: 0,
                    throttled_periods: 0,
                    throttled_time: 0,
                },
            },
            io_stats: IoStats {
                io_service_bytes_recursive: Vec::new(),
                io_serviced_recursive: Vec::new(),
            },
            network_stats: NetworkStats {
                rx_bytes: 0,
                rx_packets: 0,
                rx_errors: 0,
                rx_dropped: 0,
                tx_bytes: 0,
                tx_packets: 0,
                tx_errors: 0,
                tx_dropped: 0,
            },
        })
    }

    fn parse_compose_project_json(&self, json: serde_json::Value) -> Result<ComposeProject, LiteError> {
        Ok(ComposeProject {
            name: json["Name"].as_str().unwrap_or("").to_string(),
            status: json["Status"].as_str().unwrap_or("").to_string(),
            config_files: json["ConfigFiles"].as_array()
                .map(|a| a.iter().map(|v| v.as_str().unwrap_or("").to_string()).collect())
                .unwrap_or_default(),
            services: json["Services"].as_array()
                .map(|a| a.iter().filter_map(|v| {
                    Some(ComposeService {
                        name: v["Name"].as_str()?.to_string(),
                        image: v["Image"].as_str().unwrap_or("").to_string(),
                        state: v["State"].as_str()?.to_string(),
                        replicas: v["Replicas"].as_i64().unwrap_or(0) as i32,
                        ports: self.parse_ports_json(&v["Ports"]),
                        health: v["Health"].as_str().map(|s| s.to_string()),
                    })
                }).collect())
                .unwrap_or_default(),
        })
    }
}

impl Default for DockerManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Docker system information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DockerSystemInfo {
    pub server_version: String,
    pub api_version: String,
    pub driver: String,
    pub driver_status: Vec<(String, String)>,
    pub system_status: Option<Vec<(String, String)>>,
    pub plugins: PluginsInfo,
    pub memory_limit: bool,
    pub swap_limit: bool,
    pub kernel_memory: bool,
    pub cpu_cfs_period: bool,
    pub cpu_cfs_quota: bool,
    pub cpu_shares: bool,
    pub cpu_set: bool,
    pub ipv4_forwarding: bool,
    pub bridge_nf_iptables: bool,
    pub bridge_nf_ip6tables: bool,
    pub debug: bool,
    pub nfd: i64,
    pub oom_kill_disable: bool,
    pub n_goroutines: i64,
    pub system_time: String,
    pub logging_driver: String,
    pub cgroup_driver: String,
    pub n_events_listener: i64,
    pub kernel_version: String,
    pub operating_system: String,
    pub os_type: String,
    pub architecture: String,
    pub n_cpus: i64,
    pub mem_total: i64,
    pub docker_root_dir: String,
    pub http_proxy: String,
    pub https_proxy: String,
    pub no_proxy: String,
    pub name: String,
    pub labels: Vec<String>,
    pub experimental_build: bool,
    pub server_experimental: bool,
    pub cluster_store: String,
    pub cluster_advertise: String,
    pub runtimes: HashMap<String, RuntimeInfo>,
    pub default_runtime: String,
    pub swarm: SwarmInfo,
}

/// Plugins information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginsInfo {
    pub volume: Vec<String>,
    pub network: Vec<String>,
    pub authorization: Vec<String>,
    pub log: Vec<String>,
}

/// Runtime information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeInfo {
    pub path: String,
    pub runtime_args: Vec<String>,
}

/// Swarm information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwarmInfo {
    pub node_id: String,
    pub node_addr: String,
    pub local_node_state: String,
    pub control_available: bool,
    pub error: String,
    pub remote_managers: Option<Vec<RemoteManager>>,
    pub nodes: Option<i64>,
    pub managers: Option<i64>,
    pub cluster: Option<ClusterInfo>,
}

/// Remote manager
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteManager {
    pub node_id: String,
    pub addr: String,
}

/// Cluster information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterInfo {
    pub id: String,
    pub version: i64,
    pub created_at: String,
    pub updated_at: String,
    pub spec: ClusterSpec,
}

/// Cluster specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterSpec {
    pub name: String,
    pub orchestration: OrchestrationConfig,
    pub raft: RaftConfig,
    pub dispatcher: DispatcherConfig,
    pub ca_config: CAConfig,
    pub encryption_config: EncryptionConfig,
}

/// Orchestration configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrchestrationConfig {
    pub task_history_retention_limit: i64,
}

/// Raft configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RaftConfig {
    pub snapshot_interval: i64,
    pub keep_old_snapshots: i64,
    pub log_entries_for_slow_followers: i64,
    pub election_tick: i64,
    pub heartbeat_tick: i64,
}

/// Dispatcher configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DispatcherConfig {
    pub heartbeat_period: i64,
}

/// CA configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CAConfig {
    pub node_cert_expiry: i64,
    pub external_cas: Vec<ExternalCA>,
}

/// External CA
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExternalCA {
    pub protocol: String,
    pub url: String,
    pub options: HashMap<String, String>,
}

/// Encryption configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptionConfig {
    pub auto_lock_managers: bool,
}

/// Docker events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DockerEvent {
    pub action: String,
    pub entity_type: String,
    pub actor: Actor,
    pub scope: String,
    pub timestamp: i64,
    pub time_nano: i64,
}

/// Actor
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Actor {
    pub id: String,
    pub attributes: HashMap<String, String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_container_status_variants() {
        assert_eq!(ContainerStatus::Running.is_running(), true);
        assert_eq!(ContainerStatus::Exited.is_running(), false);
        assert_eq!(ContainerStatus::Paused.is_running(), false);
    }

    #[test]
    fn test_container_status_actions() {
        assert!(ContainerStatus::Exited.can_start());
        assert!(ContainerStatus::Running.can_stop());
        assert!(ContainerStatus::Running.can_restart());
        assert!(ContainerStatus::Running.can_pause());
        assert!(ContainerStatus::Paused.can_unpause());
    }

    #[test]
    fn test_container_status_display() {
        assert_eq!(format!("{}", ContainerStatus::Running), "running");
        assert_eq!(format!("{}", ContainerStatus::Exited), "exited");
    }

    #[test]
    fn test_parse_size() {
        let manager = DockerManager::new();
        assert_eq!(manager.parse_size("100 MB"), 100 * 1024 * 1024);
        assert_eq!(manager.parse_size("2.5 GB"), (2.5 * 1024.0 * 1024.0 * 1024.0) as i64);
        assert_eq!(manager.parse_size("500 KB"), 500 * 1024);
    }

    #[test]
    fn test_parse_status() {
        let manager = DockerManager::new();
        assert_eq!(manager.parse_status("running"), ContainerStatus::Running);
        assert_eq!(manager.parse_status("exited"), ContainerStatus::Exited);
        assert_eq!(manager.parse_status("paused"), ContainerStatus::Paused);
        assert_eq!(manager.parse_status("unknown"), ContainerStatus::Dead);
    }
}
