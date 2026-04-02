# EasySSH Lite 快速入门
# EasySSH Lite Quick Start

> **English Version**: [Jump to English Section](#quick-start-guide)

---

## 快速入门指南 (中文)

### 1. 首次启动

启动 EasySSH Lite 后，您会看到一个简洁的主界面：

```
┌─────────────────────────────────────────────┐
│  EasySSH Lite  v0.3.0                        │
├─────────────────────────────────────────────┤
│  🔍 搜索服务器...                            │
├─────────────────────────────────────────────┤
│                                              │
│  📁 开发环境                                 │
│    ├── 🖥️  生产服务器-01                      │
│    ├── 🖥️  生产服务器-02                      │
│    └── 🖥️  测试服务器                        │
│                                              │
│  📁 个人项目                                 │
│    ├── 🖥️  VPS-东京                          │
│    └── 🖥️  VPS-新加坡                        │
│                                              │
└─────────────────────────────────────────────┘
```

### 2. 配置主密码 (首次使用)

> **安全性提示**: 主密码用于保护您的 SSH 配置加密存储，请务必牢记！

```
┌─────────────────────────────────────────────┐
│  🔐 设置主密码                                │
├─────────────────────────────────────────────┤
│                                              │
│  请输入主密码:     [****************]         │
│  确认密码:         [****************]         │
│                                              │
│  ⚠️  此密码用于加密您的 SSH 配置，            │
│     丢失后将无法恢复数据！                    │
│                                              │
│         [  确 认  ]                          │
│                                              │
└─────────────────────────────────────────────┘
```

### 3. 添加第一台服务器

#### 3.1 点击 "+" 按钮或按 `Ctrl+N`

#### 3.2 填写连接信息

| 字段 | 说明 | 示例 |
|------|------|------|
| 服务器名称 | 显示名称 | "生产服务器-01" |
| 主机地址 | IP 或域名 | "192.168.1.100" 或 "server.example.com" |
| 端口 | SSH 端口 (默认 22) | 22 |
| 用户名 | SSH 用户名 | "root" 或 "ubuntu" |
| 认证方式 | 密码或密钥 | 密码 / 密钥 |

#### 3.3 密码认证

```
认证方式: 密码
用户名:   root
密码:     [****************] (安全存储到系统钥匙串)
```

#### 3.4 密钥认证

```
认证方式: SSH 密钥
用户名:   ubuntu
私钥路径: ~/.ssh/id_rsa
          (或点击 "生成新密钥对")
```

### 4. 连接服务器

双击服务器条目或按 `Enter` 键：

```
┌─────────────────────────────────────────────┐
│  正在连接: 生产服务器-01...                   │
│  主机: 192.168.1.100:22                       │
│  用户: root                                  │
├─────────────────────────────────────────────┤
│                                              │
│  [✓] 建立 TCP 连接                           │
│  [✓] 身份验证成功                            │
│  [✓] 启动终端...                             │
│                                              │
└─────────────────────────────────────────────┘
```

系统终端将自动打开并连接到服务器。

### 5. 常用快捷键

| 快捷键 | 功能 |
|--------|------|
| `Ctrl+N` | 新建服务器配置 |
| `Ctrl+E` | 编辑选中的服务器 |
| `Ctrl+D` | 删除选中的服务器 |
| `Ctrl+F` | 搜索/过滤服务器 |
| `Enter` | 连接选中的服务器 |
| `Ctrl+G` | 新建分组 |
| `F5` | 刷新列表 |
| `Ctrl+Q` | 退出程序 |

### 6. 服务器分组管理

#### 创建分组
```
右键点击空白区域 → 新建分组
输入分组名称: "开发环境"
```

#### 移动服务器到分组
```
右键点击服务器 → 移动到 → 选择分组
或拖拽服务器到分组
```

### 7. 搜索和过滤

```
🔍 搜索: "prod"

结果:
├── 📁 开发环境
│   ├── 🖥️  生产服务器-01   ← 匹配
│   └── 🖥️  生产服务器-02   ← 匹配
│
└── 📁 测试环境
    └── 🖥️  测试服务器      (不匹配，隐藏)
```

支持模糊搜索：输入 "p1" 可匹配 "生产服务器-01"

### 8. 数据导入导出

#### 导出配置 (备份)
```
菜单 → 文件 → 导出配置
选择路径: ~/backup/easyssh-backup-2026-04-02.json
```

导出的文件为加密格式，需要主密码才能解密。

#### 导入配置 (恢复)
```
菜单 → 文件 → 导入配置
选择备份文件并输入主密码
```

---

## Quick Start Guide (English)

### 1. First Launch

When you start EasySSH Lite, you'll see a clean main interface.

### 2. Set Master Password (First Time)

> **Security Notice**: The master password protects your encrypted SSH configurations. Never lose it!

### 3. Add Your First Server

Press `Ctrl+N` or click the "+" button:

| Field | Description | Example |
|-------|-------------|---------|
| Server Name | Display name | "Production-01" |
| Host | IP or hostname | "192.168.1.100" |
| Port | SSH port (default 22) | 22 |
| Username | SSH username | "root" |
| Auth Method | Password or Key | Password / Key |

### 4. Connect to Server

Double-click a server entry or press `Enter`.

Your system terminal will automatically open with the SSH connection established.

### 5. Common Shortcuts

| Shortcut | Action |
|----------|--------|
| `Ctrl+N` | New server |
| `Ctrl+E` | Edit server |
| `Ctrl+D` | Delete server |
| `Ctrl+F` | Search/filter |
| `Enter` | Connect to selected |
| `Ctrl+G` | New group |
| `F5` | Refresh list |
| `Ctrl+Q` | Quit |

### 6. Server Groups

- **Create Group**: Right-click → New Group
- **Move Server**: Right-click → Move to → Select group
- **Drag & Drop**: Drag server to group

### 7. Search & Filter

Type in the search box to filter servers. Fuzzy search supported:
- "p1" matches "Production-01"
- "aws" matches all AWS servers

### 8. Backup & Restore

**Export (Backup)**:
```
Menu → File → Export Config
```

**Import (Restore)**:
```
Menu → File → Import Config
Enter master password to decrypt
```

---

## 下一步 / Next Steps

- **中文**: 阅读 [Lite 功能详解](./lite-features.md) 了解更多高级功能
- **English**: Read [Lite Features](./lite-features.md) for advanced features
- 查看 [安装指南](./installation.md) 了解各平台安装方法
- Check [Installation Guide](./installation.md) for platform-specific setup

---

## 截图占位符 / Screenshots

> 以下截图将在后续版本补充

### 主界面 / Main Interface
```
[截图占位符: 主界面展示服务器列表和分组]
[Screenshot placeholder: Main interface with server list and groups]
```

### 添加服务器 / Add Server Dialog
```
[截图占位符: 添加服务器对话框]
[Screenshot placeholder: Add server dialog]
```

### 搜索功能 / Search Feature
```
[截图占位符: 搜索过滤功能展示]
[Screenshot placeholder: Search and filter demonstration]
```

---

**文档版本**: v0.3.0
**最后更新**: 2026-04-02
**适用平台**: Windows, Linux, macOS
