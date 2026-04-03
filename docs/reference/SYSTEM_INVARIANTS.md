# EasySSH 系统不变量与设计约束 (v1.0)

> **本文档约束所有未来实现**。任何修改必须遵守这些不变量。
>
> **目标读者：开发者与 AI Agent**。不要绕过这些规则。
>
> **核心原则**：系统中的核心实体（Connection、Session、Terminal、SFTP、Forward）有严格的依赖和生命周期关系。违反这些关系会导致资源泄漏、静默数据损坏或安全漏洞。

---

## 0. 核心架构约束（最高优先级）

### 0.1 Strong Consistency Sync（强一致性同步）

**定义**：前端/平台层状态必须与核心层状态保持绝对一致，通过事件驱动的主动拉取实现。

**不变量**：
- **任何连接状态变更必须触发 `connection_state_changed` 事件**
- **平台层收到事件后必须调用 `refresh_connections()` 获取最新快照**
- **禁止平台层维护连接状态的"缓存副本"**

```
Backend State Change → emit connection_state_changed → refresh_connections() → Latest Snapshot → Platform UI
```

**必须触发 `connection_state_changed` 的场景**：

| 场景 | 触发者 | trigger 值 |
|------|--------|-----------|
| SSH 连接建立 | `connect()` | `user_action` |
| 心跳失败 | Heartbeat Task | `heartbeat_fail` |
| 自动重连成功 | Reconnect Task | `reconnect_success` |
| 用户断开 | `disconnect()` | `user_action` |
| 空闲超时 | Idle Timer | `idle_timeout` |
| 跳板机级联故障 | `propagate_link_down()` | `cascade_fail` |

### 0.2 Key-Driven Reset（键驱动重置）

**定义**：利用 UI 框架的 key 机制，当连接 ID 变化时物理销毁旧组件。

**不变量**：
- **所有依赖连接状态的 UI 组件必须使用包含 `connection_id` 的 key**
- **组件销毁时必须清理所有句柄和订阅**
- **组件重建时必须从全局 Memory Map 恢复上下文**

**必须使用 Key-Driven Reset 的组件**：

| 组件 | Key 格式 | Memory Map |
|------|---------|------------|
| `TerminalView` | `{connection_id}-{terminal_id}` | N/A |
| `SFTPView` | `sftp-{connection_id}` | 按 `connection_id` 键存储路径 |
| `ForwardsView` | `forwards-{connection_id}` | N/A |

### 0.3 State Gating（状态门禁）

**定义**：所有 IO 操作必须在连接状态为 `active` 时才能执行。

**不变量**：
- **API 调用前必须检查 `connection.state === ConnectionState::Active`**
- **状态非 Active 时必须返回错误，禁止发送请求**
- **后端同样执行门禁检查，双重保护**

```rust
// 门禁实现
pub fn check_connection_ready(conn: &Connection) -> Result<()> {
    match conn.state {
        ConnectionState::Active => Ok(()),
        _ => Err(EasySSHError::ConnectionNotReady(conn.id.clone())),
    }
}
```

### 0.4 Resource Ownership（资源所有权）

**定义**：每个资源有且只有一个所有者，明确生命周期边界。

**不变量**：
- **SSH Connection 拥有 Session、SFTP、Forward 的生命周期**
- **Session 销毁时必须级联销毁所有子资源**
- **禁止跨 Connection 共享资源句柄**

```
Connection
├── Session (owned)
│   ├── Channels (owned)
│   └── Terminal (owned)
├── SFTP (owned)
│   └── File Handles (owned)
└── PortForwards (owned)
    └── ForwardChannels (owned)
```

---

## 1. 终端子系统约束

### 1.1 PTY 生命周期

**不变量**：
- **PTY 必须在 Connection Active 后创建**
- **PTY 销毁时必须先关闭主通道，再释放资源**
- **PTY 输出回调不能阻塞主线程**

```rust
// 正确的 PTY 生命周期
impl TerminalSession {
    pub async fn create(conn: &Connection) -> Result<Self> {
        check_connection_ready(conn)?;
        // 创建 PTY...
    }

    pub async fn destroy(&mut self) {
        self.close_main_channel().await;  // 先关闭通道
        self.release_resources().await;    // 再释放资源
    }
}
```

### 1.2 滚动缓冲区

**不变量**：
- **滚动缓冲区大小有上限（默认 10000 行）**
- **超出上限时采用 FIFO 策略丢弃旧行**
- **搜索操作不能阻塞输出处理**

### 1.3 终端重连

**不变量**：
- **重连时必须重新创建 PTY 通道**
- **重连后滚动缓冲区内容保留（用户可见历史）**
- **重连失败超过最大次数后标记 Connection 为 Failed**

---

## 2. SSH 连接约束

### 2.1 连接状态机

```
┌─────────┐    connect()    ┌─────────┐
│  Idle   │ ──────────────► │Connecting│
└─────────┘                 └────┬────┘
     ▲                           │
     │                     ┌─────┴─────┐
     │                     │           │
     │                  success     failure
     │                     │           │
     │                     ▼           ▼
     │              ┌─────────┐  ┌──────────┐
     │              │  Active │  │  Failed  │
     │              └────┬────┘  └──────────┘
     │                   │
     │            disconnect()
     │                   │
     └───────────────────┘
```

**不变量**：
- **状态转换必须是原子的**
- **Failed 状态必须保存错误原因**
- **重连只能在 Failed 或 Idle 状态下触发**

### 2.2 连接池

**不变量**：
- **每个目标服务器最多一个 Connection 实例**
- **连接池大小有上限（默认 100）**
- **空闲连接超时后自动断开（默认 30 分钟）**

### 2.3 认证安全

**不变量**：
- **密码/私钥必须存储在系统钥匙串，禁止明文存储**
- **认证失败后必须清除内存中的敏感数据**
- **私钥文件权限必须为 600（Unix）或受限（Windows）**

---

## 3. SFTP 约束

### 3.1 文件传输

**不变量**：
- **传输必须支持断点续传（记录 offset）**
- **传输取消时必须清理临时文件**
- **传输超时后必须重试（最多 3 次）**

### 3.2 路径安全

**不变量**：
- **所有路径必须经过规范化（禁止 `..` 路径穿越）**
- **符号链接必须解析到真实路径**
- **禁止访问用户主目录之外的敏感路径（可配置）**

---

## 4. 端口转发约束

### 4.1 转发规则

**不变量**：
- **本地端口监听失败时必须返回明确错误**
- **远程端口转发必须检查服务器权限**
- **转发通道关闭时必须清理本地监听端口**

### 4.2 跳板机级联

**不变量**：
- **级联转发必须按顺序建立（从跳板机到目标）**
- **任意一级失败时必须回滚已建立的连接**
- **级联深度有上限（默认 5 层）**

---

## 5. 自动重连约束

### 5.1 重连编排器

**不变量**：
- **重连延迟采用指数退避（base * 2^attempt）**
- **最大重连次数有上限（默认 10 次）**
- **最大延迟有上限（默认 60 秒）**
- **用户主动断开时不触发自动重连**

```rust
pub struct ReconnectConfig {
    pub max_retries: u32,      // 默认 10
    pub base_delay: Duration,  // 默认 1s
    pub max_delay: Duration,   // 默认 60s
    pub jitter: f64,           // 默认 0.3 (30% 抖动)
}

impl ReconnectConfig {
    pub fn calculate_delay(&self, attempt: u32) -> Duration {
        let delay = self.base_delay * 2u32.pow(attempt);
        let delay = delay.min(self.max_delay);
        let jitter = delay.mul_f64(1.0 + (rand() - 0.5) * 2.0 * self.jitter);
        jitter
    }
}
```

### 5.2 心跳检测

**不变量**：
- **心跳间隔可配置（默认 30 秒）**
- **心跳超时后标记连接为不稳定**
- **连续 3 次心跳失败后触发重连**

---

## 6. 多平台 UI 约束

### 6.1 平台抽象

**不变量**：
- **所有 UI 操作必须通过 Platform trait**
- **禁止直接访问平台特定 API（除 Platform 实现）**
- **事件回调必须在主线程执行**

```rust
pub trait Platform: Send + Sync {
    // 连接管理
    fn show_connection_dialog(&self) -> Option<ConnectionConfig>;
    fn update_connection_list(&self, connections: Vec<Connection>);

    // 终端
    fn create_terminal_view(&self, id: &str) -> Box<dyn TerminalView>;
    fn destroy_terminal_view(&self, id: &str);

    // SFTP
    fn create_sftp_view(&self, id: &str) -> Box<dyn SFTPView>;
    fn destroy_sftp_view(&self, id: &str);

    // 通知
    fn show_notification(&self, title: &str, message: &str);
    fn show_error(&self, title: &str, message: &str);
}
```

### 6.2 状态同步

**不变量**：
- **平台层状态变更必须通过 Core API**
- **Core 状态变更必须通过事件通知平台层**
- **禁止平台层直接修改 Core 数据结构**

---

## 7. 错误处理约束

### 7.1 错误传播

**不变量**：
- **所有错误必须包含上下文信息（操作、目标、原因）**
- **网络错误必须区分临时/永久**
- **用户错误必须提供可操作的建议**

```rust
pub enum EasySSHError {
    Connection(ConnectionError),
    SFTP(SFTPError),
    Terminal(TerminalError),
    Config(ConfigError),
    Platform(PlatformError),
}

impl EasySSHError {
    pub fn is_retryable(&self) -> bool {
        match self {
            Self::Connection(e) => e.is_retryable(),
            Self::SFTP(e) => e.is_retryable(),
            _ => false,
        }
    }

    pub fn user_suggestion(&self) -> Option<&str> {
        match self {
            Self::Connection(ConnectionError::AuthFailed) => 
                Some("请检查用户名和密码，或尝试使用密钥认证"),
            Self::Connection(ConnectionError::HostUnreachable) => 
                Some("请检查网络连接和服务器地址"),
            _ => None,
        }
    }
}
```

### 7.2 错误恢复

**不变量**：
- **可恢复错误必须自动重试（带退避）**
- **不可恢复错误必须通知用户并保存日志**
- **资源泄漏必须记录到诊断日志**

---

## 8. 性能约束

### 8.1 响应时间

| 操作 | 目标延迟 | 最大延迟 |
|------|----------|----------|
| 终端字符输出 | < 10ms | 50ms |
| 终端输入响应 | < 5ms | 20ms |
| SFTP 目录列表 | < 100ms | 1s |
| 连接建立 | < 2s | 10s |
| 文件传输启动 | < 500ms | 2s |

### 8.2 资源限制

| 资源 | Lite 限制 | Standard 限制 | Pro 限制 |
|------|-----------|---------------|----------|
| 最大连接数 | 10 | 50 | 500 |
| 最大终端数 | 5 | 20 | 100 |
| 滚动缓冲区 | 5000 行 | 10000 行 | 50000 行 |
| 最大传输队列 | 3 | 10 | 50 |

---

## 9. 安全约束

### 9.1 数据保护

**不变量**：
- **敏感数据（密码、私钥）内存中使用后必须清零**
- **日志中禁止记录敏感数据**
- **配置文件加密存储（Standard+）**

### 9.2 网络安全

**不变量**：
- **SSH 主机密钥必须验证（首次询问，后续校验）**
- **禁止明文传输认证信息**
- **支持 FIPS 140-2 加密套件（Pro 版本）**

---

## 10. 测试约束

### 10.1 测试覆盖

**不变量**：
- **核心模块单元测试覆盖率 > 80%**
- **所有错误路径必须有测试**
- **集成测试必须使用 mock 服务器**

### 10.2 测试隔离

**不变量**：
- **测试之间必须完全隔离（无共享状态）**
- **测试必须可重复运行**
- **禁止测试连接真实服务器（除非明确标记）**

---

## 版本历史

| 版本 | 日期 | 变更 |
|------|------|------|
| 1.0 | 2026-04-03 | 初始版本，参考 OxideTerm SYSTEM_INVARIANTS |

---

*本文档由 EasySSH 架构团队创建 - 2026-04-03*