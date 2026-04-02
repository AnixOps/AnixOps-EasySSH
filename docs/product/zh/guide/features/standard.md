# Standard 版功能详解

EasySSH Standard 是全功能个人工作台，提供嵌入式终端、分屏布局和 SFTP 文件管理。

## 产品定位

```
┌──────────────────────────────────────────────────────────────┐
│                    EasySSH Standard                           │
│                  全功能个人工作台                             │
├──────────────────────────────────────────────────────────────┤
│                                                              │
│  核心价值：嵌入式终端 + 分屏 + 多会话 + SFTP                  │
│                                                              │
│  • 类 Termius 的专业终端体验                                  │
│  • WebGL 加速的多标签页终端                                   │
│  • 灵活的分屏布局管理                                         │
│  • 集成的 SFTP 文件传输                                       │
│  • 本地加密存储                                               │
│                                                              │
└──────────────────────────────────────────────────────────────┘
```

## 界面概览

```
┌───────────────────────────────────────────────────────────────────────┐
│ 🔍                    EasySSH Standard                    [🌙] [👤]   │
├─────────────────┬───────────────────────────────────┬─────────────────┤
│                 │                                   │                 │
│  📁 分组         │  ┌───────────┬───────────┐       │  📋 服务器详情   │
│                 │  │           │           │       │                 │
│  🏠 生产环境     │  │  终端 1   │  终端 2   │       │  名称: web-01   │
│  ├─ web-01 ●    │  │           │           │       │  状态: 🟢 在线   │
│  ├─ web-02 ●    │  ├───────────┼───────────┤       │  主机: 1.2.3.4  │
│  └─ db-01  ●    │  │           │           │       │  用户: deploy   │
│                 │  │  终端 3   │  SFTP     │       │                 │
│  🧪 测试环境     │  │           │           │       │  [连接] [编辑]  │
│  ├─ web-stg ●   │  └───────────┴───────────┘       │  [SFTP] [日志]  │
│  └─ db-stg  ●   │                                   │                 │
│                 │  🏷️ Tab 1 | Tab 2 | +           │                 │
├─────────────────┴───────────────────────────────────┴─────────────────┤
│  Status: 3 sessions active | Last sync: 2 min ago                    │
└───────────────────────────────────────────────────────────────────────┘
```

## 工作区布局

### 四大区域

1. **左侧边栏**: 服务器分组树、搜索、过滤
2. **中央工作区**: 终端标签页、分屏面板
3. **右侧面板**: 服务器详情、SFTP、监控
4. **底部状态栏**: 连接状态、同步状态、快捷操作

### 布局自定义

```bash
# 保存当前布局
easyssh layout save "Default"

# 切换布局
easyssh layout switch "Monitoring"

# 导出布局
easyssh layout export --output my-layout.json
```

## 核心功能

### 嵌入式终端

#### 终端引擎

基于 `xterm.js` + `WebGL 渲染器`：
- **GPU 加速**: WebGL 渲染大量输出
- **低延迟**: 本地回显优化
- **丰富功能**: 搜索、选择、复制、粘贴

#### 终端特性

| 功能 | 说明 |
|------|------|
| 字体渲染 | 支持 Powerline、Nerd Fonts |
| 颜色主题 | 16 种预设，支持自定义 |
| 光标样式 | 块状、线形、下划线 |
| 回滚缓冲 | 默认 10000 行，可配置 |
| 搜索 | 支持正则表达式 |
| 链接识别 | 自动识别 URL、文件路径 |
| 图片显示 | 支持 Sixel、iTerm2 图片协议 |

#### 终端设置

```bash
# 配置终端
easyssh config set terminal.font-family "JetBrains Mono"
easyssh config set terminal.font-size 14
easyssh config set terminal.line-height 1.2
easyssh config set terminal.theme "Dracula"
easyssh config set terminal.scrollback 50000
```

### 多标签页

#### 标签管理

```
操作:
- 双击服务器: 在新标签页打开
- 中键点击标签: 关闭标签
- Cmd/Ctrl + T: 新建标签
- Cmd/Ctrl + W: 关闭标签
- Cmd/Ctrl + Shift + T: 恢复关闭的标签
- Cmd/Ctrl + 1-9: 切换到对应标签
```

#### 标签组

```bash
# 创建标签组
easyssh tab-group create "Production"

# 将标签加入组
easyssh tab-group add "Production" <tab-id>

# 切换标签组显示
easyssh tab-group switch "Production"
```

### 分屏系统

#### 分屏操作

```
快捷键:
- Cmd/Ctrl + \: 垂直分屏
- Cmd/Ctrl + Shift + \: 水平分屏
- Cmd/Ctrl + Arrow: 切换面板
- Cmd/Ctrl + Shift + X: 关闭当前面板
- Cmd/Ctrl + Shift + F: 最大化/恢复面板
```

#### 预设布局

```bash
# 2x2 网格
easyssh layout apply "2x2"

# 主终端 + 右侧监控
easyssh layout apply "main-monitor"

# 三终端水平排列
easyssh layout apply "triple"
```

#### 自定义布局

```json
{
  "name": "My Layout",
  "root": {
    "type": "split",
    "direction": "vertical",
    "ratio": 0.6,
    "children": [
      {
        "type": "panel",
        "content": "terminal",
        "session": "main"
      },
      {
        "type": "split",
        "direction": "horizontal",
        "children": [
          {
            "type": "panel",
            "content": "terminal",
            "session": "logs"
          },
          {
            "type": "panel",
            "content": "sftp"
          }
        ]
      }
    ]
  }
}
```

### SFTP 文件管理

#### 文件浏览器

```
┌─────────────────────────────────────────┐
│  SFTP - web-01                          │
├─────────────────────────────────────────┤
│  📁 /var/www/app                        │
├─────────────────────────────────────────┤
│  [⬆️ 上级] [🏠 主页] [📁 新建] [🗑️ 删除]   │
├─────────────────────────────────────────┤
│  📁 config/          2026-01-15 10:30   │
│  📁 logs/            2026-01-15 10:30   │
│  📄 app.js           2026-01-15 12:00   │
│  📄 package.json     2026-01-15 11:00   │
│  📄 .env            2026-01-15 09:00   │
├─────────────────────────────────────────┤
│  📂 本地: ~/Downloads                     │
│  📂 远程: /var/www/app                   │
└─────────────────────────────────────────┘
```

#### 文件操作

| 操作 | 方法 |
|------|------|
| 上传 | 拖拽本地文件到 SFTP 面板 |
| 下载 | 双击远程文件或右键 → 下载 |
| 删除 | 右键 → 删除 |
| 重命名 | 右键 → 重命名 |
| 新建文件夹 | 工具栏按钮或右键 |
| 编辑 | 双击文本文件打开编辑器 |

#### 同步浏览

```bash
# 启用同步浏览
easyssh sftp sync --local ~/project --remote /var/www/app

# 自动同步修改
easyssh sftp watch --local ~/project --remote /var/www/app
```

### 会话管理

#### 会话恢复

```
功能：
- 自动保存会话状态
- 重启后恢复连接
- 恢复标签页布局
- 恢复命令历史
```

#### 会话信息

```bash
# 查看会话详情
easyssh session info <session-id>

# 输出:
Session: web-01
Host: 192.168.1.100
User: deploy
Connected: 2026-01-15 10:30:00
Idle: 5 minutes
Bytes sent: 1.2 MB
Bytes received: 45.6 MB
```

### 命令历史

```
功能：
- 记录所有执行过的命令
- 支持搜索历史
- 跨会话共享历史
- 导出命令脚本
```

```bash
# 查看历史
easyssh history

# 搜索历史
easyssh history --search "docker"

# 导出为脚本
easyssh history --export --output deploy.sh
```

### 监控小组件

#### 系统资源监控

```
┌─────────────────────────────┐
│  📊 系统监控 - web-01        │
├─────────────────────────────┤
│  CPU    ████████░░  45%     │
│  Memory ██████████  78%     │
│  Disk   ████░░░░░░  23%     │
│  Net    ↑ 1.2MB ↓ 5.6MB    │
│                             │
│  [刷新] [详细] [图表]        │
└─────────────────────────────┘
```

#### 进程监控

实时显示 Top 进程，支持：
- 按 CPU/内存排序
- 搜索进程
- 发送信号
- 查看详情

### 搜索功能

#### 全局搜索

`Cmd/Ctrl + Shift + F` 打开全局搜索：
- 搜索服务器
- 搜索命令历史
- 搜索文件内容（SFTP）
- 搜索会话输出

#### 终端内搜索

`Cmd/Ctrl + F` 在当前终端搜索：
- 支持正则表达式
- 支持忽略大小写
- 高亮所有匹配
- 循环导航

## 高级功能

### SSH Agent 转发

```bash
# 配置服务器启用 Agent 转发
easyssh edit-server <id> --agent-forward true

# 或在连接时临时启用
easyssh connect <id> --agent-forward
```

### ProxyJump

```bash
# 配置跳板机
easyssh add-server --name "Bastion" --host "bastion.example.com"
easyssh add-server --name "Internal" --host "10.0.0.5" --proxy-jump "Bastion"

# 多跳
easyssh add-server --name "Secure" --host "192.168.1.1" \
  --proxy-jump "Bastion,Intermediate"
```

### 端口转发

```bash
# 本地端口转发
easyssh connect <id> --local-forward 8080:localhost:80

# 远程端口转发
easyssh connect <id> --remote-forward 9090:localhost:3000

# 动态 SOCKS 代理
easyssh connect <id> --dynamic-forward 1080
```

### 自动重连

```bash
# 配置自动重连
easyssh config set connection.auto-reconnect true
easyssh config set connection.reconnect-attempts 5
easyssh config set connection.reconnect-interval 5
```

## 快捷键

### 全局快捷键

| 快捷键 | 功能 |
|--------|------|
| `Cmd/Ctrl + K` | 全局搜索 |
| `Cmd/Ctrl + Shift + K` | 命令面板 |
| `Cmd/Ctrl + ,` | 打开设置 |
| `Cmd/Ctrl + B` | 切换侧边栏 |
| `Cmd/Ctrl + J` | 切换 SFTP 面板 |
| `Cmd/Ctrl + Shift + F` | 全局搜索 |

### 终端快捷键

| 快捷键 | 功能 |
|--------|------|
| `Cmd/Ctrl + T` | 新建标签 |
| `Cmd/Ctrl + W` | 关闭标签 |
| `Cmd/Ctrl + \` | 垂直分屏 |
| `Cmd/Ctrl + Shift + \` | 水平分屏 |
| `Cmd/Ctrl + Arrow` | 切换面板 |
| `Cmd/Ctrl + Shift + F` | 终端内搜索 |
| `Cmd/Ctrl + Shift + C` | 复制 |
| `Cmd/Ctrl + Shift + V` | 粘贴 |
| `Cmd/Ctrl + +/-` | 放大/缩小字体 |
| `Cmd/Ctrl + 0` | 重置字体 |
| `Cmd/Ctrl + Shift + X` | 清屏 |

## 与 Lite 版的数据兼容

### 升级路径

1. Standard 自动识别 Lite 数据
2. 服务器、分组、密钥完整保留
3. 配置自动迁移
4. 获得终端、SFTP 等新功能

### 降级注意

降级到 Lite 会丢失：
- 会话历史
- 分屏布局
- 命令历史
- SFTP 书签

## 性能优化

### WebGL 设置

```bash
# 启用 WebGL 加速（默认开启）
easyssh config set terminal.webgl true

# 设置渲染批量大小
easyssh config set terminal.webgl-batch-size 4096
```

### 连接池

```bash
# 配置连接池
easyssh config set ssh.pool-max-connections 10
easyssh config set ssh.pool-idle-timeout 300
easyssh config set ssh.pool-max-age 3600
```

### 内存管理

```bash
# 限制回滚缓冲区
easyssh config set terminal.scrollback 10000

# 自动清理旧会话
easyssh config set session.auto-cleanup 7d
```

## 最佳实践

### 效率工作流

```
1. 按环境创建分组（开发/测试/生产）
2. 为常用服务器设置快捷键
3. 使用分屏同时监控多服务器
4. 配置 SFTP 同步浏览快速部署
5. 保存常用布局（开发、监控、调试）
```

### 安全建议

```
1. 生产环境禁用密码认证，只用密钥
2. 启用 Agent 转发时注意跳板机安全
3. 敏感操作使用单独的标签组
4. 定期清理命令历史
5. 使用端口转发替代直接暴露服务
```

## 故障排查

### 终端显示问题

```bash
# 检查 WebGL 支持
easyssh --check-webgl

# 禁用 WebGL 回退到 Canvas
easyssh config set terminal.webgl false

# 重置终端设置
easyssh config reset terminal
```

### 连接问题

详见 [故障排查指南](/zh/troubleshooting/connection)。

## 下一步

- [终端主题定制](/zh/guide/terminal-themes)
- [分屏高级用法](/zh/guide/split-panels)
- [SFTP 同步工作流](/zh/guide/sftp-workflow)
- [升级到 Pro 团队协作](/zh/guide/editions)
