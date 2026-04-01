use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::sync::{Mutex, RwLock};

#[derive(Error, Debug)]
pub enum K8sError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
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

/// Kubernetes manager for cluster operations
pub struct K8sManager {
    clusters: RwLock<HashMap<String, K8sCluster>>,
    port_forwards: RwLock<HashMap<String, K8sPortForward>>,
    kubeconfig_cache: Mutex<HashMap<String, serde_json::Value>>,
}

impl K8sManager {
    pub fn new() -> Self {
        Self {
            clusters: RwLock::new(HashMap::new()),
            port_forwards: RwLock::new(HashMap::new()),
            kubeconfig_cache: Mutex::new(HashMap::new()),
        }
    }

    /// Import kubeconfig from file path
    pub async fn import_kubeconfig_from_path(&self, path: &str) -> Result<Vec<K8sCluster>> {
        let content = tokio::fs::read_to_string(path).await?;
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
                    let cluster = self
                        .parse_cluster_from_context(&kubeconfig, context_name, path, content)
                        .await?;
                    clusters.push(cluster);
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
    pub async fn get_namespaces(&self, _cluster_id: &str) -> Result<Vec<K8sNamespace>> {
        // Placeholder - would use kube-rs client in real implementation
        Ok(vec![
            K8sNamespace {
                name: "default".to_string(),
                status: "Active".to_string(),
                labels: HashMap::new(),
                annotations: HashMap::new(),
                created_at: Utc::now(),
            },
            K8sNamespace {
                name: "kube-system".to_string(),
                status: "Active".to_string(),
                labels: HashMap::new(),
                annotations: HashMap::new(),
                created_at: Utc::now(),
            },
        ])
    }

    /// Get pods in namespace
    pub async fn get_pods(
        &self,
        _cluster_id: &str,
        _namespace: &str,
        _label_selector: Option<&str>,
    ) -> Result<Vec<K8sPod>> {
        // Placeholder - would use kube-rs client in real implementation
        Ok(Vec::new())
    }

    /// Get pod details
    pub async fn get_pod(&self, _cluster_id: &str, _namespace: &str, _name: &str) -> Result<K8sPod> {
        // Placeholder
        Err(K8sError::NotSupported("Not yet implemented".to_string()))
    }

    /// Delete pod
    pub async fn delete_pod(&self, _cluster_id: &str, _namespace: &str, _name: &str) -> Result<()> {
        // Placeholder
        Err(K8sError::NotSupported("Not yet implemented".to_string()))
    }

    /// Restart pod (delete and let deployment recreate)
    pub async fn restart_pod(&self, _cluster_id: &str, _namespace: &str, _name: &str) -> Result<()> {
        // Placeholder
        Err(K8sError::NotSupported("Not yet implemented".to_string()))
    }

    /// Get pod logs
    pub async fn get_pod_logs(
        &self,
        _cluster_id: &str,
        _namespace: &str,
        _pod_name: &str,
        _options: &LogOptions,
    ) -> Result<String> {
        // Placeholder
        Err(K8sError::NotSupported("Not yet implemented".to_string()))
    }

    /// Stream pod logs
    pub async fn stream_pod_logs(
        &self,
        _cluster_id: &str,
        _namespace: &str,
        _pod_name: &str,
        _options: &LogOptions,
    ) -> Result<tokio::sync::mpsc::UnboundedReceiver<String>> {
        // Placeholder
        Err(K8sError::NotSupported("Not yet implemented".to_string()))
    }

    /// Exec command in pod
    pub async fn exec_in_pod(
        &self,
        _cluster_id: &str,
        _namespace: &str,
        _pod_name: &str,
        _options: &ExecOptions,
    ) -> Result<String> {
        // Placeholder
        Err(K8sError::NotSupported("Not yet implemented".to_string()))
    }

    /// Start interactive exec session
    pub async fn exec_interactive(
        &self,
        _cluster_id: &str,
        _namespace: &str,
        _pod_name: &str,
        _container: Option<&str>,
    ) -> Result<(
        tokio::sync::mpsc::UnboundedSender<String>,
        tokio::sync::mpsc::UnboundedReceiver<String>,
    )> {
        // Placeholder
        Err(K8sError::NotSupported("Not yet implemented".to_string()))
    }

    /// Get nodes
    pub async fn get_nodes(&self, _cluster_id: &str) -> Result<Vec<K8sNode>> {
        // Placeholder
        Ok(Vec::new())
    }

    /// Get node details
    pub async fn get_node(&self, _cluster_id: &str, _name: &str) -> Result<K8sNode> {
        // Placeholder
        Err(K8sError::NotSupported("Not yet implemented".to_string()))
    }

    /// Get deployments
    pub async fn get_deployments(
        &self,
        _cluster_id: &str,
        _namespace: &str,
    ) -> Result<Vec<K8sDeployment>> {
        // Placeholder
        Ok(Vec::new())
    }

    /// Scale deployment
    pub async fn scale_deployment(
        &self,
        _cluster_id: &str,
        _namespace: &str,
        _name: &str,
        _replicas: i32,
    ) -> Result<()> {
        // Placeholder
        Err(K8sError::NotSupported("Not yet implemented".to_string()))
    }

    /// Get services
    pub async fn get_services(
        &self,
        _cluster_id: &str,
        _namespace: &str,
    ) -> Result<Vec<K8sService>> {
        // Placeholder
        Ok(Vec::new())
    }

    /// Get ConfigMaps
    pub async fn get_configmaps(
        &self,
        _cluster_id: &str,
        _namespace: &str,
    ) -> Result<Vec<K8sConfigMap>> {
        // Placeholder
        Ok(Vec::new())
    }

    /// Get Secrets
    pub async fn get_secrets(
        &self,
        _cluster_id: &str,
        _namespace: &str,
    ) -> Result<Vec<K8sSecret>> {
        // Placeholder
        Ok(Vec::new())
    }

    /// Get events
    pub async fn get_events(
        &self,
        _cluster_id: &str,
        _namespace: Option<&str>,
        _resource_kind: Option<&str>,
        _resource_name: Option<&str>,
    ) -> Result<Vec<K8sEvent>> {
        // Placeholder
        Ok(Vec::new())
    }

    /// Watch events
    pub async fn watch_events(
        &self,
        _cluster_id: &str,
        _namespace: Option<&str>,
    ) -> Result<tokio::sync::mpsc::UnboundedReceiver<K8sEvent>> {
        // Placeholder
        Err(K8sError::NotSupported("Not yet implemented".to_string()))
    }

    /// Apply YAML resource
    pub async fn apply_yaml(
        &self,
        _cluster_id: &str,
        _yaml: &str,
        _namespace: Option<&str>,
    ) -> Result<K8sResource> {
        // Placeholder
        Err(K8sError::NotSupported("Not yet implemented".to_string()))
    }

    /// Delete resource by YAML
    pub async fn delete_yaml(
        &self,
        _cluster_id: &str,
        _yaml: &str,
        _namespace: Option<&str>,
    ) -> Result<()> {
        // Placeholder
        Err(K8sError::NotSupported("Not yet implemented".to_string()))
    }

    /// Get resource YAML
    pub async fn get_resource_yaml(
        &self,
        _cluster_id: &str,
        _kind: &str,
        _name: &str,
        _namespace: &str,
    ) -> Result<String> {
        // Placeholder
        Err(K8sError::NotSupported("Not yet implemented".to_string()))
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

        Ok(port_forward)
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
}
