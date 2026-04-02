//! Docker types - Data structures for Docker management

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Container status
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
    type Err = crate::error::LiteError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "created" => Ok(Self::Created),
            "running" => Ok(Self::Running),
            "paused" => Ok(Self::Paused),
            "restarting" => Ok(Self::Restarting),
            "removing" => Ok(Self::Removing),
            "exited" => Ok(Self::Exited),
            "dead" => Ok(Self::Dead),
            _ => Err(crate::error::LiteError::Docker(format!(
                "Unknown container status: {}",
                s
            ))),
        }
    }
}

/// Container information
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

/// Container network info
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

/// Image information
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

/// Network info
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

/// Network container
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkContainer {
    pub name: String,
    pub endpoint_id: String,
    pub mac_address: String,
    pub ipv4_address: String,
    pub ipv6_address: String,
}

/// Volume info
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

/// Container stats
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

/// Compose project
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

/// Docker connection
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

/// Docker system info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DockerSystemInfo {
    pub id: String,
    pub containers: i64,
    pub containers_running: i64,
    pub containers_paused: i64,
    pub containers_stopped: i64,
    pub images: i64,
    pub driver: String,
    pub driver_status: Vec<Vec<String>>,
    pub system_status: Option<Vec<Vec<String>>>,
    pub plugins: PluginsInfo,
    pub memory_limit: bool,
    pub swap_limit: bool,
    pub kernel_memory: bool,
    pub cpu_cfs_period: bool,
    pub cpu_cfs_quota: bool,
    pub cpu_shares: bool,
    pub cpu_set: bool,
    pub pids_limit: bool,
    pub oom_kill_disable: bool,
    pub ipv4_forwarding: bool,
    pub bridge_nf_iptables: bool,
    pub bridge_nf_ip6tables: bool,
    pub debug: bool,
    pub nfd: i64,
    pub n_goroutines: i64,
    pub system_time: String,
    pub logging_driver: String,
    pub cgroup_driver: String,
    pub cgroup_version: String,
    pub n_events_listener: i64,
    pub kernel_version: String,
    pub operating_system: String,
    pub os_type: String,
    pub architecture: String,
    pub index_server_address: String,
    pub registry_config: Option<serde_json::Value>,
    pub ncpu: i64,
    pub mem_total: i64,
    pub docker_root_dir: String,
    pub http_proxy: String,
    pub https_proxy: String,
    pub no_proxy: String,
    pub name: String,
    pub labels: Vec<String>,
    pub experimental_build: bool,
    pub server_version: String,
    pub cluster_store: String,
    pub cluster_advertise: String,
    pub runtimes: HashMap<String, RuntimeInfo>,
    pub default_runtime: String,
    pub swarm: SwarmInfo,
}

/// Plugins info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginsInfo {
    pub volume: Vec<String>,
    pub network: Vec<String>,
    pub authorization: Option<Vec<String>>,
    pub log: Vec<String>,
}

/// Runtime info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeInfo {
    pub path: String,
    pub runtime_args: Option<Vec<String>>,
}

/// Swarm info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwarmInfo {
    pub node_id: String,
    pub node_addr: String,
    pub local_node_state: String,
    pub control_available: bool,
    pub error: String,
    pub remote_managers: Option<Vec<RemoteManager>>,
    pub nodes: i64,
    pub managers: i64,
    pub cluster: Option<ClusterInfo>,
}

/// Remote manager
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteManager {
    pub node_id: String,
    pub addr: String,
}

/// Cluster info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterInfo {
    pub id: String,
    pub version: ClusterVersion,
    pub created_at: String,
    pub updated_at: String,
    pub spec: ClusterSpec,
}

/// Cluster version
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterVersion {
    pub index: u64,
}

/// Cluster spec
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterSpec {
    pub name: String,
    pub labels: HashMap<String, String>,
    pub orchestration: OrchestrationConfig,
    pub raft: RaftConfig,
    pub dispatcher: DispatcherConfig,
    pub ca_config: CAConfig,
    pub encryption_config: EncryptionConfig,
}

/// Orchestration config
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrchestrationConfig {
    pub task_history_retention_limit: i64,
}

/// Raft config
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RaftConfig {
    pub snapshot_interval: i64,
    pub keep_old_snapshots: i64,
    pub log_entries_for_slow_followers: i64,
    pub election_tick: i64,
    pub heartbeat_tick: i64,
}

/// Dispatcher config
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DispatcherConfig {
    pub heartbeat_period: i64,
}

/// CA config
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CAConfig {
    pub node_cert_expiry: i64,
    pub external_cas: Option<Vec<ExternalCA>>,
}

/// External CA
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExternalCA {
    pub protocol: String,
    pub url: String,
    pub options: HashMap<String, String>,
    pub ca_cert: String,
}

/// Encryption config
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptionConfig {
    pub auto_lock_managers: bool,
}

/// Docker event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DockerEvent {
    pub timestamp: i64,
    pub event_type: String,
    pub action: String,
    pub actor: Actor,
    pub scope: String,
    pub time_nano: i64,
}

/// Actor
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Actor {
    pub id: String,
    pub attributes: HashMap<String, String>,
}
