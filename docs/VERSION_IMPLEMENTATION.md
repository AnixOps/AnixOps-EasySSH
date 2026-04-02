# EasySSH 三版本标识系统实现

## 实现概述

本文档描述了三版本（Lite/Standard/Pro）标识系统的核心代码实现。

## 文件结构

### 核心模块

| 文件 | 说明 |
|------|------|
| `core/src/version.rs` | 版本系统扩展模块，补充平台信息和构建信息 |
| `core/src/version_ffi.rs` | 版本系统FFI接口，供各平台UI调用 |
| `core/build.rs` | 构建脚本，收集Git信息、构建日期等 |
| `core/Cargo.toml` | 已更新，添加`build.rs`引用和`chrono`构建依赖 |

### 平台集成示例

| 文件 | 说明 |
|------|------|
| `examples/version_integration_windows.rs` | Windows egui版本显示组件 |
| `examples/version_integration_gtk4.rs` | Linux GTK4版本对话框 |
| `examples/version_integration_tui.rs` | TUI启动横幅和状态栏 |

## 核心类型

### PlatformInfo（平台信息）

```rust
pub struct PlatformInfo {
    pub os: String,      // 操作系统
    pub arch: String,    // CPU架构
    pub family: String,  // 操作系统家族
}
```

### FullBuildInfo（完整构建信息）

```rust
pub struct FullBuildInfo {
    pub version_info: VersionInfo,    // 来自 edition 模块
    pub git_branch: Option<String>,   // Git分支
    pub build_date: String,           // 构建日期
    pub rustc_version: Option<String>, // Rust编译器版本
    pub platform: PlatformInfo,       // 平台信息
    pub user_agent: String,           // User-Agent字符串
    pub version_id: String,          // 版本标识符
}
```

### VersionCompatibility（版本兼容性）

```rust
impl VersionCompatibility {
    /// 检查版本兼容性
    pub fn is_compatible(from: Edition, to: Edition) -> bool;

    /// 获取迁移建议
    pub fn migration_advice(from: Edition, to: Edition) -> &'static str;

    /// 获取升级路径步骤
    pub fn upgrade_path(from: Edition, to: Edition) -> Vec<String>;
}
```

## FFI接口

### C结构体

```c
// 平台信息
typedef struct {
    char* os;
    char* arch;
    char* family;
    int is_windows;
    int is_macos;
    int is_linux;
    int is_64bit;
} CPlatformInfo;

// 完整构建信息
typedef struct {
    char* git_branch;
    char* build_date;
    char* rustc_version;
    CPlatformInfo* platform;
    char* user_agent;
    char* version_id;
} CFullBuildInfo;
```

### 主要函数

```c
// 获取平台信息
CPlatformInfo* version_get_platform_info();
void version_free_platform_info(CPlatformInfo* info);

// 获取构建信息
CFullBuildInfo* version_get_build_info();
void version_free_build_info(CFullBuildInfo* info);

// 获取字符串信息
char* version_get_user_agent();           // User-Agent
char* version_get_version_id();           // 版本ID
char* version_get_build_date();           // 构建日期
char* version_get_git_branch();           // Git分支
char* version_get_rustc_version();          // Rust版本
char* version_get_verbose_info();         // 详细信息
char* version_get_summary();              // 版本摘要
char* version_get_build_info_json();      // JSON格式

// 兼容性检查
int version_check_compatibility(int from_edition, int to_edition);
char* version_get_migration_advice(int from_edition, int to_edition);

// 释放字符串
void version_free_string(char* s);
```

## 构建脚本输出

构建时自动设置的环境变量：

- `EASYSSH_GIT_HASH` - Git commit hash
- `EASYSSH_GIT_BRANCH` - Git分支名
- `EASYSSH_BUILD_DATE` - 构建日期 (YYYY-MM-DD)
- `EASYSSH_BUILD_TIME` - 构建时间 (HH:MM:SS UTC)
- `EASYSSH_RUSTC_VERSION` - Rust编译器版本

## 使用示例

### Rust代码

```rust
use easyssh_core::version::{FullBuildInfo, PlatformInfo, VersionCompatibility};
use easyssh_core::edition::{Edition, VersionInfo};

// 获取完整构建信息
let build_info = FullBuildInfo::current();
println!("User-Agent: {}", build_info.user_agent);
println!("Platform: {}", build_info.platform.display());

// 检查版本兼容性
let compatible = VersionCompatibility::is_compatible(
    Edition::Lite,
    Edition::Standard
);

// 获取迁移建议
let advice = VersionCompatibility::migration_advice(
    Edition::Standard,
    Edition::Pro
);
```

### C代码

```c
#include <stdio.h>

// 获取版本信息
char* version_id = version_get_version_id();
printf("Version: %s\n", version_id);
version_free_string(version_id);

// 检查兼容性
int can_upgrade = version_check_compatibility(0, 1); // Lite -> Standard
if (can_upgrade) {
    printf("可以升级\n");
}
```

## 与现有系统的关系

```
edition 模块（原有）           version 模块（新增）
     │                              │
     ├── Edition enum               ├── PlatformInfo
     ├── BuildType                  ├── FullBuildInfo
     ├── VersionInfo                └── VersionCompatibility
     ├── AppIdentity
     ├── VersionComparator
     └── edition_ffi
```

两个模块互补：
- `edition` 提供核心版本类型、颜色、功能检查
- `version` 提供平台信息、构建信息、兼容性分析

## 集成指南

### Windows egui集成

```rust
// 标题栏显示版本
let info = FullBuildInfo::current();
let (text, color) = match info.version_info.edition {
    Edition::Lite => ("Lite", egui::Color32::TEAL),
    Edition::Standard => ("Standard", egui::Color32::BLUE),
    Edition::Pro => ("Pro", egui::Color32::PURPLE),
};
ui.colored_label(color, text);
```

### Linux GTK4集成

```rust
// 关于对话框
gtk4::AboutDialog::builder()
    .program_name("EasySSH")
    .version(&info.version_info.version)
    .comments(&info.version_info.edition.tagline())
    .build();
```

### TUI集成

```rust
// 启动横幅
let info = FullBuildInfo::current();
writeln!(stdout, "EasySSH {} Edition v{}",
    info.version_info.edition.name(),
    info.version_info.version
);
```

## 版本升级路径

| 从 | 到 | 兼容性 | 注意事项 |
|----|----|----|----------|
| Lite | Standard | ✓ | 平滑升级，所有配置保留 |
| Lite | Pro | ✓ | 平滑升级，建议配置团队权限 |
| Standard | Pro | ✓ | 平滑升级，建议启用审计日志 |
| Standard | Lite | ✓ | 降级注意：分屏、监控不可用 |
| Pro | Lite | ✓ | 降级警告：团队数据将丢失！ |
| Pro | Standard | ✗ | 不兼容：团队协作数据丢失 |

## 测试

运行模块测试：

```bash
cd core
cargo test version --features lite
```

## 构建

构建特定版本：

```bash
# Lite版本
cargo build -p easyssh-core --features lite

# Standard版本
cargo build -p easyssh-core --features standard

# Pro版本
cargo build -p easyssh-core --features pro
```

## 特性对照

| 特性 | Lite | Standard | Pro |
|------|------|----------|-----|
| SSH连接 | ✓ | ✓ | ✓ |
| 密钥管理 | ✓ | ✓ | ✓ |
| 原生终端 | ✓ | ✓ | ✓ |
| 嵌入式终端 | ✗ | ✓ | ✓ |
| 分屏功能 | ✗ | ✓ | ✓ |
| SFTP传输 | ✗ | ✓ | ✓ |
| 服务器监控 | ✗ | ✓ | ✓ |
| 团队协作 | ✗ | ✗ | ✓ |
| 审计日志 | ✗ | ✗ | ✓ |
| SSO集成 | ✗ | ✗ | ✓ |
