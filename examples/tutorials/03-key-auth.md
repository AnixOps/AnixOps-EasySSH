# 教程 03：配置密钥认证

> 预计时间：10分钟  |  难度：中级

## 目标

完成本教程后，你将学会：
- 理解 SSH 密钥认证的原理
- 生成新的 SSH 密钥对
- 将公钥添加到远程服务器
- 在 EasySSH Lite 中配置密钥认证
- 使用 SSH Agent 管理多个密钥

---

## 为什么使用密钥认证？

### 密码认证 vs 密钥认证

| 对比项 | 密码认证 | 密钥认证 |
|--------|----------|----------|
| **安全性** | 容易被暴力破解 | 2048位+加密，几乎无法破解 |
| **便利性** | 每次输入密码 | 一次配置，免密登录 |
| **自动化** | 难以自动化 | 支持自动化脚本 |
| **管理** | 多个服务器多个密码 | 一个密钥可登录多台服务器 |

### 密钥认证原理（简化）

```
┌─────────────────────┐                    ┌─────────────────────┐
│     你的电脑        │                    │    远程服务器       │
│                     │                    │                     │
│  ┌───────────────┐  │     连接请求       │  ┌───────────────┐  │
│  │   私钥        │  │ ─────────────────> │  │   公钥        │  │
│  │  (id_rsa)     │  │  "请证明你是谁"    │  │ (authorized_  │  │
│  │   保密存储     │  │                    │  │   keys)       │  │
│  └───────────────┘  │                    │  └───────────────┘  │
│         │           │   签名挑战响应      │         │           │
│         ▼           │ <───────────────── │         ▼           │
│  ┌───────────────┐  │  "这是我的签名"     │  ┌───────────────┐  │
│  │  签名生成      │  │ ─────────────────> │  │  签名验证      │  │
│  │  (用私钥)     │  │                    │  │  (用公钥)     │  │
│  └───────────────┘  │                    │  └───────────────┘  │
│                     │                    │                     │
│                     │     连接成功！     │                     │
│                     │ <───────────────── │                     │
└─────────────────────┘                    └─────────────────────┘
```

**关键点：**
- 私钥留在本地，绝不能泄露
- 公钥放在服务器上，用于验证
- 数学原理保证：公钥只能验证，无法反推私钥

---

## 步骤详解

### 步骤 1：检查现有密钥

**打开系统终端，执行：**

```bash
# 查看 ~/.ssh 目录
ls -la ~/.ssh/
```

**可能的输出：**

```bash
# 已有密钥的情况
-rw-------  1 user user 2602 Jan 10 09:00 id_rsa      # ← 私钥
-rw-r--r--  1 user user  565 Jan 10 09:00 id_rsa.pub  # ← 公钥
-rw-------  1 user user 2602 Jan 12 14:30 id_ed25519      # ← 新的更好算法
-rw-r--r--  1 user user  105 Jan 12 14:30 id_ed25519.pub

# 没有密钥的情况
ls: cannot access '.ssh/': No such file or directory
```

**判断是否需要生成新密钥：**
- 已有密钥：可直接使用，跳到步骤 3
- 没有密钥：继续步骤 2

---

### 步骤 2：生成 SSH 密钥对

**执行命令：**

```bash
# 创建 .ssh 目录（如果不存在）
mkdir -p ~/.ssh
chmod 700 ~/.ssh

# 生成密钥（推荐 ed25519 算法）
ssh-keygen -t ed25519 -C "your_email@example.com"

# 或者使用 RSA 算法（兼容性更好）
ssh-keygen -t rsa -b 4096 -C "your_email@example.com"
```

**交互过程：**

```bash
Generating public/private ed25519 key pair.

# 询问保存位置，默认即可
Enter file in which to save the key (/home/user/.ssh/id_ed25519):
[直接回车]

# 设置密码短语（可选但推荐）
Enter passphrase (empty for no passphrase):
[输入密码短语，或直接回车留空]
Enter same passphrase again:
[再次输入]

# 生成成功
Your identification has been saved in /home/user/.ssh/id_ed25519
Your public key has been saved in /home/user/.ssh/id_ed25519.pub
The key fingerprint is:
SHA256:xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx your_email@example.com
The key's randomart image is:
+--[ED25519 256]--+
|        .o.      |
|       .+o.      |
|       .o+       |
|      .o.        |
|     .  S        |
|    .  . .       |
|   .  . .        |
|  .  . .         |
| .  . .          |
+----[SHA256]-----+
```

**密码短语说明：**
- 设置后：即使私钥被盗，也需要密码才能使用
- 不设置：更便捷，但安全性稍低
- EasySSH Lite 会记住密码短语，无需每次输入

---

### 步骤 3：将公钥添加到服务器

**方法一：使用 ssh-copy-id（推荐）**

```bash
# 自动复制公钥到服务器
ssh-copy-id -i ~/.ssh/id_ed25519.pub user@server_ip

# 示例
ssh-copy-id -i ~/.ssh/id_ed25519.pub root@192.168.1.100
```

**交互过程：**

```bash
/usr/bin/ssh-copy-id: INFO: Source of key(s) to be installed: "/home/user/.ssh/id_ed25519.pub"
/usr/bin/ssh-copy-id: INFO: attempting to log in with the new key(s), to filter out any that are already installed
/usr/bin/ssh-copy-id: INFO: 1 key(s) remain to be installed -- if you are prompted now it is to install the new keys
root@192.168.1.100's password:
[输入服务器密码]

Number of key(s) added: 1

Now try logging into the machine, with:   "ssh 'root@192.168.1.100'"
and check to make sure that the only key(s) you were added are there.
```

**方法二：手动复制**

如果 ssh-copy-id 不可用，手动操作：

```bash
# 1. 在本地查看公钥内容
cat ~/.ssh/id_ed25519.pub
# 输出：ssh-ed25519 AAAAC3NzaC... your_email@example.com

# 2. 登录到远程服务器
ssh root@192.168.1.100

# 3. 在服务器上执行
echo "ssh-ed25519 AAAAC3NzaC... your_email@example.com" >> ~/.ssh/authorized_keys
chmod 600 ~/.ssh/authorized_keys

# 4. 退出并重新连接测试
exit
ssh root@192.168.1.100
```

---

### 步骤 4：验证密钥登录

**测试免密登录：**

```bash
# 尝试 SSH 连接，应该无需输入密码
ssh root@192.168.1.100

# 如果设置了密码短语，第一次会提示输入
Enter passphrase for key '/home/user/.ssh/id_ed25519':
[输入密码短语]

# 登录成功
Welcome to Ubuntu 22.04.3 LTS...
root@my-server:~#
```

**排查问题：**

如果仍然要求密码，检查：

```bash
# 在服务器上检查
ls -la ~/.ssh/
# 确保 authorized_keys 存在且权限正确

# 检查 SSH 配置
cat /etc/ssh/sshd_config | grep -E "PubkeyAuthentication|PasswordAuthentication"
# 应该显示 PubkeyAuthentication yes

# 重启 SSH 服务
sudo systemctl restart sshd
```

---

### 步骤 5：在 EasySSH Lite 中配置密钥

**编辑服务器配置：**

1. 在服务器列表中找到要配置的服务器
2. 右键点击，选择 **"编辑"**
3. 进入认证方式选择页面

```
┌───────────────────────────────────────────────────────┐
│  编辑服务器 - My Server                                │
├───────────────────────────────────────────────────────┤
│                                                       │
│  基本信息                                             │
│  名称:   [My Server________________]                 │
│  主机:   [192.168.1.100____________]                 │
│  端口:   [22_____________________]                 │
│  用户:   [root_____________________]                 │
│                                                       │
├───────────────────────────────────────────────────────┤
│  认证方式                                             │
│                                                       │
│  ○ 密码认证                                          │
│    密码: [••••••••_________________]                 │
│                                                       │
│  ● SSH 密钥                                          │
│    密钥文件: [/home/user/.ssh/id_ed25519____]       │
│              [选择文件...]                          │
│    密码短语: [••••••••_________________] (可选)     │
│                                                       │
│  ○ SSH Agent                                         │
│      使用系统 SSH Agent                              │
│                                                       │
│  [取消]                           [保存 ✓]           │
└───────────────────────────────────────────────────────┘
```

**选择 "SSH 密钥" 选项：**

1. 点击 **"选择文件..."** 按钮
2. 浏览到 `~/.ssh/` 目录
3. 选择私钥文件（如 `id_ed25519`，不带 `.pub` 后缀）
4. 如果密钥设置了密码短语，输入密码短语
5. 点击 **"保存"**

---

### 步骤 6：使用 SSH Agent（高级）

**什么是 SSH Agent？**

SSH Agent 是一个后台程序，临时保存解密后的私钥，避免每次输入密码短语。

```
┌──────────────────────────────────────────────────────────────┐
│                     SSH Agent 工作流程                        │
├──────────────────────────────────────────────────────────────┤
│                                                               │
│  ┌───────────────┐        ┌───────────────┐                │
│  │   私钥文件     │ ──────> │   ssh-agent   │                │
│  │ (带密码短语)   │  输入   │  (内存中保存   │                │
│  │               │ 密码    │   解密后的私钥) │                │
│  └───────────────┘        └───────┬───────┘                │
│                                   │                         │
│                                   │ 自动提供                 │
│                                   ▼ 身份验证                │
│                           ┌───────────────┐                │
│                           │  连接服务器   │                │
│                           └───────────────┘                │
│                                                               │
└──────────────────────────────────────────────────────────────┘
```

**配置步骤：**

**macOS / Linux:**

```bash
# 1. 启动 ssh-agent
eval "$(ssh-agent -s)"

# 2. 添加私钥到 agent
ssh-add ~/.ssh/id_ed25519
# 输入密码短语（只输这一次）
Enter passphrase for /home/user/.ssh/id_ed25519:
Identity added: /home/user/.ssh/id_ed25519 (your_email@example.com)

# 3. 验证
ssh-add -l
# 显示已添加的密钥指纹
```

**Windows:**

Windows 10/11 内置 OpenSSH，包含 ssh-agent：

```powershell
# 1. 以管理员身份运行 PowerShell
# 2. 启动并启用 ssh-agent
Get-Service ssh-agent | Set-Service -StartupType Automatic
Start-Service ssh-agent

# 3. 添加密钥
ssh-add $env:USERPROFILE\.ssh\id_ed25519
```

**在 EasySSH Lite 中选择 SSH Agent：**

```
认证方式选择：
│                                                       │
│  ○ 密码认证                                          │
│                                                       │
│  ○ SSH 密钥                                          │
│    密钥文件: [________________________]             │
│    密码短语: [________________________]             │
│                                                       │
│  ● SSH Agent (推荐)                                  │
│    ✓ 检测到系统 SSH Agent 正在运行                   │
│    已添加密钥:                                       │
│    • id_ed25519 (ED25519)                            │
│    • id_rsa (RSA)                                    │
│                                                       │
│  [取消]                           [保存 ✓]            │
```

**优势：**
- 只需在终端输入一次密码短语
- EasySSH Lite 自动使用 agent 中的密钥
- 重启后需要重新添加（可配置自动启动）

---

### 步骤 7：配置多台服务器使用同一密钥

**场景：** 10 台服务器使用同一密钥

```bash
# 1. 将公钥复制到所有服务器
for server in 192.168.1.{10..19}; do
    ssh-copy-id -i ~/.ssh/id_ed25519.pub root@$server
done

# 2. 在 EasySSH Lite 中
# 对每台服务器选择相同的密钥文件或 SSH Agent
```

**另一种做法：多台服务器不同密钥**

```
密钥命名规范：
~/.ssh/
├── id_ed25519_personal      # 个人服务器
├── id_ed25519_personal.pub
├── id_ed25519_work          # 工作服务器
├── id_ed25519_work.pub
├── id_ed25519_aws           # AWS 服务器
└── id_ed25519_aws.pub
```

在 EasySSH Lite 中为不同服务器选择对应密钥。

---

## 常见问题

### Q1: "Permission denied (publickey)"

**原因：** 服务器拒绝了密钥认证

**排查步骤：**

```bash
# 1. 确认公钥已正确添加到服务器
cat ~/.ssh/authorized_keys
# 应该包含你的公钥内容

# 2. 检查文件权限
chmod 700 ~/.ssh
chmod 600 ~/.ssh/authorized_keys

# 3. 检查 SSH 服务配置
sudo nano /etc/ssh/sshd_config
# 确保以下配置
PubkeyAuthentication yes
AuthorizedKeysFile .ssh/authorized_keys

# 4. 重启 SSH
sudo systemctl restart sshd

# 5. 本地测试详细模式
ssh -v root@192.168.1.100
# 查看调试信息
```

### Q2: "Enter passphrase" 每次都要输入

**解决方案：**

```bash
# 使用 ssh-agent
eval "$(ssh-agent -s)"
ssh-add ~/.ssh/id_ed25519

# macOS 可配置自动启动
# 添加到 ~/.zshrc 或 ~/.bash_profile
if [ -z "$SSH_AUTH_SOCK" ]; then
   eval "$(ssh-agent -s)"
   ssh-add ~/.ssh/id_ed25519
fi
```

### Q3: Windows 上找不到 .ssh 目录

**PowerShell 命令：**

```powershell
# 创建 .ssh 目录
New-Item -ItemType Directory -Path "$env:USERPROFILE\.ssh" -Force

# 生成密钥
ssh-keygen -t ed25519 -C "email@example.com"

# 查看公钥
Get-Content $env:USERPROFILE\.ssh\id_ed25519.pub
```

### Q4: 密钥文件选择错误

**错误选择：** `id_ed25519.pub`（公钥）
**正确选择：** `id_ed25519`（私钥，不带后缀）

**提示：** EasySSH Lite 会检测文件内容，如果选择了公钥会给出警告。

### Q5: 如何提高安全性？

**最佳实践：**

1. **使用强密码短语**
   ```bash
   # 更改现有密钥的密码短语
   ssh-keygen -p -f ~/.ssh/id_ed25519
   ```

2. **定期更换密钥**
   ```bash
   # 生成新密钥
   ssh-keygen -t ed25519 -f ~/.ssh/id_ed25519_new

   # 分发新公钥到所有服务器
   # 测试新密钥可用后，删除旧密钥
   ```

3. **禁用密码登录**（服务器配置）
   ```bash
   # /etc/ssh/sshd_config
   PasswordAuthentication no
   PubkeyAuthentication yes
   ```

4. **使用不同的密钥对不同服务器组**
   - 生产环境：一个密钥
   - 测试环境：另一个密钥
   - 避免一钥走天下

---

## 快捷操作汇总

| 操作 | 命令/方法 |
|------|-----------|
| 生成密钥 | `ssh-keygen -t ed25519 -C "email"` |
| 复制公钥 | `ssh-copy-id -i ~/.ssh/id_ed25519.pub user@host` |
| 启动 agent | `eval "$(ssh-agent -s)"` |
| 添加密钥到 agent | `ssh-add ~/.ssh/id_ed25519` |
| 列出 agent 中的密钥 | `ssh-add -l` |
| 更改密码短语 | `ssh-keygen -p -f ~/.ssh/id_ed25519` |

---

## 下一步

密钥认证配置完成！继续学习：

- **[教程 04：导入 SSH 配置](./04-import-config.md)** - 批量导入现有服务器
- **[教程 05：快捷键使用](./05-shortcuts.md)** - 提升工作效率

---

## 视频演示

[观看视频教程：配置密钥认证（8分钟）](./videos/03-key-auth.mp4)

**视频关键时间点：**
- 00:00 - 密钥认证原理
- 01:30 - 生成密钥对
- 04:00 - 复制公钥到服务器
- 06:00 - EasySSH Lite 配置
- 07:15 - SSH Agent 使用
