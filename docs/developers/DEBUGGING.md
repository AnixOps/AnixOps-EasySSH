# EasySSH 调试指南

> 全面的调试技术、工具使用和故障排查方法

---

## 目录

1. [调试基础](#1-调试基础)
2. [Rust 调试](#2-rust-调试)
3. [前端调试](#3-前端调试)
4. [Tauri 调试](#4-tauri-调试)
5. [SSH 连接调试](#5-ssh-连接调试)
6. [数据库调试](#6-数据库调试)
7. [性能调试](#7-性能调试)
8. [远程调试](#8-远程调试)

---

## 1. 调试基础

### 1.1 日志级别配置

```rust
// 初始化日志 (core/src/lib.rs)
pub fn init_logging() {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::from_default_env()
                .add_directive("easyssh_core=debug".parse().unwrap())
                .add_directive("ssh2=info".parse().unwrap())
        )
        .with_target(true)
        .with_thread_ids(true)
        .init();
}
```

```bash
# 运行时设置日志级别
RUST_LOG=easyssh_core=debug,ssh2=info cargo run

# 详细日志
RUST_LOG=trace cargo run 2>&1 | tee debug.log
```

### 1.2 日志输出格式

```rust
use tracing::{debug, info, warn, error, span};

// 结构化日志
let span = span!(Level::INFO, "ssh_connect", host = %config.host);
let _enter = span.enter();

info!(connection_id = %id, "建立SSH连接");
debug!(command = ?cmd, "执行命令");

// 带上下文的错误
error!(error = ?e, "连接失败");
```

---

## 2. Rust 调试

### 2.1 使用 LLDB/GDB

```bash
# 启动调试会话
lldb target/debug/easyssh-gtk4

# 常用命令
(lldb) breakpoint set --name connect_ssh
(lldb) run
(lldb) frame variable
(lldb) next
(lldb) step
(lldb) continue
(lldb) quit
```

### 2.2 VS Code 调试配置

```json
// .vscode/launch.json
{
  "version": "0.2.0",
  "configurations": [
    {
      "type": "lldb",
      "request": "launch",
      "name": "Debug Unit Test",
      "cargo": {
        "args": ["test", "--no-run", "--lib"],
        "filter": {
          "name": "easyssh-core",
          "kind": "lib"
        }
      },
      "args": ["test_name"],
      "cwd": "${workspaceFolder}"
    },
    {
      "type": "lldb",
      "request": "launch",
      "name": "Debug GTK4 App",
      "cargo": {
        "args": ["build", "-p", "easyssh-gtk4"],
        "filter": {
          "name": "easyssh-gtk4",
          "kind": "bin"
        }
      },
      "env": {
        "RUST_LOG": "debug",
        "RUST_BACKTRACE": "1"
      }
    }
  ]
}
```

### 2.3 条件断点

```rust
// 代码中设置断点
#[cfg(debug_assertions)]
if session_id == "test-session" {
    // 在此设置断点
    std::hint::black_box(());
}
```

### 2.4 断言和不变量检查

```rust
// debug_assert 仅在 debug 模式生效
debug_assert!(!host.is_empty(), "主机名不能为空");
debug_assert!(port > 0 && port <= 65535, "端口范围错误: {}", port);

// 自定义不变量检查
#[cfg(debug_assertions)]
fn invariant_check(state: &AppState) {
    assert!(
        state.servers.len() >= state.groups.len(),
        "服务器数不能少于分组数"
    );
}
```

---

## 3. 前端调试

### 3.1 React DevTools

```bash
# 安装浏览器扩展
# Chrome: React Developer Tools
# Firefox: React Developer Tools

# 代码中启用 DevTools
import { setupDevTools } from './devtools';
if (process.env.NODE_ENV === 'development') {
    setupDevTools();
}
```

### 3.2 状态调试 (Zustand)

```typescript
// stores/debugMiddleware.ts
export const debugMiddleware = (store: any) => (set: any, get: any, api: any) =>
  (args: any) => {
    console.log('  [Zustand] Before:', get());
    set(args);
    console.log('  [Zustand] After:', get());
  };

// store 中使用
import { create } from 'zustand';
import { devtools } from 'zustand/middleware';

export const useServerStore = create(
  devtools(
    (set, get) => ({
      // ... store implementation
    }),
    { name: 'ServerStore' }
  )
);
```

### 3.3 性能分析

```typescript
// 组件渲染性能
import { Profiler } from 'react';

function onRenderCallback(
  id: string,
  phase: 'mount' | 'update',
  actualDuration: number
) {
  console.log(`${id} ${phase}: ${actualDuration.toFixed(2)}ms`);
}

// 使用
<Profiler id="ServerList" onRender={onRenderCallback}>
  <ServerList />
</Profiler>
```

---

## 4. Tauri 调试

### 4.1 WebView 调试

```bash
# 启用 WebView 开发者工具
# Windows: 自动可用
# macOS: 在 tauri.conf.json 中设置
tauri://localhost 右键 -> 检查元素

# Linux (WebKitGTK)
export WEBKIT_INSPECTOR_SERVER=127.0.0.1:9222
# 然后使用 Chrome DevTools 连接
```

### 4.2 Tauri 命令调试

```rust
#[tauri::command]
async fn debug_command(state: State<'_, AppState>) -> Result<String, String> {
    // 添加详细日志
    tracing::debug!("Command called with state: {:?}", state);

    // 模拟延迟便于调试
    #[cfg(debug_assertions)]
    tokio::time::sleep(std::time::Duration::from_secs(2)).await;

    match internal_logic().await {
        Ok(result) => {
            tracing::info!("Command succeeded: {:?}", result);
            Ok(result)
        }
        Err(e) => {
            tracing::error!("Command failed: {:?}", e);
            Err(e.to_string())
        }
    }
}
```

### 4.3 进程间通信调试

```rust
// 在 main.rs 中启用 IPC 日志
#[cfg(debug_assertions)]
fn setup_ipc_logging() {
    tauri::Builder::default()
        .setup(|app| {
            app.listen_global("tauri://event", |event| {
                tracing::debug!("IPC Event: {:?}", event);
            });
            Ok(())
        });
}
```

---

## 5. SSH 连接调试

### 5.1 SSH2 库调试

```rust
use ssh2::Session;

pub fn create_debug_session() -> Session {
    let mut session = Session::new().unwrap();

    // 启用详细日志
    session.set_banner("EasySSH Debug Client").unwrap();

    // 调试握手过程
    tracing::debug!("Starting SSH handshake...");

    session
}

// 捕获详细错误
pub async fn connect_with_debug(config: &SshConfig) -> Result<Session, SshError> {
    let tcp = tokio::net::TcpStream::connect((config.host.as_str(), config.port))
        .await
        .map_err(|e| {
            tracing::error!("TCP连接失败: {}:{}, 错误: {:?}", config.host, config.port, e);
            SshError::ConnectionFailed(e.to_string())
        })?;

    // 更多调试...
}
```

### 5.2 密钥认证调试

```rust
#[cfg(debug_assertions)]
pub fn debug_key_auth(key_path: &Path) -> Result<(), KeyError> {
    tracing::debug!("加载密钥: {:?}", key_path);

    // 检查文件存在
    if !key_path.exists() {
        return Err(KeyError::NotFound(key_path.to_path_buf()));
    }

    // 检查权限
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let meta = std::fs::metadata(key_path)?;
        let mode = meta.permissions().mode();
        tracing::debug!("密钥文件权限: {:o}", mode);

        if mode & 0o077 != 0 {
            tracing::warn!("密钥文件权限过于开放");
        }
    }

    // 尝试解析密钥
    let content = std::fs::read_to_string(key_path)?;
    tracing::debug!("密钥内容长度: {}", content.len());

    Ok(())
}
```

### 5.3 连接问题诊断

```rust
pub async fn diagnose_connection(config: &SshConfig) -> ConnectionDiagnostics {
    let mut results = Vec::new();

    // 1. DNS 解析
    match tokio::net::lookup_host((config.host.as_str(), config.port)).await {
        Ok(addrs) => results.push(DiagnosticStep::DnsResolved(addrs.collect())),
        Err(e) => results.push(DiagnosticStep::DnsFailed(e.to_string())),
    }

    // 2. TCP 连接
    match tokio::time::timeout(
        Duration::from_secs(5),
        tokio::net::TcpStream::connect((config.host.as_str(), config.port))
    ).await {
        Ok(Ok(_)) => results.push(DiagnosticStep::TcpConnected),
        Ok(Err(e)) => results.push(DiagnosticStep::TcpFailed(e.to_string())),
        Err(_) => results.push(DiagnosticStep::TcpTimeout),
    }

    // 3. SSH 握手
    // ...

    ConnectionDiagnostics { steps: results }
}
```

---

## 6. 数据库调试

### 6.1 SQL 查询日志

```rust
use rusqlite::{Connection, OpenFlags};

pub fn create_debug_connection(path: &Path) -> Result<Connection, DbError> {
    let conn = Connection::open_with_flags(
        path,
        OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_CREATE
    )?;

    // 启用查询日志
    #[cfg(debug_assertions)]
    {
        conn.execute_batch("
            PRAGMA query_only = 0;
            PRAGMA foreign_keys = ON;
        ")?;

        // 自定义日志处理器
        conn.profile(Some(|sql, duration| {
            if duration.as_millis() > 100 {
                tracing::warn!("慢查询 ({}ms): {}", duration.as_millis(), sql);
            } else {
                tracing::debug!("查询 ({}ms): {}", duration.as_micros(), sql);
            }
        }));
    }

    Ok(conn)
}
```

### 6.2 数据库状态检查

```rust
#[cfg(debug_assertions)]
pub fn debug_database_state(conn: &Connection) -> Result<DbState, DbError> {
    let tables: Vec<String> = conn
        .prepare("SELECT name FROM sqlite_master WHERE type='table'")?
        .query_map([], |row| row.get(0))?
        .collect::<Result<Vec<_>, _>>()?;

    let mut table_info = HashMap::new();
    for table in &tables {
        let count: i64 = conn.query_row(
            &format!("SELECT COUNT(*) FROM {}", table),
            [],
            |row| row.get(0)
        )?;
        table_info.insert(table.clone(), count);
    }

    tracing::debug!("数据库表状态: {:?}", table_info);

    Ok(DbState { tables, table_info })
}
```

---

## 7. 性能调试

### 7.1 火焰图生成

```bash
# 安装 cargo-flamegraph
cargo install flamegraph

# 生成火焰图
cargo flamegraph --bin easyssh-gtk4

# 或针对特定测试
cargo flamegraph --unit-test -p easyssh-core test_name

# 查看结果
# flamegraph.svg
```

### 7.2 内存分析

```bash
# 使用 valgrind (Linux)
valgrind --tool=massif target/debug/easyssh-gtk4
ms_print massif.out.* > memory_report.txt

# 使用 heaptrack (Linux)
heaptrack target/debug/easyssh-gtk4
heaptrack_gui heaptrack.easyssh-gtk4.*.gz
```

### 7.3 异步任务调试

```rust
use tokio::runtime::Handle;

#[cfg(debug_assertions)]
pub async fn debug_async_tasks() {
    let handle = Handle::current();

    // 获取运行时指标
    let metrics = handle.metrics();

    tracing::debug!(
        "Active tasks: {}, Blocking tasks: {}, Idle threads: {}",
        metrics.active_tasks_count(),
        metrics.blocking_tasks_count(),
        metrics.idle_blocking_threads_count()
    );
}
```

---

## 8. 远程调试

### 8.1 远程 LLDB 调试

```bash
# 在目标机器上启动调试服务器
lldb-server platform --listen "*:1234" --server

# 本地连接
lldb
target remote 192.168.1.100:1234
file target/debug/easyssh-gtk4
breakpoint set --name main
continue
```

### 8.2 日志远程收集

```rust
use tracing_appender::rolling::{RollingFileAppender, Rotation};

pub fn setup_remote_logging() {
    let file_appender = RollingFileAppender::new(
        Rotation::DAILY,
        "/var/log/easyssh",
        "debug.log"
    );

    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);

    tracing_subscriber::fmt()
        .with_writer(non_blocking)
        .with_ansi(false)
        .init();

    // 保持 guard 存活
    std::mem::forget(_guard);
}
```

---

## 9. 调试工具箱

### 9.1 常用 Cargo 工具

```bash
# 代码覆盖率
cargo install cargo-tarpaulin
cargo tarpaulin --out Html

# 性能基准测试
cargo install cargo-criterion

# 依赖检查
cargo install cargo-tree
cargo tree -d  # 查看重复依赖

# 安全审计
cargo install cargo-audit
cargo audit

# 死代码检测
cargo install cargo-udeps
cargo +nightly udeps
```

### 9.2 调试宏集合

```rust
// 调试宏
core/src/debug_macros.rs

#[macro_export]
macro_rules! debug_var {
    ($var:expr) => {
        tracing::debug!("{} = {:?}", stringify!($var), $var)
    };
}

#[macro_export]
macro_rules! debug_enter {
    ($fn:expr) => {
        tracing::debug!(">>> Entering: {}", $fn);
        let _guard = $crate::debug::FnGuard($fn);
    };
}

// 使用示例
fn complex_function() {
    debug_enter!("complex_function");
    debug_var!(config);
    // ... 代码
}
```

---

## 10. 相关文档

- [设置指南](./SETUP.md) - 环境配置
- [测试指南](./TESTING.md) - 测试策略
- [性能分析指南](./PROFILING.md) - 性能优化
- [故障排除指南](./TROUBLESHOOTING.md) - 常见问题

---

*最后更新: 2026-04-01*
