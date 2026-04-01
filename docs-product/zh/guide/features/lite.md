# Lite 版功能详解

EasySSH Lite 是面向注重隐私的开发者设计的 SSH 配置保险箱。

## 产品定位

```
┌──────────────────────────────────────────────┐
│              EasySSH Lite                     │
│           SSH 配置保险箱                      │
├──────────────────────────────────────────────┤
│                                              │
│  核心价值：安全存储 + 一键连接 + 隐私优先    │
│                                              │
│  • 本地加密存储所有连接配置                  │
│  • 一键唤起系统原生终端                        │
│  • 数据完全本地，不上传云端                    │
│  • 极简界面，零学习成本                        │
│                                              │
└──────────────────────────────────────────────┘
```

## 界面概览

```
┌──────────────────────────────────────────────┐
│  🔍 搜索服务器              [+] 添加         │
├──────────────────────────────────────────────┤
│                                              │
│  📁 生产环境                                 │
│    ├─ 🖥️ web-prod-01   ● 在线              │
│    ├─ 🖥️ web-prod-02   ○ 离线              │
│    └─ 🖥️ db-prod-01    ● 在线              │
│                                              │
│  📁 测试环境                                 │
│    ├─ 🖥️ web-staging   ● 在线              │
│    └─ 🖥️ db-staging    ● 在线              │
│                                              │
├──────────────────────────────────────────────┤
│  选中: web-prod-01                           │
│  主机: 192.168.1.10                          │
│  用户: deploy                                │
│  [连接] [编辑] [复制命令]                    │
└──────────────────────────────────────────────┘
```

## 核心功能

### 服务器管理

#### 添加服务器

**图形界面：**
1. 点击「+」按钮
2. 填写表单：
   - **名称**: 显示名称（如 "Production Web"）
   - **主机**: IP 或域名
   - **端口**: 默认 22
   - **用户名**: 登录账号
   - **认证方式**:
     - 密码（保存在 Keychain）
     - SSH 密钥（选择私钥文件）
     - SSH Agent（使用系统 agent）
3. 点击「测试连接」验证
4. 保存

**命令行：**

```bash
# 交互式添加
easyssh add-server

# 非交互式添加
easyssh add-server \
  --name "Production" \
  --host "192.168.1.100" \
  --port 22 \
  --user "deploy" \
  --auth-type "key" \
  --key-path "~/.ssh/production" \
  --group "Production"
```

#### 编辑服务器

右键点击服务器 → 编辑

可修改项：
- 名称、主机、端口
- 用户名
- 认证方式
- 分组
- 标签
- 备注

#### 删除服务器

```
右键点击 → 删除
或
命令行: easyssh delete-server --id <server-id>
```

::: warning 警告
删除操作不可撤销。建议先导出备份。
:::

### 分组管理

#### 创建分组

```bash
# 图形界面
右键侧边栏 → 新建分组

# 命令行
easyssh add-group --name "Production" --color "#ff0000"
```

#### 分组属性

- **名称**: 分组显示名
- **颜色**: 分组标识色（8 种预设）
- **图标**: 可选 emoji 或自定义图标
- **排序**: 手动排序或按名称排序

### 搜索与过滤

#### 快速搜索

```
搜索框输入:
- "prod" → 匹配名称、主机、备注
- "192.168" → 匹配 IP 段
- "tag:web" → 按标签过滤
- "group:prod" → 按分组过滤
- "online" / "offline" → 按状态过滤
```

快捷键：`Cmd/Ctrl + K` 或 `Cmd/Ctrl + P`

#### 高级搜索

```bash
# 搜索并直接连接
easyssh search "prod" --connect

# 列出所有离线服务器
easyssh list --offline
```

### 连接功能

#### 一键连接

1. 点击服务器行
2. 按 `Enter` 或点击「连接」按钮
3. 系统自动：
   - 检测系统终端（iTerm2、Terminal、Windows Terminal 等）
   - 构建 SSH 命令
   - 唤起终端并执行连接

#### 连接选项

右键菜单提供：
- **连接**: 默认连接方式
- **复制 SSH 命令**: 复制到剪贴板
- **连接并执行**: 连接后执行指定命令

```bash
# 连接并执行命令
easyssh connect "Production" --exec "tail -f /var/log/app.log"
```

### 安全特性

#### 主密码保护

首次启动设置主密码：
- 用于加密数据库密钥
- 每次启动需输入
- 忘记密码无法恢复数据

```bash
# 修改主密码
easyssh change-master-password

# 重置（会删除所有数据）
easyssh reset --force
```

#### Keychain 集成

- 密码存储在系统 Keychain
- 主密钥由 Argon2id 派生
- 数据库使用 AES-256-GCM 加密

```
macOS: 系统钥匙串
Windows: Credential Manager
Linux: Secret Service API (GNOME Keyring/KWallet)
```

#### 数据加密流程

```
用户主密码
    ↓
Argon2id (100ms 计算时间)
    ↓
派生密钥 (256-bit)
    ↓
AES-256-GCM 加密数据库
    ↓
存储于 ~/.easyssh/data.enc
```

### 导入导出

#### 从 SSH Config 导入

```bash
# 自动读取 ~/.ssh/config
easyssh import --source ssh-config

# 指定文件路径
easyssh import --source ssh-config --file ~/.ssh/config.custom
```

支持格式：
```ssh-config
Host production
    HostName 192.168.1.100
    User deploy
    Port 2222
    IdentityFile ~/.ssh/production

Host *.staging
    User ubuntu
    IdentityFile ~/.ssh/staging
```

#### 从其他工具导入

```bash
# Termius (CSV)
easyssh import --source termius --file termius-export.csv

# MobaXterm
easyssh import --source mobaxterm --file sessions.mxtsessions

# SecureCRT
easyssh import --source securecrt --dir ~/SecureCRT/
```

#### 导出数据

```bash
# 导出为通用格式
easyssh export --format json --output easyssh-backup.json

# 导出为 SSH config
easyssh export --format ssh-config --output ~/.ssh/config.generated
```

### 系统终端检测

Lite 版会自动检测并使用以下终端：

**macOS:**
- iTerm2（优先）
- Terminal.app
- Alacritty
- Kitty
- WezTerm

**Windows:**
- Windows Terminal（优先）
- PowerShell
- CMD
- Cmder/ConEmu

**Linux:**
- GNOME Terminal
- Konsole
- Alacritty
- Kitty
- xterm

```bash
# 强制使用特定终端
easyssh config set terminal "iTerm"

# 自定义终端命令
easyssh config set terminal-command "alacritty --working-directory ~ -e ssh"
```

## 快捷键

| 快捷键 | 功能 |
|--------|------|
| `Cmd/Ctrl + K` | 打开搜索 |
| `Cmd/Ctrl + N` | 添加服务器 |
| `Cmd/Ctrl + D` | 删除选中服务器 |
| `Cmd/Ctrl + E` | 编辑选中服务器 |
| `Enter` | 连接选中服务器 |
| `Cmd/Ctrl + 1-9` | 快速打开对应分组 |
| `Esc` | 取消搜索/关闭弹窗 |

## 命令行使用

### 完整命令列表

```bash
# 服务器管理
easyssh add-server [options]      # 添加服务器
easyssh edit-server <id>          # 编辑服务器
easyssh delete-server <id>        # 删除服务器
easyssh list-servers              # 列出服务器
easyssh search <query>            # 搜索服务器

# 连接
easyssh connect <name>            # 连接服务器
easyssh connect <name> --exec "cmd"  # 连接并执行命令

# 分组管理
easyssh add-group <name>          # 添加分组
easyssh delete-group <name>       # 删除分组

# 导入导出
easyssh import --source <source>  # 导入配置
easyssh export --format <format>  # 导出配置

# 配置
easyssh config get <key>          # 获取配置
easyssh config set <key> <value>  # 设置配置
easyssh config list               # 列出配置

# 其他
easyssh --version                 # 显示版本
easyssh --health-check            # 健康检查
easyssh change-master-password    # 修改主密码
```

## 数据存储

### 存储位置

```
macOS:   ~/Library/Application Support/EasySSH/
Windows: %APPDATA%\EasySSH\
Linux:   ~/.config/easyssh/
```

### 文件结构

```
~/.easyssh/
├── data.enc          # 加密数据库
├── config.toml       # 应用配置
├── logs/
│   └── easyssh.log   # 应用日志
└── backups/
    └── auto-*.enc    # 自动备份
```

### 自动备份

```bash
# 默认每天自动备份
easyssh config get backup.enabled  # true
easyssh config get backup.interval # 1d

# 手动备份
easyssh backup create

# 列出备份
easyssh backup list

# 恢复备份
easyssh backup restore auto-2026-01-15.enc
```

## 与其他版本的区别

| 特性 | Lite | Standard |
|------|:----:|:--------:|
| 界面 | 简洁列表 | 工作台布局 |
| 终端 | 唤起原生 | 内嵌 WebGL |
| 分屏 | ❌ | ✅ |
| SFTP | ❌ | ✅ |
| 会话恢复 | ❌ | ✅ |
| 同步 | 手动 | 手动/E2EE |

## 最佳实践

### 安全建议

1. **设置强主密码**: 至少 12 位，包含大小写字母、数字、符号
2. **定期备份**: 使用自动备份功能
3. **密钥管理**: 使用 SSH Agent 而非保存私钥
4. **分组策略**: 按环境（开发/测试/生产）分组
5. **命名规范**: 使用 `环境-角色-序号` 格式

### 效率技巧

```
1. 使用搜索快速连接 (Cmd+K)
2. 设置常用服务器为收藏
3. 使用标签标记服务器用途
4. 配置默认终端偏好
5. 导入现有 SSH 配置快速迁移
```

## 故障排查

### 连接失败

```bash
# 检查连接
easyssh connect "Server" --verbose

# 查看日志
tail -f ~/.easyssh/logs/easyssh.log
```

### 主密码忘记

::: danger 数据无法恢复
Lite 版使用本地加密，忘记主密码意味着数据永久丢失。
:::

**预防措施：**
- 定期导出备份到安全位置
- 使用密码管理器保存主密码
- 考虑升级到 Standard/Pro 版获得云端备份

## 下一步

- [快捷键完整列表](/zh/guide/shortcuts)
- [导入配置指南](/zh/guide/import-config)
- [密钥管理最佳实践](/zh/guide/key-management)
- [升级到 Standard](/zh/guide/editions)
