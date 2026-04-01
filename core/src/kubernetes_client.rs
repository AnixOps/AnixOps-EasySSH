use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use chrono::{DateTime, Utc};
use futures::{StreamExt, AsyncBufReadExt};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::sync::{Mutex, RwLock, mpsc};
use bytes::Bytes;
use serde_json::json;

#[cfg(feature = "kubernetes")]
use k8s_openapi::api::core::v1::{
    ConfigMap, Event, Namespace, Node, Pod, Secret,
    Service, Container, Volume,
    NodeAddress, NodeCondition,
};

#[cfg(feature = "kubernetes")]
use k8s_openapi::api::apps::v1::{
    Deployment,
};


#[cfg(feature = "kubernetes")]
use k8s_openapi::apimachinery::pkg::api::resource::Quantity;

#[cfg(feature = "kubernetes")]
use kube::{
    Client, Config, api::{Api, ListParams, DeleteParams, Patch, PatchParams, WatchParams, WatchEvent},
    config::{KubeConfigOptions, Kubeconfig},
};

#[derive(Error, Debug, Clone, Serialize, Deserialize)]
pub enum K8sError {
    #[error("IO error: {0}")]
    Io(String),
    #[error("YAML parse error: {0}")]
    Yaml(String),
    #[error("Cluster not found: {0}")]
    ClusterNotFound(String),
    #[error("Namespace not found: {0}")]
    NamespaceNotFound(String),
    #[error("Pod not found: {0}")]
    PodNotFound(String),
    #[error("Deployment not found: {0}")]
    DeploymentNotFound(String),
    #[error("Service not found: {0}")]
    ServiceNotFound(String),
    #[error("ConfigMap not found: {0}")]
    ConfigMapNotFound(String),
    #[error("Secret not found: {0}")]
    SecretNotFound(String),
    #[error("Invalid kubeconfig: {0}")]
    InvalidKubeconfig(String),
    #[error("Connection failed: {0}")]
    ConnectionFailed(String),
    #[error("Exec error: {0}")]
    ExecError(String),
    #[error("Port forward error: {0}")]
    PortForwardError(String),
    #[error("Helm error: {0}")]
    HelmError(String),
    #[error("Operation not supported: {0}")]
    NotSupported(String),
    #[error("Permission denied: {0}")]
    PermissionDenied(String),
    #[error("Kube API error: {0}")]
    ApiError(String),
}

pub type Result<T> = std::result::Result<T, K8sError>;

/// Kubernetes cluster configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct K8sCluster {
    pub id: String,
    pub name: String,
    pub kubeconfig_path: Option<String>,
    pub kubeconfig_content: Option<String>,
    pub context: String,
    pub server_url: String,
    pub current_namespace: String,
    pub is_connected: bool,
    pub last_connected: Option<DateTime<Utc>>,
    pub labels: HashMap<String, String>,
    pub tags: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Namespace information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct K8sNamespace {
    pub name: String,
    pub status: String,
    pub labels: HashMap<String, String>,
    pub annotations: HashMap<String, String>,
    pub created_at: DateTime<Utc>,
}

/// Pod information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct K8sPod {
    pub name: String,
    pub namespace: String,
    pub status: PodStatus,
    pub phase: String,
    pub restarts: i32,
    pub node: String,
    pub pod_ip: Option<String>,
    pub labels: HashMap<String, String>,
    pub annotations: HashMap<String, String>,
    pub containers: Vec<K8sContainer>,
    pub init_containers: Vec<K8sContainer>,
    pub conditions: Vec<K8sPodCondition>,
    pub volumes: Vec<K8sVolume>,
    pub created_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub resource_usage: Option<K8sResourceUsage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum PodStatus {
    Running,
    Pending,
    Succeeded,
    Failed,
    Unknown,
    Terminating,
    ContainerCreating,
    ImagePullBackOff,
    CrashLoopBackOff,
    Error,
    Completed,
    Evicted,
    OutOfMemory,
}

impl From<&str> for PodStatus {
    fn from(s: &str) -> Self {
        match s {
            "Running" => PodStatus::Running,
            "Pending" => PodStatus::Pending,
            "Succeeded" => PodStatus::Succeeded,
            "Failed" => PodStatus::Failed,
            "Terminating" => PodStatus::Terminating,
            "ContainerCreating" => PodStatus::ContainerCreating,
            "ImagePullBackOff" => PodStatus::ImagePullBackOff,
            "CrashLoopBackOff" => PodStatus::CrashLoopBackOff,
            "Evicted" => PodStatus::Evicted,
            "OOMKilled" => PodStatus::OutOfMemory,
            _ => PodStatus::Unknown,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct K8sContainer {
    pub name: String,
    pub image: String,
    pub ready: bool,
    pub restart_count: i32,
    pub state: ContainerState,
    pub ports: Vec<K8sContainerPort>,
    pub resources: Option<K8sResourceRequirements>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase", tag = "state")]
pub enum ContainerState {
    Running { started_at: Option<DateTime<Utc>> },
    Waiting { reason: String, message: String },
    Terminated { exit_code: i32, reason: String, finished_at: Option<DateTime<Utc>> },
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct K8sContainerPort {
    pub name: String,
    pub container_port: i32,
    pub protocol: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct K8sResourceRequirements {
    pub limits: HashMap<String, String>,
    pub requests: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct K8sPodCondition {
    pub condition_type: String,
    pub status: String,
    pub last_probe_time: Option<DateTime<Utc>>,
    pub last_transition_time: Option<DateTime<Utc>>,
    pub reason: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct K8sVolume {
    pub name: String,
    pub volume_type: String,
    pub source: String,
}

/// Resource usage metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct K8sResourceUsage {
    pub cpu_usage: String,
    pub memory_usage: String,
    pub cpu_percent: f64,
    pub memory_percent: f64,
}

/// Node information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct K8sNode {
    pub name: String,
    pub status: String,
    pub roles: Vec<String>,
    pub version: String,
    pub os_image: String,
    pub kernel_version: String,
    pub container_runtime: String,
    pub labels: HashMap<String, String>,
    pub annotations: HashMap<String, String>,
    pub addresses: Vec<K8sNodeAddress>,
    pub capacity: HashMap<String, String>,
    pub allocatable: HashMap<String, String>,
    pub conditions: Vec<K8sNodeCondition>,
    pub resource_usage: Option<K8sNodeResourceUsage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct K8sNodeAddress {
    pub address_type: String,
    pub address: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct K8sNodeCondition {
    pub condition_type: String,
    pub status: String,
    pub last_heartbeat_time: Option<DateTime<Utc>>,
    pub last_transition_time: Option<DateTime<Utc>>,
    pub reason: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct K8sNodeResourceUsage {
    pub cpu_cores: i32,
    pub cpu_usage_percent: f64,
    pub memory_total: String,
    pub memory_used: String,
    pub memory_usage_percent: f64,
    pub pod_count: i32,
    pub pod_capacity: i32,
}

/// Deployment information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct K8sDeployment {
    pub name: String,
    pub namespace: String,
    pub replicas: i32,
    pub available_replicas: i32,
    pub ready_replicas: i32,
    pub updated_replicas: i32,
    pub labels: HashMap<String, String>,
    pub annotations: HashMap<String, String>,
    pub selector: HashMap<String, String>,
    pub strategy: String,
    pub conditions: Vec<K8sDeploymentCondition>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct K8sDeploymentCondition {
    pub condition_type: String,
    pub status: String,
    pub last_update_time: Option<DateTime<Utc>>,
    pub last_transition_time: Option<DateTime<Utc>>,
    pub reason: String,
    pub message: String,
}

/// Service information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct K8sService {
    pub name: String,
    pub namespace: String,
    pub service_type: String,
    pub cluster_ip: String,
    pub external_ips: Vec<String>,
    pub ports: Vec<K8sServicePort>,
    pub selector: HashMap<String, String>,
    pub labels: HashMap<String, String>,
    pub annotations: HashMap<String, String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct K8sServicePort {
    pub name: String,
    pub port: i32,
    pub target_port: String,
    pub node_port: Option<i32>,
    pub protocol: String,
}

/// ConfigMap information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct K8sConfigMap {
    pub name: String,
    pub namespace: String,
    pub data: HashMap<String, String>,
    pub binary_data: HashMap<String, String>,
    pub labels: HashMap<String, String>,
    pub annotations: HashMap<String, String>,
    pub created_at: DateTime<Utc>,
}

/// Secret information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct K8sSecret {
    pub name: String,
    pub namespace: String,
    pub secret_type: String,
    pub data_keys: Vec<String>,
    pub labels: HashMap<String, String>,
    pub annotations: HashMap<String, String>,
    pub created_at: DateTime<Utc>,
}

/// Event information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct K8sEvent {
    pub name: String,
    pub namespace: Option<String>,
    pub reason: String,
    pub message: String,
    pub event_type: String,
    pub involved_object: K8sObjectReference,
    pub count: i32,
    pub first_timestamp: Option<DateTime<Utc>>,
    pub last_timestamp: Option<DateTime<Utc>>,
    pub source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct K8sObjectReference {
    pub kind: String,
    pub name: String,
    pub namespace: Option<String>,
    pub uid: Option<String>,
}

/// Port forward configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct K8sPortForward {
    pub id: String,
    pub cluster_id: String,
    pub namespace: String,
    pub pod_name: String,
    pub service_name: Option<String>,
    pub local_port: u16,
    pub remote_port: u16,
    pub protocol: String,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
}

/// Helm chart information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HelmChart {
    pub name: String,
    pub version: String,
    pub app_version: String,
    pub description: String,
    pub keywords: Vec<String>,
    pub maintainers: Vec<HelmMaintainer>,
    pub icon: String,
    pub urls: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HelmMaintainer {
    pub name: String,
    pub email: String,
}

/// Helm release information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HelmRelease {
    pub name: String,
    pub namespace: String,
    pub chart: String,
    pub chart_version: String,
    pub app_version: String,
    pub revision: i32,
    pub status: String,
    pub updated: DateTime<Utc>,
    pub values: serde_json::Value,
}

/// Helm repository
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HelmRepo {
    pub name: String,
    pub url: String,
}

/// Kubernetes resource for YAML editor
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct K8sResource {
    pub api_version: String,
    pub kind: String,
    pub metadata: K8sResourceMetadata,
    pub spec: Option<serde_json::Value>,
    pub status: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct K8sResourceMetadata {
    pub name: String,
    pub namespace: Option<String>,
    pub labels: Option<HashMap<String, String>>,
    pub annotations: Option<HashMap<String, String>>,
    pub uid: Option<String>,
    pub resource_version: Option<String>,
    pub creation_timestamp: Option<DateTime<Utc>>,
}

/// Log stream options
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogOptions {
    pub follow: bool,
    pub tail_lines: Option<i64>,
    pub since_seconds: Option<i64>,
    pub timestamps: bool,
    pub previous: bool,
    pub container: Option<String>,
}

impl Default for LogOptions {
    fn default() -> Self {
        Self {
            follow: false,
            tail_lines: Some(100),
            since_seconds: None,
            timestamps: false,
            previous: false,
            container: None,
        }
    }
}

/// Exec options for kubectl exec
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecOptions {
    pub container: Option<String>,
    pub stdin: bool,
    pub tty: bool,
    pub command: Vec<String>,
}

/// Runtime client for a specific cluster
#[cfg(feature = "kubernetes")]
struct ClusterClient {
    client: Client,
    cluster: K8sCluster,
}

/// Kubernetes manager for cluster operations
pub struct K8sManager {
    clusters: RwLock<HashMap<String, K8sCluster>>,
    #[cfg(feature = "kubernetes")]
    clients: RwLock<HashMap<String, ClusterClient>>,
    port_forwards: RwLock<HashMap<String, K8sPortForward>>,
    kubeconfig_cache: Mutex<HashMap<String, serde_json::Value>>,
}

impl K8sManager {
    pub fn new() -> Self {
        Self {
            clusters: RwLock::new(HashMap::new()),
            #[cfg(feature = "kubernetes")]
            clients: RwLock::new(HashMap::new()),
            port_forwards: RwLock::new(HashMap::new()),
            kubeconfig_cache: Mutex::new(HashMap::new()),
        }
    }

    /// Import kubeconfig from file path
    pub async fn import_kubeconfig_from_path(&self, path: &str) -> Result<Vec<K8sCluster>> {
        let content = tokio::fs::read_to_string(path)
            .await
            .map_err(|e| K8sError::Io(e.to_string()))?;
        self.import_kubeconfig(&content, Some(path)).await
    }

    /// Import kubeconfig from content string
    pub async fn import_kubeconfig(
        &self,
        content: &str,
        path: Option<&str>,
    ) -> Result<Vec<K8sCluster>> {
        let kubeconfig: serde_json::Value = serde_yaml::from_str(content)
            .map_err(|e| K8sError::InvalidKubeconfig(e.to_string()))?;

        let mut clusters = Vec::new();

        // Parse contexts
        if let Some(contexts) = kubeconfig.get("contexts").and_then(|c| c.as_array()) {
            for ctx in contexts {
                if let Some(context_name) = ctx.get("name").and_then(|n| n.as_str()) {
                    match self.parse_cluster_from_context(&kubeconfig, context_name, path, content).await {
                        Ok(cluster) => clusters.push(cluster),
                        Err(e) => log::warn!("Failed to parse context {}: {}", context_name, e),
                    }
                }
            }
        }

        // Store in cache
        let mut cache = self.kubeconfig_cache.lock().await;
        for cluster in &clusters {
            cache.insert(cluster.id.clone(), kubeconfig.clone());
        }

        // Store clusters
        let mut cluster_map = self.clusters.write().await;
        for cluster in &clusters {
            cluster_map.insert(cluster.id.clone(), cluster.clone());
        }

        Ok(clusters)
    }

    async fn parse_cluster_from_context(
        &self,
        kubeconfig: &serde_json::Value,
        context_name: &str,
        path: Option<&str>,
        content: &str,
    ) -> Result<K8sCluster> {
        let context = kubeconfig
            .get("contexts")
            .and_then(|c| c.as_array())
            .and_then(|arr| {
                arr.iter()
                    .find(|c| c.get("name").and_then(|n| n.as_str()) == Some(context_name))
            })
            .and_then(|c| c.get("context"))
            .ok_or_else(|| K8sError::InvalidKubeconfig(format!("Context not found: {}", context_name)))?;

        let cluster_name = context
            .get("cluster")
            .and_then(|c| c.as_str())
            .ok_or_else(|| K8sError::InvalidKubeconfig("Missing cluster reference".to_string()))?;

        let namespace = context
            .get("namespace")
            .and_then(|n| n.as_str())
            .unwrap_or("default")
            .to_string();

        let cluster_info = kubeconfig
            .get("clusters")
            .and_then(|c| c.as_array())
            .and_then(|arr| {
                arr.iter()
                    .find(|c| c.get("name").and_then(|n| n.as_str()) == Some(cluster_name))
            })
            .and_then(|c| c.get("cluster"))
            .ok_or_else(|| K8sError::InvalidKubeconfig(format!("Cluster not found: {}", cluster_name)))?;

        let server_url = cluster_info
            .get("server")
            .and_then(|s| s.as_str())
            .ok_or_else(|| K8sError::InvalidKubeconfig("Missing server URL".to_string()))?;

        let id = format!("{}-{}", cluster_name, uuid::Uuid::new_v4());

        Ok(K8sCluster {
            id,
            name: cluster_name.to_string(),
            kubeconfig_path: path.map(|p| p.to_string()),
            kubeconfig_content: Some(content.to_string()),
            context: context_name.to_string(),
            server_url: server_url.to_string(),
            current_namespace: namespace,
            is_connected: false,
            last_connected: None,
            labels: HashMap::new(),
            tags: Vec::new(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        })
    }

    /// Connect to cluster and create client
    #[cfg(feature = "kubernetes")]
    pub async fn connect_cluster(&self, cluster_id: &str) -> Result<()> {
        let cluster = self.get_cluster(cluster_id).await?;

        let kubeconfig = if let Some(path) = &cluster.kubeconfig_path {
            Kubeconfig::read_from(path).map_err(|e| K8sError::InvalidKubeconfig(e.to_string()))?
        } else if let Some(content) = &cluster.kubeconfig_content {
            serde_yaml::from_str(content).map_err(|e| K8sError::InvalidKubeconfig(e.to_string()))?
        } else {
            return Err(K8sError::InvalidKubeconfig("No kubeconfig available".to_string()));
        };

        let options = KubeConfigOptions {
            context: Some(cluster.context.clone()),
            cluster: None,
            user: None,
        };

        let config = Config::from_custom_kubeconfig(kubeconfig, &options)
            .await
            .map_err(|e| K8sError::ConnectionFailed(e.to_string()))?;

        let client = Client::try_from(config)
            .map_err(|e| K8sError::ConnectionFailed(e.to_string()))?;

        // Test connection by listing namespaces
        let ns_api: Api<Namespace> = Api::all(client.clone());
        ns_api.list(&ListParams::default().limit(1))
            .await
            .map_err(|e| K8sError::ConnectionFailed(e.to_string()))?;

        // Store client
        let mut clients = self.clients.write().await;
        clients.insert(cluster_id.to_string(), ClusterClient { client, cluster: cluster.clone() });

        // Update cluster status
        let mut clusters = self.clusters.write().await;
        if let Some(c) = clusters.get_mut(cluster_id) {
            c.is_connected = true;
            c.last_connected = Some(Utc::now());
        }

        Ok(())
    }

    #[cfg(not(feature = "kubernetes"))]
    pub async fn connect_cluster(&self, _cluster_id: &str) -> Result<()> {
        Err(K8sError::NotSupported("Kubernetes feature not enabled".to_string()))
    }

    /// Disconnect from cluster
    #[cfg(feature = "kubernetes")]
    pub async fn disconnect_cluster(&self, cluster_id: &str) -> Result<()> {
        let mut clients = self.clients.write().await;
        clients.remove(cluster_id);

        let mut clusters = self.clusters.write().await;
        if let Some(c) = clusters.get_mut(cluster_id) {
            c.is_connected = false;
        }

        Ok(())
    }

    #[cfg(not(feature = "kubernetes"))]
    pub async fn disconnect_cluster(&self, _cluster_id: &str) -> Result<()> {
        Err(K8sError::NotSupported("Kubernetes feature not enabled".to_string()))
    }

    /// Get all clusters
    pub async fn get_clusters(&self) -> Vec<K8sCluster> {
        let clusters = self.clusters.read().await;
        clusters.values().cloned().collect()
    }

    /// Get cluster by ID
    pub async fn get_cluster(&self, id: &str) -> Result<K8sCluster> {
        let clusters = self.clusters.read().await;
        clusters
            .get(id)
            .cloned()
            .ok_or_else(|| K8sError::ClusterNotFound(id.to_string()))
    }

    /// Delete cluster
    pub async fn delete_cluster(&self, id: &str) -> Result<()> {
        // Disconnect first
        let _ = self.disconnect_cluster(id).await;

        let mut clusters = self.clusters.write().await;
        clusters
            .remove(id)
            .ok_or_else(|| K8sError::ClusterNotFound(id.to_string()))?;

        let mut cache = self.kubeconfig_cache.lock().await;
        cache.remove(id);

        Ok(())
    }

    /// Update cluster
    pub async fn update_cluster(&self, cluster: K8sCluster) -> Result<()> {
        let mut clusters = self.clusters.write().await;
        if !clusters.contains_key(&cluster.id) {
            return Err(K8sError::ClusterNotFound(cluster.id.clone()));
        }
        clusters.insert(cluster.id.clone(), cluster);
        Ok(())
    }

    /// Set current namespace for cluster
    pub async fn set_namespace(&self, cluster_id: &str, namespace: &str) -> Result<()> {
        let mut clusters = self.clusters.write().await;
        let cluster = clusters
            .get_mut(cluster_id)
            .ok_or_else(|| K8sError::ClusterNotFound(cluster_id.to_string()))?;
        cluster.current_namespace = namespace.to_string();
        cluster.updated_at = Utc::now();
        Ok(())
    }

    /// Get namespaces for cluster
    #[cfg(feature = "kubernetes")]
    pub async fn get_namespaces(&self, cluster_id: &str) -> Result<Vec<K8sNamespace>> {
        let clients = self.clients.read().await;
        let client = clients.get(cluster_id)
            .ok_or_else(|| K8sError::ConnectionFailed("Not connected".to_string()))?;

        let api: Api<Namespace> = Api::all(client.client.clone());
        let lp = ListParams::default();
        let namespaces = api.list(&lp).await.map_err(|e| K8sError::ApiError(e.to_string()))?;

        let result = namespaces.items.into_iter().map(|ns| {
            let status = ns.status.as_ref()
                .and_then(|s| s.phase.as_ref())
                .map(|p| p.to_string())
                .unwrap_or_else(|| "Unknown".to_string());

            K8sNamespace {
                name: ns.metadata.name.unwrap_or_default(),
                status,
                labels: ns.metadata.labels.unwrap_or_default().into_iter().collect(),
                annotations: ns.metadata.annotations.unwrap_or_default().into_iter().collect(),
                created_at: ns.metadata.creation_timestamp
                    .map(|t| t.0)
                    .unwrap_or_else(|| Utc::now()),
            }
        }).collect();

        Ok(result)
    }

    #[cfg(not(feature = "kubernetes"))]
    pub async fn get_namespaces(&self, _cluster_id: &str) -> Result<Vec<K8sNamespace>> {
        Err(K8sError::NotSupported("Kubernetes feature not enabled".to_string()))
    }

    /// Get pods in namespace
    #[cfg(feature = "kubernetes")]
    pub async fn get_pods(
        &self,
        cluster_id: &str,
        namespace: &str,
        label_selector: Option<&str>,
    ) -> Result<Vec<K8sPod>> {
        let clients = self.clients.read().await;
        let client = clients.get(cluster_id)
            .ok_or_else(|| K8sError::ConnectionFailed("Not connected".to_string()))?;

        let api: Api<Pod> = if namespace == "all" {
            Api::all(client.client.clone())
        } else {
            Api::namespaced(client.client.clone(), namespace)
        };

        let mut lp = ListParams::default();
        if let Some(selector) = label_selector {
            lp = lp.labels(selector);
        }

        let pods = api.list(&lp).await.map_err(|e| K8sError::ApiError(e.to_string()))?;

        let result = pods.items.into_iter().map(|pod| {
            let status = pod.status.as_ref();
            let spec = pod.spec.as_ref();

            let phase = status.and_then(|s| s.phase.clone()).unwrap_or_default();
            let pod_status = if pod.metadata.deletion_timestamp.is_some() {
                PodStatus::Terminating
            } else {
                PodStatus::from(phase.as_str())
            };

            let restarts = status
                .and_then(|s| s.container_statuses.as_ref())
                .map(|cs| cs.iter().map(|c| c.restart_count).sum())
                .unwrap_or(0);

            let containers = spec
                .map(|s| s.containers.iter().map(convert_container).collect())
                .unwrap_or_default();

            let init_containers = spec
                .and_then(|s| s.init_containers.as_ref())
                .map(|cs| cs.iter().map(convert_container).collect())
                .unwrap_or_default();

            K8sPod {
                name: pod.metadata.name.clone().unwrap_or_default(),
                namespace: pod.metadata.namespace.clone().unwrap_or_default(),
                status: pod_status,
                phase,
                restarts,
                node: status.and_then(|s| s.nominated_node_name.clone()).unwrap_or_default(),
                pod_ip: status.and_then(|s| s.pod_ip.clone()),
                labels: pod.metadata.labels.clone().unwrap_or_default().into_iter().collect(),
                annotations: pod.metadata.annotations.clone().unwrap_or_default().into_iter().collect(),
                containers,
                init_containers,
                conditions: status
                    .and_then(|s| s.conditions.clone())
                    .map(|c| c.iter().map(convert_condition).collect())
                    .unwrap_or_default(),
                volumes: spec
                    .and_then(|s| s.volumes.clone())
                    .map(|v| v.iter().map(convert_volume).collect())
                    .unwrap_or_default(),
                created_at: pod.metadata.creation_timestamp
                    .map(|t| t.0)
                    .unwrap_or_else(|| Utc::now()),
                started_at: status
                    .and_then(|s| s.start_time.clone())
                    .map(|t| t.0),
                resource_usage: None,
            }
        }).collect();

        Ok(result)
    }

    #[cfg(not(feature = "kubernetes"))]
    pub async fn get_pods(
        &self,
        _cluster_id: &str,
        _namespace: &str,
        _label_selector: Option<&str>,
    ) -> Result<Vec<K8sPod>> {
        Err(K8sError::NotSupported("Kubernetes feature not enabled".to_string()))
    }

    /// Get pod details
    pub async fn get_pod(&self, cluster_id: &str, namespace: &str, name: &str) -> Result<K8sPod> {
        let pods = self.get_pods(cluster_id, namespace, None).await?;
        pods.into_iter()
            .find(|p| p.name == name)
            .ok_or_else(|| K8sError::PodNotFound(name.to_string()))
    }

    /// Delete pod
    #[cfg(feature = "kubernetes")]
    pub async fn delete_pod(&self, cluster_id: &str, namespace: &str, name: &str) -> Result<()> {
        let clients = self.clients.read().await;
        let client = clients.get(cluster_id)
            .ok_or_else(|| K8sError::ConnectionFailed("Not connected".to_string()))?;

        let api: Api<Pod> = Api::namespaced(client.client.clone(), namespace);
        let dp = DeleteParams::default();
        api.delete(name, &dp).await.map_err(|e| K8sError::ApiError(e.to_string()))?;

        Ok(())
    }

    #[cfg(not(feature = "kubernetes"))]
    pub async fn delete_pod(&self, _cluster_id: &str, _namespace: &str, _name: &str) -> Result<()> {
        Err(K8sError::NotSupported("Kubernetes feature not enabled".to_string()))
    }

    /// Restart pod (delete and let deployment recreate)
    pub async fn restart_pod(&self, cluster_id: &str, namespace: &str, name: &str) -> Result<()> {
        self.delete_pod(cluster_id, namespace, name).await
    }

    /// Get pod logs
    #[cfg(feature = "kubernetes")]
    pub async fn get_pod_logs(
        &self,
        cluster_id: &str,
        namespace: &str,
        pod_name: &str,
        options: &LogOptions,
    ) -> Result<String> {
        let clients = self.clients.read().await;
        let client = clients.get(cluster_id)
            .ok_or_else(|| K8sError::ConnectionFailed("Not connected".to_string()))?;

        let api: Api<Pod> = Api::namespaced(client.client.clone(), namespace);

        let mut lp = kube::api::LogParams::default();
        lp.container = options.container.clone();
        lp.follow = options.follow;
        lp.pretty = true;
        lp.previous = options.previous;
        lp.timestamps = options.timestamps;
        lp.tail_lines = options.tail_lines;
        lp.since_seconds = options.since_seconds;

        let logs = api.logs(pod_name, &lp).await
            .map_err(|e| K8sError::ApiError(e.to_string()))?;

        Ok(logs)
    }

    #[cfg(not(feature = "kubernetes"))]
    pub async fn get_pod_logs(
        &self,
        _cluster_id: &str,
        _namespace: &str,
        _pod_name: &str,
        _options: &LogOptions,
    ) -> Result<String> {
        Err(K8sError::NotSupported("Kubernetes feature not enabled".to_string()))
    }

    /// Stream pod logs
    #[cfg(feature = "kubernetes")]
    pub async fn stream_pod_logs(
        &self,
        cluster_id: &str,
        namespace: &str,
        pod_name: &str,
        options: &LogOptions,
    ) -> Result<mpsc::UnboundedReceiver<String>> {
        let clients = self.clients.read().await;
        let client = clients.get(cluster_id)
            .ok_or_else(|| K8sError::ConnectionFailed("Not connected".to_string()))?;

        let api: Api<Pod> = Api::namespaced(client.client.clone(), namespace);

        let mut lp = kube::api::LogParams::default();
        lp.container = options.container.clone();
        lp.follow = true;
        lp.pretty = true;
        lp.previous = options.previous;
        lp.timestamps = options.timestamps;
        lp.tail_lines = options.tail_lines;
        lp.since_seconds = options.since_seconds;

        let (tx, rx) = mpsc::unbounded_channel();

        let stream = api.log_stream(pod_name, &lp).await
            .map_err(|e| K8sError::ApiError(e.to_string()))?;

        tokio::spawn(async move {
            let mut lines = stream.lines();
            while let Some(Ok(line)) = lines.next().await {
                let _ = tx.send(line);
            }
        });

        Ok(rx)
    }

    #[cfg(not(feature = "kubernetes"))]
    pub async fn stream_pod_logs(
        &self,
        _cluster_id: &str,
        _namespace: &str,
        _pod_name: &str,
        _options: &LogOptions,
    ) -> Result<mpsc::UnboundedReceiver<String>> {
        Err(K8sError::NotSupported("Kubernetes feature not enabled".to_string()))
    }

    /// Exec command in pod (non-interactive)
    #[cfg(feature = "kubernetes")]
    pub async fn exec_in_pod(
        &self,
        cluster_id: &str,
        namespace: &str,
        pod_name: &str,
        options: &ExecOptions,
    ) -> Result<String> {
        let clients = self.clients.read().await;
        let client = clients.get(cluster_id)
            .ok_or_else(|| K8sError::ConnectionFailed("Not connected".to_string()))?;

        let api: Api<Pod> = Api::namespaced(client.client.clone(), namespace);

        let ap = kube::api::AttachParams {
            container: options.container.clone(),
            stdin: options.stdin,
            stdout: true,
            stderr: true,
            tty: options.tty,
            ..Default::default()
        };

        let mut attached = api.exec(pod_name, &options.command, &ap).await
            .map_err(|e| K8sError::ExecError(e.to_string()))?;

        // Read stdout
        let mut stdout = String::new();
        if let Some(mut stdout_reader) = attached.stdout().take() {
            use tokio::io::AsyncReadExt;
            let mut buf = Vec::new();
            stdout_reader.read_to_end(&mut buf).await.map_err(|e| K8sError::ExecError(e.to_string()))?;
            stdout = String::from_utf8_lossy(&buf).to_string();
        }

        // Read stderr
        let mut stderr = String::new();
        if let Some(mut stderr_reader) = attached.stderr().take() {
            use tokio::io::AsyncReadExt;
            let mut buf = Vec::new();
            stderr_reader.read_to_end(&mut buf).await.map_err(|e| K8sError::ExecError(e.to_string()))?;
            stderr = String::from_utf8_lossy(&buf).to_string();
        }

        if !stderr.is_empty() {
            Ok(format!("{stdout}\n{stderr}"))
        } else {
            Ok(stdout)
        }
    }

    #[cfg(not(feature = "kubernetes"))]
    pub async fn exec_in_pod(
        &self,
        _cluster_id: &str,
        _namespace: &str,
        _pod_name: &str,
        _options: &ExecOptions,
    ) -> Result<String> {
        Err(K8sError::NotSupported("Kubernetes feature not enabled".to_string()))
    }

    /// Get nodes
    #[cfg(feature = "kubernetes")]
    pub async fn get_nodes(&self, cluster_id: &str) -> Result<Vec<K8sNode>> {
        let clients = self.clients.read().await;
        let client = clients.get(cluster_id)
            .ok_or_else(|| K8sError::ConnectionFailed("Not connected".to_string()))?;

        let api: Api<Node> = Api::all(client.client.clone());
        let lp = ListParams::default();
        let nodes = api.list(&lp).await.map_err(|e| K8sError::ApiError(e.to_string()))?;

        let result = nodes.items.into_iter().map(|node| {
            let metadata = &node.metadata;
            let spec = node.spec.as_ref();
            let status = node.status.as_ref();

            let roles: Vec<String> = metadata.labels.as_ref()
                .map(|l| {
                    l.iter()
                        .filter(|(k, _)| k.starts_with("node-role.kubernetes.io/"))
                        .map(|(k, _)| k.trim_start_matches("node-role.kubernetes.io/").to_string())
                        .collect()
                })
                .unwrap_or_default();

            K8sNode {
                name: metadata.name.clone().unwrap_or_default(),
                status: status
                    .and_then(|s| s.conditions.as_ref())
                    .map(|c| c.iter().find(|c| c.type_ == "Ready").map(|c| c.status.clone()).unwrap_or_default())
                    .unwrap_or_default(),
                roles,
                version: status.and_then(|s| s.node_info.as_ref()).map(|i| i.kubelet_version.clone()).unwrap_or_default(),
                os_image: status.and_then(|s| s.node_info.as_ref()).map(|i| i.os_image.clone()).unwrap_or_default(),
                kernel_version: status.and_then(|s| s.node_info.as_ref()).map(|i| i.kernel_version.clone()).unwrap_or_default(),
                container_runtime: status.and_then(|s| s.node_info.as_ref()).map(|i| i.container_runtime_version.clone()).unwrap_or_default(),
                labels: metadata.labels.clone().unwrap_or_default().into_iter().collect(),
                annotations: metadata.annotations.clone().unwrap_or_default().into_iter().collect(),
                addresses: status
                    .and_then(|s| s.addresses.clone())
                    .map(|a| a.iter().map(convert_node_address).collect())
                    .unwrap_or_default(),
                capacity: status.and_then(|s| s.capacity.clone()).map(convert_resources).unwrap_or_default(),
                allocatable: status.and_then(|s| s.allocatable.clone()).map(convert_resources).unwrap_or_default(),
                conditions: status
                    .and_then(|s| s.conditions.clone())
                    .map(|c| c.iter().map(convert_node_condition).collect())
                    .unwrap_or_default(),
                resource_usage: None,
            }
        }).collect();

        Ok(result)
    }

    #[cfg(not(feature = "kubernetes"))]
    pub async fn get_nodes(&self, _cluster_id: &str) -> Result<Vec<K8sNode>> {
        Err(K8sError::NotSupported("Kubernetes feature not enabled".to_string()))
    }

    /// Get deployments
    #[cfg(feature = "kubernetes")]
    pub async fn get_deployments(
        &self,
        cluster_id: &str,
        namespace: &str,
    ) -> Result<Vec<K8sDeployment>> {
        let clients = self.clients.read().await;
        let client = clients.get(cluster_id)
            .ok_or_else(|| K8sError::ConnectionFailed("Not connected".to_string()))?;

        let api: Api<Deployment> = Api::namespaced(client.client.clone(), namespace);
        let lp = ListParams::default();
        let deployments = api.list(&lp).await.map_err(|e| K8sError::ApiError(e.to_string()))?;

        let result = deployments.items.into_iter().map(|d| {
            let metadata = &d.metadata;
            let spec = d.spec.as_ref();
            let status = d.status.as_ref();

            let selector: HashMap<String, String> = spec
                .and_then(|s| s.selector.match_labels.clone())
                .map(|m| m.into_iter().collect())
                .unwrap_or_default();

            K8sDeployment {
                name: metadata.name.clone().unwrap_or_default(),
                namespace: metadata.namespace.clone().unwrap_or_default(),
                replicas: spec.and_then(|s| s.replicas).unwrap_or(0),
                available_replicas: status.map(|s| s.available_replicas.unwrap_or(0)).unwrap_or(0),
                ready_replicas: status.map(|s| s.ready_replicas.unwrap_or(0)).unwrap_or(0),
                updated_replicas: status.map(|s| s.updated_replicas.unwrap_or(0)).unwrap_or(0),
                labels: metadata.labels.clone().unwrap_or_default().into_iter().collect(),
                annotations: metadata.annotations.clone().unwrap_or_default().into_iter().collect(),
                selector,
                strategy: spec.and_then(|s| s.strategy.as_ref().and_then(|st| st.type_.clone())).unwrap_or_default(),
                conditions: status
                    .and_then(|s| s.conditions.clone())
                    .map(|c| c.iter().map(convert_deployment_condition).collect())
                    .unwrap_or_default(),
                created_at: metadata.creation_timestamp
                    .clone()
                    .map(|t| t.0)
                    .unwrap_or_else(|| Utc::now()),
                updated_at: metadata.creation_timestamp
                    .clone()
                    .map(|t| t.0)
                    .unwrap_or_else(|| Utc::now()),
            }
        }).collect();

        Ok(result)
    }

    #[cfg(not(feature = "kubernetes"))]
    pub async fn get_deployments(
        &self,
        _cluster_id: &str,
        _namespace: &str,
    ) -> Result<Vec<K8sDeployment>> {
        Err(K8sError::NotSupported("Kubernetes feature not enabled".to_string()))
    }

    /// Scale deployment
    #[cfg(feature = "kubernetes")]
    pub async fn scale_deployment(
        &self,
        cluster_id: &str,
        namespace: &str,
        name: &str,
        replicas: i32,
    ) -> Result<()> {
        let clients = self.clients.read().await;
        let client = clients.get(cluster_id)
            .ok_or_else(|| K8sError::ConnectionFailed("Not connected".to_string()))?;

        let api: Api<Deployment> = Api::namespaced(client.client.clone(), namespace);

        let patch = json!({
            "spec": {
                "replicas": replicas
            }
        });

        let pp = PatchParams::default();
        api.patch(name, &pp, &Patch::Merge(&patch)).await
            .map_err(|e| K8sError::ApiError(e.to_string()))?;

        Ok(())
    }

    #[cfg(not(feature = "kubernetes"))]
    pub async fn scale_deployment(
        &self,
        _cluster_id: &str,
        _namespace: &str,
        _name: &str,
        _replicas: i32,
    ) -> Result<()> {
        Err(K8sError::NotSupported("Kubernetes feature not enabled".to_string()))
    }

    /// Get services
    #[cfg(feature = "kubernetes")]
    pub async fn get_services(
        &self,
        cluster_id: &str,
        namespace: &str,
    ) -> Result<Vec<K8sService>> {
        let clients = self.clients.read().await;
        let client = clients.get(cluster_id)
            .ok_or_else(|| K8sError::ConnectionFailed("Not connected".to_string()))?;

        let api: Api<Service> = Api::namespaced(client.client.clone(), namespace);
        let lp = ListParams::default();
        let services = api.list(&lp).await.map_err(|e| K8sError::ApiError(e.to_string()))?;

        let result = services.items.into_iter().map(|s| {
            let metadata = &s.metadata;
            let spec = s.spec.as_ref();

            K8sService {
                name: metadata.name.clone().unwrap_or_default(),
                namespace: metadata.namespace.clone().unwrap_or_default(),
                service_type: spec.map(|s| s.type_.clone().unwrap_or_default()).unwrap_or_default(),
                cluster_ip: spec.map(|s| s.cluster_ip.clone().unwrap_or_default()).unwrap_or_default(),
                external_ips: spec.and_then(|s| s.external_ips.clone()).unwrap_or_default(),
                ports: spec
                    .and_then(|s| s.ports.clone())
                    .map(|p| p.iter().map(convert_service_port).collect())
                    .unwrap_or_default(),
                selector: spec.and_then(|s| s.selector.clone()).map(|s| s.into_iter().collect()).unwrap_or_default(),
                labels: metadata.labels.clone().unwrap_or_default().into_iter().collect(),
                annotations: metadata.annotations.clone().unwrap_or_default().into_iter().collect(),
                created_at: metadata.creation_timestamp
                    .clone()
                    .map(|t| t.0)
                    .unwrap_or_else(|| Utc::now()),
            }
        }).collect();

        Ok(result)
    }

    #[cfg(not(feature = "kubernetes"))]
    pub async fn get_services(
        &self,
        _cluster_id: &str,
        _namespace: &str,
    ) -> Result<Vec<K8sService>> {
        Err(K8sError::NotSupported("Kubernetes feature not enabled".to_string()))
    }

    /// Get ConfigMaps
    #[cfg(feature = "kubernetes")]
    pub async fn get_configmaps(
        &self,
        cluster_id: &str,
        namespace: &str,
    ) -> Result<Vec<K8sConfigMap>> {
        let clients = self.clients.read().await;
        let client = clients.get(cluster_id)
            .ok_or_else(|| K8sError::ConnectionFailed("Not connected".to_string()))?;

        let api: Api<ConfigMap> = Api::namespaced(client.client.clone(), namespace);
        let lp = ListParams::default();
        let configmaps = api.list(&lp).await.map_err(|e| K8sError::ApiError(e.to_string()))?;

        let result = configmaps.items.into_iter().map(|cm| {
            let metadata = &cm.metadata;

            K8sConfigMap {
                name: metadata.name.clone().unwrap_or_default(),
                namespace: metadata.namespace.clone().unwrap_or_default(),
                data: cm.data.clone().map(|d| d.into_iter().collect()).unwrap_or_default(),
                binary_data: cm.binary_data.clone().map(|d| d.into_iter().map(|(k, v)| (k, String::from_utf8_lossy(&v.0).to_string())).collect()).unwrap_or_default(),
                labels: metadata.labels.clone().unwrap_or_default().into_iter().collect(),
                annotations: metadata.annotations.clone().unwrap_or_default().into_iter().collect(),
                created_at: metadata.creation_timestamp
                    .clone()
                    .map(|t| t.0)
                    .unwrap_or_else(|| Utc::now()),
            }
        }).collect();

        Ok(result)
    }

    #[cfg(not(feature = "kubernetes"))]
    pub async fn get_configmaps(
        &self,
        _cluster_id: &str,
        _namespace: &str,
    ) -> Result<Vec<K8sConfigMap>> {
        Err(K8sError::NotSupported("Kubernetes feature not enabled".to_string()))
    }

    /// Get Secrets
    #[cfg(feature = "kubernetes")]
    pub async fn get_secrets(
        &self,
        cluster_id: &str,
        namespace: &str,
    ) -> Result<Vec<K8sSecret>> {
        let clients = self.clients.read().await;
        let client = clients.get(cluster_id)
            .ok_or_else(|| K8sError::ConnectionFailed("Not connected".to_string()))?;

        let api: Api<Secret> = Api::namespaced(client.client.clone(), namespace);
        let lp = ListParams::default();
        let secrets = api.list(&lp).await.map_err(|e| K8sError::ApiError(e.to_string()))?;

        let result = secrets.items.into_iter().map(|s| {
            let metadata = &s.metadata;

            K8sSecret {
                name: metadata.name.clone().unwrap_or_default(),
                namespace: metadata.namespace.clone().unwrap_or_default(),
                secret_type: s.type_.clone().unwrap_or_default(),
                data_keys: s.data.clone().map(|d| d.keys().cloned().collect()).unwrap_or_default(),
                labels: metadata.labels.clone().unwrap_or_default().into_iter().collect(),
                annotations: metadata.annotations.clone().unwrap_or_default().into_iter().collect(),
                created_at: metadata.creation_timestamp
                    .clone()
                    .map(|t| t.0)
                    .unwrap_or_else(|| Utc::now()),
            }
        }).collect();

        Ok(result)
    }

    #[cfg(not(feature = "kubernetes"))]
    pub async fn get_secrets(
        &self,
        _cluster_id: &str,
        _namespace: &str,
    ) -> Result<Vec<K8sSecret>> {
        Err(K8sError::NotSupported("Kubernetes feature not enabled".to_string()))
    }

    /// Get events
    #[cfg(feature = "kubernetes")]
    pub async fn get_events(
        &self,
        cluster_id: &str,
        namespace: Option<&str>,
        resource_kind: Option<&str>,
        resource_name: Option<&str>,
    ) -> Result<Vec<K8sEvent>> {
        let clients = self.clients.read().await;
        let client = clients.get(cluster_id)
            .ok_or_else(|| K8sError::ConnectionFailed("Not connected".to_string()))?;

        let api: Api<Event> = if let Some(ns) = namespace {
            Api::namespaced(client.client.clone(), ns)
        } else {
            Api::all(client.client.clone())
        };

        let mut lp = ListParams::default().limit(100);

        if let Some(kind) = resource_kind {
            if let Some(name) = resource_name {
                lp = lp.fields(&format!("involvedObject.kind={},involvedObject.name={}", kind, name));
            } else {
                lp = lp.fields(&format!("involvedObject.kind={}", kind));
            }
        }

        let events = api.list(&lp).await.map_err(|e| K8sError::ApiError(e.to_string()))?;

        let result = events.items.into_iter().map(|e| {
            let metadata = &e.metadata;
            let involved = e.involved_object;

            K8sEvent {
                name: metadata.name.clone().unwrap_or_default(),
                namespace: metadata.namespace.clone(),
                reason: e.reason.clone().unwrap_or_default(),
                message: e.message.clone().unwrap_or_default(),
                event_type: e.type_.clone().unwrap_or_default(),
                involved_object: K8sObjectReference {
                    kind: involved.kind.clone().unwrap_or_default(),
                    name: involved.name.clone().unwrap_or_default(),
                    namespace: involved.namespace.clone(),
                    uid: involved.uid.clone(),
                },
                count: e.count.unwrap_or(1) as i32,
                first_timestamp: e.first_timestamp.map(|t| t.0),
                last_timestamp: e.last_timestamp.map(|t| t.0),
                source: format!("{:?}", e.source),
            }
        }).collect();

        Ok(result)
    }

    #[cfg(not(feature = "kubernetes"))]
    pub async fn get_events(
        &self,
        _cluster_id: &str,
        _namespace: Option<&str>,
        _resource_kind: Option<&str>,
        _resource_name: Option<&str>,
    ) -> Result<Vec<K8sEvent>> {
        Err(K8sError::NotSupported("Kubernetes feature not enabled".to_string()))
    }

    /// Watch events
    #[cfg(feature = "kubernetes")]
    pub async fn watch_events(
        &self,
        cluster_id: &str,
        namespace: Option<&str>,
    ) -> Result<mpsc::UnboundedReceiver<K8sEvent>> {
        let clients = self.clients.read().await;
        let client = clients.get(cluster_id)
            .ok_or_else(|| K8sError::ConnectionFailed("Not connected".to_string()))?;

        let api: Api<Event> = if let Some(ns) = namespace {
            Api::namespaced(client.client.clone(), ns)
        } else {
            Api::all(client.client.clone())
        };

        let (tx, rx) = mpsc::unbounded_channel();

        let wp = WatchParams::default().timeout(300);
        let stream = api.watch(&wp, "0").await
            .map_err(|e| K8sError::ApiError(e.to_string()))?;

        tokio::spawn(async move {
            let mut stream = Box::pin(stream);
            while let Some(event) = stream.next().await {
                match event {
                    Ok(WatchEvent::Added(e) | WatchEvent::Modified(e)) => {
                        let metadata = &e.metadata;
                        let involved = &e.involved_object;
                        let k8s_event = K8sEvent {
                            name: metadata.name.clone().unwrap_or_default(),
                            namespace: metadata.namespace.clone(),
                            reason: e.reason.clone().unwrap_or_default(),
                            message: e.message.clone().unwrap_or_default(),
                            event_type: e.type_.clone().unwrap_or_default(),
                            involved_object: K8sObjectReference {
                                kind: involved.kind.clone().unwrap_or_default(),
                                name: involved.name.clone().unwrap_or_default(),
                                namespace: involved.namespace.clone(),
                                uid: involved.uid.clone(),
                            },
                            count: e.count.unwrap_or(1) as i32,
                            first_timestamp: e.first_timestamp.map(|t| t.0),
                            last_timestamp: e.last_timestamp.map(|t| t.0),
                            source: format!("{:?}", e.source),
                        };
                        let _ = tx.send(k8s_event);
                    }
                    _ => {}
                }
            }
        });

        Ok(rx)
    }

    #[cfg(not(feature = "kubernetes"))]
    pub async fn watch_events(
        &self,
        _cluster_id: &str,
        _namespace: Option<&str>,
    ) -> Result<mpsc::UnboundedReceiver<K8sEvent>> {
        Err(K8sError::NotSupported("Kubernetes feature not enabled".to_string()))
    }

    /// Apply YAML resource
    #[cfg(feature = "kubernetes")]
    pub async fn apply_yaml(
        &self,
        _cluster_id: &str,
        _yaml: &str,
        _namespace: Option<&str>,
    ) -> Result<K8sResource> {
        // TODO: Fix http crate version conflict (kube uses http 0.2, we use http 1.0)
        Err(K8sError::NotSupported("apply_yaml temporarily unavailable due to http crate version conflict".to_string()))
    }

    #[cfg(not(feature = "kubernetes"))]
    pub async fn apply_yaml(
        &self,
        _cluster_id: &str,
        _yaml: &str,
        _namespace: Option<&str>,
    ) -> Result<K8sResource> {
        Err(K8sError::NotSupported("Kubernetes feature not enabled".to_string()))
    }

    /// Get resource YAML
    #[cfg(feature = "kubernetes")]
    pub async fn get_resource_yaml(
        &self,
        _cluster_id: &str,
        _kind: &str,
        _name: &str,
        _namespace: &str,
    ) -> Result<String> {
        // TODO: Fix http crate version conflict (kube uses http 0.2, we use http 1.0)
        Err(K8sError::NotSupported("get_resource_yaml temporarily unavailable due to http crate version conflict".to_string()))
    }

    #[cfg(not(feature = "kubernetes"))]
    pub async fn get_resource_yaml(
        &self,
        _cluster_id: &str,
        _kind: &str,
        _name: &str,
        _namespace: &str,
    ) -> Result<String> {
        Err(K8sError::NotSupported("Kubernetes feature not enabled".to_string()))
    }

    /// Create port forward
    pub async fn create_port_forward(
        &self,
        cluster_id: &str,
        namespace: &str,
        pod_name: &str,
        local_port: u16,
        remote_port: u16,
    ) -> Result<K8sPortForward> {
        let id = uuid::Uuid::new_v4().to_string();
        let port_forward = K8sPortForward {
            id,
            cluster_id: cluster_id.to_string(),
            namespace: namespace.to_string(),
            pod_name: pod_name.to_string(),
            service_name: None,
            local_port,
            remote_port,
            protocol: "TCP".to_string(),
            is_active: true,
            created_at: Utc::now(),
        };

        let mut forwards = self.port_forwards.write().await;
        forwards.insert(port_forward.id.clone(), port_forward.clone());

        // Start port forward in background
        self.start_port_forward(&port_forward).await?;

        Ok(port_forward)
    }

    /// Start port forward process
    #[cfg(feature = "kubernetes")]
    async fn start_port_forward(&self, pf: &K8sPortForward) -> Result<()> {
        let clients = self.clients.read().await;
        let client = clients.get(&pf.cluster_id)
            .ok_or_else(|| K8sError::ConnectionFailed("Not connected".to_string()))?
            .client.clone();

        let api: Api<Pod> = Api::namespaced(client, &pf.namespace);
        let pod_name = pf.pod_name.clone();
        let local_port = pf.local_port;
        let remote_port = pf.remote_port;

        tokio::spawn(async move {
            // Port forward using kube API
            let _ = api.portforward(&pod_name, &[remote_port]).await;
        });

        Ok(())
    }

    #[cfg(not(feature = "kubernetes"))]
    async fn start_port_forward(&self, _pf: &K8sPortForward) -> Result<()> {
        Err(K8sError::NotSupported("Kubernetes feature not enabled".to_string()))
    }

    /// Create service port forward
    pub async fn create_service_port_forward(
        &self,
        cluster_id: &str,
        namespace: &str,
        service_name: &str,
        local_port: u16,
        remote_port: u16,
    ) -> Result<K8sPortForward> {
        let id = uuid::Uuid::new_v4().to_string();
        let port_forward = K8sPortForward {
            id,
            cluster_id: cluster_id.to_string(),
            namespace: namespace.to_string(),
            pod_name: "".to_string(),
            service_name: Some(service_name.to_string()),
            local_port,
            remote_port,
            protocol: "TCP".to_string(),
            is_active: true,
            created_at: Utc::now(),
        };

        let mut forwards = self.port_forwards.write().await;
        forwards.insert(port_forward.id.clone(), port_forward.clone());

        Ok(port_forward)
    }

    /// Stop port forward
    pub async fn stop_port_forward(&self, id: &str) -> Result<()> {
        let mut forwards = self.port_forwards.write().await;
        if let Some(pf) = forwards.get_mut(id) {
            pf.is_active = false;
            Ok(())
        } else {
            Err(K8sError::NotSupported("Port forward not found".to_string()))
        }
    }

    /// Delete port forward
    pub async fn delete_port_forward(&self, id: &str) -> Result<()> {
        let mut forwards = self.port_forwards.write().await;
        forwards
            .remove(id)
            .ok_or_else(|| K8sError::NotSupported("Port forward not found".to_string()))?;
        Ok(())
    }

    /// Get all port forwards
    pub async fn get_port_forwards(&self) -> Vec<K8sPortForward> {
        let forwards = self.port_forwards.read().await;
        forwards.values().cloned().collect()
    }

    /// Get port forwards for cluster
    pub async fn get_cluster_port_forwards(&self, cluster_id: &str) -> Vec<K8sPortForward> {
        let forwards = self.port_forwards.read().await;
        forwards
            .values()
            .filter(|pf| pf.cluster_id == cluster_id)
            .cloned()
            .collect()
    }

    /// Get Helm releases
    pub async fn get_helm_releases(
        &self,
        _cluster_id: &str,
        _namespace: Option<&str>,
    ) -> Result<Vec<HelmRelease>> {
        // Placeholder - would call helm CLI or use Helm SDK
        Ok(Vec::new())
    }

    /// Install Helm chart
    pub async fn helm_install(
        &self,
        _cluster_id: &str,
        _release_name: &str,
        _chart: &str,
        _namespace: &str,
        _values: Option<serde_json::Value>,
        _version: Option<&str>,
        _repo: Option<&str>,
    ) -> Result<HelmRelease> {
        // Placeholder
        Err(K8sError::HelmError("Not yet implemented".to_string()))
    }

    /// Upgrade Helm release
    pub async fn helm_upgrade(
        &self,
        _cluster_id: &str,
        _release_name: &str,
        _chart: &str,
        _namespace: &str,
        _values: Option<serde_json::Value>,
        _version: Option<&str>,
    ) -> Result<HelmRelease> {
        // Placeholder
        Err(K8sError::HelmError("Not yet implemented".to_string()))
    }

    /// Rollback Helm release
    pub async fn helm_rollback(
        &self,
        _cluster_id: &str,
        _release_name: &str,
        _namespace: &str,
        _revision: i32,
    ) -> Result<HelmRelease> {
        // Placeholder
        Err(K8sError::HelmError("Not yet implemented".to_string()))
    }

    /// Uninstall Helm release
    pub async fn helm_uninstall(
        &self,
        _cluster_id: &str,
        _release_name: &str,
        _namespace: &str,
    ) -> Result<()> {
        // Placeholder
        Err(K8sError::HelmError("Not yet implemented".to_string()))
    }

    /// Get Helm release history
    pub async fn get_helm_history(
        &self,
        _cluster_id: &str,
        _release_name: &str,
        _namespace: &str,
    ) -> Result<Vec<HelmRelease>> {
        // Placeholder
        Ok(Vec::new())
    }

    /// Add Helm repository
    pub async fn add_helm_repo(&self, name: &str, url: &str) -> Result<HelmRepo> {
        // Placeholder - would call helm repo add
        Ok(HelmRepo {
            name: name.to_string(),
            url: url.to_string(),
        })
    }

    /// List Helm repositories
    pub async fn list_helm_repos(&self) -> Result<Vec<HelmRepo>> {
        // Placeholder
        Ok(Vec::new())
    }

    /// Search Helm charts
    pub async fn search_helm_charts(&self, _keyword: &str) -> Result<Vec<HelmChart>> {
        // Placeholder
        Ok(Vec::new())
    }

    /// Get resource metrics (CPU/Memory usage)
    pub async fn get_pod_metrics(
        &self,
        _cluster_id: &str,
        _namespace: &str,
        _pod_name: &str,
    ) -> Result<K8sResourceUsage> {
        // Placeholder - would use metrics-server API
        Err(K8sError::NotSupported("Not yet implemented".to_string()))
    }

    /// Get all pods metrics in namespace
    pub async fn get_namespace_metrics(
        &self,
        _cluster_id: &str,
        _namespace: &str,
    ) -> Result<HashMap<String, K8sResourceUsage>> {
        // Placeholder
        Ok(HashMap::new())
    }

    /// Get node metrics
    pub async fn get_node_metrics(&self, _cluster_id: &str, _node_name: &str) -> Result<K8sNodeResourceUsage> {
        // Placeholder
        Err(K8sError::NotSupported("Not yet implemented".to_string()))
    }
}

impl Default for K8sManager {
    fn default() -> Self {
        Self::new()
    }
}

// Helper functions for conversion
#[cfg(feature = "kubernetes")]
fn convert_container(container: &Container) -> K8sContainer {
    K8sContainer {
        name: container.name.clone(),
        image: container.image.clone().unwrap_or_default(),
        ready: false,
        restart_count: 0,
        state: ContainerState::Unknown,
        ports: container.ports.as_ref()
            .map(|p| p.iter().map(|port| K8sContainerPort {
                name: port.name.clone().unwrap_or_default(),
                container_port: port.container_port,
                protocol: port.protocol.clone().unwrap_or_else(|| "TCP".to_string()),
            }).collect())
            .unwrap_or_default(),
        resources: container.resources.as_ref().map(|r| K8sResourceRequirements {
            limits: r.limits.as_ref().map(|l| l.iter().map(|(k, v)| (k.clone(), v.0.clone())).collect()).unwrap_or_default(),
            requests: r.requests.as_ref().map(|r| r.iter().map(|(k, v)| (k.clone(), v.0.clone())).collect()).unwrap_or_default(),
        }),
    }
}

#[cfg(feature = "kubernetes")]
fn convert_condition(condition: &k8s_openapi::api::core::v1::PodCondition) -> K8sPodCondition {
    K8sPodCondition {
        condition_type: condition.type_.clone(),
        status: condition.status.clone(),
        last_probe_time: condition.last_probe_time.as_ref()
            .map(|t| t.0),
        last_transition_time: condition.last_transition_time.as_ref()
            .map(|t| t.0),
        reason: condition.reason.clone().unwrap_or_default(),
        message: condition.message.clone().unwrap_or_default(),
    }
}

#[cfg(feature = "kubernetes")]
fn convert_volume(volume: &Volume) -> K8sVolume {
    let volume_type = if volume.empty_dir.is_some() { "EmptyDir" }
        else if volume.host_path.is_some() { "HostPath" }
        else if volume.persistent_volume_claim.is_some() { "PVC" }
        else if volume.config_map.is_some() { "ConfigMap" }
        else if volume.secret.is_some() { "Secret" }
        else if volume.projected.is_some() { "Projected" }
        else if volume.downward_api.is_some() { "DownwardAPI" }
        else { "Unknown" };

    K8sVolume {
        name: volume.name.clone(),
        volume_type: volume_type.to_string(),
        source: format!("{:?}", volume),
    }
}

#[cfg(feature = "kubernetes")]
fn convert_node_address(addr: &NodeAddress) -> K8sNodeAddress {
    K8sNodeAddress {
        address_type: addr.type_.clone(),
        address: addr.address.clone(),
    }
}

#[cfg(feature = "kubernetes")]
fn convert_node_condition(condition: &NodeCondition) -> K8sNodeCondition {
    K8sNodeCondition {
        condition_type: condition.type_.clone(),
        status: condition.status.clone(),
        last_heartbeat_time: condition.last_heartbeat_time.as_ref()
            .map(|t| t.0),
        last_transition_time: condition.last_transition_time.as_ref()
            .map(|t| t.0),
        reason: condition.reason.clone().unwrap_or_default(),
        message: condition.message.clone().unwrap_or_default(),
    }
}

#[cfg(feature = "kubernetes")]
fn convert_deployment_condition(condition: &k8s_openapi::api::apps::v1::DeploymentCondition) -> K8sDeploymentCondition {
    K8sDeploymentCondition {
        condition_type: condition.type_.clone(),
        status: condition.status.clone(),
        last_update_time: condition.last_update_time.as_ref()
            .map(|t| t.0),
        last_transition_time: condition.last_transition_time.as_ref()
            .map(|t| t.0),
        reason: condition.reason.clone().unwrap_or_default(),
        message: condition.message.clone().unwrap_or_default(),
    }
}

#[cfg(feature = "kubernetes")]
fn convert_service_port(port: &k8s_openapi::api::core::v1::ServicePort) -> K8sServicePort {
    K8sServicePort {
        name: port.name.clone().unwrap_or_default(),
        port: port.port,
        target_port: format!("{:?}", port.target_port),
        node_port: port.node_port,
        protocol: port.protocol.clone().unwrap_or_else(|| "TCP".to_string()),
    }
}

#[cfg(feature = "kubernetes")]
fn convert_resources(resources: BTreeMap<String, Quantity>) -> HashMap<String, String> {
    resources.into_iter().map(|(k, v)| (k, v.0)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_k8s_manager_new() {
        let manager = K8sManager::new();
        let clusters = manager.get_clusters().await;
        assert!(clusters.is_empty());
    }

    #[tokio::test]
    async fn test_import_kubeconfig() {
        let kubeconfig = r#"
apiVersion: v1
kind: Config
clusters:
- cluster:
    server: https://localhost:8443
  name: test-cluster
contexts:
- context:
    cluster: test-cluster
    user: test-user
    namespace: default
  name: test-context
current-context: test-context
users:
- name: test-user
  user:
    token: test-token
"#;

        let manager = K8sManager::new();
        let clusters = manager.import_kubeconfig(kubeconfig, None).await;
        assert!(clusters.is_ok());
        let clusters = clusters.unwrap();
        assert_eq!(clusters.len(), 1);
        assert_eq!(clusters[0].name, "test-cluster");
        assert_eq!(clusters[0].context, "test-context");
        assert_eq!(clusters[0].server_url, "https://localhost:8443");
    }

    #[tokio::test]
    async fn test_port_forward() {
        let manager = K8sManager::new();
        let pf = manager
            .create_port_forward("cluster-1", "default", "pod-1", 8080, 80)
            .await;
        assert!(pf.is_ok());

        let pf = pf.unwrap();
        assert_eq!(pf.local_port, 8080);
        assert_eq!(pf.remote_port, 80);
        assert!(pf.is_active);

        let forwards = manager.get_port_forwards().await;
        assert_eq!(forwards.len(), 1);

        manager.delete_port_forward(&pf.id).await.unwrap();
        let forwards = manager.get_port_forwards().await;
        assert!(forwards.is_empty());
    }

    #[test]
    fn test_pod_status_from_str() {
        assert!(matches!(PodStatus::from("Running"), PodStatus::Running));
        assert!(matches!(PodStatus::from("Pending"), PodStatus::Pending));
        assert!(matches!(PodStatus::from("CrashLoopBackOff"), PodStatus::CrashLoopBackOff));
        assert!(matches!(PodStatus::from("Unknown"), PodStatus::Unknown));
    }
}
