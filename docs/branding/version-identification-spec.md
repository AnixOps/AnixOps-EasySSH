# EasySSH 版本标识系统设计规范

## 概述

本文档定义EasySSH Lite/Standard/Pro三个版本的统一标识系统规范，包括编译时版本标识、运行时版本检测、视觉标识和文件命名规范。

---

## 1. 版本架构

### 1.1 版本类型

| 版本 | 标识符 | 显示名 | 定位 | 主色调 |
|------|--------|--------|------|--------|
| **Lite** | `lite` | EasySSH Lite | 极简配置管理 + 原生终端 | `#10B981` (清新绿) |
| **Standard** | `standard` | EasySSH Standard | 嵌入式终端 + 分屏 + 监控 | `#3B82F6` (科技蓝) |
| **Pro** | `pro` | EasySSH Pro | 团队协作 + 审计 + SSO | `#8B5CF6` (尊贵紫) |

### 1.2 版本层级

```
Pro (tier=3)
  └── Standard (tier=2)
        └── Lite (tier=1)
```

升级路径：Lite → Standard → Pro（仅支持向上升级）

---

## 2. 实现代码

### 2.1 核心类型定义

```rust
// core/src/edition.rs

/// 版本类型枚举
pub enum Edition {
    Lite,      // 极简版
    Standard,  // 标准版
    Pro,       // 专业版
}

/// 构建类型
pub enum BuildType {
    Release,   // 正式版本
    Dev,       // 开发者模式
}

/// 完整版本信息
pub struct VersionInfo {
    pub edition: Edition,
    pub edition_name: String,
    pub edition_full_name: String,
    pub version: String,           // "0.3.0"
    pub build_type: BuildType,
    pub build_type_name: String,
    pub git_hash: Option<String>,
    pub build_time: String,
    pub features: Vec<String>,
    pub primary_color: String,     // 如 "#10B981"
    pub secondary_color: String,
    pub accent_color: String,
    pub tagline: String,
}
```

### 2.2 编译时版本确定

Cargo.toml feature定义：

```toml
[features]
default = ["lite", "dev-tools"]

# 基础版本
lite = ["dep:hex", "dev-tools"]

# 标准版包含Lite + 扩展功能
standard = [
    "lite",
    "embedded-terminal",
    "split-screen",
    "sftp",
    "monitoring",
    "remote-desktop",
    "log-monitor",
    "docker",
    "dev-tools"
]

# 专业版包含Standard + 企业功能
pro = [
    "standard",
    "team",
    "audit",
    "sso",
    "sync",
    "dev-tools"
]
```

编译时版本选择逻辑：

```rust
impl Edition {
    pub const fn current() -> Self {
        #[cfg(feature = "pro")]
        return Edition::Pro;
        #[cfg(all(feature = "standard", not(feature = "pro")))]
        return Edition::Standard;
        #[cfg(not(any(feature = "standard", feature = "pro")))]
        return Edition::Lite;
    }
}
```

### 2.3 构建类型确定

```rust
impl BuildType {
    pub const fn current() -> Self {
        #[cfg(debug_assertions)]
        return BuildType::Dev;
        #[cfg(not(debug_assertions))]
        return BuildType::Release;
    }
}
```

---

## 3. 视觉标识系统

### 3.1 色彩规范

#### Lite版 - 清新绿
- 主色：`#10B981` (emerald-500)
- 次色：`#34D399` (emerald-400)
- 强调：`#059669` (emerald-600)

#### Standard版 - 科技蓝
- 主色：`#3B82F6` (blue-500)
- 次色：`#60A5FA` (blue-400)
- 强调：`#2563EB` (blue-600)

#### Pro版 - 尊贵紫
- 主色：`#8B5CF6` (violet-500)
- 次色：`#A78BFA` (violet-400)
- 强调：`#7C3AED` (violet-600)

### 3.2 图标规范

应用图标命名：
```
icons/
├── icon-lite.png        # Lite版图标
├── icon-standard.png    # Standard版图标
├── icon-pro.png         # Pro版图标
├── icon-dev.png         # Dev模式覆盖层
└── icon.icns/.ico       # 平台特定格式
```

### 3.3 UI适配方案

#### Windows (WinUI3)
```cpp
// 获取版本信息并设置窗口标题
auto title = edition_get_window_title();
window.Title(winrt::to_hstring(title));
edition_free_string(title);

// 设置主题色
auto color = edition_get_primary_color_rgb();
// 转换为Windows.UI.Color
```

#### Linux (GTK4)
```c
// 获取版本信息
CVersionInfo* info = edition_get_version_info();
// 应用主题色
gtk_widget_set_name(window, info->edition_name);
// 释放资源
edition_free_version_info(info);
```

#### macOS (SwiftUI)
```swift
// 通过FFI获取版本信息
let info = edition_get_version_info()
// 应用主题色到SwiftUI
let color = Color(hex: String(cString: info.pointee.primary_color))
```

---

## 4. 文件命名规范

### 4.1 构建产物命名

格式：`easyssh-{edition}-{version}-{arch}.{ext}`

| 平台 | 文件名示例 |
|------|-----------|
| Windows (EXE) | `easyssh-lite-0.3.0-x64.exe` |
| Windows (MSI) | `easyssh-lite-0.3.0-x64.msi` |
| macOS (DMG) | `easyssh-standard-0.3.0-arm64.dmg` |
| Linux (AppImage) | `easyssh-standard-0.3.0-x64.AppImage` |
| Linux (DEB) | `easyssh-pro_0.3.0_amd64.deb` |
| Linux (RPM) | `easyssh-pro-0.3.0-1.x86_64.rpm` |

### 4.2 目录结构

```
target/
├── lite/           # Lite版本构建输出
│   ├── debug/
│   └── release/
├── standard/     # Standard版本构建输出
│   ├── debug/
│   └── release/
└── pro/          # Pro版本构建输出
    ├── debug/
    └── release/
```

### 4.3 包管理器命名

| 包管理器 | Lite | Standard | Pro |
|---------|------|----------|-----|
| Homebrew | `easyssh-lite` | `easyssh` | `easyssh-pro` |
| Chocolatey | `easyssh-lite` | `easyssh` | `easyssh-pro` |
| APT | `easyssh-lite` | `easyssh` | `easyssh-pro` |
| AUR | `easyssh-lite-bin` | `easyssh-bin` | `easyssh-pro-bin` |

---

## 5. 运行时版本检测

### 5.1 应用启动时

```rust
// 在应用初始化时显示版本信息
let info = VersionInfo::current();
log::info!("Starting {}", info.full_version_string());

// 设置窗口标题
window.set_title(&info.window_title());
```

### 5.2 关于对话框

```rust
pub fn get_about_info() -> String {
    let info = VersionInfo::current();
    format!(
        "{}\nVersion {}\nBuild Type: {}\n\n{}",
        info.edition_full_name,
        info.version,
        info.build_type_name,
        info.tagline
    )
}
```

### 5.3 日志和遥测

```rust
// 所有日志包含版本标识
log::info!("[{} {}] Session started", edition.name(), version);

// 遥测事件包含版本信息
telemetry.track_event("app_start", json!({
    "edition": edition.identifier(),
    "version": version,
    "build_type": build_type.name()
}));
```

---

## 6. FFI接口规范

### 6.1 C结构体定义

```c
typedef struct {
    int edition;           // 0=Lite, 1=Standard, 2=Pro
    int build_type;        // 0=Release, 1=Dev
    char* version;         // 版本号
    char* edition_name;    // 版本名称
    char* full_name;       // 完整名称
    char* primary_color;   // 主色调 (#RRGGBB)
    char* secondary_color; // 次色调
    char* accent_color;    // 强调色
    char* tagline;         // 版本描述
    int feature_count;     // 功能数量
    char** features;       // 功能列表
} CVersionInfo;
```

### 6.2 核心FFI函数

| 函数 | 说明 |
|------|------|
| `edition_get_version_info()` | 获取完整版本信息 |
| `edition_get_current()` | 获取当前版本类型 (0/1/2) |
| `edition_get_build_type()` | 获取构建类型 (0/1) |
| `edition_is_dev_mode()` | 是否开发者模式 |
| `edition_get_window_title()` | 获取窗口标题 |
| `edition_get_primary_color_rgb()` | 获取主色 (0xRRGGBB) |
| `edition_has_feature(feature)` | 检查功能支持 |
| `edition_free_string(s)` | 释放字符串 |
| `edition_free_version_info(info)` | 释放版本信息 |

---

## 7. 构建脚本

### 7.1 构建命令

```bash
# Lite版本
cargo build --features lite --target-dir target/lite

# Standard版本
cargo build --features standard --target-dir target/standard

# Pro版本
cargo build --features pro --target-dir target/pro

# 带Git hash的构建
EASYSSH_GIT_HASH=$(git rev-parse HEAD) cargo build --features standard
```

### 7.2 CI/CD集成

```yaml
# .github/workflows/build.yml
strategy:
  matrix:
    edition: [lite, standard, pro]
    target: [x86_64-pc-windows-msvc, x86_64-unknown-linux-gnu]

steps:
  - name: Build
    run: cargo build --features ${{ matrix.edition }} --release

  - name: Rename artifact
    run: |
      mv target/release/easyssh.exe \
         target/release/easyssh-${{ matrix.edition }}-${{ matrix.target }}.exe
```

---

## 8. 使用示例

### 8.1 Rust代码中检查功能

```rust
use easyssh_core::edition::Edition;
use easyssh_core::check_feature;

// 运行时检查
if Edition::current().has_embedded_terminal() {
    // 启用终端功能
}

// 编译时检查
if check_feature!(embedded_terminal) {
    // 编译期确定的功能代码
}
```

### 8.2 UI层调用（C++）

```cpp
// 初始化时设置主题
void SetupVersionTheme() {
    auto info = edition_get_version_info();

    // 应用主题色
    int primaryColor = edition_get_primary_color_rgb();
    ApplyThemeColor(primaryColor);

    // 设置窗口标题
    auto title = edition_get_window_title();
    SetWindowTitle(title);
    edition_free_string(title);

    edition_free_version_info(info);
}
```

### 8.3 版本升级检查

```rust
use easyssh_core::edition::{Edition, VersionComparator};

// 检查升级可行性
let current = Edition::Standard;
let target = Edition::Pro;

if target.can_upgrade_from(current) {
    println!("可以升级到Pro版");
}

// 版本号比较
let needs_update = VersionComparator::needs_update("0.2.0", "0.3.0");
```

---

## 9. 附录

### 9.1 宏定义

```rust
// 检查功能是否可用
macro_rules! check_feature {
    (embedded_terminal) => { cfg!(feature = "embedded-terminal") };
    (split_screen) => { cfg!(feature = "split-screen") };
    // ...
}

// 版本特定代码块
macro_rules! edition_match {
    (lite => $lite:expr, standard => $standard:expr, pro => $pro:expr) => {
        match Edition::current() {
            Edition::Lite => $lite,
            Edition::Standard => $standard,
            Edition::Pro => $pro,
        }
    };
}
```

### 9.2 版本标识速查表

| 属性 | Lite | Standard | Pro |
|------|------|----------|-----|
| 标识符 | `lite` | `standard` | `pro` |
| 短标识 | `L` | `S` | `P` |
| 层级 | 1 | 2 | 3 |
| 主色 | `#10B981` | `#3B82F6` | `#8B5CF6` |
| Bundle ID | `com.anixops.easyssh.lite` | `com.anixops.easyssh.standard` | `com.anixops.easyssh.pro` |
| 数据目录 | `easyssh-lite` | `easyssh-standard` | `easyssh-pro` |

---

*文档版本: 1.0*
*最后更新: 2026-04-02*
