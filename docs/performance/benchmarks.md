# EasySSH 性能基准测试

## 概述

本文档定义 EasySSH 关键操作的性能基准，用于性能回归检测和持续优化。

## 测试场景

### 1. 冷启动时间 (Cold Start)

| 指标 | 目标 | 测试方法 |
|-----|------|---------|
| Lite版本 | < 500ms | 从进程启动到UI可用 |
| Standard版本 | < 800ms | 包含终端初始化 |
| Pro版本 | < 1000ms | 包含团队协作模块 |

**测试命令:**
```bash
# 使用自定义计时器
./target/release/easyssh-lite --benchmark-startup
```

### 2. 数据库查询性能

| 操作 | 目标 (<1000条) | 目标 (<10000条) |
|-----|---------------|-----------------|
| 单条读取 | < 1ms | < 1ms |
| 列表读取 | < 10ms | < 50ms |
| 批量插入(100条) | < 100ms | < 100ms |
| 搜索过滤 | < 20ms | < 100ms |

**测试命令:**
```bash
cargo bench --bench db_bench
```

### 3. 加密/解密性能

| 数据大小 | 加密目标 | 解密目标 |
|---------|---------|---------|
| 1 KB | < 0.5ms | < 0.3ms |
| 10 KB | < 1ms | < 0.5ms |
| 100 KB | < 5ms | < 3ms |
| 1 MB | < 30ms | < 20ms |

**密钥派生 (Argon2id):**
- 目标: < 200ms (默认参数)

**测试命令:**
```bash
cargo bench --bench crypto_bench
```

### 4. SSH连接性能

| 操作 | 目标 |
|-----|------|
| 连接池创建 | < 1µs |
| 会话复用 | < 10ms |
| 新连接建立 | < 500ms (网络依赖) |
| 命令执行延迟 | < 50ms (不含网络) |

**测试命令:**
```bash
cargo bench --bench ssh_bench
```

### 5. 搜索响应时间

| 数据量 | 简单搜索 | 复杂搜索 |
|-------|---------|---------|
| 100条 | < 1ms | < 5ms |
| 1000条 | < 5ms | < 20ms |
| 10000条 | < 20ms | < 100ms |

**测试命令:**
```bash
cargo bench --bench search_bench
```

## 性能基准框架

### 目录结构

```
crates/easyssh-core/benches/
├── crypto_bench.rs       # 加密/解密性能测试
├── db_bench.rs          # 数据库查询性能测试
├── ssh_bench.rs         # SSH连接性能测试
├── search_bench.rs      # 搜索响应时间测试
├── sftp_bench.rs        # SFTP传输性能测试
└── workflow_bench.rs    # 工作流执行性能测试
```

### 基准测试配置

```rust
// Cargo.toml [[bench]] 配置示例
[[bench]]
name = "crypto_bench"
harness = false

[[bench]]
name = "db_bench"
harness = false
```

### 运行所有基准测试

```bash
# 运行所有基准测试
cargo bench --manifest-path crates/easyssh-core/Cargo.toml

# 运行特定基准测试
cargo bench --bench crypto_bench
cargo bench --bench db_bench
cargo bench --bench ssh_bench
cargo bench --bench search_bench

# 保存基准线
cargo bench --bench crypto_bench -- --save-baseline baseline_v1

# 与基准线对比
cargo bench --bench crypto_bench -- --baseline baseline_v1
```

## CI集成

### GitHub Actions 性能回归检测

```yaml
# .github/workflows/benchmark.yml
name: Performance Regression Check

on:
  push:
    branches: [main]
  pull_request:
    branches: [main]

jobs:
  benchmark:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3

      - name: Run benchmarks
        run: |
          cargo bench --manifest-path crates/easyssh-core/Cargo.toml -- --save-baseline pr

      - name: Checkout main
        run: |
          git checkout main
          cargo bench --manifest-path crates/easyssh-core/Cargo.toml -- --baseline pr

      - name: Upload results
        uses: actions/upload-artifact@v3
        with:
          name: benchmark-results
          path: target/criterion/
```

### 性能报告

基准测试自动生成HTML报告:

```
target/criterion/
├── crypto_bench/
│   ├── report/index.html      # 总览报告
│   └── encryption/report/     # 具体测试报告
├── db_bench/
├── ssh_bench/
└── search_bench/
```

查看报告:
```bash
# 本地查看
cargo bench && open target/criterion/crypto_bench/report/index.html
```

## 性能回归检测

### 阈值设置

| 测试类别 | 回归阈值 | 报警阈值 |
|---------|---------|---------|
| 加密/解密 | +10% | +25% |
| 数据库查询 | +15% | +30% |
| SSH管理 | +10% | +20% |
| 搜索 | +20% | +40% |

### 自动化检测脚本

```bash
#!/bin/bash
# scripts/check_performance.sh

# 运行基准测试
cargo bench -- --save-baseline current 2>&1 | tee bench_output.txt

# 检查性能回归
if grep -q "Performance has regressed" bench_output.txt; then
    echo "⚠️  Performance regression detected!"
    exit 1
fi

if grep -q "Performance has improved" bench_output.txt; then
    echo "✅ Performance improvement detected!"
fi
```

## 性能优化指南

### 数据库优化

1. **索引优化**
   ```sql
   CREATE INDEX idx_servers_group ON servers(group_id);
   CREATE INDEX idx_servers_status ON servers(status);
   CREATE INDEX idx_servers_name ON servers(name);
   ```

2. **批量操作**
   ```rust
   // 使用事务批量插入
   db.transaction(|conn| {
       for server in servers {
           conn.execute(...)?;
       }
       Ok(())
   })?;
   ```

3. **连接池**
   ```rust
   // SQLx连接池配置
   let pool = SqlitePoolOptions::new()
       .max_connections(10)
       .min_connections(2)
       .connect("sqlite:easyssh.db").await?;
   ```

### 加密优化

1. **并行加密**
   ```rust
   // 大文件分块并行加密
   let chunks: Vec<_> = data.par_chunks(65536)
       .map(|chunk| encrypt_chunk(chunk))
       .collect();
   ```

2. **密钥缓存**
   ```rust
   // 避免重复派生密钥
   lazy_static! {
       static ref CRYPTO_STATE: Arc<RwLock<CryptoState>> = ...;
   }
   ```

### 搜索优化

1. **前缀索引**
   ```rust
   // 使用Trie结构
   let trie = Trie::from_servers(&servers);
   let results = trie.prefix_search("prod");
   ```

2. **缓存热门查询**
   ```rust
   let cache = LRUCache::new(100);
   if let Some(cached) = cache.get(&query) {
       return cached.clone();
   }
   ```

## 性能报告模板

### 月度性能报告

```markdown
# EasySSH 性能报告 - 2026年4月

## 测试环境
- OS: Windows 11 / macOS 14 / Ubuntu 22.04
- CPU: AMD Ryzen 9 / Apple M3 / Intel i7
- RAM: 32GB
- Rust: 1.75.0

## 测试结果
| 测试 | 目标 | 实际 | 状态 |
|-----|------|------|------|
| 冷启动 | <500ms | 450ms | 达标 |
| DB读取(1000条) | <10ms | 8ms | 达标 |
| 加密1MB | <30ms | 25ms | 达标 |
| 搜索1000条 | <20ms | 15ms | 达标 |

## 回归检测
- 无性能回归 ✅

## 优化建议
1. 数据库批量插入可优化20%
2. 考虑添加搜索缓存
```

## 工具与资源

### 性能分析工具

```bash
# 火焰图生成
cargo install flamegraph
cargo flamegraph --bench crypto_bench

# 内存分析
cargo bench --features heaptrack

# 缓存分析
cargo bench --features cachegrind
```

### 相关文档

- [Criterion.rs 文档](https://bheisler.github.io/criterion.rs/book/)
- [Rust性能优化指南](https://nnethercote.github.io/perf-book/)
- [SQLite性能优化](https://www.sqlite.org/optoverview.html)

## 更新历史

| 日期 | 版本 | 变更 |
|-----|------|------|
| 2026-04-02 | 1.0 | 初始版本 |
