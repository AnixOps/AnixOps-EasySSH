use crate::kubernetes_client::{
    ExecOptions, HelmChart, HelmRelease, HelmRepo, K8sCluster, K8sConfigMap, K8sDeployment,
    K8sEvent, K8sNamespace, K8sNode, K8sPod, K8sPortForward, K8sResource, K8sSecret, K8sService,
    LogOptions,
};
use crate::AppState;
use std::sync::Arc;
use tauri::State;

/// Get all Kubernetes clusters
#[tauri::command]
pub async fn k8s_get_clusters(state: State<'_, AppState>) -> Result<Vec<K8sCluster>, String> {
    let k8s_manager = state.k8s_manager.read().await;
    Ok(k8s_manager.get_clusters().await)
}

/// Import kubeconfig from file path
#[tauri::command]
pub async fn k8s_import_kubeconfig_path(
    state: State<'_, AppState>,
    path: String,
) -> Result<Vec<K8sCluster>, String> {
    let k8s_manager = state.k8s_manager.read().await;
    k8s_manager
        .import_kubeconfig_from_path(&path)
        .await
        .map_err(|e| e.to_string())
}

/// Import kubeconfig from content string
#[tauri::command]
pub async fn k8s_import_kubeconfig_content(
    state: State<'_, AppState>,
    content: String,
) -> Result<Vec<K8sCluster>, String> {
    let k8s_manager = state.k8s_manager.read().await;
    k8s_manager
        .import_kubeconfig(&content, None)
        .await
        .map_err(|e| e.to_string())
}

/// Connect to a Kubernetes cluster
#[tauri::command]
pub async fn k8s_connect_cluster(
    state: State<'_, AppState>,
    cluster_id: String,
) -> Result<(), String> {
    let k8s_manager = state.k8s_manager.read().await;
    k8s_manager
        .connect_cluster(&cluster_id)
        .await
        .map_err(|e| e.to_string())
}

/// Disconnect from a Kubernetes cluster
#[tauri::command]
pub async fn k8s_disconnect_cluster(
    state: State<'_, AppState>,
    cluster_id: String,
) -> Result<(), String> {
    let k8s_manager = state.k8s_manager.read().await;
    k8s_manager
        .disconnect_cluster(&cluster_id)
        .await
        .map_err(|e| e.to_string())
}

/// Delete a Kubernetes cluster configuration
#[tauri::command]
pub async fn k8s_delete_cluster(
    state: State<'_, AppState>,
    cluster_id: String,
) -> Result<(), String> {
    let k8s_manager = state.k8s_manager.read().await;
    k8s_manager
        .delete_cluster(&cluster_id)
        .await
        .map_err(|e| e.to_string())
}

/// Update a Kubernetes cluster configuration
#[tauri::command]
pub async fn k8s_update_cluster(
    state: State<'_, AppState>,
    cluster: K8sCluster,
) -> Result<(), String> {
    let k8s_manager = state.k8s_manager.read().await;
    k8s_manager
        .update_cluster(cluster)
        .await
        .map_err(|e| e.to_string())
}

/// Get namespaces for a cluster
#[tauri::command]
pub async fn k8s_get_namespaces(
    state: State<'_, AppState>,
    cluster_id: String,
) -> Result<Vec<K8sNamespace>, String> {
    let k8s_manager = state.k8s_manager.read().await;
    k8s_manager
        .get_namespaces(&cluster_id)
        .await
        .map_err(|e| e.to_string())
}

/// Set current namespace for a cluster
#[tauri::command]
pub async fn k8s_set_namespace(
    state: State<'_, AppState>,
    cluster_id: String,
    namespace: String,
) -> Result<(), String> {
    let k8s_manager = state.k8s_manager.read().await;
    k8s_manager
        .set_namespace(&cluster_id, &namespace)
        .await
        .map_err(|e| e.to_string())
}

/// Get pods in a namespace
#[tauri::command]
pub async fn k8s_get_pods(
    state: State<'_, AppState>,
    cluster_id: String,
    namespace: String,
    label_selector: Option<String>,
) -> Result<Vec<K8sPod>, String> {
    let k8s_manager = state.k8s_manager.read().await;
    k8s_manager
        .get_pods(&cluster_id, &namespace, label_selector.as_deref())
        .await
        .map_err(|e| e.to_string())
}

/// Delete a pod
#[tauri::command]
pub async fn k8s_delete_pod(
    state: State<'_, AppState>,
    cluster_id: String,
    namespace: String,
    pod_name: String,
) -> Result<(), String> {
    let k8s_manager = state.k8s_manager.read().await;
    k8s_manager
        .delete_pod(&cluster_id, &namespace, &pod_name)
        .await
        .map_err(|e| e.to_string())
}

/// Restart a pod
#[tauri::command]
pub async fn k8s_restart_pod(
    state: State<'_, AppState>,
    cluster_id: String,
    namespace: String,
    pod_name: String,
) -> Result<(), String> {
    let k8s_manager = state.k8s_manager.read().await;
    k8s_manager
        .restart_pod(&cluster_id, &namespace, &pod_name)
        .await
        .map_err(|e| e.to_string())
}

/// Get pod logs
#[tauri::command]
pub async fn k8s_get_pod_logs(
    state: State<'_, AppState>,
    cluster_id: String,
    namespace: String,
    pod_name: String,
    follow: bool,
    tail_lines: Option<i64>,
    since_seconds: Option<i64>,
    timestamps: bool,
    previous: bool,
    container: Option<String>,
) -> Result<String, String> {
    let k8s_manager = state.k8s_manager.read().await;
    let options = LogOptions {
        follow,
        tail_lines,
        since_seconds,
        timestamps,
        previous,
        container,
    };
    k8s_manager
        .get_pod_logs(&cluster_id, &namespace, &pod_name, &options)
        .await
        .map_err(|e| e.to_string())
}

/// Stream pod logs (returns channel)
#[tauri::command]
pub async fn k8s_stream_pod_logs(
    state: State<'_, AppState>,
    cluster_id: String,
    namespace: String,
    pod_name: String,
    tail_lines: Option<i64>,
    container: Option<String>,
    window: tauri::Window,
) -> Result<(), String> {
    let k8s_manager = state.k8s_manager.read().await;
    let options = LogOptions {
        follow: true,
        tail_lines,
        since_seconds: None,
        timestamps: false,
        previous: false,
        container: container.clone(),
    };

    let mut rx = k8s_manager
        .stream_pod_logs(&cluster_id, &namespace, &pod_name, &options)
        .await
        .map_err(|e| e.to_string())?;

    tokio::spawn(async move {
        while let Some(log) = rx.recv().await {
            let _ = window.emit(
                "k8s-log",
                serde_json::json!({
                    "podName": pod_name,
                    "container": container,
                    "log": log,
                }),
            );
        }
    });

    Ok(())
}

/// Execute command in pod
#[tauri::command]
pub async fn k8s_exec_in_pod(
    state: State<'_, AppState>,
    cluster_id: String,
    namespace: String,
    pod_name: String,
    command: String,
    container: Option<String>,
) -> Result<String, String> {
    let k8s_manager = state.k8s_manager.read().await;
    let options = ExecOptions {
        container,
        stdin: false,
        tty: false,
        command: command.split_whitespace().map(|s| s.to_string()).collect(),
    };
    k8s_manager
        .exec_in_pod(&cluster_id, &namespace, &pod_name, &options)
        .await
        .map_err(|e| e.to_string())
}

/// Get nodes in cluster
#[tauri::command]
pub async fn k8s_get_nodes(
    state: State<'_, AppState>,
    cluster_id: String,
) -> Result<Vec<K8sNode>, String> {
    let k8s_manager = state.k8s_manager.read().await;
    k8s_manager
        .get_nodes(&cluster_id)
        .await
        .map_err(|e| e.to_string())
}

/// Get deployments in namespace
#[tauri::command]
pub async fn k8s_get_deployments(
    state: State<'_, AppState>,
    cluster_id: String,
    namespace: String,
) -> Result<Vec<K8sDeployment>, String> {
    let k8s_manager = state.k8s_manager.read().await;
    k8s_manager
        .get_deployments(&cluster_id, &namespace)
        .await
        .map_err(|e| e.to_string())
}

/// Scale a deployment
#[tauri::command]
pub async fn k8s_scale_deployment(
    state: State<'_, AppState>,
    cluster_id: String,
    namespace: String,
    deployment_name: String,
    replicas: i32,
) -> Result<(), String> {
    let k8s_manager = state.k8s_manager.read().await;
    k8s_manager
        .scale_deployment(&cluster_id, &namespace, &deployment_name, replicas)
        .await
        .map_err(|e| e.to_string())
}

/// Get services in namespace
#[tauri::command]
pub async fn k8s_get_services(
    state: State<'_, AppState>,
    cluster_id: String,
    namespace: String,
) -> Result<Vec<K8sService>, String> {
    let k8s_manager = state.k8s_manager.read().await;
    k8s_manager
        .get_services(&cluster_id, &namespace)
        .await
        .map_err(|e| e.to_string())
}

/// Get ConfigMaps in namespace
#[tauri::command]
pub async fn k8s_get_configmaps(
    state: State<'_, AppState>,
    cluster_id: String,
    namespace: String,
) -> Result<Vec<K8sConfigMap>, String> {
    let k8s_manager = state.k8s_manager.read().await;
    k8s_manager
        .get_configmaps(&cluster_id, &namespace)
        .await
        .map_err(|e| e.to_string())
}

/// Get Secrets in namespace
#[tauri::command]
pub async fn k8s_get_secrets(
    state: State<'_, AppState>,
    cluster_id: String,
    namespace: String,
) -> Result<Vec<K8sSecret>, String> {
    let k8s_manager = state.k8s_manager.read().await;
    k8s_manager
        .get_secrets(&cluster_id, &namespace)
        .await
        .map_err(|e| e.to_string())
}

/// Get events
#[tauri::command]
pub async fn k8s_get_events(
    state: State<'_, AppState>,
    cluster_id: String,
    namespace: Option<String>,
) -> Result<Vec<K8sEvent>, String> {
    let k8s_manager = state.k8s_manager.read().await;
    k8s_manager
        .get_events(&cluster_id, namespace.as_deref(), None, None)
        .await
        .map_err(|e| e.to_string())
}

/// Watch events (streaming)
#[tauri::command]
pub async fn k8s_watch_events(
    state: State<'_, AppState>,
    cluster_id: String,
    namespace: Option<String>,
    window: tauri::Window,
) -> Result<(), String> {
    let k8s_manager = state.k8s_manager.read().await;
    let mut rx = k8s_manager
        .watch_events(&cluster_id, namespace.as_deref())
        .await
        .map_err(|e| e.to_string())?;

    tokio::spawn(async move {
        while let Some(event) = rx.recv().await {
            let _ = window.emit("k8s-event", event);
        }
    });

    Ok(())
}

/// Create port forward to pod
#[tauri::command]
pub async fn k8s_create_port_forward(
    state: State<'_, AppState>,
    cluster_id: String,
    namespace: String,
    pod_name: String,
    local_port: u16,
    remote_port: u16,
) -> Result<K8sPortForward, String> {
    let k8s_manager = state.k8s_manager.read().await;
    k8s_manager
        .create_port_forward(&cluster_id, &namespace, &pod_name, local_port, remote_port)
        .await
        .map_err(|e| e.to_string())
}

/// Get all port forwards
#[tauri::command]
pub async fn k8s_get_port_forwards(
    state: State<'_, AppState>,
) -> Result<Vec<K8sPortForward>, String> {
    let k8s_manager = state.k8s_manager.read().await;
    Ok(k8s_manager.get_port_forwards().await)
}

/// Stop a port forward
#[tauri::command]
pub async fn k8s_stop_port_forward(
    state: State<'_, AppState>,
    forward_id: String,
) -> Result<(), String> {
    let k8s_manager = state.k8s_manager.read().await;
    k8s_manager
        .stop_port_forward(&forward_id)
        .await
        .map_err(|e| e.to_string())
}

/// Delete a port forward
#[tauri::command]
pub async fn k8s_delete_port_forward(
    state: State<'_, AppState>,
    forward_id: String,
) -> Result<(), String> {
    let k8s_manager = state.k8s_manager.read().await;
    k8s_manager
        .delete_port_forward(&forward_id)
        .await
        .map_err(|e| e.to_string())
}

/// Get Helm releases
#[tauri::command]
pub async fn k8s_get_helm_releases(
    state: State<'_, AppState>,
    cluster_id: String,
    namespace: Option<String>,
) -> Result<Vec<HelmRelease>, String> {
    let k8s_manager = state.k8s_manager.read().await;
    k8s_manager
        .get_helm_releases(&cluster_id, namespace.as_deref())
        .await
        .map_err(|e| e.to_string())
}

/// Install Helm chart
#[tauri::command]
pub async fn k8s_helm_install(
    state: State<'_, AppState>,
    cluster_id: String,
    release_name: String,
    chart: String,
    namespace: String,
    version: Option<String>,
) -> Result<HelmRelease, String> {
    let k8s_manager = state.k8s_manager.read().await;
    k8s_manager
        .helm_install(
            &cluster_id,
            &release_name,
            &chart,
            &namespace,
            None,
            version.as_deref(),
            None,
        )
        .await
        .map_err(|e| e.to_string())
}

/// Upgrade Helm release
#[tauri::command]
pub async fn k8s_helm_upgrade(
    state: State<'_, AppState>,
    cluster_id: String,
    release_name: String,
    chart: String,
    namespace: String,
    version: Option<String>,
) -> Result<HelmRelease, String> {
    let k8s_manager = state.k8s_manager.read().await;
    k8s_manager
        .helm_upgrade(
            &cluster_id,
            &release_name,
            &chart,
            &namespace,
            None,
            version.as_deref(),
        )
        .await
        .map_err(|e| e.to_string())
}

/// Rollback Helm release
#[tauri::command]
pub async fn k8s_helm_rollback(
    state: State<'_, AppState>,
    cluster_id: String,
    release_name: String,
    namespace: String,
    revision: i32,
) -> Result<HelmRelease, String> {
    let k8s_manager = state.k8s_manager.read().await;
    k8s_manager
        .helm_rollback(&cluster_id, &release_name, &namespace, revision)
        .await
        .map_err(|e| e.to_string())
}

/// Uninstall Helm release
#[tauri::command]
pub async fn k8s_helm_uninstall(
    state: State<'_, AppState>,
    cluster_id: String,
    release_name: String,
    namespace: String,
) -> Result<(), String> {
    let k8s_manager = state.k8s_manager.read().await;
    k8s_manager
        .helm_uninstall(&cluster_id, &release_name, &namespace)
        .await
        .map_err(|e| e.to_string())
}

/// List Helm repositories
#[tauri::command]
pub async fn k8s_list_helm_repos(state: State<'_, AppState>) -> Result<Vec<HelmRepo>, String> {
    let k8s_manager = state.k8s_manager.read().await;
    k8s_manager
        .list_helm_repos()
        .await
        .map_err(|e| e.to_string())
}

/// Add Helm repository
#[tauri::command]
pub async fn k8s_add_helm_repo(
    state: State<'_, AppState>,
    name: String,
    url: String,
) -> Result<HelmRepo, String> {
    let k8s_manager = state.k8s_manager.read().await;
    k8s_manager
        .add_helm_repo(&name, &url)
        .await
        .map_err(|e| e.to_string())
}

/// Search Helm charts
#[tauri::command]
pub async fn k8s_search_helm_charts(
    state: State<'_, AppState>,
    keyword: String,
) -> Result<Vec<HelmChart>, String> {
    let k8s_manager = state.k8s_manager.read().await;
    k8s_manager
        .search_helm_charts(&keyword)
        .await
        .map_err(|e| e.to_string())
}

/// Apply YAML resource
#[tauri::command]
pub async fn k8s_apply_yaml(
    state: State<'_, AppState>,
    cluster_id: String,
    yaml: String,
    namespace: Option<String>,
) -> Result<K8sResource, String> {
    let k8s_manager = state.k8s_manager.read().await;
    k8s_manager
        .apply_yaml(&cluster_id, &yaml, namespace.as_deref())
        .await
        .map_err(|e| e.to_string())
}

/// Get resource YAML
#[tauri::command]
pub async fn k8s_get_resource_yaml(
    state: State<'_, AppState>,
    cluster_id: String,
    kind: String,
    name: String,
    namespace: String,
) -> Result<String, String> {
    let k8s_manager = state.k8s_manager.read().await;
    k8s_manager
        .get_resource_yaml(&cluster_id, &kind, &name, &namespace)
        .await
        .map_err(|e| e.to_string())
}

/// Delete resource
#[tauri::command]
pub async fn k8s_delete_resource(
    state: State<'_, AppState>,
    cluster_id: String,
    kind: String,
    name: String,
    namespace: String,
) -> Result<(), String> {
    let k8s_manager = state.k8s_manager.read().await;
    // Get resource YAML first to verify it exists
    let _ = k8s_manager
        .get_resource_yaml(&cluster_id, &kind, &name, &namespace)
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

/// Get all Tauri commands for Kubernetes
pub fn get_k8s_commands() -> Vec<Box<dyn tauri::Plugin>> {
    vec![]
}
