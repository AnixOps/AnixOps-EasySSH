//! Docker containers - Container operations

use crate::error::LiteError;
use crate::ssh::SshSessionManager;
use std::collections::HashMap;
use std::io::Read;
use std::time::Duration;
use tokio::sync::mpsc;

use super::client::DockerManager;
use super::types::{ContainerInfo, ContainerStats, ContainerStatus, PortMapping};

impl DockerManager {
    /// List containers
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
                // Fallback to simple format
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
                        ports: self.parse_ports(parts.get(4).map(|s| *s)),
                        labels: HashMap::new(),
                        size_rw: None,
                        size_root_fs: None,
                        host_config: super::types::HostConfig {
                            network_mode: String::new(),
                            cpu_shares: None,
                            memory: None,
                            memory_swap: None,
                            cpu_percent: None,
                            cpu_quota: None,
                            cpu_period: None,
                        },
                        network_settings: super::types::NetworkSettings {
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
            Err(LiteError::Docker(format!(
                "Failed to start container: {}",
                output
            )))
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
        let cmd = format!("docker stop{} {}", timeout_flag, container_id);
        let output = ssh_manager.execute_via_sftp(ssh_session_id, &cmd).await?;

        if output.trim() == container_id || output.trim() == container_id[..12].to_string() {
            Ok(())
        } else {
            Err(LiteError::Docker(format!(
                "Failed to stop container: {}",
                output
            )))
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
            Err(LiteError::Docker(format!(
                "Failed to restart container: {}",
                output
            )))
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
            Err(LiteError::Docker(format!(
                "Failed to pause container: {}",
                output
            )))
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
            Err(LiteError::Docker(format!(
                "Failed to unpause container: {}",
                output
            )))
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
            Err(LiteError::Docker(format!(
                "Failed to kill container: {}",
                output
            )))
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
            Err(LiteError::Docker(format!(
                "Failed to remove container: {}",
                output
            )))
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
        ports: &[(u16, u16, &str)],
        volumes: &[(&str, &str)],
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
            "docker create{}{}{}{}{}{}{} {}",
            name_flag, ports_flags, volumes_flags, env_flags, network_flag, restart_flag, labels_flags, image
        );

        let output = ssh_manager.execute_via_sftp(ssh_session_id, &cmd).await?;
        let container_id = output.trim();

        if container_id.len() == 64 && container_id.chars().all(|c| c.is_ascii_hexdigit()) {
            Ok(container_id.to_string())
        } else {
            Err(LiteError::Docker(format!(
                "Failed to create container: {}",
                output
            )))
        }
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
                    return self.parse_container_inspect_json(first.clone());
                }
            }
        }

        Err(LiteError::Docker(format!(
            "Failed to inspect container: {}",
            output
        )))
    }

    /// Get container stats
    pub async fn get_container_stats(
        &self,
        ssh_manager: &SshSessionManager,
        ssh_session_id: &str,
        container_id: &str,
    ) -> Result<ContainerStats, LiteError> {
        let cmd = format!("docker stats {} --no-stream --format '{{{{json .}}}}'", container_id);
        let output = ssh_manager.execute_via_sftp(ssh_session_id, &cmd).await?;

        if let Ok(stats) = serde_json::from_str::<ContainerStats>(&output) {
            Ok(stats)
        } else {
            Err(LiteError::Docker(format!(
                "Failed to get container stats: {}",
                output
            )))
        }
    }

    /// Stream container logs
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

        let session_arc = ssh_manager
            .get_sftp_session_arc(ssh_session_id)
            .ok_or_else(|| crate::error::LiteError::SshSessionNotFound(ssh_session_id.to_string()))?;

        let container_id = container_id.to_string();
        let session_id = ssh_session_id.to_string();

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
            })
            .await;

            log::info!("Log stream ended for container {}", container_id);
        });

        let mut active_logs = self.active_logs.write().await;
        active_logs.insert(format!("{}_{}", session_id, container_id), handle);

        let mut log_channels = self.log_channels.write().await;
        log_channels.insert(format!("{}_{}", session_id, container_id), tx_for_insert);

        Ok(rx)
    }

    /// Stop log stream
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

    /// Parse container inspect JSON
    fn parse_container_inspect_json(&self, value: serde_json::Value) -> Result<ContainerInfo, LiteError> {
        // Extract basic info from inspect output
        let config = value.get("Config").and_then(|v| v.as_object());
        let host_config = value.get("HostConfig").and_then(|v| v.as_object());
        let network_settings = value.get("NetworkSettings").and_then(|v| v.as_object());

        Ok(ContainerInfo {
            id: value.get("Id").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            names: vec![value
                .get("Name")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .trim_start_matches('/')
                .to_string()],
            image: config
                .and_then(|c| c.get("Image"))
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            image_id: value
                .get("Image")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            command: config
                .and_then(|c| c.get("Cmd"))
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str())
                        .collect::<Vec<_>>()
                        .join(" ")
                })
                .unwrap_or_default(),
            created: 0,
            status: ContainerStatus::Created,
            state: value
                .get("State")
                .and_then(|v| v.get("Status"))
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            ports: Vec::new(),
            labels: config
                .and_then(|c| c.get("Labels"))
                .and_then(|v| v.as_object())
                .map(|obj| {
                    obj.iter()
                        .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
                        .collect()
                })
                .unwrap_or_default(),
            size_rw: None,
            size_root_fs: None,
            host_config: super::types::HostConfig {
                network_mode: host_config
                    .and_then(|h| h.get("NetworkMode"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
                cpu_shares: host_config.and_then(|h| h.get("CpuShares")).and_then(|v| v.as_i64()),
                memory: host_config.and_then(|h| h.get("Memory")).and_then(|v| v.as_i64()),
                memory_swap: host_config.and_then(|h| h.get("MemorySwap")).and_then(|v| v.as_i64()),
                cpu_percent: None,
                cpu_quota: host_config.and_then(|h| h.get("CpuQuota")).and_then(|v| v.as_i64()),
                cpu_period: host_config.and_then(|h| h.get("CpuPeriod")).and_then(|v| v.as_i64()),
            },
            network_settings: super::types::NetworkSettings {
                networks: network_settings
                    .and_then(|n| n.get("Networks"))
                    .and_then(|v| v.as_object())
                    .map(|obj| {
                        obj.iter()
                            .filter_map(|(k, v)| {
                                v.as_object().map(|net| {
                                    (k.clone(), super::types::ContainerNetworkInfo {
                                        network_id: net
                                            .get("NetworkID")
                                            .and_then(|v| v.as_str())
                                            .unwrap_or("")
                                            .to_string(),
                                        endpoint_id: net
                                            .get("EndpointID")
                                            .and_then(|v| v.as_str())
                                            .unwrap_or("")
                                            .to_string(),
                                        gateway: net
                                            .get("Gateway")
                                            .and_then(|v| v.as_str())
                                            .unwrap_or("")
                                            .to_string(),
                                        ip_address: net
                                            .get("IPAddress")
                                            .and_then(|v| v.as_str())
                                            .unwrap_or("")
                                            .to_string(),
                                        ip_prefix_len: net
                                            .get("IPPrefixLen")
                                            .and_then(|v| v.as_i64())
                                            .unwrap_or(0) as i32,
                                        mac_address: net
                                            .get("MacAddress")
                                            .and_then(|v| v.as_str())
                                            .unwrap_or("")
                                            .to_string(),
                                    })
                                })
                            })
                            .collect()
                    })
                    .unwrap_or_default(),
                ip_address: network_settings
                    .and_then(|n| n.get("IPAddress"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
                gateway: network_settings
                    .and_then(|n| n.get("Gateway"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
                mac_address: network_settings
                    .and_then(|n| n.get("MacAddress"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
            },
            mounts: Vec::new(),
        })
    }
}
