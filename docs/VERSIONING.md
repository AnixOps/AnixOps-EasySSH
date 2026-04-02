# EasySSH 版本标识规范

## 概述

EasySSH 采用三版本战略：Lite、Standard、Pro。每个版本有不同的功能集和目标用户群体。版本通过 Cargo feature flags 在编译时确定。

## 三个版本

### Lite (极简版)

| 属性 | 值 |
|------|-----|
| 目标用户 | 注重隐私的开发者 |
| 核心价值 | SSH配置保险箱 |
| 技术特点 | 原生终端唤起，无嵌入式终端 |
| 功能范围 | 基础SSH + 密钥管理 |

**包含功能:**
- SSH密码/密钥认证
- SSH Agent支持
- Keychain集成
- 服务器分组(单层)
- 原生终端唤起(外部终端)
- 主密码保护
- 配置导入/导出

**编译特性:** `lite`

---

### Standard (标准版)

| 属性 | 值 |
|------|-----|
| 目标用户 | 多服务器管理者 |
| 核心价值 | 全功能客户端 |
| 技术特点 | 嵌入式终端，分屏，监控 |
| 功能范围 | Lite + 高级功能 |

**包含功能:**
- 所有Lite功能
- 嵌入式Web终端(xterm.js + WebGL)
- 多标签页/分屏(golden-layout)
- SFTP文件管理
- 服务器监控小组件
- Agent转发
- ProxyJump
- 自动重连
- 嵌套服务器分组
- 批量操作
- 导入 ~/.ssh/config

**编译特性:** `standard` (自动包含 `lite`)

---

### Pro (专业版)

| 属性 | 值 |
|------|-----|
| 目标用户 | IT团队/企业 |
| 核心价值 | 团队协作平台 |
| 技术特点 | 团队管理，审计，SSO |
| 功能范围 | Standard + 企业功能 |

**包含功能:**
- 所有Standard功能
- 团队管理 + 成员邀请
- RBAC权限控制
- SSO (SAML/OIDC)
- 审计日志
- 共享Snippets
- 配置同步
- 协作功能

**编译特性:** `pro` (自动包含 `standard` 和 `lite`)

---

## 版本检测

### 编译时检测

```rust
use easyssh_core::version::{VersionInfo, Edition};

// 获取当前版本信息
let info = VersionInfo::current();
println!("Edition: {}", info.edition_name);  // "Lite"/"Standard"/"Pro"
println!("Version: {}", info.version);        // "0.3.0"
println!("Features: {:?}", info.features);  // ["ssh", "keychain", ...]

// 版本判断
match Edition::current() {
    Edition::Lite => { /* Lite only code */ },
    Edition::Standard => { /* Standard code */ },
    Edition::Pro => { /* Pro code */ },
}
```

### 功能检测宏

```rust
use easyssh_core::check_feature;

// 编译时功能检测
if check_feature!(embedded_terminal) {
    // 嵌入式终端代码
}

if check_feature!(team) {
    // 团队功能代码
}
```

### 运行时功能检测

```rust
let edition = Edition::current();

if edition.has_embedded_terminal() {
    // 启用嵌入式终端UI
}

if edition.has_team() {
    // 启用团队管理UI
}

if edition.has_audit() {
    // 启用审计日志UI
}
```

---

## 构建配置

### Core Crate 特性链

```
lite
└── dev-tools
    └── hex

standard
├── lite
├── embedded-terminal
├── split-screen
├── sftp
├── monitoring
├── remote-desktop
├── log-monitor
└── docker

pro
├── standard
├── team
├── audit
├── sso
└── sync
```

### Windows UI 特性链

```
lite
├── easyssh-core/lite
└── sftp

standard
├── easyssh-core/standard
├── sftp
├── ai-terminal
├── remote-desktop
├── workflow
├── code-editor
├── monitoring
├── team
├── file-preview
├── docker
└── cloud-sync

pro
├── easyssh-core/pro
├── standard
├── enterprise
├── backup
├── sync
├── team
├── database-client
└── kubernetes
```

---

## 构建命令

### 独立构建各版本

```bash
# Lite版本
cargo build --release --package easyssh-core --features lite
cargo build --release --package easyssh-winui --features lite --no-default-features

# Standard版本
cargo build --release --package easyssh-core --features standard
cargo build --release --package easyssh-winui --features standard --no-default-features

# Pro版本
cargo build --release --package easyssh-core --features pro
cargo build --release --package easyssh-winui --features pro --no-default-features
```

### 使用构建脚本

```bash
# 构建所有版本
./scripts/build-version.sh all

# 构建指定版本
./scripts/build-version.sh lite
./scripts/build-version.sh standard
./scripts/build-version.sh pro
```

---

## 目标目录结构

```
target/
├── release/           # 默认构建输出
├── release-lite/      # Lite优化配置
│   └── easyssh-winui/
│       └── EasySSH.exe
└── versioned/         # 版本分离构建
    ├── lite/
    │   └── EasySSH-Lite.exe
    ├── standard/
    │   └── EasySSH-Standard.exe
    └── pro/
        └── EasySSH-Pro.exe
```

---

## 版本信息结构

```rust
pub struct VersionInfo {
    pub edition: Edition,           // Lite/Standard/Pro
    pub edition_name: &'static str, // "Lite"/"Standard"/"Pro"
    pub version: &'static str,      // 如 "0.3.0"
    pub features: Vec<&'static str>, // 启用功能列表
}

pub enum Edition {
    Lite,
    Standard,
    Pro,
}
```

---

## 功能矩阵

| 功能 | Lite | Standard | Pro |
|------|:----:|:--------:|:---:|
| SSH连接 | ✓ | ✓ | ✓ |
| 密码/密钥认证 | ✓ | ✓ | ✓ |
| SSH Agent | ✓ | ✓ | ✓ |
| Keychain集成 | ✓ | ✓ | ✓ |
| 服务器分组 | ✓(单层) | ✓(嵌套) | ✓(团队) |
| 原生终端唤起 | ✓ | - | - |
| 嵌入式Web终端 | - | ✓ | ✓ |
| 分屏 | - | ✓ | ✓ |
| WebGL加速 | - | ✓ | ✓ |
| SFTP | ✓ | ✓ | ✓ |
| Agent转发 | - | ✓ | ✓ |
| ProxyJump | - | ✓ | ✓ |
| 自动重连 | - | ✓ | ✓ |
| 服务器监控 | - | ✓ | ✓ |
| 批量操作 | - | ✓ | ✓ |
| 团队管理 | - | - | ✓ |
| RBAC权限 | - | - | ✓ |
| SSO | - | - | ✓ |
| 审计日志 | - | - | ✓ |
| 共享Snippets | - | - | ✓ |
| 配置同步 | - | - | ✓ |

---

## Dev模式与Debug功能

所有版本在 `debug_assertions` 开启时(即debug构建)都包含AI编程接口:

```rust
#[cfg(debug_assertions)]
pub mod ai_programming;  // 完整AI编程能力

#[cfg(not(debug_assertions))]
pub mod ai_programming {
    pub fn enabled() -> bool { false }
}
```

**Debug功能入口:**
- `Ctrl+Shift+D` 连续3次 → Lite/Standard debug菜单
- `Ctrl+Alt+Shift+D` → Pro admin面板

---

## 版本验证

使用版本检查工具验证构建:

```bash
# 检查各版本构建状态
cargo run --package version-checker

# 输出示例:
# [OK] Lite版本: target/lite/EasySSH-Lite.exe
# [OK] Standard版本: target/standard/EasySSH-Standard.exe
# [OK] Pro版本: target/pro/EasySSH-Pro.exe
# [OK] 版本标识正确
# [OK] 功能分离验证通过
```

---

## 相关文档

- [整体架构](../architecture/overall-architecture.md)
- [代码质量标准](../standards/code-quality.md)
- [调试接口](../standards/debug-interface.md)
- [Lite版本规划](../easyssh-lite-planning.md)
- [Standard版本规划](../easyssh-standard-planning.md)
- [Pro版本规划](../easyssh-pro-planning.md)

---

*文档版本: 0.3.0*
*更新日期: 2026-04-02*
