# SSH 模块 API

EasySSH Core SSH 模块提供完整的 SSH 连接管理，包括会话池、执行命令、流式输出等功能。

## 结构

```rust
pub struct SshSessionManager {
    pool: ConnectionPool,
    sessions: HashMap<String, SessionHandle>,
}

pub struct SessionMetadata {
    pub session_id: String,
    pub host: String,
    pub port: u16,
    pub username: String,
    pub connected_at: DateTime<Utc>,
    pub auth_method: AuthMethod,
}

pub struct ConnectionHealth {
    pub is_connected: bool,
    pub last_activity: DateTime<Utc>,
    pub latency_ms: u64,
}
```

## 主要类型

### `SshSessionManager`

SSH 会话管理器，维护连接池和活动会话。

```rust
// 创建默认管理器
let manager = SshSessionManager::new();

// 使用自定义池配置
let manager = SshSessionManager::new()
    .with_pool_config(20, 600, 7200); // max, idle_timeout, max_age
```

### `SessionMetadata`

会话元数据，标识一个 SSH 连接。

| 字段 | 类型 | 说明 |
|------|------|------|
| `session_id` | `String` | 唯一会话标识 |
| `host` | `String` | 服务器主机 |
| `port` | `u16` | SSH 端口 |
| `username` | `String` | 登录用户 |
| `connected_at` | `DateTime<Utc>` | 连接时间 |
| `auth_method` | `AuthMethod` | 认证方式 |

## 方法

### 建立连接

```rust
/// 建立 SSH 连接
pub async fn connect(
    &mut self,
    session_id: &str,
    host: &str,
    port: u16,
    username: &str,
    password: Option<&str>,
) -> Result<SessionMetadata, LiteError>
```

**参数：**
- `session_id`: 自定义会话 ID，或留空自动生成
- `host`: 主机名或 IP
- `port`: SSH 端口（通常 22）
- `username`: 登录用户名
- `password`: 密码（如使用密钥认证可传 None）

**示例：**

```rust
use easyssh_core::{AppState, ssh_connect};

// 使用密码
let metadata = ssh_connect(&state, "server-1", "192.168.1.100", 22, "admin", Some("password")).await?;

// 使用密钥（从数据库读取）
let server = get_server(&state, "server-id")?;
let metadata = ssh_connect(&state, &server.id, None).await?;
```

### 执行命令

```rust
/// 执行单次命令
pub async fn execute(
    &self,
    session_id: &str,
    command: &str,
) -> Result<String, LiteError>

/// 带重试的命令执行
pub async fn execute_with_retry(
    &self,
    session_id: &str,
    command: &str,
    max_retries: u32,
) -> Result<String, LiteError>
```

**示例：**

```rust
use easyssh_core::ssh_execute;

// 简单执行
let output = ssh_execute(&state, "session-123", "uptime").await?;
println!("Output: {}", output);

// 带重试（网络不稳定时）
let output = ssh_execute_with_retry(&state, "session-123", "docker ps", 3).await?;
```

### 流式执行

```rust
/// 启动流式命令会话（用于交互式 shell）
pub async fn execute_stream(
    &mut self,
    session_id: &str,
    command: &str,
) -> Result<mpsc::UnboundedReceiver<String>, LiteError>

/// 写入 shell 输入
pub async fn write_shell_input(
    &self,
    session_id: &str,
    input: &[u8],
) -> Result<(), LiteError>

/// 发送中断信号 (Ctrl+C)
pub async fn interrupt_command(
    &self,
    session_id: &str,
) -> Result<(), LiteError>
```

**示例：**

```rust
use easyssh_core::ssh_execute_stream;

// 启动交互式会话
let mut receiver = ssh_execute_stream(&state, "session-123", "/bin/bash").await?;

// 读取输出
while let Some(line) = receiver.recv().await {
    println!("{}", line);
}

// 发送命令
ssh_write_shell_input(&state, "session-123", b"ls -la\n").await?;

// 中断
crate::ssh_interrupt(&state, "session-123").await?;
```

### 断开连接

```rust
/// 断开指定会话
pub async fn disconnect(
    &mut self,
    session_id: &str,
) -> Result<(), LiteError>

/// 断开所有会话
pub async fn disconnect_all(&mut self) -> Result<(), LiteError>
```

**示例：**

```rust
use easyssh_core::ssh_disconnect;

// 断开单个会话
ssh_disconnect(&state, "session-123").await?;

// 断开所有
{
    let mut manager = state.ssh_manager.lock().await;
    manager.disconnect_all().await?;
}
```

### 会话查询

```rust
/// 列出所有活动会话
pub fn list_sessions(&self) -> Vec<String>

/// 获取会话元数据
pub fn get_metadata(&self, session_id: &str) -> Option<SessionMetadata>

/// 检查连接健康状态
pub async fn check_health(
    &self,
    session_id: &str,
) -> Result<ConnectionHealth, LiteError>

/// 获取连接池统计
pub fn get_pool_stats(&self) -> PoolStats
```

**示例：**

```rust
// 列出会话
let sessions = ssh_list_sessions(&state);
for session_id in sessions {
    println!("Active: {}", session_id);
}

// 获取元数据
if let Some(meta) = ssh_get_metadata(&state, "session-123").await {
    println!("Connected to {}@{}", meta.username, meta.host);
}

// 检查健康
let health = {
    let manager = state.ssh_manager.lock().await;
    manager.check_health("session-123").await?
};
println!("Connected: {}, Latency: {}ms",
    health.is_connected, health.latency_ms);

// 池统计
let stats = ssh_get_pool_stats(&state);
println!("Pool: {}/{} connections", stats.active, stats.max);
```

## 高级功能

### 端口转发

```rust
use easyssh_core::ssh::PortForward;

// 本地端口转发
let forward = PortForward::local(8080, "localhost", 80);
manager.setup_port_forward("session-123", forward).await?;

// 远程端口转发
let forward = PortForward::remote(9090, "localhost", 3000);
manager.setup_port_forward("session-123", forward).await?;

// 动态 SOCKS 代理
let forward = PortForward::dynamic(1080);
manager.setup_port_forward("session-123", forward).await?;
```

### Agent 转发

```rust
// 启用 SSH Agent 转发
let metadata = manager
    .connect_with_options(
        "session-1",
        "host",
        22,
        "user",
        ConnectOptions {
            agent_forward: true,
            ..Default::default()
        }
    ).await?;
```

### ProxyJump

```rust
// 通过跳板机连接
let metadata = manager
    .connect_with_proxy(
        "session-1",
        "target.internal",
        22,
        "user",
        &["bastion.example.com", "intermediate.internal"]
    ).await?;
```

### SFTP 子系统

```rust
use easyssh_core::ssh_create_sftp;

// 创建 SFTP 会话
let sftp = ssh_create_sftp(&state, "session-123").await?;

// 使用 ssh2::Sftp 进行文件操作
let file = sftp.open(Path::new("/etc/hosts"))?;
```

## 配置选项

### 连接池配置

```rust
use easyssh_core::PoolConfig;

let config = PoolConfig {
    max_connections: 20,        // 最大连接数
    idle_timeout: 600,          // 空闲超时（秒）
    max_age: 7200,              // 最大存活时间（秒）
    connection_timeout: 30,     // 连接超时（秒）
    retry_attempts: 3,          // 重试次数
    retry_delay: 1000,          // 重试间隔（毫秒）
};

let manager = SshSessionManager::new().with_pool_config(config);
```

### SSH 选项

```rust
use easyssh_core::ssh::SshOptions;

let options = SshOptions {
    compress: true,                      // 启用压缩
    keepalive_interval: 30,            // Keepalive 间隔
    keepalive_count_max: 3,            // Keepalive 失败次数
    key_exchange_algorithms: vec![      // 密钥交换算法
        "curve25519-sha256".to_string(),
        "ecdh-sha2-nistp256".to_string(),
    ],
    ciphers: vec![                      // 加密算法
        "aes256-gcm@openssh.com".to_string(),
        "aes128-gcm@openssh.com".to_string(),
    ],
};
```

## 错误处理

### SSH 错误类型

```rust
use easyssh_core::LiteError;

match result {
    Err(LiteError::SshConnection(e)) => {
        eprintln!("连接失败: {}", e);
    }
    Err(LiteError::SshAuthentication(e)) => {
        eprintln!("认证失败: {}", e);
    }
    Err(LiteError::SshTimeout) => {
        eprintln!("连接超时");
    }
    Err(LiteError::SshCommand(e)) => {
        eprintln!("命令执行失败: {}", e);
    }
    _ => {}
}
```

### 重试策略

```rust
use easyssh_core::RetryPolicy;

let policy = RetryPolicy::exponential_backoff()
    .max_attempts(5)
    .initial_delay(Duration::from_millis(100))
    .max_delay(Duration::from_secs(10));

let result = manager
    .execute_with_policy("session-123", "command", policy)
    .await?;
```

## FFI 接口

### C 头文件

```c
#ifndef EASYSH_SSH_H
#define EASYSH_SSH_H

#include <stdint.h>

typedef struct EasySSHSession EasySSHSession;
typedef struct EasySSHStream EasySSHStream;

// 连接
EasySSHSession* easyssh_connect(
    EasySSHState* state,
    const char* host,
    uint16_t port,
    const char* username,
    const char* password
);

// 执行命令
char* easyssh_execute(
    EasySSHState* state,
    const char* session_id,
    const char* command
);

// 流式执行
EasySSHStream* easyssh_execute_stream(
    EasySSHState* state,
    const char* session_id,
    const char* command
);

// 读取流输出
const char* easyssh_stream_read(EasySSHStream* stream);

// 写入输入
int easyssh_stream_write(
    EasySSHState* state,
    const char* session_id,
    const char* data,
    size_t len
);

// 中断
int easyssh_interrupt(EasySSHState* state, const char* session_id);

// 断开
void easyssh_disconnect(EasySSHState* state, const char* session_id);

// 释放字符串
void easyssh_free_string(char* str);

#endif
```

### Swift 绑定示例

```swift
import EasySSHCore

class SSHClient {
    private let state: AppState

    func connect(to host: String, username: String, password: String) async throws -> Session {
        let metadata = try await ssh_connect(
            state,
            nil,
            host,
            22,
            username,
            password
        )
        return Session(id: metadata.session_id, metadata: metadata)
    }

    func execute(command: String, sessionId: String) async throws -> String {
        return try await ssh_execute(state, sessionId, command)
    }
}
```

## 性能优化

### 连接复用

```rust
// 连接池自动复用连接
// 相同 host+port+username 的连接会被复用

// 手动控制连接生命周期
{
    let manager = state.ssh_manager.lock().await;
    let metadata = manager.connect(...).await?;
    // 使用连接...
} // 连接返回池中
```

### 批量操作

```rust
use futures::future::join_all;

let futures: Vec<_> = servers.iter().map(|s| {
    let state = state.clone();
    async move {
        let meta = ssh_connect(&state, &s.id, None).await?;
        let result = ssh_execute(&state, &meta.session_id, "uptime").await?;
        ssh_disconnect(&state, &meta.session_id).await?;
        Ok::<_, LiteError>(result)
    }
}).collect();

let results = join_all(futures).await;
```

## 完整示例

```rust
use easyssh_core::*;

#[tokio::main]
async fn main() -> Result<(), LiteError> {
    // 初始化
    let state = AppState::with_ssh_pool_config(10, 300, 3600);
    init_database(&state)?;

    // 添加服务器
    let server = NewServer {
        name: "Web Server".to_string(),
        host: "192.168.1.100".to_string(),
        port: 22,
        username: "deploy".to_string(),
        auth_type: "key".to_string(),
        key_path: Some("~/.ssh/id_rsa".to_string()),
        ..Default::default()
    };
    add_server(&state, &server)?;

    // 获取服务器 ID
    let servers = get_servers(&state)?;
    let server_id = &servers[0].id;

    // 连接
    let metadata = ssh_connect(&state, server_id, None).await?;
    println!("Connected: {}", metadata.session_id);

    // 执行命令
    let output = ssh_execute(&state, &metadata.session_id, "uptime").await?;
    println!("Uptime: {}", output);

    // 流式命令
    let mut receiver = ssh_execute_stream(&state, &metadata.session_id, "tail -f /var/log/app.log").await?;

    tokio::spawn(async move {
        while let Some(line) = receiver.recv().await {
            println!("Log: {}", line);
        }
    });

    // 工作一段时间后断开
    tokio::time::sleep(Duration::from_secs(30)).await;

    // 断开
    ssh_disconnect(&state, &metadata.session_id).await?;
    println!("Disconnected");

    Ok(())
}
```
