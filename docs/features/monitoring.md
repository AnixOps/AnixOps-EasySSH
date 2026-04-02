# EasySSH Standard 版本 - 监控模块

## 概述

监控模块是 EasySSH Standard 版本的核心功能之一，提供轻量级、实时的服务器性能监控。

## 架构

```
crates/easyssh-core/src/monitoring/
├── mod.rs           # 模块入口，导出所有类型
├── metrics.rs       # 指标数据结构 (SystemMetrics, ServerMetrics)
├── collector.rs     # 指标收集器 (MetricsCollector, SimpleCollector)
├── session.rs       # 监控会话管理 (MonitoringSession)
├── alerts.rs        # 告警引擎 (AlertEngine)
├── storage.rs       # 历史存储 (MetricsStorage)
├── dashboard.rs     # 图表数据生成
├── notifications.rs # 通知管理
├── topology.rs      # 网络拓扑
└── sla.rs           # SLA 监控
```

## 核心数据结构

### SystemMetrics（Standard 版本专用）

简化的实时监控数据结构：

```rust
pub struct SystemMetrics {
    pub cpu_percent: f32,       // CPU 使用率 (0-100%)
    pub memory_used: u64,       // 内存使用 (bytes)
    pub memory_total: u64,      // 总内存 (bytes)
    pub disk_used: u64,         // 磁盘使用 (bytes)
    pub disk_total: u64,        // 总磁盘 (bytes)
    pub network_rx: u64,        // 网络接收 (bytes)
    pub network_tx: u64,        // 网络发送 (bytes)
    pub load_avg: [f32; 3],     // 负载平均值 (1min, 5min, 15min)
    pub timestamp: DateTime<Utc>,
}
```

### MonitoringSession

单服务器监控会话管理：

```rust
pub struct MonitoringSession {
    pub server_id: String,
    pub ssh_session: Session,
    pub metrics_history: Vec<SystemMetrics>,
    pub alert_rules: Vec<AlertRule>,
}
```

## 数据来源

监控数据通过直接读取 `/proc` 文件系统获取，避免执行命令：

| 指标 | 来源 |
|------|------|
| CPU | `/proc/stat` |
| 内存 | `/proc/meminfo` |
| 磁盘 | `df` + `/proc/diskstats` |
| 网络 | `/proc/net/dev` |
| 负载 | `/proc/loadavg` |

## 使用示例

### 1. 创建监控会话

```rust
use easyssh_core::monitoring::{
    MonitoringSession, AuthMethod
};

// 创建会话
let session = MonitoringSession::with_interval("server-001".to_string(), 5);

// 配置连接
session.configure_connection(
    "192.168.1.100".to_string(),
    22,
    "admin".to_string(),
    AuthMethod::PrivateKey {
        key_path: "~/.ssh/id_rsa".to_string(),
        passphrase: None,
    },
).await;

// 连接并启动监控
session.connect().await?;
session.start().await?;
```

### 2. 获取实时指标

```rust
// 获取最新指标
if let Some(metrics) = session.get_latest_metrics().await {
    println!("CPU: {:.1}%", metrics.cpu_percent);
    println!("Memory: {:.1}%", metrics.memory_percent());
    println!("Load: {:?}", metrics.load_avg);
}

// 获取历史数据
let history = session.get_history(Some(60)).await;
```

### 3. 生成图表数据

```rust
use easyssh_core::monitoring::ChartData;

// 生成 Sparkline 数据
let sparkline = ChartData::generate_sparkline(&history, "cpu");

// 生成时间序列
let timeseries = ChartData::generate_timeseries(&history, "memory");

// 计算统计信息
let stats = ChartData::calculate_stats(&history, "disk");
println!("Min: {:.1}%, Max: {:.1}%, Avg: {:.1}%",
    stats.min, stats.max, stats.avg);
```

### 4. 配置告警规则

```rust
use easyssh_core::monitoring::{
    AlertRule, AlertCondition, AlertSeverity, MetricType
};

let rule = AlertRule {
    id: "high-cpu".to_string(),
    name: "High CPU Usage".to_string(),
    enabled: true,
    severity: AlertSeverity::Warning,
    metric_type: MetricType::CpuUsage,
    condition: AlertCondition::GreaterThan,
    threshold: 80.0,
    duration_secs: 300,    // 持续 5 分钟
    cooldown_secs: 600,    // 冷却 10 分钟
    server_ids: vec!["server-001".to_string()],
    tags: vec!["infrastructure".to_string()],
    notification_channels: vec!["email".to_string()],
    auto_resolve: true,
    resolve_after_secs: 600,
    created_at: 0,
    updated_at: 0,
};

session.add_alert_rule(rule).await?;
```

## 监控指标

### 1. CPU 使用率

- 数据来源: `/proc/stat`
- 计算方式: 总 CPU 时间 - 空闲时间
- 更新频率: 5 秒

### 2. 内存使用率

- 数据来源: `/proc/meminfo`
- 计算方式: `MemTotal - MemFree - Buffers - Cached`
- 更新频率: 5 秒

### 3. 磁盘使用率

- 数据来源: `df -B1 /`
- 计算方式: 已用空间 / 总空间
- 更新频率: 30 秒

### 4. 网络 IO

- 数据来源: `/proc/net/dev`
- 计算方式: 间隔时间内的字节差值
- 更新频率: 5 秒

### 5. 负载平均值

- 数据来源: `/proc/loadavg`
- 指标: 1分钟、5分钟、15分钟平均值
- 更新频率: 5 秒

## 性能优化

1. **最小开销**: 只读取需要的 /proc 文件
2. **批量读取**: 使用单个 SSH 命令获取所有指标
3. **本地计算**: 在本地解析数据，减少服务器负载
4. **增量更新**: 网络流量使用增量计算

## 历史存储

- 内存存储: 默认保存 8640 个点（24 小时 @ 10 秒间隔）
- 数据库存储: SQLite 支持长期历史查询
- 数据保留: 支持配置保留策略

## 告警机制

### 告警条件

- `GreaterThan`: 大于阈值
- `GreaterThanOrEqual`: 大于等于阈值
- `LessThan`: 小于阈值
- `LessThanOrEqual`: 小于等于阈值
- `Equal`: 等于阈值
- `Between`: 在范围内
- `Outside`: 在范围外

### 告警状态

- `Active`: 活跃告警
- `Acknowledged`: 已确认
- `Resolved`: 已解决
- `Silenced`: 静默

## 集成示例

### 与 UI 集成

```rust
// 获取最新的图表数据
let history = session.get_history(Some(60)).await;

// CPU 曲线图
let cpu_data = ChartData::generate_timeseries(&history, "cpu");

// 资源对比图
let comparison = ChartData::generate_resource_comparison(&history);

// 统计信息
let stats = ChartData::calculate_stats(&history, "memory");
```

### 与告警系统集成

```rust
// 检查当前指标是否触发告警
match metrics.overall_health() {
    ServerHealthStatus::Critical => {
        // 发送紧急通知
    }
    ServerHealthStatus::Warning => {
        // 发送警告通知
    }
    _ => {}
}
```

## 安全考虑

1. **SSH 密钥认证**: 推荐使用密钥而非密码
2. **最小权限**: 只需要读取 /proc 的权限
3. **连接复用**: 复用 SSH 连接减少开销
4. **数据加密**: 所有数据传输通过 SSH 加密

## 限制与约束

1. **Linux 专用**: /proc 文件系统仅在 Linux 上可用
2. **SSH 访问**: 需要 SSH 连接权限
3. **网络延迟**: 监控频率受网络延迟影响
4. **资源占用**: 高频监控会增加服务器负载

## 开发计划

- [x] 基础监控会话 (`MonitoringSession`)
- [x] 指标收集 (`/proc` 文件解析)
- [x] 图表数据生成 (`ChartData`)
- [x] 告警引擎 (`AlertEngine`)
- [x] 历史存储 (`MetricsStorage`)
- [ ] macOS 支持（使用 `sysctl`）
- [ ] Windows 支持（使用 WMI）
- [ ] 多服务器聚合监控
- [ ] 预测性告警

## 参考文档

- `/proc` 文件系统: https://www.kernel.org/doc/html/latest/filesystems/proc.html
- SSH2 协议: https://tools.ietf.org/html/rfc4251
