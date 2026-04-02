# Debug Access 隐藏入口 - 实现文档

## 概述

本文档描述了EasySSH三个版本（Lite/Standard/Pro）统一的Debug功能隐藏入口实现。

## 核心组件

### 1. `core/src/debug_access.rs`

Debug访问控制核心模块，提供：

- **DebugAccess**: 主控制器，管理激活状态、会话、审计日志
- **DebugAccessMethod**: 激活方式枚举（组合键、CLI标志、环境变量等）
- **DebugFeature**: 可访问的Debug功能枚举
- **DebugAccessLevel**: 权限级别（Viewer/Developer/Admin）
- **KeySequenceDetector**: 组合键序列检测器
- **EditionActivationConfig**: 各版本激活配置

### 2. `core/src/debug_access_ffi.rs`

FFI接口，为平台UI提供C兼容函数：

```rust
// 初始化和状态检查
debug_access_init()
debug_access_is_enabled()
debug_access_get_level()

// 组合键检测
key_detector_create_lite()
key_detector_on_key()
key_detector_destroy()

// 激活控制
debug_access_activate_from_env()
debug_access_deactivate()

// 功能检查
debug_access_can_use_feature()
debug_access_is_ai_enabled()
```

### 3. `core/src/lib.rs` 集成

- 在release builds中，`ai_programming`模块通过`debug_access`检查是否可用
- 导出所有Debug访问类型和FFI函数

## 各版本激活方式

### Lite版本

```rust
EditionActivationConfig::lite()
```

支持：
1. **组合键**: `Ctrl+Shift+D` (3秒内连续按3次)
2. **CLI标志**: `easyssh-lite --dev-mode`
3. **环境变量**: `EASYSSH_DEV=1`

**默认超时**: 1小时 (3600秒)
**默认访问级别**: Developer

### Standard版本

```rust
EditionActivationConfig::standard()
```

支持：
1. **组合键**: `Ctrl+Alt+Shift+D` (单次)
2. **隐藏手势**: 设置菜单连续点击版本号5次
3. **CLI标志**: `easyssh-standard --dev-mode`
4. **环境变量**: `EASYSSH_DEV=1`
5. **配置文件**: `debug.conf`

**默认超时**: 2小时 (7200秒)
**默认访问级别**: Developer

### Pro版本

```rust
EditionActivationConfig::pro()
```

支持：
1. **管理后台开关**: 管理员在后台启用开发者模式
2. **CLI标志**: `easyssh-pro --dev-mode`
3. **环境变量**: `EASYSSH_DEV=1`
4. **API调用**: 需管理员权限token (`/api/admin/debug`)
5. **组合键**: `Ctrl+Alt+Shift+D`
6. **隐藏手势**: 设置菜单连续点击版本号7次

**默认超时**: 4小时 (14400秒)
**默认访问级别**: Admin (通过管理后台激活)
**需要认证**: 是

## 安全设计

### 1. 审计日志

每次激活、停用、功能访问都被记录：

```rust
pub struct DebugAuditRecord {
    pub id: String,
    pub timestamp: u64,
    pub session_id: Option<String>,
    pub action: DebugAuditAction,
    pub method: Option<DebugAccessMethod>,
    pub result: DebugAuditResult,
    pub details: Option<String>,
    pub actor: Option<String>,
    pub client_info: DebugClientInfo,
}
```

### 2. 自动超时

可配置的超时时间，默认：
- Lite: 1小时
- Standard: 2小时
- Pro: 4小时

超时后自动停用并记录审计日志。

### 3. 权限级别控制

不同功能需要不同权限：

| 功能 | 需要级别 | 需要额外认证 |
|------|----------|--------------|
| AI编程 | Developer | 否 |
| 性能监控 | Viewer | 否 |
| 日志查看 | Viewer | 否 |
| 测试运行 | Developer | 否 |
| 数据库控制台 | Admin | 是 |
| 审计日志查看 | Admin | 是 |
| 网络抓包 | Admin | 是 |
| 特性开关 | Admin | 是 |

### 4. UI指示器

启用Debug模式后显示明显的警告条，表明处于开发者模式。

## 使用示例

### Rust API

```rust
use easyssh_core::{
    init_global_debug_access,
    DebugAccessMethod,
    DebugClientInfo,
    try_activate_from_env,
    try_activate_from_cli,
};

// 初始化
let access = init_global_debug_access();

// 从环境变量激活
match try_activate_from_env() {
    Ok(session) => println!("Debug模式已激活: {:?}", session.id),
    Err(e) => println!("激活失败: {}", e),
}

// 从CLI参数激活
let args = vec!["--dev-mode".to_string()];
try_activate_from_cli(&args)?;

// 检查权限
if access.can_access_feature(DebugFeature::AiProgramming) {
    // 使用AI编程接口
}
```

### C/FFI API

```c
// 初始化
debug_access_init();

// 检查是否已启用
if (debug_access_is_enabled()) {
    printf("Debug模式已启用\n");
}

// 获取访问级别 (0=未启用, 1=Viewer, 2=Developer, 3=Admin)
int level = debug_access_get_level();

// 检查特定功能权限 (1=AI编程, 2=性能监控...)
if (debug_access_can_use_feature(1)) {
    // 使用AI编程功能
}

// 停用
debug_access_deactivate("manual");
```

### 平台UI集成示例 (egui)

```rust
// 在App实现中添加组合键检测
pub struct EasySSHApp {
    key_detector: Option<KeySequenceDetector>,
    debug_indicator: bool,
}

impl EasySSHApp {
    fn check_debug_activation(&mut self, ctx: &egui::Context) {
        // 检查组合键
        if ctx.input(|i| i.key_pressed(egui::Key::D)
            && i.modifiers.ctrl
            && i.modifiers.shift) {

            if let Some(ref detector) = self.key_detector {
                if detector.on_key("ctrl+shift+d") {
                    // 激活Debug模式
                    let access = get_debug_access().unwrap();
                    let _ = access.activate(
                        DebugAccessMethod::KeyCombination {
                            sequence: "ctrl+shift+d x3".to_string()
                        },
                        None,
                        DebugClientInfo::default(),
                    );
                }
            }
        }
    }

    fn show_debug_indicator(&mut self, ctx: &egui::Context) {
        if self.debug_indicator {
            egui::TopBottomPanel::top("debug_bar").show(ctx, |ui| {
                ui.horizontal_centered(|ui| {
                    ui.colored_label(
                        egui::Color32::RED,
                        "⚠ DEBUG MODE ACTIVE ⚠"
                    );
                    if ui.button("Deactivate").clicked() {
                        if let Some(access) = get_debug_access() {
                            let _ = access.deactivate("manual");
                        }
                    }
                });
            });
        }
    }
}
```

## AI编程接口集成

在release builds中，AI编程接口默认被禁用。通过debug_access激活后：

```rust
// release builds (原来的占位实现)
#[cfg(not(debug_assertions))]
pub mod ai_programming {
    pub fn is_ai_programming_enabled() -> bool {
        get_debug_access()
            .map(|access| access.can_access_feature(DebugFeature::AiProgramming))
            .unwrap_or(false)
    }

    pub async fn ai_read_code(path: String) -> Result<String, String> {
        if !is_ai_programming_enabled() {
            return Err("AI编程接口未启用".to_string());
        }
        // 验证权限并记录审计
        check_debug_access(DebugFeature::AiProgramming)?;
        // 执行实际功能...
    }
}
```

## 配置示例

### 环境变量激活

```bash
# Linux/macOS
export EASYSSH_DEV=1
./easyssh-lite

# Windows
set EASYSSH_DEV=1
easyssh-lite.exe
```

### CLI参数激活

```bash
# 所有版本都支持
./easyssh-lite --dev-mode
./easyssh-standard --dev-mode
./easyssh-pro --dev-mode

# 或者使用短形式
./easyssh-lite -d
```

### 配置文件激活 (Standard/Pro)

创建 `debug.conf`:
```json
{
    "debug_enabled": true,
    "access_level": "developer",
    "timeout_seconds": 7200,
    "features": ["ai_programming", "performance_monitor"]
}
```

## 测试

运行单元测试：

```bash
cd core
cargo test debug_access --features standard
```

测试用例包括：
- 激活方法验证
- 权限级别检查
- 组合键检测
- 超时自动停用
- 审计日志记录

## 注意事项

1. **生产环境**: 默认情况下debug_access不会自动激活，需要明确的用户操作
2. **审计日志**: 所有操作都被记录，包括激活者、时间、方法、IP地址等
3. **自动超时**: Debug模式不会无限期保持激活，超时后自动退出
4. **UI指示器**: 启用时应有明显视觉提示，防止用户不知情
5. **权限隔离**: 不同功能需要不同权限级别，防止越权访问

## 与现有代码的整合

1. **debug_assertions**: 原有`#[cfg(debug_assertions)]`保护的代码仍然有效
2. **ai_programming**: release builds中通过debug_access启用
3. **audit**: Pro版本的审计系统记录debug_access操作
4. **feature flags**: 各版本的特性标志控制功能可用性
