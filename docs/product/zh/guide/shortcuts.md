# EasySSH 快捷键参考

完整的键盘快捷键列表，帮助您高效使用 EasySSH。

## 全局快捷键

### 导航

| 快捷键 | 功能 | 版本 |
|--------|------|:----:|
| `Cmd/Ctrl + K` | 打开全局搜索/命令面板 | 全部 |
| `Cmd/Ctrl + Shift + K` | 打开命令面板 | Standard/Pro |
| `Cmd/Ctrl + P` | 快速打开服务器 | 全部 |
| `Cmd/Ctrl + 1-9` | 切换到对应分组 | 全部 |
| `Cmd/Ctrl + B` | 切换侧边栏显示 | Standard/Pro |
| `Esc` | 关闭面板/取消操作 | 全部 |

### 服务器操作

| 快捷键 | 功能 | 版本 |
|--------|------|:----:|
| `Cmd/Ctrl + N` | 新建服务器 | 全部 |
| `Cmd/Ctrl + E` | 编辑选中服务器 | 全部 |
| `Cmd/Ctrl + D` | 删除选中服务器 | 全部 |
| `Enter` | 连接选中服务器 | 全部 |
| `Cmd/Ctrl + Shift + C` | 复制 SSH 命令 | Lite |
| `Space` | 预览服务器详情 | Standard/Pro |

### 应用控制

| 快捷键 | 功能 | 版本 |
|--------|------|:----:|
| `Cmd/Ctrl + ,` | 打开设置 | 全部 |
| `Cmd/Ctrl + Shift + ,` | 打开配置文件 | 全部 |
| `Cmd/Ctrl + Q` | 退出应用 | 全部 |
| `Cmd/Ctrl + H` | 隐藏应用 (macOS) | 全部 |
| `Cmd/Ctrl + M` | 最小化窗口 | 全部 |
| `F11` | 全屏切换 | Standard/Pro |

## 终端快捷键 (Standard/Pro)

### 标签页管理

| 快捷键 | 功能 |
|--------|------|
| `Cmd/Ctrl + T` | 新建标签页 |
| `Cmd/Ctrl + W` | 关闭标签页 |
| `Cmd/Ctrl + Shift + T` | 恢复关闭的标签页 |
| `Cmd/Ctrl + Shift + W` | 关闭其他标签页 |
| `Cmd/Ctrl + 1-9` | 切换到对应标签页 |
| `Cmd/Ctrl + Tab` | 切换到下一个标签页 |
| `Cmd/Ctrl + Shift + Tab` | 切换到上一个标签页 |
| `Cmd/Ctrl + Option/Alt + Arrow` | 移动标签页 |

### 分屏管理

| 快捷键 | 功能 |
|--------|------|
| `Cmd/Ctrl + \` | 垂直分屏 |
| `Cmd/Ctrl + Shift + \` | 水平分屏 |
| `Cmd/Ctrl + Arrow` | 切换到相邻面板 |
| `Cmd/Ctrl + Shift + Arrow` | 调整面板大小 |
| `Cmd/Ctrl + Shift + X` | 关闭当前面板 |
| `Cmd/Ctrl + Shift + F` | 最大化/恢复面板 |
| `Cmd/Ctrl + Shift + M` | 合并所有面板 |

### 终端操作

| 快捷键 | 功能 |
|--------|------|
| `Cmd/Ctrl + C` | 复制选中文本 |
| `Cmd/Ctrl + V` | 粘贴 |
| `Cmd/Ctrl + Shift + C` | 复制 (发送到终端时) |
| `Cmd/Ctrl + Shift + V` | 粘贴 (发送到终端时) |
| `Cmd/Ctrl + F` | 终端内搜索 |
| `Cmd/Ctrl + G` | 查找下一个 |
| `Cmd/Ctrl + Shift + G` | 查找上一个 |
| `Cmd/Ctrl + A` | 全选 |
| `Cmd/Ctrl + Shift + X` | 清屏 |
| `Cmd/Ctrl + +` | 放大字体 |
| `Cmd/Ctrl + -` | 缩小字体 |
| `Cmd/Ctrl + 0` | 重置字体大小 |

### 终端功能

| 快捷键 | 功能 |
|--------|------|
| `Cmd/Ctrl + Shift + S` | 保存终端输出 |
| `Cmd/Ctrl + Shift + R` | 重新连接 |
| `Cmd/Ctrl + Shift + D` | 断开连接 |
| `Cmd/Ctrl + Option/Alt + R` | 滚动到顶部 |
| `Cmd/Ctrl + Option/Alt + E` | 滚动到底部 |
| `Cmd/Ctrl + Shift + U` | 上传文件 (SFTP) |
| `Cmd/Ctrl + Shift + D` | 下载文件 (SFTP) |

## 搜索快捷键

### 全局搜索

| 快捷键 | 功能 | 版本 |
|--------|------|:----:|
| `Cmd/Ctrl + K` | 打开搜索 | 全部 |
| `Cmd/Ctrl + Shift + F` | 全局搜索 | Standard/Pro |
| `↑/↓` | 导航搜索结果 | 全部 |
| `Enter` | 打开选中项 | 全部 |
| `Cmd/Ctrl + Enter` | 在新标签页打开 | Standard/Pro |
| `Cmd/Ctrl + Shift + Enter` | 在分屏打开 | Standard/Pro |
| `Tab` | 切换搜索类型 | 全部 |
| `Esc` | 关闭搜索 | 全部 |

### 搜索语法

```
搜索示例:
- "prod"              → 搜索名称包含 "prod"
- "192.168"           → 搜索 IP 段
- "tag:web"           → 搜索标签
- "group:production"  → 搜索分组
- "status:offline"    → 搜索状态
- "type:database"     → 搜索类型
- "user:admin"        → 搜索用户 (Pro)
```

## SFTP 快捷键 (Standard/Pro)

| 快捷键 | 功能 |
|--------|------|
| `Cmd/Ctrl + J` | 切换 SFTP 面板 |
| `Cmd/Ctrl + Shift + J` | 在新标签页打开 SFTP |
| `Enter` | 打开文件/文件夹 |
| `Backspace` | 返回上级目录 |
| `Cmd/Ctrl + U` | 上传文件 |
| `Cmd/Ctrl + Shift + U` | 上传文件夹 |
| `Cmd/Ctrl + D` | 下载选中项 |
| `Delete` | 删除选中项 |
| `F2` | 重命名 |
| `F5` | 刷新 |
| `F7` | 新建文件夹 |
| `Space` | 预览文件 |

## 编辑模式快捷键

### 服务器编辑

| 快捷键 | 功能 |
|--------|------|
| `Tab` | 切换到下一个字段 |
| `Shift + Tab` | 切换到上一个字段 |
| `Cmd/Ctrl + S` | 保存 |
| `Esc` | 取消/关闭 |
| `Cmd/Ctrl + Enter` | 测试连接 |

### 文本编辑 (配置编辑等)

| 快捷键 | 功能 |
|--------|------|
| `Cmd/Ctrl + Z` | 撤销 |
| `Cmd/Ctrl + Shift + Z` | 重做 |
| `Cmd/Ctrl + X` | 剪切 |
| `Cmd/Ctrl + C` | 复制 |
| `Cmd/Ctrl + V` | 粘贴 |
| `Cmd/Ctrl + F` | 查找 |
| `Cmd/Ctrl + H` | 替换 |
| `Cmd/Ctrl + /` | 注释/取消注释 |
| `Cmd/Ctrl + [` | 减少缩进 |
| `Cmd/Ctrl + ]` | 增加缩进 |

## 可自定义快捷键

以下快捷键可在设置中自定义：

```json
{
  "shortcuts": {
    "global.search": "Cmd+K",
    "global.new_server": "Cmd+N",
    "terminal.new_tab": "Cmd+T",
    "terminal.close_tab": "Cmd+W",
    "terminal.split_vertical": "Cmd+\\",
    "terminal.split_horizontal": "Cmd+Shift+\\",
    "sftp.toggle": "Cmd+J"
  }
}
```

## 平台差异

### macOS 特有

| 快捷键 | 功能 |
|--------|------|
| `Cmd + H` | 隐藏 EasySSH |
| `Cmd + Option + H` | 隐藏其他应用 |
| `Cmd + M` | 最小化到 Dock |
| `Cmd + Option + M` | 最小化所有窗口 |
| `Cmd + ` | 切换窗口 |
| `Cmd + Shift + ?` | 帮助 |

### Windows/Linux 特有

| 快捷键 | 功能 |
|--------|------|
| `Ctrl + Shift + Esc` | 打开任务管理器 |
| `Alt + F4` | 关闭窗口 |
| `Alt + Space` | 窗口菜单 |
| `Win + Arrow` | 窗口分屏 |

## 快捷键冲突解决

### 与其他应用冲突

如果快捷键被其他应用占用：

1. **更改 EasySSH 快捷键**
   ```
   设置 → 快捷键 → 找到冲突项 → 修改为其他组合
   ```

2. **更改其他应用快捷键**
   在冲突应用中禁用或更改快捷键

3. **使用备用快捷键**
   EasySSH 为常用功能提供多个快捷键：
   - 搜索：`Cmd+K` 或 `Cmd+Shift+K`
   - 新建：`Cmd+N` 或 `Cmd+Shift+N`

### 与终端应用冲突

终端内使用的快捷键：
```
终端内优先使用:
- Cmd+Shift+C 代替 Cmd+C (复制)
- Cmd+Shift+V 代替 Cmd+V (粘贴)
```

## 快捷键速查表

### 最常使用 (每天)

| 快捷键 | 用途 |
|--------|------|
| `Cmd/Ctrl + K` | 快速搜索连接 |
| `Enter` | 连接服务器 |
| `Cmd/Ctrl + T` | 新标签 |
| `Cmd/Ctrl + W` | 关闭标签 |
| `Cmd/Ctrl + \` | 垂直分屏 |

### 提升效率 (每周)

| 快捷键 | 用途 |
|--------|------|
| `Cmd/Ctrl + N` | 添加服务器 |
| `Cmd/Ctrl + E` | 编辑服务器 |
| `Cmd/Ctrl + F` | 搜索终端内容 |
| `Cmd/Ctrl + J` | 打开 SFTP |
| `Cmd/Ctrl + 1-9` | 快速切换 |

### 高级操作 (偶尔)

| 快捷键 | 用途 |
|--------|------|
| `Cmd/Ctrl + Shift + K` | 命令面板 |
| `Cmd/Ctrl + Shift + T` | 恢复标签 |
| `Cmd/Ctrl + Shift + F` | 全局搜索 |
| `F11` | 全屏 |
| `Cmd/Ctrl + ,` | 设置 |

## 学习快捷键

### 提示功能

首次使用时，EasySSH 会：
- 显示快捷键提示气泡
- 高亮显示可用快捷键
- 记录使用习惯优化建议

### 快捷键练习

```bash
# 打开快捷键练习模式
easyssh --practice-shortcuts

# 或
设置 → 帮助 → 快捷键练习
```

## 导出快捷键列表

```bash
# 导出为 Markdown
easyssh shortcuts export --format markdown

# 导出为 PDF 打印版
easyssh shortcuts export --format pdf

# 导出为 JSON (用于自定义)
easyssh shortcuts export --format json
```
