# API Guide / API 使用指南

> EasySSH Core Library Complete API Reference
> EasySSH 核心库完整 API 参考

**[English](#english) | [中文](#中文)**

---

# English

## Table of Contents

1. [Core Concepts](#core-concepts)
2. [App State Management](#app-state-management)
3. [Database Operations](#database-operations)
4. [SSH Connection](#ssh-connection)
5. [Encryption System](#encryption-system)
6. [Server Management](#server-management)
7. [Group Management](#group-management)
8. [Error Handling](#error-handling)
9. [Complete Examples](#complete-examples)

---

## Core Concepts

### AppState

`AppState` is the core state container for EasySSH, holding all shared resources:

```rust
use easyssh_core::AppState;

// Create default state
let state = AppState::new();

// With custom SSH pool configuration
let state = AppState::with_ssh_pool_config(
    10,     // max connections
    600,    // idle timeout (seconds)
    3600    // max connection age (seconds)
);
```

### Feature Flags

EasySSH uses Cargo feature flags to control functionality:

```toml
# Cargo.toml
[dependencies]
easyssh-core = {
    version = "0.3",
    features = ["standard", "sftp", "docker"]
}
```

Available features:
- `lite` - Lite edition features (default)
- `standard` - Standard edition features
- `pro` - Pro edition features
- `sftp` - SFTP file transfer support
- `monitoring` - Server monitoring
- `docker` - Docker container management
- `kubernetes` - Kubernetes management

---

## App State Management

### Initialize Database

```rust
use easyssh_core::{AppState, init_database, LiteError};

fn setup() -> Result<(), LiteError> {
    let state = AppState::new();

    // Initialize SQLite database
    init_database(&state)?;

    Ok(())
}
```

### Get Database Path

```rust
use easyssh_core::get_db_path;
use std::path::PathBuf;

let path: PathBuf = get_db_path();
println!("Database at: {:?}", path);
```

---

## Database Operations

### Server Records

```rust
use easyssh_core::{
    NewServer, UpdateServer, ServerRecord,
    add_server, get_server, get_servers,
    update_server, delete_server
};

// Add a server
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

// Query server
let server = get_server(&state, "server-001")?;
let all_servers = get_servers(&state)?;

// Update server
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

// Delete server
delete_server(&state, "server-001")?;
```

### Group Records

```rust
use easyssh_core::{
    NewGroup, UpdateGroup, GroupRecord,
    add_group, get_groups, update_group, delete_group
};

// Add a group
let group = NewGroup {
    id: "group-001".to_string(),
    name: "Production".to_string(),
};
add_group(&state, &group)?;

// Query groups
let groups = get_groups(&state)?;

// Update group
let update = UpdateGroup {
    id: "group-001".to_string(),
    name: Some("Production Servers".to_string()),
};
update_group(&state, &update)?;

// Delete group
delete_group(&state, "group-001")?;
```

---

## SSH Connection

### Establish Connection

```rust
use easyssh_core::{ssh_connect, ssh_disconnect, SessionMetadata};

async fn connect() -> Result<SessionMetadata, LiteError> {
    // Connect to server
    let metadata = ssh_connect(
        &state,           // AppState
        "server-001",    // server ID
        None             // password (None to use SSH agent)
    ).await?;

    println!("Session ID: {}", metadata.id);
    println!("Connected at: {:?}", metadata.connected_at);

    Ok(metadata)
}
```

### Execute Commands

```rust
use easyssh_core::{ssh_execute, ssh_execute_once, ssh_execute_stream};

// Execute command (with retry)
let output = ssh_execute(&state, "session-001", "uname -a").await?;
println!("Output: {}", output);

// Execute command (no retry)
let output = ssh_execute_once(&state, "session-001", "ls -la").await?;

// Stream execution
let mut rx = ssh_execute_stream(
    &state,
    "session-001",
    "tail -f /var/log/app.log"
).await?;

while let Some(line) = rx.recv().await {
    println!("{}", line);
}
```

### Shell Interaction

```rust
use easyssh_core::{
    ssh_write_shell_input,
    ssh_interrupt,
    ssh_get_metadata
};

// Send input to shell
ssh_write_shell_input(&state, "session-001", b"echo hello\n").await?;

// Interrupt command (Ctrl+C)
ssh_interrupt(&state, "session-001").await?;

// Get session metadata
if let Some(metadata) = ssh_get_metadata(&state, "session-001").await {
    println!("Host: {}:{}", metadata.host, metadata.port);
}
```

### Session Management

```rust
use easyssh_core::{
    ssh_list_sessions,
    ssh_get_pool_stats,
    ssh_disconnect
};

// List active sessions
let sessions = ssh_list_sessions(&state);
for session_id in sessions {
    println!("Active: {}", session_id);
}

// Get connection pool stats
let stats = ssh_get_pool_stats(&state);
println!("Total connections: {}", stats.total_connections);
println!("Active sessions: {}", stats.active_sessions);

// Disconnect
ssh_disconnect(&state, "session-001").await?;
```

### SFTP Operations (requires `sftp` feature)

```rust,ignore
#[cfg(feature = "sftp")]
use easyssh_core::ssh_create_sftp;

// Create SFTP session
let sftp = ssh_create_sftp(&state, "session-001").await?;

// Use ssh2::Sftp for file operations
let stat = sftp.stat("/home/user/file.txt")?;
```

---

## Encryption System

### CryptoState

```rust
use easyssh_core::crypto::CryptoState;

// Create new state
let mut crypto = CryptoState::new();
assert!(!crypto.is_unlocked());

// Initialize (first use)
crypto.initialize("my_master_password")?;
assert!(crypto.is_unlocked());

// Get salt (for future unlocking)
let salt = crypto.get_salt()?;

// Encrypt data
let plaintext = b"sensitive data";
let encrypted = crypto.encrypt(plaintext)?;

// Decrypt data
let decrypted = crypto.decrypt(&encrypted)?;
assert_eq!(plaintext.to_vec(), decrypted);

// Lock (clear keys)
crypto.lock();
assert!(!crypto.is_unlocked());

// Unlock with existing salt
let mut crypto2 = CryptoState::new();
crypto2.set_salt(salt.try_into().unwrap());
crypto2.unlock("my_master_password")?;
```

### Encryption Algorithms

```rust
use easyssh_core::crypto::{CryptoConfig, EncryptionAlgorithm, KdfAlgorithm};

let config = CryptoConfig {
    algorithm: EncryptionAlgorithm::Aes256Gcm,
    kdf: KdfAlgorithm::Argon2id {
        memory: 65536,  // 64MB
        iterations: 3,
        parallelism: 4,
    },
};
```

---

## Server Management

### Import/Export

```rust
use easyssh_core::{
    import_ssh_config,
    export_servers_to_json,
    export_servers_to_yaml
};

// Import from ~/.ssh/config
let imported = import_ssh_config(&state, "/home/user/.ssh/config").await?;
println!("Imported {} servers", imported.len());

// Export to JSON
let json = export_servers_to_json(&state)?;
std::fs::write("servers.json", json)?;

// Export to YAML
let yaml = export_servers_to_yaml(&state)?;
std::fs::write("servers.yaml", yaml)?;
```

### Connection Test

```rust
use easyssh_core::test_server_connection;

// Test connection without saving
let result = test_server_connection(
    &state,
    "192.168.1.100",
    22,
    "admin",
    &auth_method
).await;

match result {
    Ok(_) => println!("Connection successful"),
    Err(e) => println!("Connection failed: {}", e),
}
```

---

## Group Management

### Nested Groups (Standard/Pro)

```rust
use easyssh_core::{
    NewGroup, add_group, move_group,
    get_group_tree, GroupTreeNode
};

// Create nested groups
let parent = NewGroup {
    id: "prod".to_string(),
    name: "Production".to_string(),
    parent_id: None,
};
add_group(&state, &parent)?;

let child = NewGroup {
    id: "prod-web".to_string(),
    name: "Web Servers".to_string(),
    parent_id: Some("prod".to_string()),
};
add_group(&state, &child)?;

// Move group
move_group(&state, "prod-web", Some("staging"))?;

// Get group tree
let tree = get_group_tree(&state)?;
for node in tree {
    print_group_tree(&node, 0);
}

fn print_group_tree(node: &GroupTreeNode, depth: usize) {
    let indent = "  ".repeat(depth);
    println!("{}{}", indent, node.name);
    for child in &node.children {
        print_group_tree(child, depth + 1);
    }
}
```

---

## Error Handling

### LiteError

EasySSH uses a unified error type `LiteError`:

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

### Internationalization

Errors support i18n translation:

```rust
use easyssh_core::{t_args, LiteError};

let err = LiteError::Database("connection failed".to_string());
let key = err.translation_key();
let args = err.translation_args();

// Frontend can translate using key and args
println!("Translation key: {}", key);
```

---

## Complete Examples

### Connect and Execute Commands

```rust
use easyssh_core::{
    AppState, init_database, ssh_connect,
    ssh_execute, ssh_disconnect, LiteError
};

async fn main() -> Result<(), LiteError> {
    // Initialize
    let state = AppState::new();
    init_database(&state)?;

    // Connect
    let session = ssh_connect(&state, "my-server", None).await?;
    println!("Connected: {}", session.id);

    // Execute command
    let output = ssh_execute(&state, &session.id, "uptime").await?;
    println!("Uptime: {}", output);

    // Disconnect
    ssh_disconnect(&state, &session.id).await?;
    println!("Disconnected");

    Ok(())
}
```

### Encrypt Sensitive Configuration

```rust
use easyssh_core::crypto::CryptoState;
use easyssh_core::{add_server, NewServer};

async fn secure_setup() -> Result<(), LiteError> {
    let mut crypto = CryptoState::new();
    crypto.initialize("strong_master_password")?;

    // Encrypt password
    let password = b"secret_password";
    let encrypted = crypto.encrypt(password)?;

    // Create server config
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

### Server Monitoring Loop

```rust
use easyssh_core::{
    ssh_connect, ssh_execute_stream,
    ssh_disconnect
};
use tokio::time::{interval, Duration};

async fn monitoring_loop(server_id: &str) -> Result<(), LiteError> {
    let state = AppState::new();

    // Connect
    let session = ssh_connect(&state, server_id, None).await?;

    // Setup monitoring interval
    let mut ticker = interval(Duration::from_secs(60));

    loop {
        ticker.tick().await;

        // Get system metrics
        let output = ssh_execute(
            &state,
            &session.id,
            "cat /proc/loadavg"
        ).await?;

        println!("Load average: {}", output);
    }
}
```

---

# 中文

## 目录

1. [核心概念](#核心概念)
2. [应用状态管理](#应用状态管理)
3. [数据库操作](#数据库操作)
4. [SSH 连接](#ssh-连接)
5. [加密系统](#加密系统)
6. [服务器管理](#服务器管理)
7. [分组管理](#分组管理)
8. [错误处理](#错误处理)
9. [完整示例](#完整示例)

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

```toml
# Cargo.toml
[dependencies]
easyssh-core = {
    version = "0.3",
    features = ["standard", "sftp", "docker"]
}
```

可用特性：
- `lite` - Lite 版本功能（默认）
- `standard` - Standard 版本功能
- `pro` - Pro 版本功能
- `sftp` - SFTP 文件传输支持
- `monitoring` - 服务器监控
- `docker` - Docker 容器管理
- `kubernetes` - Kubernetes 管理

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
let mut rx = ssh_execute_stream(
    &state,
    "session-001",
    "tail -f /var/log/app.log"
).await?;

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

```rust,ignore
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

### 加密算法

```rust
use easyssh_core::crypto::{CryptoConfig, EncryptionAlgorithm, KdfAlgorithm};

let config = CryptoConfig {
    algorithm: EncryptionAlgorithm::Aes256Gcm,
    kdf: KdfAlgorithm::Argon2id {
        memory: 65536,  // 64MB
        iterations: 3,
        parallelism: 4,
    },
};
```

---

## 服务器管理

### 导入/导出

```rust
use easyssh_core::{
    import_ssh_config,
    export_servers_to_json,
    export_servers_to_yaml
};

// 从 ~/.ssh/config 导入
let imported = import_ssh_config(&state, "/home/user/.ssh/config").await?;
println!("Imported {} servers", imported.len());

// 导出为 JSON
let json = export_servers_to_json(&state)?;
std::fs::write("servers.json", json)?;

// 导出为 YAML
let yaml = export_servers_to_yaml(&state)?;
std::fs::write("servers.yaml", yaml)?;
```

### 连接测试

```rust
use easyssh_core::test_server_connection;

// 测试连接而不保存
let result = test_server_connection(
    &state,
    "192.168.1.100",
    22,
    "admin",
    &auth_method
).await;

match result {
    Ok(_) => println!("Connection successful"),
    Err(e) => println!("Connection failed: {}", e),
}
```

---

## 分组管理

### 嵌套分组 (Standard/Pro)

```rust
use easyssh_core::{
    NewGroup, add_group, move_group,
    get_group_tree, GroupTreeNode
};

// 创建嵌套分组
let parent = NewGroup {
    id: "prod".to_string(),
    name: "Production".to_string(),
    parent_id: None,
};
add_group(&state, &parent)?;

let child = NewGroup {
    id: "prod-web".to_string(),
    name: "Web Servers".to_string(),
    parent_id: Some("prod".to_string()),
};
add_group(&state, &child)?;

// 移动分组
move_group(&state, "prod-web", Some("staging"))?;

// 获取分组树
let tree = get_group_tree(&state)?;
for node in tree {
    print_group_tree(&node, 0);
}

fn print_group_tree(node: &GroupTreeNode, depth: usize) {
    let indent = "  ".repeat(depth);
    println!("{}{}", indent, node.name);
    for child in &node.children {
        print_group_tree(child, depth + 1);
    }
}
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

### 服务器监控循环

```rust
use easyssh_core::{
    ssh_connect, ssh_execute_stream,
    ssh_disconnect
};
use tokio::time::{interval, Duration};

async fn monitoring_loop(server_id: &str) -> Result<(), LiteError> {
    let state = AppState::new();

    // 连接
    let session = ssh_connect(&state, server_id, None).await?;

    // 设置监控间隔
    let mut ticker = interval(Duration::from_secs(60));

    loop {
        ticker.tick().await;

        // 获取系统指标
        let output = ssh_execute(
            &state,
            &session.id,
            "cat /proc/loadavg"
        ).await?;

        println!("Load average: {}", output);
    }
}
```

---

*For more examples, see the `examples/` directory.*
*更多示例请查看 `examples/` 目录。*
