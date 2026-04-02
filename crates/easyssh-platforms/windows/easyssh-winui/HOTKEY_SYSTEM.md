# EasySSH Windows 快捷键系统

## 概述

专业级快捷键管理系统已集成到 EasySSH Windows UI 中，提供全局快捷键、应用内快捷键、命令面板和快捷键配置功能。

## 功能特性

### 1. 全局快捷键 (Windows RegisterHotKey API)
即使在窗口没有焦点时也能使用：

| 快捷键 | 动作 | 状态 |
|--------|------|------|
| Ctrl+Alt+T | 快速连接到上次使用的服务器 | ✅ 已实现 |
| Ctrl+Alt+N | 新建连接窗口 | ✅ 已实现 |

### 2. 应用内快捷键

#### 标签页管理
| 快捷键 | 动作 | 状态 |
|--------|------|------|
| Ctrl+T | 新建标签 | ✅ 已实现 |
| Ctrl+W | 关闭当前标签 | ✅ 已实现 |
| Ctrl+Tab | 切换到下一个标签 | ✅ 已实现 |
| Ctrl+Shift+Tab | 切换到上一个标签 | ✅ 已实现 |
| Ctrl+1-9 | 切换到指定编号的标签 | ✅ 已实现 |

#### UI 导航
| 快捷键 | 动作 | 状态 |
|--------|------|------|
| Ctrl+K | 命令面板 | ✅ 已实现 |
| Ctrl+Shift+F | 全局搜索 | ✅ 已实现 |
| F11 | 切换全屏 | ✅ 已实现 |

#### 终端操作
| 快捷键 | 动作 | 状态 |
|--------|------|------|
| Ctrl+Plus | 终端字体放大 | ✅ 已实现 |
| Ctrl+Minus | 终端字体缩小 | ✅ 已实现 |
| Ctrl+0 | 重置终端字体大小 | ✅ 已实现 |
| Ctrl+C | 中断当前命令(终端聚焦时) | ✅ 已实现 |
| Ctrl+L | 清屏 | ✅ 已实现 |

#### 其他
| 快捷键 | 动作 | 状态 |
|--------|------|------|
| Ctrl+Shift+S | 聚焦服务器列表 | ✅ 已实现 |
| Ctrl+Shift+T | 聚焦终端 | ✅ 已实现 |
| Ctrl+Shift+B | 聚焦文件浏览器 | ✅ 已实现 |
| Ctrl+B | 切换侧边栏 | ✅ 已实现 |
| Ctrl+Shift+P | 打开代码片段 | ✅ 已实现 |
| Ctrl+Shift+Space | 插入代码片段 | ✅ 已实现 |

### 3. 命令面板 (VS Code 风格)
- 快捷键: **Ctrl+K**
- 显示所有可用命令
- 支持搜索过滤
- 显示快捷键提示
- 最近使用的命令优先显示

### 4. 快捷键配置
- 设置界面允许用户自定义快捷键
- 快捷键冲突检测
- 支持录制新的快捷键组合
- 一键重置为默认值

## 技术实现

### 核心模块

```
platforms/windows/easyssh-winui/src/
├── hotkeys.rs           # 核心快捷键管理
├── hotkey_helpers.rs    # 快捷键辅助函数
└── main.rs              # 集成和UI渲染
```

### Windows API 使用

```rust
// 使用 Windows RegisterHotKey API 注册全局快捷键
RegisterHotKey(hwnd, id, MOD_CONTROL | MOD_ALT, VK_T);  // Ctrl+Alt+T
```

### 数据结构

```rust
/// 快捷键动作枚举
pub enum HotkeyAction {
    QuickConnectLast,      // 快速连接到最后服务器
    NewConnectionWindow,   // 新建连接窗口
    NewTab,                // 新建标签
    CloseTab,              // 关闭标签
    NextTab,               // 下一个标签
    PrevTab,               // 上一个标签
    SwitchTab1..=SwitchTab9, // 切换标签 1-9
    CommandPalette,        // 命令面板
    GlobalSearch,          // 全局搜索
    ToggleFullscreen,      // 全屏切换
    TerminalZoomIn,        // 终端放大
    TerminalZoomOut,       // 终端缩小
    TerminalZoomReset,     // 重置终端缩放
    TerminalClear,         // 清屏
    FocusServers,          // 聚焦服务器列表
    FocusTerminal,         // 聚焦终端
    FocusFileBrowser,      // 聚焦文件浏览器
    ToggleSidebar,         // 切换侧边栏
    OpenSnippets,          // 打开代码片段
    InsertSnippet,         // 插入代码片段
    Custom(String),        // 自定义动作
}

/// 快捷键绑定
pub struct KeyBinding {
    pub keys: Vec<Key>,    // 按键组合
    pub global: bool,      // 是否为全局快捷键
    pub when_focused: bool, // 是否需要窗口聚焦
}
```

## 使用方法

### 在代码中添加快捷键

```rust
// 注册新的快捷键
let mut manager = HotkeyManager::new();
manager.register_binding(
    HotkeyAction::Custom("MyAction".to_string()),
    KeyBinding::new(vec![Key::Control, Key::Shift, Key::M])
);

// 设置回调函数
manager.set_callback(|action| {
    println!("Hotkey triggered: {:?}", action);
});
```

### 打开命令面板

```rust
// 显示命令面板
self.open_command_palette();
```

### 显示快捷键设置

```rust
// 显示快捷键配置界面
self.show_hotkey_settings = true;
```

## 配置存储

快捷键配置可以通过 JSON 序列化保存和加载：

```rust
// 保存配置
let config_json = manager.save_config()?;

// 加载配置
manager.load_config(&config_json)?;
```

## 未来扩展

### 计划功能
1. 多窗口支持 (Ctrl+Alt+N)
2. 自定义宏快捷键
3. 上下文感知快捷键
4. 快捷键统计和分析
5. 云同步快捷键配置

### 全局快捷键增强
在 Windows 上，可以通过设置窗口句柄启用真正的全局快捷键：

```rust
#[cfg(windows)]
hotkeys::setup_global_hotkeys(hwnd, &hotkey_manager);
```

## 总结

✅ 完整的快捷键管理系统已集成
✅ 支持全局和应用内快捷键
✅ 提供 VS Code 风格的命令面板
✅ 允许用户自定义快捷键配置
✅ 内置快捷键冲突检测
✅ 所有主要功能已可正常使用
