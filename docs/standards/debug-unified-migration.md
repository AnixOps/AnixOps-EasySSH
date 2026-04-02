# Debug功能统一迁移计划

## 目标

将基于 `#[cfg(debug_assertions)]` 的Debug功能迁移到基于 `feature = "dev-tools"` 的统一实现。

## 当前状态

### 已迁移的模块

- ✅ `core/src/debug/mod.rs` - 统一入口
- ✅ `core/src/debug/access.rs` - 访问控制包装
- ✅ `core/src/debug/ai_integration.rs` - AI编程接口
- ✅ `core/src/debug/commands.rs` - 测试命令
- ✅ `core/src/debug/features.rs` - 功能定义
- ✅ `core/src/debug/network.rs` - 网络检查
- ✅ `core/src/debug/performance.rs` - 性能监控
- ✅ `core/src/debug/types.rs` - 共享类型
- ✅ `core/src/debug/database_console.rs` - 数据库控制台
- ✅ `core/src/debug/logging.rs` - 日志系统
- ✅ `core/Cargo.toml` - dev-tools feature
- ✅ `core/src/lib.rs` - 导出更新

### 现有模块（继续使用）

- ✅ `core/src/debug_access.rs` - 核心访问控制
- ✅ `core/src/debug_access_ffi.rs` - FFI接口
- ✅ `core/src/debug_ws.rs` - WebSocket服务

### 待处理

- 🔄 `core/src/ai_programming.rs` - 需要更新以使用debug_access

## 迁移步骤

### Phase 1: 基础设施 (已完成)

1. ✅ 创建 `debug/` 目录结构
2. ✅ 添加 `dev-tools` feature
3. ✅ 实现核心模块
4. ✅ 更新 `Cargo.toml`

### Phase 2: 整合 (已完成)

1. ✅ 更新 `lib.rs` 导出
2. ✅ 保持向后兼容
3. ✅ 添加新API导出

### Phase 3: 测试 (待进行)

1. 🔄 验证Lite版本功能
2. 🔄 验证Standard版本功能
3. 🔄 验证Pro版本功能
4. 🔄 测试激活方式
5. 🔄 测试访问控制

### Phase 4: 弃用 (计划中)

1. 📋 标记 `#[cfg(debug_assertions)]` 代码为已弃用
2. 📋 添加编译警告
3. 📋 更新文档
4. 📋 等待v1.0移除

## 代码迁移示例

### 旧代码（条件编译）

```rust
#[cfg(debug_assertions)]
fn run_debug_test() {
    ai_programming::debug_test_all().unwrap();
}

#[cfg(not(debug_assertions))]
fn run_debug_test() {
    // 空实现
}
```

### 新代码（运行时检查）

```rust
fn run_debug_test() {
    if debug::can_access_feature(DebugFeature::TestRunner) {
        debug::commands::test_all().unwrap();
    }
}
```

## API迁移对照表

| 旧API | 新API | 状态 |
|-------|-------|------|
| `ai_programming::ai_read_code` | `debug::ai_integration::read_code` | ✅ |
| `ai_programming::ai_search_code` | `debug::ai_integration::search_code` | ✅ |
| `ai_programming::ai_run_tests` | `debug::ai_integration::run_tests` | ✅ |
| `ai_programming::debug_test_all` | `debug::commands::test_all` | ✅ |
| `ai_programming::debug_test_db` | `debug::commands::test_db` | ✅ |
| `ai_programming::debug_test_crypto` | `debug::commands::test_crypto` | ✅ |
| `ai_programming::debug_test_ssh` | `debug::commands::test_ssh` | ✅ |
| `ai_programming::debug_test_terminal` | `debug::commands::test_terminal` | ✅ |
| `ai_programming::debug_test_pro` | `debug::commands::test_pro` | ✅ |
| `ai_programming::ai_health_check` | `debug::health_check` | ✅ |

## 测试计划

### 单元测试

```bash
# Lite版本测试
cargo test --features lite --package easyssh-core debug::

# Standard版本测试
cargo test --features standard --package easyssh-core debug::

# Pro版本测试
cargo test --features pro --package easyssh-core debug::
```

### 集成测试

```bash
# 测试激活流程
cargo test --features dev-tools test_activation

# 测试功能访问控制
cargo test --features dev-tools test_access_control
```

## 时间线

| 阶段 | 时间 | 任务 |
|------|------|------|
| Phase 1 | 2026-04-02 | 基础设施完成 ✅ |
| Phase 2 | 2026-04-02 | 整合完成 ✅ |
| Phase 3 | 2026-04-03 | 测试验证 🔄 |
| Phase 4 | 2026-04-05 | 弃用警告 📋 |
| 移除旧代码 | v1.0 | 移除条件编译 📋 |

## 风险评估

| 风险 | 概率 | 影响 | 缓解措施 |
|------|------|------|---------|
| 向后兼容性破坏 | 低 | 高 | 保留旧API，添加弃用警告 |
| 性能下降 | 低 | 中 | 功能默认禁用，零开销 |
| 安全漏洞 | 中 | 高 | 审计日志，自动超时，权限控制 |
| 编译失败 | 低 | 高 | CI测试所有版本组合 |

## 回滚计划

如果迁移出现问题：

1. 恢复 `lib.rs` 中的旧导出
2. 移除 `dev-tools` feature
3. 保留新模块但不默认启用
4. 在 `README.md` 中记录问题

---

负责人: AI Assistant
创建时间: 2026-04-02
