# API 文档概览

EasySSH Core 库提供完整的 Rust API，并支持 FFI 供其他语言调用。

## 架构

```
┌────────────────────────────────────────────────────────────┐
│                        FFI Layer                            │
│  (C ABI compatible - for Swift/Kotlin/Dart interop)        │
├────────────────────────────────────────────────────────────┤
│                    Core Library (Rust)                       │
│  ┌─────────┐ ┌─────────┐ ┌─────────┐ ┌─────────┐        │
│  │   SSH   │ │   DB    │ │  Crypto │ │  SFTP   │        │
│  ├─────────┤ ├─────────┤ ├─────────┤ ├─────────┤        │
│  │ Session │ │ SQLite  │ │Argon2id │ │ Transfer│        │
│  │  Pool   │ │  Encrypt│ │AES-256  │ │  Manage │        │
│  └─────────┘ └─────────┘ └─────────┘ └─────────┘        │
│  ┌─────────┐ ┌─────────┐ ┌─────────┐ ┌─────────┐        │
│  │ Terminal│ │  Layout │ │  Team   │ │  Audit  │ (Pro) │
│  └─────────┘ └─────────┘ └─────────┘ └─────────┘        │
└────────────────────────────────────────────────────────────┘
```

## 快速开始

### Rust 使用

```rust
use easyssh_core::{AppState, init_database, add_server, NewServer, ssh_connect};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 创建应用状态
    let state = AppState::new();

    // 初始化数据库
    init_database(&state)?;

    // 添加服务器
    let server = NewServer {
        name: "Production".to_string(),
        host: "192.168.1.100".to_string(),
        port: 22,
        username: "deploy".to_string(),
        auth_type: "key".to_string(),
        key_path: Some("~/.ssh/id_rsa".to_string()),
        ..Default::default()
    };
    add_server(&state, &server)?;

    // 建立 SSH 连接
    let metadata = ssh_connect(&state, "server-id", None).await?;
    println!("Connected: {:?}", metadata);

    Ok(())
}
```

### FFI 调用 (C/Swift)

```c
// C 头文件示例
#include "easyssh_ffi.h"

// 初始化
EasySSHState* state = easyssh_init();
easyssh_init_database(state);

// 添加服务器
EasySSHServer server = {
    .name = "Production",
    .host = "192.168.1.100",
    .port = 22,
    .username = "deploy",
    .auth_type = "key",
    .key_path = "~/.ssh/id_rsa"
};
easyssh_add_server(state, &server);

// 连接
EasySSHSession* session = easyssh_connect(state, "server-id", NULL);

// 执行命令
char* result = easyssh_execute(state, session->id, "uptime");
printf("Result: %s\n", result);

// 清理
easyssh_free_string(result);
easyssh_disconnect(state, session->id);
easyssh_destroy(state);
```

## 模块结构

### Core 模块

| 模块 | 功能 | 文档 |
|------|------|------|
| `ssh` | SSH 连接管理、会话池 | [详情](/zh/api/core/ssh) |
| `db` | SQLite 数据库、加密存储 | [详情](/zh/api/core/db) |
| `crypto` | 加密、密钥派生 | [详情](/zh/api/core/crypto) |
| `sftp` | SFTP 文件传输 | [详情](/zh/api/core/sftp) |
| `terminal` | 终端模拟器 | [详情](/zh/api/core/terminal) |
| `layout` | 分屏布局管理 | [详情](/zh/api/core/layout) |
| `keychain` | 系统钥匙串集成 | [详情](/zh/api/core/keychain) |

### Pro 模块

| 模块 | 功能 | 文档 |
|------|------|------|
| `team` | 团队管理 | [详情](/zh/api/pro/team) |
| `rbac` | 权限控制 | [详情](/zh/api/pro/rbac) |
| `audit` | 审计日志 | [详情](/zh/api/pro/audit) |
| `sync` | 数据同步 | [详情](/zh/api/pro/sync) |

## 错误处理

### Rust

```rust
use easyssh_core::LiteError;

match operation() {
    Ok(result) => println!("Success: {:?}", result),
    Err(LiteError::Ssh(e)) => eprintln!("SSH Error: {}", e),
    Err(LiteError::Database(e)) => eprintln!("DB Error: {}", e),
    Err(LiteError::Crypto(e)) => eprintln!("Crypto Error: {}", e),
    Err(e) => eprintln!("Other Error: {}", e),
}
```

### FFI

```c
// 错误码定义
typedef enum {
    EASYSH_OK = 0,
    EASYSH_ERROR_SSH = 1,
    EASYSH_ERROR_DB = 2,
    EASYSH_ERROR_CRYPTO = 3,
    EASYSH_ERROR_INVALID_PARAM = 4,
    EASYSH_ERROR_NOT_FOUND = 5,
    EASYSH_ERROR_CONNECTION = 6,
    EASYSH_ERROR_TIMEOUT = 7,
    EASYSH_ERROR_PERMISSION = 8
} EasySSHErrorCode;

// 获取错误信息
const char* error = easyssh_last_error();
printf("Error: %s\n", error);
```

## 版本兼容性

| Core 版本 | Rust 版本 | FFI 版本 | 说明 |
|-----------|-----------|----------|------|
| 1.0.x | 1.70+ | 1.0 | 初始版本 |
| 1.1.x | 1.70+ | 1.0 | 新增 SFTP |
| 1.2.x | 1.75+ | 1.1 | 新增 Pro 模块 |
| 2.0.x | 1.80+ | 2.0 | API 重构 |

## 性能指标

### 连接池

```rust
// 默认配置
let state = AppState::with_ssh_pool_config(
    10,     // 最大连接数
    300,    // 空闲超时（秒）
    3600    // 最大年龄（秒）
);
```

### 数据库

- 读写分离支持
- 连接池默认 10 连接
- 自动 WAL 模式

### 内存使用

| 组件 | 内存占用 |
|------|----------|
| Core (Lite) | ~10MB |
| + SSH Pool (10 conn) | +5MB |
| + Terminal | +15MB |
| + SFTP | +5MB |
| Standard 总计 | ~35MB |

## 示例代码

### 批量操作

```rust
use easyssh_core::{get_servers, ssh_connect, ssh_execute};

async fn batch_execute(state: &AppState, command: &str) -> Result<(), LiteError> {
    let servers = get_servers(state)?;

    let mut handles = vec![];
    for server in servers {
        let state_ref = state;
        let handle = tokio::spawn(async move {
            let metadata = ssh_connect(state_ref, &server.id, None).await?;
            let result = ssh_execute(state_ref, &metadata.session_id, command).await?;
            println!("{}: {}", server.name, result);
            ssh_disconnect(state_ref, &metadata.session_id).await
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.await??;
    }

    Ok(())
}
```

### 文件传输

```rust
#[cfg(feature = "sftp")]
use easyssh_core::{ssh_connect, ssh_create_sftp};

async fn upload_file(
    state: &AppState,
    server_id: &str,
    local_path: &str,
    remote_path: &str
) -> Result<(), LiteError> {
    let metadata = ssh_connect(state, server_id, None).await?;
    let sftp = ssh_create_sftp(state, &metadata.session_id).await?;

    let mut local_file = std::fs::File::open(local_path)?;
    let mut remote_file = sftp.create(Path::new(remote_path))?;

    std::io::copy(&mut local_file, &mut remote_file)?;

    Ok(())
}
```

## 调试 API

### 日志级别

```rust
// 启用调试日志
std::env::set_var("RUST_LOG", "easyssh_core=debug");

// 或使用 API
use easyssh_core::debug::set_log_level;
set_log_level(easyssh_core::debug::LogLevel::Debug);
```

### 健康检查

```rust
use easyssh_core::debug::health_check;

let report = health_check(&state).await?;
println!("Health: {:?}", report);
// HealthReport {
//     database: Ok,
//     ssh_pool: Ok,
//     keychain: Ok,
//     ...
// }
```

## 下一步

- [SSH 模块 API](/zh/api/core/ssh)
- [数据库 API](/zh/api/core/db)
- [加密 API](/zh/api/core/crypto)
- [FFI 接口](/zh/api/ffi)
