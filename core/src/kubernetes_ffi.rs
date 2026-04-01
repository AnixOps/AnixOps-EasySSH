use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_int};
use std::sync::Arc;
use serde_json;
use crate::kubernetes_client::{
    K8sCluster, K8sManager, K8sPod, K8sNamespace, K8sNode, K8sDeployment, K8sService,
    K8sConfigMap, K8sSecret, K8sEvent, K8sPortForward, LogOptions, ExecOptions,
    HelmRelease, HelmRepo, HelmChart, K8sResource,
};
use crate::AppState;

/// Opaque handle for K8sManager
pub struct K8sManagerHandle {
    manager: Arc<K8sManager>,
}

/// Create a new K8sManager instance
/// Returns a handle that must be freed with k8s_manager_free
#[no_mangle]
pub extern "C" fn k8s_manager_new() -> *mut K8sManagerHandle {
    let manager = Arc::new(K8sManager::new());
    Box::into_raw(Box::new(K8sManagerHandle { manager }))
}

/// Free a K8sManager handle
#[no_mangle]
pub extern "C" fn k8s_manager_free(handle: *mut K8sManagerHandle) {
    if !handle.is_null() {
        unsafe {
            let _ = Box::from_raw(handle);
        }
    }
}

/// Import kubeconfig from path
/// Returns JSON array of clusters on success, null on error
#[no_mangle]
pub extern "C" fn k8s_import_kubeconfig_path(
    handle: *mut K8sManagerHandle,
    path: *const c_char,
) -> *mut c_char {
    if handle.is_null() || path.is_null() {
        return std::ptr::null_mut();
    }

    let handle = unsafe { &*handle };
    let path_str = unsafe { CStr::from_ptr(path).to_string_lossy() };

    let runtime = match tokio::runtime::Runtime::new() {
        Ok(rt) => rt,
        Err(_) => return std::ptr::null_mut(),
    };

    let result = runtime.block_on(async {
        handle.manager.import_kubeconfig_from_path(&path_str).await
    });

    match result {
        Ok(clusters) => {
            match serde_json::to_string(&clusters) {
                Ok(json) => {
                    match CString::new(json) {
                        Ok(cstr) => cstr.into_raw(),
                        Err(_) => std::ptr::null_mut(),
                    }
                }
                Err(_) => std::ptr::null_mut(),
            }
        }
        Err(_) => std::ptr::null_mut(),
    }
}

/// Import kubeconfig from content
#[no_mangle]
pub extern "C" fn k8s_import_kubeconfig_content(
    handle: *mut K8sManagerHandle,
    content: *const c_char,
) -> *mut c_char {
    if handle.is_null() || content.is_null() {
        return std::ptr::null_mut();
    }

    let handle = unsafe { &*handle };
    let content_str = unsafe { CStr::from_ptr(content).to_string_lossy() };

    let runtime = match tokio::runtime::Runtime::new() {
        Ok(rt) => rt,
        Err(_) => return std::ptr::null_mut(),
    };

    let result = runtime.block_on(async {
        handle.manager.import_kubeconfig(&content_str, None).await
    });

    match result {
        Ok(clusters) => {
            match serde_json::to_string(&clusters) {
                Ok(json) => {
                    match CString::new(json) {
                        Ok(cstr) => cstr.into_raw(),
                        Err(_) => std::ptr::null_mut(),
                    }
                }
                Err(_) => std::ptr::null_mut(),
            }
        }
        Err(_) => std::ptr::null_mut(),
    }
}

/// Get all clusters as JSON
#[no_mangle]
pub extern "C" fn k8s_get_clusters(handle: *mut K8sManagerHandle) -> *mut c_char {
    if handle.is_null() {
        return std::ptr::null_mut();
    }

    let handle = unsafe { &*handle };

    let runtime = match tokio::runtime::Runtime::new() {
        Ok(rt) => rt,
        Err(_) => return std::ptr::null_mut(),
    };

    let clusters = runtime.block_on(async {
        handle.manager.get_clusters().await
    });

    match serde_json::to_string(&clusters) {
        Ok(json) => {
            match CString::new(json) {
                Ok(cstr) => cstr.into_raw(),
                Err(_) => std::ptr::null_mut(),
            }
        }
        Err(_) => std::ptr::null_mut(),
    }
}

/// Connect to cluster
#[no_mangle]
pub extern "C" fn k8s_connect_cluster(
    handle: *mut K8sManagerHandle,
    cluster_id: *const c_char,
) -> c_int {
    if handle.is_null() || cluster_id.is_null() {
        return -1;
    }

    let handle = unsafe { &*handle };
    let id = unsafe { CStr::from_ptr(cluster_id).to_string_lossy() };

    let runtime = match tokio::runtime::Runtime::new() {
        Ok(rt) => rt,
        Err(_) => return -1,
    };

    match runtime.block_on(async {
        handle.manager.connect_cluster(&id).await
    }) {
        Ok(_) => 0,
        Err(_) => -1,
    }
}

/// Disconnect from cluster
#[no_mangle]
pub extern "C" fn k8s_disconnect_cluster(
    handle: *mut K8sManagerHandle,
    cluster_id: *const c_char,
) -> c_int {
    if handle.is_null() || cluster_id.is_null() {
        return -1;
    }

    let handle = unsafe { &*handle };
    let id = unsafe { CStr::from_ptr(cluster_id).to_string_lossy() };

    let runtime = match tokio::runtime::Runtime::new() {
        Ok(rt) => rt,
        Err(_) => return -1,
    };

    match runtime.block_on(async {
        handle.manager.disconnect_cluster(&id).await
    }) {
        Ok(_) => 0,
        Err(_) => -1,
    }
}

/// Delete cluster
#[no_mangle]
pub extern "C" fn k8s_delete_cluster(
    handle: *mut K8sManagerHandle,
    cluster_id: *const c_char,
) -> c_int {
    if handle.is_null() || cluster_id.is_null() {
        return -1;
    }

    let handle = unsafe { &*handle };
    let id = unsafe { CStr::from_ptr(cluster_id).to_string_lossy() };

    let runtime = match tokio::runtime::Runtime::new() {
        Ok(rt) => rt,
        Err(_) => return -1,
    };

    match runtime.block_on(async {
        handle.manager.delete_cluster(&id).await
    }) {
        Ok(_) => 0,
        Err(_) => -1,
    }
}

/// Get namespaces
#[no_mangle]
pub extern "C" fn k8s_get_namespaces(
    handle: *mut K8sManagerHandle,
    cluster_id: *const c_char,
) -> *mut c_char {
    if handle.is_null() || cluster_id.is_null() {
        return std::ptr::null_mut();
    }

    let handle = unsafe { &*handle };
    let id = unsafe { CStr::from_ptr(cluster_id).to_string_lossy() };

    let runtime = match tokio::runtime::Runtime::new() {
        Ok(rt) => rt,
        Err(_) => return std::ptr::null_mut(),
    };

    let result = runtime.block_on(async {
        handle.manager.get_namespaces(&id).await
    });

    match result {
        Ok(namespaces) => {
            match serde_json::to_string(&namespaces) {
                Ok(json) => {
                    match CString::new(json) {
                        Ok(cstr) => cstr.into_raw(),
                        Err(_) => std::ptr::null_mut(),
                    }
                }
                Err(_) => std::ptr::null_mut(),
            }
        }
        Err(_) => std::ptr::null_mut(),
    }
}

/// Set current namespace
#[no_mangle]
pub extern "C" fn k8s_set_namespace(
    handle: *mut K8sManagerHandle,
    cluster_id: *const c_char,
    namespace: *const c_char,
) -> c_int {
    if handle.is_null() || cluster_id.is_null() || namespace.is_null() {
        return -1;
    }

    let handle = unsafe { &*handle };
    let id = unsafe { CStr::from_ptr(cluster_id).to_string_lossy() };
    let ns = unsafe { CStr::from_ptr(namespace).to_string_lossy() };

    let runtime = match tokio::runtime::Runtime::new() {
        Ok(rt) => rt,
        Err(_) => return -1,
    };

    match runtime.block_on(async {
        handle.manager.set_namespace(&id, &ns).await
    }) {
        Ok(_) => 0,
        Err(_) => -1,
    }
}

/// Get pods
#[no_mangle]
pub extern "C" fn k8s_get_pods(
    handle: *mut K8sManagerHandle,
    cluster_id: *const c_char,
    namespace: *const c_char,
    label_selector: *const c_char,
) -> *mut c_char {
    if handle.is_null() || cluster_id.is_null() || namespace.is_null() {
        return std::ptr::null_mut();
    }

    let handle = unsafe { &*handle };
    let id = unsafe { CStr::from_ptr(cluster_id).to_string_lossy() };
    let ns = unsafe { CStr::from_ptr(namespace).to_string_lossy() };
    let selector = if label_selector.is_null() {
        None
    } else {
        Some(unsafe { CStr::from_ptr(label_selector).to_string_lossy().to_string() })
    };

    let runtime = match tokio::runtime::Runtime::new() {
        Ok(rt) => rt,
        Err(_) => return std::ptr::null_mut(),
    };

    let result = runtime.block_on(async {
        handle.manager.get_pods(&id, &ns, selector.as_deref()).await
    });

    match result {
        Ok(pods) => {
            match serde_json::to_string(&pods) {
                Ok(json) => {
                    match CString::new(json) {
                        Ok(cstr) => cstr.into_raw(),
                        Err(_) => std::ptr::null_mut(),
                    }
                }
                Err(_) => std::ptr::null_mut(),
            }
        }
        Err(_) => std::ptr::null_mut(),
    }
}

/// Get pod details
#[no_mangle]
pub extern "C" fn k8s_get_pod(
    handle: *mut K8sManagerHandle,
    cluster_id: *const c_char,
    namespace: *const c_char,
    pod_name: *const c_char,
) -> *mut c_char {
    if handle.is_null() || cluster_id.is_null() || namespace.is_null() || pod_name.is_null() {
        return std::ptr::null_mut();
    }

    let handle = unsafe { &*handle };
    let id = unsafe { CStr::from_ptr(cluster_id).to_string_lossy() };
    let ns = unsafe { CStr::from_ptr(namespace).to_string_lossy() };
    let name = unsafe { CStr::from_ptr(pod_name).to_string_lossy() };

    let runtime = match tokio::runtime::Runtime::new() {
        Ok(rt) => rt,
        Err(_) => return std::ptr::null_mut(),
    };

    let result = runtime.block_on(async {
        handle.manager.get_pod(&id, &ns, &name).await
    });

    match result {
        Ok(pod) => {
            match serde_json::to_string(&pod) {
                Ok(json) => {
                    match CString::new(json) {
                        Ok(cstr) => cstr.into_raw(),
                        Err(_) => std::ptr::null_mut(),
                    }
                }
                Err(_) => std::ptr::null_mut(),
            }
        }
        Err(_) => std::ptr::null_mut(),
    }
}

/// Delete pod
#[no_mangle]
pub extern "C" fn k8s_delete_pod(
    handle: *mut K8sManagerHandle,
    cluster_id: *const c_char,
    namespace: *const c_char,
    pod_name: *const c_char,
) -> c_int {
    if handle.is_null() || cluster_id.is_null() || namespace.is_null() || pod_name.is_null() {
        return -1;
    }

    let handle = unsafe { &*handle };
    let id = unsafe { CStr::from_ptr(cluster_id).to_string_lossy() };
    let ns = unsafe { CStr::from_ptr(namespace).to_string_lossy() };
    let name = unsafe { CStr::from_ptr(pod_name).to_string_lossy() };

    let runtime = match tokio::runtime::Runtime::new() {
        Ok(rt) => rt,
        Err(_) => return -1,
    };

    match runtime.block_on(async {
        handle.manager.delete_pod(&id, &ns, &name).await
    }) {
        Ok(_) => 0,
        Err(_) => -1,
    }
}

/// Restart pod
#[no_mangle]
pub extern "C" fn k8s_restart_pod(
    handle: *mut K8sManagerHandle,
    cluster_id: *const c_char,
    namespace: *const c_char,
    pod_name: *const c_char,
) -> c_int {
    if handle.is_null() || cluster_id.is_null() || namespace.is_null() || pod_name.is_null() {
        return -1;
    }

    let handle = unsafe { &*handle };
    let id = unsafe { CStr::from_ptr(cluster_id).to_string_lossy() };
    let ns = unsafe { CStr::from_ptr(namespace).to_string_lossy() };
    let name = unsafe { CStr::from_ptr(pod_name).to_string_lossy() };

    let runtime = match tokio::runtime::Runtime::new() {
        Ok(rt) => rt,
        Err(_) => return -1,
    };

    match runtime.block_on(async {
        handle.manager.restart_pod(&id, &ns, &name).await
    }) {
        Ok(_) => 0,
        Err(_) => -1,
    }
}

/// Get pod logs
#[no_mangle]
pub extern "C" fn k8s_get_pod_logs(
    handle: *mut K8sManagerHandle,
    cluster_id: *const c_char,
    namespace: *const c_char,
    pod_name: *const c_char,
    container: *const c_char,
    tail_lines: c_int,
    follow: c_int,
) -> *mut c_char {
    if handle.is_null() || cluster_id.is_null() || namespace.is_null() || pod_name.is_null() {
        return std::ptr::null_mut();
    }

    let handle = unsafe { &*handle };
    let id = unsafe { CStr::from_ptr(cluster_id).to_string_lossy() };
    let ns = unsafe { CStr::from_ptr(namespace).to_string_lossy() };
    let name = unsafe { CStr::from_ptr(pod_name).to_string_lossy() };
    let container_name = if container.is_null() {
        None
    } else {
        Some(unsafe { CStr::from_ptr(container).to_string_lossy().to_string() })
    };

    let options = LogOptions {
        follow: follow != 0,
        tail_lines: if tail_lines > 0 { Some(tail_lines as i64) } else { None },
        since_seconds: None,
        timestamps: false,
        previous: false,
        container: container_name,
    };

    let runtime = match tokio::runtime::Runtime::new() {
        Ok(rt) => rt,
        Err(_) => return std::ptr::null_mut(),
    };

    let result = runtime.block_on(async {
        handle.manager.get_pod_logs(&id, &ns, &name, &options).await
    });

    match result {
        Ok(logs) => {
            match CString::new(logs) {
                Ok(cstr) => cstr.into_raw(),
                Err(_) => std::ptr::null_mut(),
            }
        }
        Err(_) => std::ptr::null_mut(),
    }
}

/// Exec command in pod
#[no_mangle]
pub extern "C" fn k8s_exec_in_pod(
    handle: *mut K8sManagerHandle,
    cluster_id: *const c_char,
    namespace: *const c_char,
    pod_name: *const c_char,
    command: *const c_char,
    container: *const c_char,
) -> *mut c_char {
    if handle.is_null() || cluster_id.is_null() || namespace.is_null() || pod_name.is_null() || command.is_null() {
        return std::ptr::null_mut();
    }

    let handle = unsafe { &*handle };
    let id = unsafe { CStr::from_ptr(cluster_id).to_string_lossy() };
    let ns = unsafe { CStr::from_ptr(namespace).to_string_lossy() };
    let name = unsafe { CStr::from_ptr(pod_name).to_string_lossy() };
    let cmd = unsafe { CStr::from_ptr(command).to_string_lossy() };
    let container_name = if container.is_null() {
        None
    } else {
        Some(unsafe { CStr::from_ptr(container).to_string_lossy().to_string() })
    };

    let options = ExecOptions {
        container: container_name,
        stdin: false,
        tty: false,
        command: cmd.split_whitespace().map(|s| s.to_string()).collect(),
    };

    let runtime = match tokio::runtime::Runtime::new() {
        Ok(rt) => rt,
        Err(_) => return std::ptr::null_mut(),
    };

    let result = runtime.block_on(async {
        handle.manager.exec_in_pod(&id, &ns, &name, &options).await
    });

    match result {
        Ok(output) => {
            match CString::new(output) {
                Ok(cstr) => cstr.into_raw(),
                Err(_) => std::ptr::null_mut(),
            }
        }
        Err(_) => std::ptr::null_mut(),
    }
}

/// Get nodes
#[no_mangle]
pub extern "C" fn k8s_get_nodes(
    handle: *mut K8sManagerHandle,
    cluster_id: *const c_char,
) -> *mut c_char {
    if handle.is_null() || cluster_id.is_null() {
        return std::ptr::null_mut();
    }

    let handle = unsafe { &*handle };
    let id = unsafe { CStr::from_ptr(cluster_id).to_string_lossy() };

    let runtime = match tokio::runtime::Runtime::new() {
        Ok(rt) => rt,
        Err(_) => return std::ptr::null_mut(),
    };

    let result = runtime.block_on(async {
        handle.manager.get_nodes(&id).await
    });

    match result {
        Ok(nodes) => {
            match serde_json::to_string(&nodes) {
                Ok(json) => {
                    match CString::new(json) {
                        Ok(cstr) => cstr.into_raw(),
                        Err(_) => std::ptr::null_mut(),
                    }
                }
                Err(_) => std::ptr::null_mut(),
            }
        }
        Err(_) => std::ptr::null_mut(),
    }
}

/// Get deployments
#[no_mangle]
pub extern "C" fn k8s_get_deployments(
    handle: *mut K8sManagerHandle,
    cluster_id: *const c_char,
    namespace: *const c_char,
) -> *mut c_char {
    if handle.is_null() || cluster_id.is_null() || namespace.is_null() {
        return std::ptr::null_mut();
    }

    let handle = unsafe { &*handle };
    let id = unsafe { CStr::from_ptr(cluster_id).to_string_lossy() };
    let ns = unsafe { CStr::from_ptr(namespace).to_string_lossy() };

    let runtime = match tokio::runtime::Runtime::new() {
        Ok(rt) => rt,
        Err(_) => return std::ptr::null_mut(),
    };

    let result = runtime.block_on(async {
        handle.manager.get_deployments(&id, &ns).await
    });

    match result {
        Ok(deployments) => {
            match serde_json::to_string(&deployments) {
                Ok(json) => {
                    match CString::new(json) {
                        Ok(cstr) => cstr.into_raw(),
                        Err(_) => std::ptr::null_mut(),
                    }
                }
                Err(_) => std::ptr::null_mut(),
            }
        }
        Err(_) => std::ptr::null_mut(),
    }
}

/// Scale deployment
#[no_mangle]
pub extern "C" fn k8s_scale_deployment(
    handle: *mut K8sManagerHandle,
    cluster_id: *const c_char,
    namespace: *const c_char,
    deployment_name: *const c_char,
    replicas: c_int,
) -> c_int {
    if handle.is_null() || cluster_id.is_null() || namespace.is_null() || deployment_name.is_null() {
        return -1;
    }

    let handle = unsafe { &*handle };
    let id = unsafe { CStr::from_ptr(cluster_id).to_string_lossy() };
    let ns = unsafe { CStr::from_ptr(namespace).to_string_lossy() };
    let name = unsafe { CStr::from_ptr(deployment_name).to_string_lossy() };

    let runtime = match tokio::runtime::Runtime::new() {
        Ok(rt) => rt,
        Err(_) => return -1,
    };

    match runtime.block_on(async {
        handle.manager.scale_deployment(&id, &ns, &name, replicas as i32).await
    }) {
        Ok(_) => 0,
        Err(_) => -1,
    }
}

/// Get services
#[no_mangle]
pub extern "C" fn k8s_get_services(
    handle: *mut K8sManagerHandle,
    cluster_id: *const c_char,
    namespace: *const c_char,
) -> *mut c_char {
    if handle.is_null() || cluster_id.is_null() || namespace.is_null() {
        return std::ptr::null_mut();
    }

    let handle = unsafe { &*handle };
    let id = unsafe { CStr::from_ptr(cluster_id).to_string_lossy() };
    let ns = unsafe { CStr::from_ptr(namespace).to_string_lossy() };

    let runtime = match tokio::runtime::Runtime::new() {
        Ok(rt) => rt,
        Err(_) => return std::ptr::null_mut(),
    };

    let result = runtime.block_on(async {
        handle.manager.get_services(&id, &ns).await
    });

    match result {
        Ok(services) => {
            match serde_json::to_string(&services) {
                Ok(json) => {
                    match CString::new(json) {
                        Ok(cstr) => cstr.into_raw(),
                        Err(_) => std::ptr::null_mut(),
                    }
                }
                Err(_) => std::ptr::null_mut(),
            }
        }
        Err(_) => std::ptr::null_mut(),
    }
}

/// Get ConfigMaps
#[no_mangle]
pub extern "C" fn k8s_get_configmaps(
    handle: *mut K8sManagerHandle,
    cluster_id: *const c_char,
    namespace: *const c_char,
) -> *mut c_char {
    if handle.is_null() || cluster_id.is_null() || namespace.is_null() {
        return std::ptr::null_mut();
    }

    let handle = unsafe { &*handle };
    let id = unsafe { CStr::from_ptr(cluster_id).to_string_lossy() };
    let ns = unsafe { CStr::from_ptr(namespace).to_string_lossy() };

    let runtime = match tokio::runtime::Runtime::new() {
        Ok(rt) => rt,
        Err(_) => return std::ptr::null_mut(),
    };

    let result = runtime.block_on(async {
        handle.manager.get_configmaps(&id, &ns).await
    });

    match result {
        Ok(configmaps) => {
            match serde_json::to_string(&configmaps) {
                Ok(json) => {
                    match CString::new(json) {
                        Ok(cstr) => cstr.into_raw(),
                        Err(_) => std::ptr::null_mut(),
                    }
                }
                Err(_) => std::ptr::null_mut(),
            }
        }
        Err(_) => std::ptr::null_mut(),
    }
}

/// Get Secrets
#[no_mangle]
pub extern "C" fn k8s_get_secrets(
    handle: *mut K8sManagerHandle,
    cluster_id: *const c_char,
    namespace: *const c_char,
) -> *mut c_char {
    if handle.is_null() || cluster_id.is_null() || namespace.is_null() {
        return std::ptr::null_mut();
    }

    let handle = unsafe { &*handle };
    let id = unsafe { CStr::from_ptr(cluster_id).to_string_lossy() };
    let ns = unsafe { CStr::from_ptr(namespace).to_string_lossy() };

    let runtime = match tokio::runtime::Runtime::new() {
        Ok(rt) => rt,
        Err(_) => return std::ptr::null_mut(),
    };

    let result = runtime.block_on(async {
        handle.manager.get_secrets(&id, &ns).await
    });

    match result {
        Ok(secrets) => {
            match serde_json::to_string(&secrets) {
                Ok(json) => {
                    match CString::new(json) {
                        Ok(cstr) => cstr.into_raw(),
                        Err(_) => std::ptr::null_mut(),
                    }
                }
                Err(_) => std::ptr::null_mut(),
            }
        }
        Err(_) => std::ptr::null_mut(),
    }
}

/// Get events
#[no_mangle]
pub extern "C" fn k8s_get_events(
    handle: *mut K8sManagerHandle,
    cluster_id: *const c_char,
    namespace: *const c_char,
) -> *mut c_char {
    if handle.is_null() || cluster_id.is_null() {
        return std::ptr::null_mut();
    }

    let handle = unsafe { &*handle };
    let id = unsafe { CStr::from_ptr(cluster_id).to_string_lossy() };
    let ns = if namespace.is_null() {
        None
    } else {
        Some(unsafe { CStr::from_ptr(namespace).to_string_lossy().to_string() })
    };

    let runtime = match tokio::runtime::Runtime::new() {
        Ok(rt) => rt,
        Err(_) => return std::ptr::null_mut(),
    };

    let result = runtime.block_on(async {
        handle.manager.get_events(&id, ns.as_deref(), None, None).await
    });

    match result {
        Ok(events) => {
            match serde_json::to_string(&events) {
                Ok(json) => {
                    match CString::new(json) {
                        Ok(cstr) => cstr.into_raw(),
                        Err(_) => std::ptr::null_mut(),
                    }
                }
                Err(_) => std::ptr::null_mut(),
            }
        }
        Err(_) => std::ptr::null_mut(),
    }
}

/// Create port forward
#[no_mangle]
pub extern "C" fn k8s_create_port_forward(
    handle: *mut K8sManagerHandle,
    cluster_id: *const c_char,
    namespace: *const c_char,
    pod_name: *const c_char,
    local_port: c_int,
    remote_port: c_int,
) -> *mut c_char {
    if handle.is_null() || cluster_id.is_null() || namespace.is_null() || pod_name.is_null() {
        return std::ptr::null_mut();
    }

    let handle = unsafe { &*handle };
    let id = unsafe { CStr::from_ptr(cluster_id).to_string_lossy() };
    let ns = unsafe { CStr::from_ptr(namespace).to_string_lossy() };
    let name = unsafe { CStr::from_ptr(pod_name).to_string_lossy() };

    let runtime = match tokio::runtime::Runtime::new() {
        Ok(rt) => rt,
        Err(_) => return std::ptr::null_mut(),
    };

    let result = runtime.block_on(async {
        handle.manager.create_port_forward(&id, &ns, &name, local_port as u16, remote_port as u16).await
    });

    match result {
        Ok(pf) => {
            match serde_json::to_string(&pf) {
                Ok(json) => {
                    match CString::new(json) {
                        Ok(cstr) => cstr.into_raw(),
                        Err(_) => std::ptr::null_mut(),
                    }
                }
                Err(_) => std::ptr::null_mut(),
            }
        }
        Err(_) => std::ptr::null_mut(),
    }
}

/// Get port forwards
#[no_mangle]
pub extern "C" fn k8s_get_port_forwards(
    handle: *mut K8sManagerHandle,
) -> *mut c_char {
    if handle.is_null() {
        return std::ptr::null_mut();
    }

    let handle = unsafe { &*handle };

    let runtime = match tokio::runtime::Runtime::new() {
        Ok(rt) => rt,
        Err(_) => return std::ptr::null_mut(),
    };

    let forwards = runtime.block_on(async {
        handle.manager.get_port_forwards().await
    });

    match serde_json::to_string(&forwards) {
        Ok(json) => {
            match CString::new(json) {
                Ok(cstr) => cstr.into_raw(),
                Err(_) => std::ptr::null_mut(),
            }
        }
        Err(_) => std::ptr::null_mut(),
    }
}

/// Stop port forward
#[no_mangle]
pub extern "C" fn k8s_stop_port_forward(
    handle: *mut K8sManagerHandle,
    forward_id: *const c_char,
) -> c_int {
    if handle.is_null() || forward_id.is_null() {
        return -1;
    }

    let handle = unsafe { &*handle };
    let id = unsafe { CStr::from_ptr(forward_id).to_string_lossy() };

    let runtime = match tokio::runtime::Runtime::new() {
        Ok(rt) => rt,
        Err(_) => return -1,
    };

    match runtime.block_on(async {
        handle.manager.stop_port_forward(&id).await
    }) {
        Ok(_) => 0,
        Err(_) => -1,
    }
}

/// Delete port forward
#[no_mangle]
pub extern "C" fn k8s_delete_port_forward(
    handle: *mut K8sManagerHandle,
    forward_id: *const c_char,
) -> c_int {
    if handle.is_null() || forward_id.is_null() {
        return -1;
    }

    let handle = unsafe { &*handle };
    let id = unsafe { CStr::from_ptr(forward_id).to_string_lossy() };

    let runtime = match tokio::runtime::Runtime::new() {
        Ok(rt) => rt,
        Err(_) => return -1,
    };

    match runtime.block_on(async {
        handle.manager.delete_port_forward(&id).await
    }) {
        Ok(_) => 0,
        Err(_) => -1,
    }
}

/// Apply YAML resource
#[no_mangle]
pub extern "C" fn k8s_apply_yaml(
    handle: *mut K8sManagerHandle,
    cluster_id: *const c_char,
    yaml: *const c_char,
    namespace: *const c_char,
) -> *mut c_char {
    if handle.is_null() || cluster_id.is_null() || yaml.is_null() {
        return std::ptr::null_mut();
    }

    let handle = unsafe { &*handle };
    let id = unsafe { CStr::from_ptr(cluster_id).to_string_lossy() };
    let yaml_str = unsafe { CStr::from_ptr(yaml).to_string_lossy() };
    let ns = if namespace.is_null() {
        None
    } else {
        Some(unsafe { CStr::from_ptr(namespace).to_string_lossy().to_string() })
    };

    let runtime = match tokio::runtime::Runtime::new() {
        Ok(rt) => rt,
        Err(_) => return std::ptr::null_mut(),
    };

    let result = runtime.block_on(async {
        handle.manager.apply_yaml(&id, &yaml_str, ns.as_deref()).await
    });

    match result {
        Ok(resource) => {
            match serde_json::to_string(&resource) {
                Ok(json) => {
                    match CString::new(json) {
                        Ok(cstr) => cstr.into_raw(),
                        Err(_) => std::ptr::null_mut(),
                    }
                }
                Err(_) => std::ptr::null_mut(),
            }
        }
        Err(_) => std::ptr::null_mut(),
    }
}

/// Get resource YAML
#[no_mangle]
pub extern "C" fn k8s_get_resource_yaml(
    handle: *mut K8sManagerHandle,
    cluster_id: *const c_char,
    kind: *const c_char,
    name: *const c_char,
    namespace: *const c_char,
) -> *mut c_char {
    if handle.is_null() || cluster_id.is_null() || kind.is_null() || name.is_null() || namespace.is_null() {
        return std::ptr::null_mut();
    }

    let handle = unsafe { &*handle };
    let id = unsafe { CStr::from_ptr(cluster_id).to_string_lossy() };
    let kind_str = unsafe { CStr::from_ptr(kind).to_string_lossy() };
    let name_str = unsafe { CStr::from_ptr(name).to_string_lossy() };
    let ns = unsafe { CStr::from_ptr(namespace).to_string_lossy() };

    let runtime = match tokio::runtime::Runtime::new() {
        Ok(rt) => rt,
        Err(_) => return std::ptr::null_mut(),
    };

    let result = runtime.block_on(async {
        handle.manager.get_resource_yaml(&id, &kind_str, &name_str, &ns).await
    });

    match result {
        Ok(yaml) => {
            match CString::new(yaml) {
                Ok(cstr) => cstr.into_raw(),
                Err(_) => std::ptr::null_mut(),
            }
        }
        Err(_) => std::ptr::null_mut(),
    }
}

/// Get Helm releases
#[no_mangle]
pub extern "C" fn k8s_get_helm_releases(
    handle: *mut K8sManagerHandle,
    cluster_id: *const c_char,
    namespace: *const c_char,
) -> *mut c_char {
    if handle.is_null() || cluster_id.is_null() {
        return std::ptr::null_mut();
    }

    let handle = unsafe { &*handle };
    let id = unsafe { CStr::from_ptr(cluster_id).to_string_lossy() };
    let ns = if namespace.is_null() {
        None
    } else {
        Some(unsafe { CStr::from_ptr(namespace).to_string_lossy().to_string() })
    };

    let runtime = match tokio::runtime::Runtime::new() {
        Ok(rt) => rt,
        Err(_) => return std::ptr::null_mut(),
    };

    let result = runtime.block_on(async {
        handle.manager.get_helm_releases(&id, ns.as_deref()).await
    });

    match result {
        Ok(releases) => {
            match serde_json::to_string(&releases) {
                Ok(json) => {
                    match CString::new(json) {
                        Ok(cstr) => cstr.into_raw(),
                        Err(_) => std::ptr::null_mut(),
                    }
                }
                Err(_) => std::ptr::null_mut(),
            }
        }
        Err(_) => std::ptr::null_mut(),
    }
}

/// Install Helm chart
#[no_mangle]
pub extern "C" fn k8s_helm_install(
    handle: *mut K8sManagerHandle,
    cluster_id: *const c_char,
    release_name: *const c_char,
    chart: *const c_char,
    namespace: *const c_char,
    version: *const c_char,
) -> *mut c_char {
    if handle.is_null() || cluster_id.is_null() || release_name.is_null() || chart.is_null() || namespace.is_null() {
        return std::ptr::null_mut();
    }

    let handle = unsafe { &*handle };
    let id = unsafe { CStr::from_ptr(cluster_id).to_string_lossy() };
    let rel_name = unsafe { CStr::from_ptr(release_name).to_string_lossy() };
    let chart_str = unsafe { CStr::from_ptr(chart).to_string_lossy() };
    let ns = unsafe { CStr::from_ptr(namespace).to_string_lossy() };
    let ver = if version.is_null() {
        None
    } else {
        Some(unsafe { CStr::from_ptr(version).to_string_lossy().to_string() })
    };

    let runtime = match tokio::runtime::Runtime::new() {
        Ok(rt) => rt,
        Err(_) => return std::ptr::null_mut(),
    };

    let result = runtime.block_on(async {
        handle.manager.helm_install(&id, &rel_name, &chart_str, &ns, None, ver.as_deref(), None).await
    });

    match result {
        Ok(release) => {
            match serde_json::to_string(&release) {
                Ok(json) => {
                    match CString::new(json) {
                        Ok(cstr) => cstr.into_raw(),
                        Err(_) => std::ptr::null_mut(),
                    }
                }
                Err(_) => std::ptr::null_mut(),
            }
        }
        Err(_) => std::ptr::null_mut(),
    }
}

/// Upgrade Helm release
#[no_mangle]
pub extern "C" fn k8s_helm_upgrade(
    handle: *mut K8sManagerHandle,
    cluster_id: *const c_char,
    release_name: *const c_char,
    chart: *const c_char,
    namespace: *const c_char,
    version: *const c_char,
) -> *mut c_char {
    if handle.is_null() || cluster_id.is_null() || release_name.is_null() || chart.is_null() || namespace.is_null() {
        return std::ptr::null_mut();
    }

    let handle = unsafe { &*handle };
    let id = unsafe { CStr::from_ptr(cluster_id).to_string_lossy() };
    let rel_name = unsafe { CStr::from_ptr(release_name).to_string_lossy() };
    let chart_str = unsafe { CStr::from_ptr(chart).to_string_lossy() };
    let ns = unsafe { CStr::from_ptr(namespace).to_string_lossy() };
    let ver = if version.is_null() {
        None
    } else {
        Some(unsafe { CStr::from_ptr(version).to_string_lossy().to_string() })
    };

    let runtime = match tokio::runtime::Runtime::new() {
        Ok(rt) => rt,
        Err(_) => return std::ptr::null_mut(),
    };

    let result = runtime.block_on(async {
        handle.manager.helm_upgrade(&id, &rel_name, &chart_str, &ns, None, ver.as_deref()).await
    });

    match result {
        Ok(release) => {
            match serde_json::to_string(&release) {
                Ok(json) => {
                    match CString::new(json) {
                        Ok(cstr) => cstr.into_raw(),
                        Err(_) => std::ptr::null_mut(),
                    }
                }
                Err(_) => std::ptr::null_mut(),
            }
        }
        Err(_) => std::ptr::null_mut(),
    }
}

/// Rollback Helm release
#[no_mangle]
pub extern "C" fn k8s_helm_rollback(
    handle: *mut K8sManagerHandle,
    cluster_id: *const c_char,
    release_name: *const c_char,
    namespace: *const c_char,
    revision: c_int,
) -> *mut c_char {
    if handle.is_null() || cluster_id.is_null() || release_name.is_null() || namespace.is_null() {
        return std::ptr::null_mut();
    }

    let handle = unsafe { &*handle };
    let id = unsafe { CStr::from_ptr(cluster_id).to_string_lossy() };
    let rel_name = unsafe { CStr::from_ptr(release_name).to_string_lossy() };
    let ns = unsafe { CStr::from_ptr(namespace).to_string_lossy() };

    let runtime = match tokio::runtime::Runtime::new() {
        Ok(rt) => rt,
        Err(_) => return std::ptr::null_mut(),
    };

    let result = runtime.block_on(async {
        handle.manager.helm_rollback(&id, &rel_name, &ns, revision).await
    });

    match result {
        Ok(release) => {
            match serde_json::to_string(&release) {
                Ok(json) => {
                    match CString::new(json) {
                        Ok(cstr) => cstr.into_raw(),
                        Err(_) => std::ptr::null_mut(),
                    }
                }
                Err(_) => std::ptr::null_mut(),
            }
        }
        Err(_) => std::ptr::null_mut(),
    }
}

/// Uninstall Helm release
#[no_mangle]
pub extern "C" fn k8s_helm_uninstall(
    handle: *mut K8sManagerHandle,
    cluster_id: *const c_char,
    release_name: *const c_char,
    namespace: *const c_char,
) -> c_int {
    if handle.is_null() || cluster_id.is_null() || release_name.is_null() || namespace.is_null() {
        return -1;
    }

    let handle = unsafe { &*handle };
    let id = unsafe { CStr::from_ptr(cluster_id).to_string_lossy() };
    let rel_name = unsafe { CStr::from_ptr(release_name).to_string_lossy() };
    let ns = unsafe { CStr::from_ptr(namespace).to_string_lossy() };

    let runtime = match tokio::runtime::Runtime::new() {
        Ok(rt) => rt,
        Err(_) => return -1,
    };

    match runtime.block_on(async {
        handle.manager.helm_uninstall(&id, &rel_name, &ns).await
    }) {
        Ok(_) => 0,
        Err(_) => -1,
    }
}

/// Free a string returned by K8s FFI
#[no_mangle]
pub extern "C" fn k8s_free_string(s: *mut c_char) {
    if !s.is_null() {
        unsafe {
            let _ = CString::from_raw(s);
        }
    }
}

/// Get K8sManager from AppState
pub fn get_k8s_manager(state: &AppState) -> Option<Arc<tokio::sync::RwLock<crate::kubernetes::K8sManager>>> {
    #[cfg(feature = "kubernetes")]
    {
        Some(state.k8s_manager.clone())
    }
    #[cfg(not(feature = "kubernetes"))]
    {
        None
    }
}
