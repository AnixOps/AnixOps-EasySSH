# EasySSH 日志监控中心 - 实现文档

## 概述

全平台Agent #17第二波已完整实现集中式日志监控中心，参考ELK Stack、Grafana Loki、Datadog设计。

---

## 实现文件列表

### 核心Rust代码

| 文件 | 描述 |
|------|------|
| `core/src/log_monitor.rs` | 主日志监控模块（约1700行） |
| `core/src/log_monitor_ffi.rs` | FFI桥接层 |
| `core/Cargo.toml` | 添加了`log-monitor` feature |
| `core/src/lib.rs` | 导出日志监控类型 |

### 前端代码

| 文件 | 描述 |
|------|------|
| `ui/src/log-monitor-client.js` | WebSocket客户端库 |
| `ui/log-monitor.html` | 监控中心UI页面 |

---

## 功能实现清单

### 1. 多源日志监控 (已完成)
- **多服务器支持**: 同时监控多个服务器的日志
- **多种日志类型**:
  - Systemd Journal
  - Syslog
  - Application日志
  - Nginx/Apache日志
  - Docker容器日志
  - Kubernetes Pod日志
- **SSH流式获取**: 通过SSH连接实时获取日志
- **源管理**: 添加/删除/启用/禁用日志源

### 2. 实时流式传输 (已完成)
- **WebSocket服务**: 端口8765
- **广播机制**: 使用tokio::broadcast实现多播
- **消息类型**:
  - `NewEntry`: 单条日志
  - `BatchEntries`: 批量日志
  - `StatsUpdate`: 统计更新
  - `Alert`: 告警通知
  - `SourceConnected/Disconnected`: 源状态变更

### 3. 日志聚合 (已完成)
- **时间线聚合**: 按时间序列组织日志
- **多源合并**: 统一时间轴展示多个源日志
- **序列号管理**: 保证顺序性

### 4. 搜索过滤 (已完成)
- **全文搜索**: 关键词匹配
- **正则过滤**: 支持正则表达式
- **级别过滤**: TRACE/DEBUG/INFO/WARN/ERROR/FATAL
- **时间范围**: 按时间段筛选
- **源过滤**: 选择特定日志源
- **复合过滤**: 多条件组合

### 5. 告警规则 (已完成)
- **关键词告警**: 匹配特定关键词
- **级别阈值**: 错误级别触发
- **速率告警**: 单位时间内日志数量
- **模式匹配**: 正则表达式告警
- **复合条件**: AND/OR组合
- **冷却时间**: 防止告警风暴
- **多种动作**:
  - Webhook通知
  - 邮件通知
  - 桌面通知
  - 声音告警
  - 执行命令
  - 暂停流

### 6. 日志分析 (已完成)
- **错误模式识别**: 自动识别常见错误模式
- **异常检测**: 错误率突增检测
- **趋势分析**: 错误趋势计算
- **时间序列分析**: 60点时间段统计

### 7. 可视化图表 (已完成)
- **日志趋势图**: 堆叠柱状图展示
  - 红色: 错误日志
  - 黄色: 警告日志
  - 绿色: 信息日志
- **统计指标**:
  - 总日志数
  - 错误数
  - 警告数
  - 日志/分钟
  - 错误率

### 8. 导出存档 (已完成)
- **多种格式**:
  - JSON (结构化)
  - CSV (表格)
  - 纯文本
  - HTML (着色)
- **压缩选项**: Gzip压缩支持
- **过滤导出**: 只导出符合条件的日志

### 9. 日志轮转 (已完成)
- **自动清理**: 按保留时间清理旧日志
- **容量限制**: 最大100,000条限制
- **后台任务**: 每小时自动检查
- **手动触发**: API支持手动清理

### 10. 着色显示 (已完成)
- **级别颜色**:
  - TRACE: 灰色 (#6c757d)
  - DEBUG: 蓝色 (#0d6efd)
  - INFO: 绿色 (#198754)
  - WARN: 黄色 (#ffc107)
  - ERROR: 红色 (#dc3545)
  - FATAL: 深红色 (#721c24)
- **源颜色**: 每个日志源独立颜色
- **HTML导出**: 保留着色

---

## 核心数据结构

### LogEntry
```rust
pub struct LogEntry {
    pub id: String,              // 唯一ID
    pub source_id: String,     // 来源ID
    pub source_name: String,   // 来源名称
    pub timestamp: u64,        // Unix时间戳
    pub level: LogLevel,       // 日志级别
    pub message: String,       // 消息内容
    pub raw_line: String,      // 原始行
    pub metadata: HashMap,     // 元数据
    pub color: String,         // 显示颜色
    pub sequence: u64,         // 序列号
}
```

### LogSource
```rust
pub struct LogSource {
    pub id: String,
    pub name: String,
    pub server_id: String,     // SSH服务器ID
    pub log_path: String,      // 日志路径/服务名
    pub log_type: LogType,     // 日志类型
    pub parser_config: ParserConfig,
    pub color: String,
    pub enabled: bool,
    pub max_lines_per_batch: usize,
    pub poll_interval_ms: u64,
}
```

### LogFilter
```rust
pub struct LogFilter {
    pub min_level: LogLevel,
    pub keywords: Option<Vec<String>>,
    pub regex_pattern: Option<String>,
    pub start_time: Option<u64>,
    pub end_time: Option<u64>,
    pub source_ids: Vec<String>,
    pub limit: Option<usize>,
}
```

---

## API接口

### Rust API

```rust
// 创建监控中心
let center = LogMonitorCenter::new(ssh_manager);

// 添加日志源
let source = LogSource::new(name, server_id, log_path, log_type);
center.add_source(source).await?;

// 搜索
let filter = LogFilter::new()
    .with_min_level(LogLevel::WARN)
    .with_keywords(vec!["error".to_string()]);
let entries = center.search(&filter).await;

// 统计
let stats = center.get_stats(3600).await;

// 分析
let analysis = center.analyze(3600).await;

// 导出
let config = ExportConfig { ... };
center.export(&config, "/path/to/output.json").await?;
```

### FFI API

```c
// 创建/销毁
void* log_monitor_create();
void log_monitor_destroy(void* handle);

// 源管理
int log_monitor_add_source(void* handle, const char* name,
    const char* server_id, const char* log_path, int log_type,
    char* source_id_out, size_t source_id_len);
int log_monitor_remove_source(void* handle, const char* source_id);

// 查询
int log_monitor_search(void* handle, const char* filter_json,
    char* results_buffer, size_t buffer_len);
int log_monitor_get_stats(void* handle, uint64_t time_range_seconds,
    char* buffer, size_t buffer_len);
int log_monitor_analyze(void* handle, uint64_t time_range_seconds,
    char* buffer, size_t buffer_len);

// 订阅
void log_monitor_subscribe(void* handle, void (*callback)(const char*));
```

### WebSocket API

**客户端→服务端:**
```json
{"action": "search", "min_level": "WARN", "keywords": ["error"]}
{"action": "stats", "range_seconds": 3600}
{"action": "analyze", "range_seconds": 3600}
{"action": "get_sources"}
{"action": "get_alerts"}
```

**服务端→客户端:**
```json
{"type": "entry", "entry": {...}}
{"type": "batch", "entries": [...]}
{"type": "stats", "stats": {...}}
{"type": "alert", "alert": {...}}
```

---

## 前端JavaScript API

```javascript
const client = new LogMonitorClient('ws://127.0.0.1:8765');

// 连接
client.connect();

// 事件监听
client.on('onEntry', (entry) => console.log(entry));
client.on('onStats', (stats) => updateCharts(stats));
client.on('onAlert', (alert) => showNotification(alert));

// 过滤
client.setMinLevel('WARN');
client.setKeywords(['error', 'failed']);
client.setRegexPattern('Exception:.*');

// 导出
client.exportToFile('json', 'logs.json');
client.exportToFile('csv', 'logs.csv');
```

---

## Feature配置

在`core/Cargo.toml`中:

```toml
[features]
default = ["lite"]
lite = []
standard = ["lite", ..., "log-monitor"]  # 已启用
pro = ["standard", ...]
log-monitor = []
```

---

## 技术架构

```
LogMonitorCenter
├── sources: HashMap<LogSource>     # 日志源管理
├── entries: Vec<LogEntry>          # 日志存储(10万条)
├── alert_rules: Vec<LogAlertRule> # 告警规则
├── broadcast_tx: Sender           # WebSocket广播
├── ssh_manager: SshSessionManager # SSH连接池
└── active_streams: JoinHandle     # 后台流任务

LogMonitorWebSocketServer
├── center: Arc<LogMonitorCenter>
└── port: 8765

JavaScript Client
├── WebSocket连接
├── 日志缓冲区
├── 过滤器
└── 图表组件
```

---

## 使用示例

### 1. 启动WebSocket服务器

```rust
use easyssh_core::log_monitor::*;

let ssh_manager = SshSessionManager::new();
let center = Arc::new(LogMonitorCenter::new(ssh_manager));

// 添加日志源
let source = LogSource::new(
    "nginx".to_string(),
    "server-1".to_string(),
    "/var/log/nginx/access.log".to_string(),
    LogType::Nginx
);
center.add_source(source).await?;

// 启动WebSocket服务器
let ws_server = LogMonitorWebSocketServer::new(center.clone(), 8765);
ws_server.start().await?;
```

### 2. 前端使用

打开`ui/log-monitor.html`，点击"连接"按钮即可查看实时日志。

---

## 性能指标

- **最大日志源数**: 无限制（受SSH连接池限制）
- **单源轮询间隔**: 可配置（默认1000ms）
- **日志缓冲区**: 100,000条
- **WebSocket广播**: 10,000消息队列
- **导出限制**: 可配置大小

---

## 安全特性

- SSH密钥认证支持
- 日志数据本地存储
- 可选的导出加密
- 告警冷却防止风暴

---

## 待扩展功能

- [ ] Elasticsearch集成
- [ ] Grafana数据源插件
- [ ] 机器学习异常检测
- [ ] 日志关联分析
- [ ] 分布式追踪集成

---

**实现状态**: 核心功能已全部完成，可直接使用。
