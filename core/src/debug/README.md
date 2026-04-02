# EasySSH 三版本Debug功能统一说明

## 概述

本文档描述了Lite、Standard、Pro三个版本的Debug功能统一实现方案。

## 核心架构

### 统一的Debug入口

所有版本现在使用相同的Debug核心模块，通过 `dev-tools` feature 控制编译：

```rust
// Cargo.toml
[features]
dev-tools = []

lite = ["dep:hex", "dev-tools"]
standard = ["lite", "embedded-terminal", ..., "dev-tools"]
pro = ["standard", "team", "audit", "sso", "sync", "dev-tools"]
```

### 访问控制层

Debug功能访问通过 `debug_access` 模块控制：

```rust
// 激活方式（各版本不同）
Lite:     Ctrl+Shift+D 连续按3次 (3秒窗口)
Standard: Ctrl+Alt+Shift+D 单次
Pro:      管理后台开关 或 API调用
```

## 功能矩阵

| 功能 | Lite | Standard | Pro | 访问级别 |
|------|------|----------|-----|---------|
| **Health Check** | ✅ | ✅ | ✅ | Viewer |
| **Log Viewer (基础)** | ✅ | ✅ | ✅ | Viewer |
| **Network Check** | ✅ | ✅ | ✅ | Viewer |
| **Performance Monitor** | ⚠️ 基础 | ✅ 完整 | ✅ 完整 | Developer |
| **AI Programming** | ⚠️ 只读 | ✅ 完整 | ✅ 完整 | Developer |
| **Test Runner** | ❌ | ✅ | ✅ | Developer |
| **Database Console** | ❌ | ✅ | ✅ | Admin |
| **Feature Flags** | ❌ | ✅ | ✅ | Admin |
| **Audit Log Viewer** | ❌ | ❌ | ✅ | Admin |
| **Memory Profiler** | ❌ | ⚠️ 基础 | ✅ 完整 | Developer |
| **Packet Capture** | ❌ | ❌ | ✅ | Admin |

**图例**: ✅ 完整功能 | ⚠️ 受限功能 | ❌ 不可用

## 实现对比

### 旧实现 (#[cfg(debug_assertions)])

```rust
// 问题：只在debug构建中可用
#[cfg(debug_assertions)]
pub mod ai_programming;

#[cfg(not(debug_assertions))]
pub mod ai_programming {
    // 占位实现，所有函数返回错误
}
```

### 新实现 (dev-tools feature)

```rust
// 解决方案：通过runtime feature flag控制
#[cfg(feature = "dev-tools")]
pub mod debug;

// 通过debug_access控制是否可用
pub fn can_access_feature(feature: DebugFeature) -> bool {
    // 运行时检查，所有版本可用
}
```

## 模块结构

```
core/src/debug/
├── mod.rs              # 公共接口，导出debug_access类型
├── access.rs           # 访问控制包装（从debug_access重新导出）
├── ai_integration.rs   # AI编程接口（从ai_programming迁移）
├── commands.rs         # 测试命令（debug_test_*函数）
├── features.rs         # 功能特性定义
├── network.rs          # 网络检查工具
├── performance.rs      # 性能监控
├── types.rs            # 共享类型定义
├── database_console.rs # 数据库控制台（条件编译）
└── logging.rs          # 日志系统

core/src/
├── debug_access.rs     # 核心访问控制（原有）
├── debug_access_ffi.rs # FFI接口（原有）
└── lib.rs              # 整合导出
```

## API变化

### 旧API（保留向后兼容）

```rust
// 仍可用，但建议使用新的统一API
use easyssh_core::ai_programming::{ai_read_code, ai_search_code};
use easyssh_core::{debug_test_all, debug_quick_check};
```

### 新推荐API

```rust
// 统一导入
core/src/debug/mod.rs

// 初始化
debug::init_debug(Edition::current());

// 检查能力
let caps = debug::get_debug_capabilities();
if caps.ai_programming {
    // 使用AI功能
}

// 检查特定功能权限
if debug::can_access_feature(DebugFeature::AiProgramming) {
    // 执行操作
}
```

## 版本特定行为

### Lite版本

```rust
// 可用功能
let caps = debug::get_debug_capabilities();
assert!(caps.network_check);     // ✅
assert!(caps.log_viewer);        // ✅ 基础
assert!(!caps.ai_programming);   // ❌ 需要激活
assert!(!caps.test_runner);      // ❌
assert!(!caps.database_console); // ❌

// 激活后
let detector = debug::create_lite_key_detector();
detector.record_press("ctrl+shift+d"); // 需3次
// 激活后可使用：health_check, network_check, 基础log_viewer
```

### Standard版本

```rust
// 激活后可用
let caps = debug::get_debug_capabilities();
assert!(caps.ai_programming);      // ✅
assert!(caps.performance_monitor); // ✅
assert!(caps.test_runner);         // ✅
assert!(caps.database_console);    // ✅
assert!(!caps.audit_logs);         // ❌ 仅Pro

// 激活方式
let detector = debug::create_standard_key_detector();
detector.record_press("ctrl+alt+shift+d"); // 单次
```

### Pro版本

```rust
// 全部功能可用
let caps = debug::get_debug_capabilities();
assert!(caps.ai_programming);      // ✅
assert!(caps.audit_logs);          // ✅ 仅Pro
assert!(caps.feature_flags);       // ✅

// 激活方式更多
- 管理后台开关
- API调用（需token）
- 组合键（同Standard）
```

## 迁移检查清单

### 对开发者的影响

- [ ] 使用 `cargo build --features dev-tools` 编译带Debug功能的版本
- [ ] 使用 `debug::init_debug()` 初始化，而非旧的直接调用
- [ ] 检查 `debug::can_access_feature()` 而非编译时条件
- [ ] 更新CI/CD脚本，添加 `--features dev-tools` 参数

### 对用户的影响

- [ ] Lite版本现在可以通过快捷键激活基础Debug功能
- [ ] Standard版本保留完整Debug功能
- [ ] Pro版本新增审计日志查看功能

### 向后兼容性

- [x] 旧API `ai_programming::*` 仍然可用
- [x] `debug_test_*` 函数保留
- [x] `#[cfg(debug_assertions)]` 模式暂时保留（警告期）
- [ ] 计划v1.0时移除旧的条件编译代码

## 性能影响

| 方面 | 影响 | 说明 |
|------|------|------|
| 编译时间 | 轻微增加 | dev-tools feature增加约5%编译时间 |
| 运行时 | 无影响 | 功能默认禁用，零开销 |
| 内存 | < 1MB | 仅激活时分配资源 |
| 包大小 | +100KB | 包含debug模块代码 |

## 安全注意事项

1. **审计日志**: Pro版本记录所有Debug功能访问
2. **自动超时**: 默认1-4小时自动退出（版本不同）
3. **功能隔离**: 数据库操作需要Admin级别
4. **显式指示器**: Debug模式启用时UI显示明显标识

## 相关文档

- [debug-interface.md](../../docs/standards/debug-interface.md) - 详细API文档
- [TROUBLESHOOTING.md](../../docs/developers/TROUBLESHOOTING.md) - 故障排查
- [CLAUDE.md](../../CLAUDE.md) - 项目总览

---

最后更新: 2026-04-02
