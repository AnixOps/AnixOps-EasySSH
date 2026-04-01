# API 使用指南

> EasySSH Core Library 完整 API 参考

## 目录

1. [核心概念](#核心概念)
2. [应用状态管理](#应用状态管理)
3. [数据库操作](#数据库操作)
4. [SSH 连接](#ssh-连接)
5. [加密系统](#加密系统)
6. [服务器管理](#服务器管理)
7. [分组管理](#分组管理)
8. [Docker 管理](#docker-管理)
9. [Kubernetes 管理](#kubernetes-管理)
10. [监控功能](#监控功能)
11. [团队协作](#团队协作)
12. [审计日志](#审计日志)
13. [错误处理](#错误处理)

---

## 核心概念

### AppState

`AppState` 是 EasySSH 的核心状态容器，包含所有共享资源：

```rust
use easyssh_core::AppState;

// 创建默认状态
let state = AppState::new();

// 使用自定义 SSH 池配置
let state = AppState::with_ssh_pool_config(
    10,     // 最大连接数
    600,    // 空闲超时（秒）
    3600    // 最大连接年龄（秒）
);
```

### 功能特性系统

EasySSH 使用 Cargo feature flags 控制功能编译：

```rust
// Cargo.toml
[dependencies]
easyssh-core = {
    version = "0.3",
    features = ["standard", "sftp", "docker"]
}
```

---

## 应用状态管理

### 初始化数据库

```rust
use easyssh_core::{AppState, init_database, LiteError};

fn setup() -> Result<(), LiteError> {
    let state = AppState::new();

    // 初始化 SQLite 数据库
    init_database(&state)?;

    Ok(())
}
```

### 获取数据库路径

```rust
use easyssh_core::get_db_path;
use std::path::PathBuf;

let path: PathBuf = get_db_path();
println!("Database at: {:?}", path);
```

---

## 数据库操作

### 服务器记录

```rust
use easyssh_core::{
    NewServer, UpdateServer, ServerRecord,
    add_server, get_server, get_servers,
    update_server, delete_server
};

// 添加服务器
let new_server = NewServer {
    id: "server-001".to_string(),
    name: "Production Server".to_string(),
    host: "192.168.1.100".to_string(),
    port: 22,
    username: "admin".to_string(),
    auth_type: "agent".to_string(),
    group_id: None,
    identity_file: None,
    password_encrypted: None,
};
add_server(&state, &new_server)?;

// 查询服务器
let server = get_server(&state, "server-001")?;
let all_servers = get_servers(&state)?;

// 更新服务器
let update = UpdateServer {
    id: "server-001".to_string(),
    name: Some("Updated Name".to_string()),
    host: None,
    port: None,
    username: None,
    auth_type: None,
    group_id: None,
    identity_file: None,
    password_encrypted: None,
};
update_server(&state, &update)?;

// 删除服务器
delete_server(&state, "server-001")?;
```

### 分组记录

```rust
use easyssh_core::{
    NewGroup, UpdateGroup, GroupRecord,
    add_group, get_groups, update_group, delete_group
};

// 添加分组
let group = NewGroup {
    id: "group-001".to_string(),
    name: "Production".to_string(),
};
add_group(&state, &group)?;

// 查询分组
let groups = get_groups(&state)?;

// 更新分组
let update = UpdateGroup {
    id: "group-001".to_string(),
    name: Some("Production Servers".to_string()),
};
update_group(&state, &update)?;

// 删除分组
delete_group(&state, "group-001")?;
```

---

## SSH 连接

### 建立连接

```rust
use easyssh_core::{ssh_connect, ssh_disconnect, SessionMetadata};

async fn connect() -> Result<SessionMetadata, LiteError> {
    // 连接到服务器
    let metadata = ssh_connect(
        &state,           // AppState
        "server-001",    // 服务器ID
        None             // 密码 (None 使用 SSH agent)
    ).await?;

    println!("Session ID: {}", metadata.id);
    println!("Connected at: {:?}", metadata.connected_at);

    Ok(metadata)
}
```

### 执行命令

```rust
use easyssh_core::{ssh_execute, ssh_execute_once, ssh_execute_stream};

// 执行命令（带重试）
let output = ssh_execute(&state, "session-001", "uname -a").await?;
println!("Output: {}", output);

// 执行命令（无重试）
let output = ssh_execute_once(&state, "session-001", "ls -la").await?;

// 流式执行
let mut rx = ssh_execute_stream(&state, "session-001", "tail -f /var/log/app.log").await?;
while let Some(line) = rx.recv().await {
    println!("{}", line);
}
```

### Shell 交互

```rust
use easyssh_core::{
    ssh_write_shell_input,
    ssh_interrupt,
    ssh_get_metadata
};

// 发送输入到 shell
ssh_write_shell_input(&state, "session-001", b"echo hello\n").await?;

// 中断命令 (Ctrl+C)
ssh_interrupt(&state, "session-001").await?;

// 获取会话元数据
if let Some(metadata) = ssh_get_metadata(&state, "session-001").await {
    println!("Host: {}:{}", metadata.host, metadata.port);
}
```

### 会话管理

```rust
use easyssh_core::{
    ssh_list_sessions,
    ssh_get_pool_stats,
    ssh_disconnect
};

// 列出活动会话
let sessions = ssh_list_sessions(&state);
for session_id in sessions {
    println!("Active: {}", session_id);
}

// 获取连接池统计
let stats = ssh_get_pool_stats(&state);
println!("Total connections: {}", stats.total_connections);
println!("Active sessions: {}", stats.active_sessions);

// 断开连接
ssh_disconnect(&state, "session-001").await?;
```

### SFTP 操作 (需要 `sftp` 特性)

```rust
#[cfg(feature = "sftp")]
use easyssh_core::ssh_create_sftp;

// 创建 SFTP 会话
let sftp = ssh_create_sftp(&state, "session-001").await?;

// 使用 ssh2::Sftp 进行文件操作
let stat = sftp.stat("/home/user/file.txt")?;
```

---

## 加密系统

### CryptoState

```rust
use easyssh_core::crypto::CryptoState;

// 创建新状态
let mut crypto = CryptoState::new();
assert!(!crypto.is_unlocked());

// 初始化（首次使用）
crypto.initialize("my_master_password")?;
assert!(crypto.is_unlocked());

// 获取盐值（用于后续解锁）
let salt = crypto.get_salt()?;

// 加密数据
let plaintext = b"sensitive data";
let encrypted = crypto.encrypt(plaintext)?;

// 解密数据
let decrypted = crypto.decrypt(&encrypted)?;
assert_eq!(plaintext.to_vec(), decrypted);

// 锁定（清除密钥）
crypto.lock();
assert!(!crypto.is_unlocked());

// 使用已有盐值解锁
let mut crypto2 = CryptoState::new();
crypto2.set_salt(salt.try_into().unwrap());
crypto2.unlock("my_master_password")?;
```

---

## Docker 管理

### 容器操作

```rust
#[cfg(feature = "docker")]
use easyssh_core::{
    docker_list_containers, docker_start_container,
    docker_stop_container, docker_restart_container,
    docker_remove_container, docker_inspect_container
};

// 列出容器
let containers = docker_list_containers(&state, "session-001", true).await?;
for container in containers {
    println!("{} - {:?}", container.id, container.status);
}

// 启动容器
docker_start_container(&state, "session-001", "container-id").await?;

// 停止容器
docker_stop_container(&state, "session-001", "container-id", Some(30)).await?;

// 重启容器
docker_restart_container(&state, "session-001", "container-id", Some(30)).await?;

// 删除容器
docker_remove_container(&state, "session-001", "container-id", false, false).await?;
```

### 镜像操作

```rust
#[cfg(feature = "docker")]
use easyssh_core::{
    docker_list_images, docker_pull_image,
    docker_build_image, docker_inspect_image
};

// 列出镜像
let images = docker_list_images(&state, "session-001", false).await?;

// 拉取镜像
docker_pull_image(&state, "session-001", "nginx", Some("latest")).await?;

// 构建镜像
let build_args = [("VERSION", "1.0")];
let image_id = docker_build_image(
    &state, "session-001",
    "/path/to/context",
    Some("Dockerfile.prod"),
    Some("myapp:1.0"),
    &build_args,
    false
).await?;
```

### Docker Compose

```rust
#[cfg(feature = "docker")]
use easyssh_core::{
    docker_list_compose_projects,
    docker_compose_up, docker_compose_down
};

// 列出 Compose 项目
let projects = docker_list_compose_projects(&state, "session-001").await?;

// 启动项目
let output = docker_compose_up(&state, "session-001", "/path/to/project").await?;

// 停止项目
let output = docker_compose_down(&state, "session-001", "/path/to/project").await?;
```

### 日志和监控

```rust
#[cfg(feature = "docker")]
use easyssh_core::{
    docker_stream_logs, docker_stream_stats,
    docker_get_stats, docker_top
};

// 流式日志
let mut log_rx = docker_stream_logs(
    &state, "session-001", "container-id",
    true,    // follow
    Some(100) // tail 100 lines
).await?;

while let Some(line) = log_rx.recv().await {
    println!("{}", line);
}

// 获取容器统计
let stats = docker_get_stats(&state, "session-001", "container-id").await?;
println!("CPU: {}%, Memory: {}MB",
    stats.cpu_stats.cpu_usage.total_usage,
    stats.memory_stats.usage / 1024 / 1024
);
```

---

## Kubernetes 管理

### 集群操作

```rust
#[cfg(feature = "kubernetes")]
use easyssh_core::kubernetes::{
    K8sManager, K8sCluster, K8sNamespace, K8sPod
};

// 获取 K8s 管理器
let k8s_manager = state.k8s_manager.read().await;

// 列出集群
let clusters = k8s_manager.list_clusters()?;

// 切换上下文
k8s_manager.switch_context("production").await?;
```

### 资源操作

```rust
#[cfg(feature = "kubernetes")]
use easyssh_core::kubernetes::{
    K8sPod, K8sDeployment, K8sService
};

// 列出 Pods
let pods = k8s_manager.list_pods(Some("default")).await?;

// 列出 Deployments
let deployments = k8s_manager.list_deployments(Some("default")).await?;

// 列出 Services
let services = k8s_manager.list_services(Some("default")).await?;

// 获取 Pod 日志
let logs = k8s_manager.get_pod_logs(
    "pod-name",
    Some("default"),
    Some("container-name"),
    None,
    Some(100)
).await?;
```

---

## 监控功能

### 服务器监控

```rust
#[cfg(feature = "monitoring")]
use easyssh_core::monitoring::{
    MonitoringManager, MonitoringConfig,
    ServerHealthStatus, ServerMetrics
};

// 获取监控管理器
let monitoring = state.monitoring_manager.read().await;

// 添加服务器监控
let config = ServerConnectionConfig {
    host: "192.168.1.100".to_string(),
    port: 22,
    username: "monitor".to_string(),
    auth_type: "key".to_string(),
    identity_file: Some("/path/to/key".to_string()),
    password: None,
};
monitoring.add_server("server-001", config).await?;

// 获取服务器健康状态
let health = monitoring.get_server_health("server-001").await?;
println!("Status: {:?}", health.status);

// 获取指标
let metrics = monitoring.get_server_metrics("server-001").await?;
for (metric_type, points) in metrics {
    println!("{:?}: {} points", metric_type, points.len());
}
```

### 告警系统

```rust
#[cfg(feature = "monitoring")]
use easyssh_core::monitoring::{
    AlertRule, AlertCondition, AlertSeverity,
    NotificationChannel, NotificationChannelType
};

// 创建告警规则
let rule = AlertRule {
    id: "high-cpu".to_string(),
    name: "High CPU Usage".to_string(),
    condition: AlertCondition::CpuUsageAbove(80.0),
    severity: AlertSeverity::Warning,
    channels: vec![
        NotificationChannel {
            channel_type: NotificationChannelType::Email,
            config: serde_json::json!({
                "to": "admin@example.com"
            }),
        }
    ],
};
monitoring.add_alert_rule(rule).await?;
```

---

## 团队协作

### 团队管理 (Pro 版本)

```rust
#[cfg(feature = "team")]
use easyssh_core::team::{
    TeamManager, Team, TeamMember, TeamRole
};

// 获取团队管理器
let team_mgr = state.team_manager.lock().await;

// 创建团队
let team = team_mgr.create_team(
    "Development Team",
    "Main dev team"
).await?;

// 邀请成员
let invite = team_mgr.invite_member(
    &team.id,
    "user@example.com",
    TeamRole::Developer
).await?;

// 获取团队成员
let members = team_mgr.get_team_members(&team.id).await?;
```

### 协作会话

```rust
#[cfg(feature = "pro")]
use easyssh_core::collaboration::{
    CollaborationManager, CollaborationSession,
    CollaborationRole
};

// 创建协作会话
let collab_mgr = state.collaboration_manager.lock().await;
let session = collab_mgr.create_session(
    "server-001",
    "pair-programming"
).await?;

// 邀请参与者
collab_mgr.invite_participant(
    &session.id,
    "user@example.com",
    CollaborationRole::Editor
).await?;

// 共享剪贴板
collab_mgr.share_clipboard_item(
    &session.id,
    ClipboardContentType::Text,
    "shared content".as_bytes()
).await?;
```

---

## 审计日志

### 审计系统

```rust
#[cfg(feature = "audit")]
use easyssh_core::audit::{
    AuditLogger, AuditEntry, AuditAction, AuditTarget
};

// 获取审计日志器
let audit = state.audit_logger.lock().await;

// 记录事件
audit.log(AuditEntry {
    actor: "user@example.com".to_string(),
    action: AuditAction::ServerConnect,
    target: AuditTarget::Server("server-001".to_string()),
    details: None,
    timestamp: chrono::Utc::now(),
}).await?;

// 查询审计日志
let entries = audit.query(
    Some(AuditTarget::Server("server-001".to_string())),
    Some(chrono::Utc::now() - chrono::Duration::days(7)),
    Some(chrono::Utc::now()),
).await?;
```

---

## 错误处理

### LiteError

EasySSH 使用统一的错误类型 `LiteError`：

```rust
use easyssh_core::error::LiteError;

fn handle_result(result: Result<(), LiteError>) {
    match result {
        Ok(()) => println!("Success"),
        Err(LiteError::Database(msg)) => {
            eprintln!("Database error: {}", msg);
        }
        Err(LiteError::SshConnectionFailed { host, port, message }) => {
            eprintln!("Failed to connect to {}:{} - {}", host, port, message);
        }
        Err(LiteError::SshAuthFailed { host, username }) => {
            eprintln!("Auth failed for {}@{}", username, host);
        }
        Err(e) => {
            eprintln!("Error: {}", e);
        }
    }
}
```

### 国际化错误

错误支持国际化翻译：

```rust
use easyssh_core::{t_args, LiteError};

let err = LiteError::Database("connection failed".to_string());
let key = err.translation_key();
let args = err.translation_args();

// 前端可以根据 key 和 args 进行翻译
println!("Translation key: {}", key);
```

---

## 完整示例

### 连接到服务器并执行命令

```rust
use easyssh_core::{
    AppState, init_database, ssh_connect,
    ssh_execute, ssh_disconnect, LiteError
};

async fn main() -> Result<(), LiteError> {
    // 初始化
    let state = AppState::new();
    init_database(&state)?;

    // 连接
    let session = ssh_connect(&state, "my-server", None).await?;
    println!("Connected: {}", session.id);

    // 执行命令
    let output = ssh_execute(&state, &session.id, "uptime").await?;
    println!("Uptime: {}", output);

    // 断开
    ssh_disconnect(&state, &session.id).await?;
    println!("Disconnected");

    Ok(())
}
```

### 加密敏感配置

```rust
use easyssh_core::crypto::CryptoState;
use easyssh_core::{add_server, NewServer};

async fn secure_setup() -> Result<(), LiteError> {
    let mut crypto = CryptoState::new();
    crypto.initialize("strong_master_password")?;

    // 加密密码
    let password = b"secret_password";
    let encrypted = crypto.encrypt(password)?;

    // 创建服务器配置
    let server = NewServer {
        id: "secure-server".to_string(),
        name: "Secure Server".to_string(),
        host: "192.168.1.100".to_string(),
        port: 22,
        username: "admin".to_string(),
        auth_type: "password".to_string(),
        password_encrypted: Some(encrypted),
        ..Default::default()
    };

    add_server(&state, &server)?;

    Ok(())
}
```

---

更多示例请查看 `examples/` 目录。
