# EasySSH 性能分析指南

> 性能监控、分析和优化工具使用指南

---

## 目录

1. [性能概述](#1-性能概述)
2. [性能监控](#2-性能监控)
3. [Rust 性能分析](#3-rust-性能分析)
4. [前端性能分析](#4-前端性能分析)
5. [数据库性能](#5-数据库性能)
6. [内存分析](#6-内存分析)
7. [网络性能](#7-网络性能)
8. [优化策略](#8-优化策略)

---

## 1. 性能概述

### 1.1 性能目标

| 指标 | 目标值 | 可接受范围 | 测量方法 |
|------|--------|-----------|----------|
| **启动时间** | < 2s | < 5s | 应用初始化时间 |
| **SSH 连接建立** | < 500ms | < 1s | 握手完成时间 |
| **终端响应延迟** | < 16ms | < 50ms | 输入到渲染 |
| **内存占用 (空闲)** | < 100MB | < 200MB | RSS 内存 |
| **内存占用 (100 连接)** | < 500MB | < 1GB | RSS 内存 |
| **CPU 占用 (空闲)** | < 1% | < 5% | 平均利用率 |
| **数据库查询** | < 10ms | < 100ms | 95th percentile |
| **文件传输速度** | > 80% 带宽 | > 50% 带宽 | 实际/理论 |

### 1.2 性能测试场景

```rust
// benches/scenarios.rs
use criterion::{criterion_group, criterion_main, Criterion};

fn scenario_startup(c: &mut Criterion) {
    c.bench_function("cold_startup", |b| {
        b.iter(|| {
            // 模拟冷启动
            let app = App::new();
            app.initialize();
            black_box(app)
        });
    });
}

fn scenario_multiple_connections(c: &mut Criterion) {
    let mut group = c.benchmark_group("multiple_connections");

    for count in [1, 10, 50, 100].iter() {
        group.bench_with_input(
            format!("{}_connections", count),
            count,
            |b, &count| {
                b.to_async(tokio::runtime::Runtime::new().unwrap())
                    .iter(|| async {
                        let connections = create_connections(count).await;
                        black_box(connections)
                    });
            },
        );
    }
    group.finish();
}

criterion_group!(scenarios, scenario_startup, scenario_multiple_connections);
criterion_main!(scenarios);
```

---

## 2. 性能监控

### 2.1 实时指标收集

```rust
// core/src/telemetry.rs
use std::sync::atomic::{AtomicU64, Ordering};
use metrics::{counter, gauge, histogram, Unit};

pub struct PerformanceMonitor {
    connection_count: AtomicU64,
    active_sessions: AtomicU64,
}

impl PerformanceMonitor {
    pub fn record_connection_established(&self) {
        counter!("ssh_connections_total").increment(1);
        gauge!("ssh_connections_active").increment(1.0);
    }

    pub fn record_connection_duration(&self, duration: Duration) {
        histogram!("ssh_connection_duration_seconds", Unit::Seconds)
            .record(duration.as_secs_f64());
    }

    pub fn record_terminal_latency(&self, latency: Duration) {
        histogram!("terminal_input_latency_ms", Unit::Milliseconds)
            .record(latency.as_millis() as f64);
    }

    pub fn record_memory_usage(&self) {
        if let Some(usage) = get_memory_usage() {
            gauge!("memory_usage_bytes", Unit::Bytes).set(usage as f64);
        }
    }
}

fn get_memory_usage() -> Option<usize> {
    #[cfg(target_os = "linux")]
    {
        use std::fs::read_to_string;
        let status = read_to_string("/proc/self/status").ok()?;
        for line in status.lines() {
            if line.starts_with("VmRSS:") {
                let kb = line.split_whitespace().nth(1)?.parse::<usize>().ok()?;
                return Some(kb * 1024);
            }
        }
    }
    #[cfg(target_os = "macos")]
    {
        // 使用 mach API 获取内存
    }
    None
}
```

### 2.2 Prometheus 导出

```rust
// 集成 metrics-exporter-prometheus
use metrics_exporter_prometheus::PrometheusBuilder;

pub fn setup_metrics_server(port: u16) {
    PrometheusBuilder::new()
        .with_http_listener(([0, 0, 0, 0], port))
        .install_recorder()
        .expect("Failed to setup metrics");
}

// 关键指标
const KEY_METRICS: &[&str] = &[
    "ssh_connections_total",
    "ssh_connections_active",
    "ssh_connection_duration_seconds",
    "terminal_input_latency_ms",
    "memory_usage_bytes",
    "db_query_duration_ms",
    "sftp_transfer_bytes_total",
    "sftp_transfer_duration_seconds",
];
```

### 2.3 性能仪表盘

```yaml
# Grafana 仪表盘配置 (docs/developers/grafana-dashboard.yml)
apiVersion: 1

datasources:
  - name: Prometheus
    type: prometheus
    url: http://localhost:9090
    isDefault: true

# 关键面板
panels:
  - title: SSH Connections
    type: graph
    targets:
      - expr: rate(ssh_connections_total[5m])
        legendFormat: "New connections/sec"
      - expr: ssh_connections_active
        legendFormat: "Active connections"

  - title: Terminal Latency
    type: heatmap
    targets:
      - expr: rate(terminal_input_latency_ms_bucket[5m])

  - title: Memory Usage
    type: graph
    targets:
      - expr: memory_usage_bytes
        legendFormat: "RSS Memory"
```

---

## 3. Rust 性能分析

### 3.1 火焰图分析

```bash
# 安装 flamegraph
cargo install flamegraph

# 生成火焰图
cargo flamegraph --bin easyssh-gtk4 --root

# 特定测试火焰图
cargo flamegraph --unit-test -p easyssh-core test_name

# 带参数运行
cargo flamegraph --bin easyssh-gtk4 -- --connect-to test-server

# 查看结果
# flamegraph.svg (在浏览器中打开)
```

### 3.2 Criterion 基准测试

```rust
// benches/crypto_bench.rs
use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use easyssh_core::crypto::{encrypt, decrypt, derive_key};

fn bench_encryption_sizes(c: &mut Criterion) {
    let key = derive_key("password", &[1u8; 16]);
    let sizes = vec![1024, 10240, 102400, 1048576]; // 1KB to 1MB

    let mut group = c.benchmark_group("encryption");

    for size in sizes {
        let data = vec![0u8; size];
        group.bench_with_input(
            BenchmarkId::new("encrypt", size),
            &data,
            |b, data| {
                b.iter(|| encrypt(black_box(data), black_box(&key)));
            },
        );
    }
    group.finish();
}

fn bench_key_derivation(c: &mut Criterion) {
    c.bench_function("derive_key_argon2", |b| {
        b.iter(|| {
            derive_key(
                black_box("password123"),
                black_box(&[1u8; 16])
            )
        });
    });
}

criterion_group!(crypto_benches, bench_encryption_sizes, bench_key_derivation);
criterion_main!(crypto_benches);
```

### 3.3 Coz 因果分析

```bash
# 安装 coz
git clone https://github.com/plasma-umass/coz.git
cd coz && cmake . && make && sudo make install

# 编译支持 coz 的版本
RUSTFLAGS="-g" cargo build --release

# 运行因果分析
coz run --- ./target/release/easyssh-gtk4

# 查看 coz.prof
```

### 3.4 异步性能分析

```rust
// core/src/async_profiler.rs
use tokio::runtime::{self, Runtime};

pub struct AsyncProfiler {
    runtime: Runtime,
}

impl AsyncProfiler {
    pub fn new() -> Self {
        let runtime = runtime::Builder::new_multi_thread()
            .enable_all()
            .thread_stack_size(4 * 1024 * 1024)
            .build()
            .unwrap();

        Self { runtime }
    }

    pub fn profile<F>(&self, name: &str, f: F)
    where
        F: std::future::Future,
    {
        let start = std::time::Instant::now();
        self.runtime.block_on(f);
        let elapsed = start.elapsed();

        tracing::info!("Profile [{}]: {:?}", name, elapsed);
    }
}

// 使用示例
#[cfg(feature = "profile")]
#[tokio::main]
async fn main() {
    let profiler = AsyncProfiler::new();

    profiler.profile("connection_setup", async {
        let client = SshClient::new();
        client.connect(&config).await;
    });
}
```

---

## 4. 前端性能分析

### 4.1 React 性能监控

```typescript
// src/utils/performance.ts
export class PerformanceMonitor {
  private metrics: Map<string, number[]> = new Map();

  measureRender(componentName: string, duration: number) {
    if (!this.metrics.has(componentName)) {
      this.metrics.set(componentName, []);
    }
    this.metrics.get(componentName)!.push(duration);

    if (duration > 16) { // 超过一帧
      console.warn(`[Performance] ${componentName} render took ${duration.toFixed(2)}ms`);
    }
  }

  getMetrics() {
    const result: Record<string, { avg: number; max: number; count: number }> = {};

    this.metrics.forEach((durations, name) => {
      const avg = durations.reduce((a, b) => a + b, 0) / durations.length;
      const max = Math.max(...durations);
      result[name] = { avg, max, count: durations.length };
    });

    return result;
  }
}

// Profiler 组件
import { Profiler, ProfilerOnRenderCallback } from 'react';

const onRender: ProfilerOnRenderCallback = (id, phase, actualDuration) => {
  performanceMonitor.measureRender(id, actualDuration);
};

// 使用
<Profiler id="ServerList" onRender={onRender}>
  <ServerList />
</Profiler>
```

### 4.2 Lighthouse CI

```javascript
// lighthouserc.js
module.exports = {
  ci: {
    collect: {
      url: ['http://localhost:1420'],
      numberOfRuns: 3,
    },
    assert: {
      assertions: {
        'categories:performance': ['error', { minScore: 0.9 }],
        'categories:accessibility': ['error', { minScore: 0.9 }],
        'first-contentful-paint': ['warn', { maxNumericValue: 2000 }],
        'interactive': ['error', { maxNumericValue: 4000 }],
      },
    },
    upload: {
      target: 'temporary-public-storage',
    },
  },
};
```

### 4.3 Web Vitals

```typescript
// src/utils/web-vitals.ts
import { getCLS, getFID, getFCP, getLCP, getTTFB, Metric } from 'web-vitals';

function sendToAnalytics(metric: Metric) {
  // 发送到监控系统
  fetch('/api/metrics/web-vitals', {
    method: 'POST',
    body: JSON.stringify({
      name: metric.name,
      value: metric.value,
      id: metric.id,
      delta: metric.delta,
    }),
  });
}

export function initWebVitals() {
  getCLS(sendToAnalytics);
  getFID(sendToAnalytics);
  getFCP(sendToAnalytics);
  getLCP(sendToAnalytics);
  getTTFB(sendToAnalytics);
}
```

---

## 5. 数据库性能

### 5.1 查询性能分析

```rust
// core/src/db/profiler.rs
use rusqlite::Connection;
use std::time::Instant;

pub struct QueryProfiler {
    slow_query_threshold: Duration,
}

impl QueryProfiler {
    pub fn new() -> Self {
        Self {
            slow_query_threshold: Duration::from_millis(100),
        }
    }

    pub fn profile<F, T>(&self, sql: &str, f: F) -> Result<T, rusqlite::Error>
    where
        F: FnOnce() -> Result<T, rusqlite::Error>,
    {
        let start = Instant::now();
        let result = f();
        let elapsed = start.elapsed();

        if elapsed > self.slow_query_threshold {
            tracing::warn!(
                "Slow query ({}ms): {}",
                elapsed.as_millis(),
                sql
            );
        }

        histogram!("db_query_duration_ms").record(elapsed.as_millis() as f64);

        result
    }
}

// 连接包装器
pub struct ProfiledConnection {
    conn: Connection,
    profiler: QueryProfiler,
}

impl ProfiledConnection {
    pub fn execute(&self, sql: &str, params: &[&dyn rusqlite::ToSql]) -> Result<usize, rusqlite::Error> {
        self.profiler.profile(sql, || {
            self.conn.execute(sql, params)
        })
    }
}
```

### 5.2 索引优化

```sql
-- migrations/performance_indexes.sql
-- 服务器查询索引
CREATE INDEX IF NOT EXISTS idx_servers_group_id ON servers(group_id);
CREATE INDEX IF NOT EXISTS idx_servers_name ON servers(name);
CREATE INDEX IF NOT EXISTS idx_servers_last_connected ON servers(last_connected_at DESC);

-- 会话查询索引
CREATE INDEX IF NOT EXISTS idx_sessions_server_id ON sessions(server_id);
CREATE INDEX IF NOT EXISTS idx_sessions_created_at ON sessions(created_at DESC);
CREATE INDEX IF NOT EXISTS idx_sessions_status ON sessions(status);

-- 审计日志索引
CREATE INDEX IF NOT EXISTS idx_audit_timestamp ON audit_logs(timestamp DESC);
CREATE INDEX IF NOT EXISTS idx_audit_user_id ON audit_logs(user_id);
CREATE INDEX IF NOT EXISTS idx_audit_action ON audit_logs(action);

-- 复合索引
CREATE INDEX IF NOT EXISTS idx_servers_group_name ON servers(group_id, name);
```

### 5.3 查询计划分析

```rust
// 分析查询性能
pub fn analyze_query_plan(conn: &Connection, sql: &str) -> Result<Vec<QueryPlan>, DbError> {
    let mut stmt = conn.prepare(&format!("EXPLAIN QUERY PLAN {}", sql))?;

    let plans = stmt.query_map([], |row| {
        Ok(QueryPlan {
            id: row.get(0)?,
            parent: row.get(1)?,
            detail: row.get(3)?,
        })
    })?.collect::<Result<Vec<_>, _>>()?;

    Ok(plans)
}

// 使用示例
let plan = analyze_query_plan(&conn,
    "SELECT * FROM servers WHERE group_id = 'xxx' ORDER BY name"
)?;

// 检查是否使用了索引
for step in plan {
    if step.detail.contains("SCAN") && !step.detail.contains("INDEX") {
        tracing::warn!("Full table scan detected: {}", step.detail);
    }
}
```

---

## 6. 内存分析

### 6.1 Valgrind Massif

```bash
# 内存使用分析
valgrind --tool=massif \
    --time-unit=ms \
    --max-snapshots=100 \
    target/debug/easyssh-gtk4

# 生成报告
ms_print massif.out.* > massif_report.txt

# 峰值内存分析
grep -A 20 "Peak" massif_report.txt
```

### 6.2 Heaptrack

```bash
# 记录内存分配
heaptrack target/debug/easyssh-gtk4

# 分析结果
heaptrack_gui heaptrack.easyssh-gtk4.*.gz

# 查找内存泄漏
heaptrack -a heaptrack.easyssh-gtk4.*.gz
```

### 6.3 jemalloc 统计

```toml
# Cargo.toml
[dependencies]
tikv-jemallocator = { version = "0.5", features = ["profiling", "stats"] }
tikv-jemalloc-ctl = "0.5"
```

```rust
// 使用 jemalloc
#[global_allocator]
static ALLOC: jemallocator::Jemalloc = jemallocator::Jemalloc;

// 内存统计
pub fn print_memory_stats() {
    let epoch = jemalloc_ctl::epoch::mib().unwrap();
    let allocated = jemalloc_ctl::stats::allocated::mib().unwrap();
    let resident = jemalloc_ctl::stats::resident::mib().unwrap();

    epoch.advance().unwrap();

    tracing::info!(
        "Memory stats - allocated: {}MB, resident: {}MB",
        allocated.read().unwrap() / 1024 / 1024,
        resident.read().unwrap() / 1024 / 1024
    );
}
```

### 6.4 内存泄漏检测

```rust
#[cfg(test)]
mod memory_tests {
    use super::*;

    #[tokio::test]
    async fn test_no_memory_leak_on_session_close() {
        let initial_memory = get_memory_usage();

        // 创建并关闭 100 个会话
        for i in 0..100 {
            let session = create_session(&format!("test-{}", i)).await;
            drop(session);
        }

        // 强制内存回收
        #[cfg(feature = "jemalloc")]
        jemalloc_ctl::epoch::advance().unwrap();

        tokio::time::sleep(Duration::from_millis(100)).await;

        let final_memory = get_memory_usage();
        let leaked = final_memory.saturating_sub(initial_memory);

        // 允许 5MB 的波动
        assert!(leaked < 5_000_000, "Memory leak detected: {} bytes", leaked);
    }
}
```

---

## 7. 网络性能

### 7.1 SSH 连接性能

```rust
// core/src/ssh/benchmark.rs
pub async fn benchmark_connection(config: &SshConfig) -> ConnectionMetrics {
    let start = Instant::now();

    // TCP 连接
    let tcp_start = Instant::now();
    let tcp = tokio::net::TcpStream::connect((config.host.as_str(), config.port))
        .await
        .unwrap();
    let tcp_time = tcp_start.elapsed();

    // SSH 握手
    let handshake_start = Instant::now();
    let session = establish_ssh_session(tcp).await.unwrap();
    let handshake_time = handshake_start.elapsed();

    // 认证
    let auth_start = Instant::now();
    authenticate(&session, config).await.unwrap();
    let auth_time = auth_start.elapsed();

    let total_time = start.elapsed();

    ConnectionMetrics {
        tcp_time,
        handshake_time,
        auth_time,
        total_time,
    }
}
```

### 7.2 SFTP 传输性能

```rust
pub async fn benchmark_sftp_transfer(
    session: &Session,
    size: usize,
) -> TransferMetrics {
    // 生成测试数据
    let data = vec![0u8; size];

    // 写入测试
    let write_start = Instant::now();
    let sftp = session.sftp().unwrap();
    let mut file = sftp.create(Path::new("/tmp/benchmark")).unwrap();
    file.write(&data).unwrap();
    let write_time = write_start.elapsed();

    // 读取测试
    let read_start = Instant::now();
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer).unwrap();
    let read_time = read_start.elapsed();

    TransferMetrics {
        write_throughput: size as f64 / write_time.as_secs_f64(),
        read_throughput: size as f64 / read_time.as_secs_f64(),
        write_time,
        read_time,
    }
}
```

### 7.3 网络延迟监控

```rust
use tokio::net::TcpStream;
use std::time::Duration;

pub async fn measure_latency(host: &str, port: u16, samples: usize) -> LatencyStats {
    let mut latencies = Vec::with_capacity(samples);

    for _ in 0..samples {
        let start = Instant::now();
        match timeout(Duration::from_secs(5), TcpStream::connect((host, port))).await {
            Ok(Ok(_)) => latencies.push(start.elapsed()),
            _ => latencies.push(Duration::from_secs(5)),
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }

    LatencyStats {
        min: *latencies.iter().min().unwrap(),
        max: *latencies.iter().max().unwrap(),
        avg: latencies.iter().sum::<Duration>() / latencies.len() as u32,
        p95: percentile(&latencies, 95),
        p99: percentile(&latencies, 99),
    }
}
```

---

## 8. 优化策略

### 8.1 启动时间优化

```rust
// 延迟加载策略
pub struct LazyInitializer<T> {
    value: OnceCell<T>,
    init: fn() -> T,
}

impl<T> LazyInitializer<T> {
    pub fn new(init: fn() -> T) -> Self {
        Self {
            value: OnceCell::new(),
            init,
        }
    }

    pub fn get(&self) -> &T {
        self.value.get_or_init(|| (self.init)())
    }
}

// 使用
static CRYPTO_ENGINE: LazyInitializer<CryptoEngine> =
    LazyInitializer::new(|| CryptoEngine::new());
```

### 8.2 连接池优化

```rust
pub struct ConnectionPool {
    connections: RwLock<Vec<PooledConnection>>,
    max_size: usize,
    idle_timeout: Duration,
}

impl ConnectionPool {
    pub async fn get(&self) -> Result<PooledConnection, PoolError> {
        // 尝试获取现有连接
        let mut connections = self.connections.write().await;

        // 清理过期连接
        connections.retain(|conn| {
            conn.last_used.elapsed() < self.idle_timeout
        });

        // 返回可用连接或创建新连接
        if let Some(conn) = connections.pop() {
            return Ok(conn);
        }

        // 检查连接数限制
        if connections.len() >= self.max_size {
            return Err(PoolError::MaxConnections);
        }

        drop(connections);
        self.create_connection().await
    }
}
```

### 8.3 渲染优化

```rust
// 虚拟化长列表
pub struct VirtualList<T> {
    items: Vec<T>,
    visible_range: Range<usize>,
    item_height: f64,
}

impl<T> VirtualList<T> {
    pub fn visible_items(&self) -> &[T] {
        &self.items[self.visible_range.clone()]
    }

    pub fn update_visible_range(&mut self, scroll_offset: f64, viewport_height: f64) {
        let start = (scroll_offset / self.item_height) as usize;
        let count = (viewport_height / self.item_height) as usize + 2; // 缓冲区

        self.visible_range = start..(start + count).min(self.items.len());
    }
}
```

### 8.4 编译优化

```toml
# Cargo.toml
[profile.release]
opt-level = 3
lto = "fat"
codegen-units = 1
strip = true
panic = "abort"

# 针对特定架构优化
[profile.release-x86_64]
inherits = "release"
rustflags = ["-C", "target-cpu=native"]

# 大小优化
[profile.release-lite]
inherits = "release"
opt-level = "z"
lto = true
```

---

## 9. 性能测试自动化

### 9.1 CI 性能回归检测

```yaml
# .github/workflows/performance.yml
name: Performance Regression

on: [pull_request]

jobs:
  benchmark:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Run benchmarks
        run: cargo bench --bench connection_bench | tee benchmark.txt

      - name: Compare with baseline
        uses: benchmark-action/github-action-benchmark@v1
        with:
          tool: 'cargo'
          output-file-path: benchmark.txt
          github-token: ${{ secrets.GITHUB_TOKEN }}
          auto-push: true
          alert-threshold: '150%'
          comment-on-alert: true
```

### 9.2 性能报告生成

```rust
// 生成性能报告
pub fn generate_performance_report() -> PerformanceReport {
    PerformanceReport {
        timestamp: Utc::now(),
        version: env!("CARGO_PKG_VERSION"),
        metrics: collect_metrics(),
        regressions: detect_regressions(),
        recommendations: generate_recommendations(),
    }
}
```

---

## 10. 相关文档

- [设置指南](./SETUP.md) - 环境配置
- [调试指南](./DEBUGGING.md) - 故障排查
- [测试指南](./TESTING.md) - 测试策略
- [故障排除指南](./TROUBLESHOOTING.md) - 常见问题

---

*最后更新: 2026-04-01*
