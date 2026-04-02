# Kubernetes Support Implementation Summary

This document summarizes the comprehensive Kubernetes support implementation for EasySSH.

## Overview

Full-featured Kubernetes cluster management has been integrated into EasySSH, inspired by Lens and k9s. The implementation provides a complete K8s management interface alongside SSH functionality.

## Architecture

### Backend (Rust Core)

Located in `core/src/`:

1. **kubernetes.rs** - Base types and placeholder implementation
2. **kubernetes_client.rs** - Full kube-rs integration with real K8s API
3. **kubernetes_ffi.rs** - FFI bindings for cross-platform support
4. **kubernetes_tauri.rs** - Tauri commands for frontend integration

### Frontend (TypeScript/React)

Located in `src/`:

1. **types/index.ts** - Complete TypeScript type definitions
2. **stores/k8sStore.ts** - Zustand store for K8s state management

### Database

Updated `core/src/db.rs` with new tables:
- `k8s_clusters` - Cluster configurations
- `k8s_namespaces` - Namespace cache
- `k8s_port_forwards` - Port forward rules

## Features Implemented

### 1. Cluster Connection (✓)
- Import kubeconfig from file path
- Import kubeconfig from content string
- Support for multiple clusters
- Context switching
- Connection status tracking

### 2. Pod Management (✓)
- List pods with filtering by label selector
- View detailed pod information
  - Container status and state
  - Restart counts
  - Resource usage
  - Conditions and volumes
- Delete pods
- Restart pods (recreate)

### 3. Log Viewing (✓)
- View pod logs with options:
  - Tail lines (-n)
  - Since time
  - Include timestamps
  - Previous container logs
- Stream logs in real-time (follow mode)
- Container selection for multi-container pods

### 4. Terminal Access (✓)
- kubectl exec functionality
- Execute commands in pods
- Support for interactive sessions
- Container selection

### 5. Resource Monitoring (✓)
- Node listing with capacity/allocatable info
- Node conditions and status
- Deployment listing with replica status
- Service discovery

### 6. YAML Editor Support (✓)
- Apply YAML resources
- Get resource YAML for editing
- Support for all K8s resource types
- Namespace-scoped operations

### 7. Port Forward (✓)
- Pod port forwarding
- Service port forwarding
- Local to remote port mapping
- Active connection management

### 8. Helm Support (✓)
- List Helm releases
- Install charts
- Upgrade releases
- Rollback to previous revisions
- Uninstall releases
- Repository management
- Chart search

### 9. Event Monitoring (✓)
- List cluster events
- Watch events in real-time
- Filter by namespace/resource
- Event streaming to frontend

### 10. Namespace Management (✓)
- List all namespaces
- Switch active namespace
- Namespace status display

### Additional Resources (✓)
- ConfigMaps (list, view)
- Secrets (list, view keys only)
- Services (list, view)
- Deployments (list, view, scale)
- Nodes (list, view details)

## Type Definitions

### Core Types (Rust)

```rust
K8sCluster        // Cluster configuration
K8sPod           // Pod information with containers
K8sNode          // Node information with resources
K8sDeployment    // Deployment with replica status
K8sService       // Service with ports and selectors
K8sConfigMap     // ConfigMap data
K8sSecret        // Secret (key names only)
K8sEvent         // Cluster events
K8sPortForward   // Port forward configuration
HelmRelease      // Helm release information
HelmChart        // Helm chart metadata
HelmRepo         // Helm repository
K8sResource      // Generic K8s resource for YAML
```

### TypeScript Types

All Rust types have TypeScript equivalents in `src/types/index.ts` for frontend use.

## State Management

The `k8sStore` provides:

```typescript
// State
clusters, currentCluster
namespaces, currentNamespace
pods, selectedPod, podLogs
nodes, deployments, services
configMaps, secrets, events
portForwards, helmReleases

// Actions
loadClusters(), importKubeconfig()
connectCluster(), disconnectCluster()
loadPods(), deletePod(), restartPod()
getPodLogs(), streamPodLogs()
execInPod()
loadNodes(), loadDeployments()
scaleDeployment()
loadServices(), loadConfigMaps()
loadSecrets(), loadEvents()
watchEvents(), stopWatchingEvents()
createPortForward(), deletePortForward()
loadHelmReleases(), installHelmChart()
upgradeHelmRelease(), rollbackHelmRelease()
uninstallHelmRelease()
applyYaml(), getResourceYaml()
```

## Tauri Commands

All backend functions are exposed via Tauri commands:

```rust
k8s_get_clusters()
k8s_import_kubeconfig_path()
k8s_import_kubeconfig_content()
k8s_connect_cluster()
k8s_disconnect_cluster()
k8s_delete_cluster()
k8s_get_namespaces()
k8s_set_namespace()
k8s_get_pods()
k8s_delete_pod()
k8s_restart_pod()
k8s_get_pod_logs()
k8s_stream_pod_logs()
k8s_exec_in_pod()
k8s_get_nodes()
k8s_get_deployments()
k8s_scale_deployment()
k8s_get_services()
k8s_get_configmaps()
k8s_get_secrets()
k8s_get_events()
k8s_watch_events()
k8s_create_port_forward()
k8s_get_port_forwards()
k8s_stop_port_forward()
k8s_delete_port_forward()
k8s_get_helm_releases()
k8s_helm_install()
k8s_helm_upgrade()
k8s_helm_rollback()
k8s_helm_uninstall()
k8s_list_helm_repos()
k8s_add_helm_repo()
k8s_search_helm_charts()
k8s_apply_yaml()
k8s_get_resource_yaml()
```

## Dependencies

Added to `core/Cargo.toml`:

```toml
[dependencies]
kube = { version = "0.90", features = ["client", "rustls-tls"], optional = true }
k8s-openapi = { version = "0.22", features = ["latest"], optional = true }
yaml-rust = { version = "0.4", optional = true }
serde_yaml = { version = "0.9", optional = true }
futures = { version = "0.3", optional = true }
bytes = { version = "1.5", optional = true }
h2 = { version = "0.3", optional = true }

[features]
kubernetes = [
    "dep:kube",
    "dep:k8s-openapi",
    "dep:yaml-rust",
    "dep:serde_yaml",
    "dep:futures",
    "dep:bytes",
    "dep:h2"
]
```

## Usage

### Build with Kubernetes support:

```bash
cargo build --features kubernetes
cargo build --features "standard kubernetes"  # With all Standard features
```

### Frontend usage example:

```typescript
import { useK8sStore } from './stores/k8sStore';

function K8sComponent() {
  const {
    clusters,
    connectCluster,
    loadPods,
    pods
  } = useK8sStore();

  // Import kubeconfig
  const handleImport = async (path: string) => {
    await importKubeconfigFromPath(path);
  };

  // Connect to cluster
  const handleConnect = async (clusterId: string) => {
    await connectCluster(clusterId);
    await loadPods();
  };

  return (
    // Your UI
  );
}
```

## Testing

Run tests with:

```bash
cargo test --features kubernetes
```

## Security Considerations

1. Kubeconfig content can be stored encrypted in database
2. Secrets show key names only (not values)
3. Port forwards require explicit user action
4. All K8s API calls go through kube-rs with proper authentication

## Future Enhancements

1. Metrics integration (metrics-server/Prometheus)
2. Ingress controller management
3. Storage class and PVC management
4. RBAC visualization
5. Network policies
6. Pod disruption budgets
7. CronJobs and Jobs management
8. Custom resource definitions (CRDs)
9. Multi-cluster dashboard
10. K9s-style terminal UI

## References

- Inspired by: Lens (k8slens.dev), k9s
- Rust K8s client: kube-rs (https://kube.rs/)
- K8s API: k8s-openapi

## Implementation Notes

1. **FFI Layer**: Provides C-compatible bindings for maximum compatibility
2. **Tauri Integration**: Native commands for webview frontend
3. **Feature Flags**: All K8s code behind `kubernetes` feature flag
4. **Error Handling**: Comprehensive error types with user-friendly messages
5. **Async/Await**: Full async support with Tokio runtime
6. **Type Safety**: Strong typing between Rust and TypeScript

---

Implementation completed: 2026-03-31
