# 故障排查指南

## 错误代码速查表

| 错误代码 | 描述 | 常见原因 | 解决方案 |
|----------|------|----------|----------|
| E001 | 连接超时 | 网络不通、防火墙、服务器未启动 | 检查网络、防火墙、SSH 服务 |
| E002 | 认证失败 | 密码错误、密钥无效、用户不存在 | 检查凭据、密钥权限、用户名 |
| E003 | 主机密钥变更 | 服务器重装、中间人攻击 | 更新 known_hosts 或检查安全 |
| E004 | 网络不可达 | DNS 失败、路由问题 | 检查 DNS、网络配置 |
| E005 | 端口被拒绝 | SSH 服务未运行、端口错误 | 检查 SSH 服务和端口 |
| E006 | 主密码错误 | 忘记主密码 | 使用密码管理器或重置数据 |
| E007 | 数据库损坏 | 磁盘故障、意外关机 | 从备份恢复 |
| E008 | 加密失败 | 密钥文件损坏 | 重新配置加密 |
| E009 | 权限不足 | 文件权限错误 | 修复权限设置 |
| E010 | 内存不足 | 系统资源不足 | 关闭其他应用或升级硬件 |

## 连接问题

### E001 - 连接超时

**症状：**
```
Error E001: Connection timed out after 30 seconds
```

**诊断步骤：**

1. **测试基础连通性**
   ```bash
   ping <hostname>
   # 如果失败：检查 DNS 或网络

   telnet <hostname> 22
   # 如果失败：检查 SSH 端口和防火墙
   ```

2. **检查 SSH 服务**
   ```bash
   # 在目标服务器上
   sudo systemctl status sshd  # systemd
   sudo service ssh status     # init.d

   # 检查监听端口
   sudo netstat -tlnp | grep sshd
   # 或
   sudo ss -tlnp | grep sshd
   ```

3. **检查防火墙**
   ```bash
   # 本地防火墙
   sudo iptables -L | grep 22

   # 云服务商安全组（AWS、Azure、GCP）
   # 检查控制台中的安全组设置
   ```

4. **网络路径跟踪**
   ```bash
   traceroute <hostname>
   mtr <hostname>  # 更详细的实时跟踪
   ```

**解决方案：**

```bash
# 增加超时时间
easyssh config set ssh.timeout 60
easyssh config set ssh.connection-timeout 30

# 使用 ProxyJump 时增加跳板机超时
easyssh config set ssh.proxy-timeout 30

# 禁用严格主机密钥检查（仅测试使用）
easyssh config set ssh.strict-host-key-checking false
```

### E002 - 认证失败

**症状：**
```
Error E002: Authentication failed
Permission denied (publickey,password)
```

**诊断步骤：**

1. **密码认证失败**
   ```bash
   # 直接测试
   ssh -v user@host
   # 查看详细认证过程
   ```

2. **密钥认证失败**
   ```bash
   # 检查密钥权限
   ls -la ~/.ssh/
   # 应该显示：
   # -rw------- 1 user user 1675 id_rsa
   # -rw-r--r-- 1 user user  400 id_rsa.pub

   # 检查公钥是否在服务器上
   ssh-copy-id -i ~/.ssh/id_rsa.pub user@host

   # 测试 Agent
   ssh-add -l
   # 如果为空，添加密钥
   ssh-add ~/.ssh/id_rsa
   ```

3. **服务器端日志**
   ```bash
   # 在目标服务器上查看
   sudo tail -f /var/log/auth.log      # Debian/Ubuntu
   sudo tail -f /var/log/secure        # RHEL/CentOS
   sudo journalctl -u sshd -f          # systemd
   ```

**解决方案：**

```bash
# 修复密钥权限
chmod 700 ~/.ssh
chmod 600 ~/.ssh/id_rsa
chmod 644 ~/.ssh/id_rsa.pub

# 使用正确格式密钥（如果密钥太旧）
ssh-keygen -p -m PEM -f ~/.ssh/id_rsa

# 生成新密钥对
ssh-keygen -t ed25519 -C "your_email@example.com"
ssh-copy-id -i ~/.ssh/id_ed25519.pub user@host
```

### E003 - 主机密钥变更

**症状：**
```
Error E003: Remote host identification has changed
WARNING: REMOTE HOST IDENTIFICATION HAS CHANGED!
```

**诊断：**

```bash
# 查看当前记录的密钥
ssh-keygen -F <hostname>

# 获取服务器新密钥
ssh-keyscan -t ed25519 <hostname>
```

**解决方案：**

```bash
# 方法 1: 删除旧密钥（推荐）
ssh-keygen -R <hostname>

# 方法 2: 手动编辑 known_hosts
nano ~/.ssh/known_hosts
# 删除对应行的旧密钥

# 方法 3: EasySSH 中忽略（仅测试）
easyssh config set ssh.strict-host-key-checking false

# 添加新密钥
ssh-keyscan -H <hostname> >> ~/.ssh/known_hosts
```

::: warning 安全警告
主机密钥变更可能表示中间人攻击。确认服务器确实被重装或密钥被更换后再继续。
:::

### E004 - 网络不可达

**症状：**
```
Error E004: Network is unreachable
No route to host
```

**诊断：**

```bash
# 检查路由表
ip route
route -n
netstat -rn

# 检查 DNS
nslookup <hostname>
dig <hostname>

# 检查网卡状态
ip addr
ifconfig
```

**解决方案：**

```bash
# 添加路由（如需要）
sudo ip route add 192.168.1.0/24 via 192.168.0.1

# 修改 DNS（临时）
echo "nameserver 8.8.8.8" | sudo tee /etc/resolv.conf

# 检查 VPN/代理
echo $http_proxy
echo $https_proxy
echo $ALL_PROXY

# 在 EasySSH 中配置代理
easyssh config set proxy.enabled true
easyssh config set proxy.host "proxy.company.com"
easyssh config set proxy.port 8080
```

## 认证问题

### 密钥格式不兼容

**症状：**
```
Error: Key type not supported
Load key "id_rsa": invalid format
```

**诊断：**

```bash
# 检查密钥格式
head -1 ~/.ssh/id_rsa
# 应该以 -----BEGIN OPENSSH PRIVATE KEY----- 开头（新格式）
# 或 -----BEGIN RSA PRIVATE KEY-----（旧格式）

# 检查密钥算法
ssh-keygen -l -f ~/.ssh/id_rsa
```

**解决方案：**

```bash
# 转换密钥格式
ssh-keygen -p -m PEM -f ~/.ssh/id_rsa

# 生成新格式密钥（推荐）
ssh-keygen -t ed25519 -a 100 -f ~/.ssh/id_ed25519

# 更新 EasySSH 配置
easyssh config set ssh.key-algorithms "ed25519,ecdsa,rsa"
```

### SSH Agent 问题

**症状：**
```
Error: Could not open a connection to your authentication agent
```

**诊断：**

```bash
# 检查 Agent 环境变量
echo $SSH_AGENT_SOCK
echo $SSH_AGENT_PID

# 检查 Agent 进程
ps aux | grep ssh-agent

# 列出已加载的密钥
ssh-add -l
```

**解决方案：**

```bash
# 启动 Agent
eval $(ssh-agent -s)  # Linux
eval $(ssh-agent)     # macOS

# 添加密钥
ssh-add ~/.ssh/id_rsa

# 配置自动启动（添加到 ~/.bashrc 或 ~/.zshrc）
if [ -z "$SSH_AGENT_PID" ]; then
    eval $(ssh-agent -s)
    ssh-add ~/.ssh/id_rsa
fi

# macOS 专用：让 Keychain 管理
ssh-add --apple-use-keychain ~/.ssh/id_rsa
```

### 2FA/MFA 问题

**症状：**
```
Authentication refused: bad ownership or modes
Verification code rejected
```

**解决方案：**

```bash
# 使用键盘交互方式
easyssh config set ssh.keyboard-interactive true

# 配置 TOTP
# 1. 在 EasySSH 中配置 Authenticator
# 2. 或使用手机 App 生成验证码

# 如果服务器使用 YubiKey/PGP
# 确保设备已插入并解锁
```

## 性能问题

### 终端卡顿

**症状：**
- 输入延迟高
- 大量输出时卡顿
- CPU 使用率飙升

**诊断：**

```bash
# 检查资源使用
top/htop
ps aux | grep easyssh

# 检查网络延迟
ping <host>
```

**解决方案：**

```bash
# Standard 版优化

# 1. 禁用 WebGL（如果 GPU 问题）
easyssh config set terminal.webgl false

# 2. 减少回滚缓冲区
easyssh config set terminal.scrollback 10000

# 3. 限制帧率
easyssh config set terminal.render-fps 30

# 4. 禁用动画
easyssh config set ui.animations false

# 5. 使用更快的加密算法
easyssh config set ssh.ciphers "chacha20-poly1305@openssh.com,aes128-gcm@openssh.com"
```

### 内存泄漏

**症状：**
- 内存持续增长
- 应用变慢后崩溃

**诊断：**

```bash
# macOS
leaks --atExit -- ./easyssh

# Linux
valgrind --tool=memcheck --leak-check=full ./easyssh

# Windows
# 使用任务管理器或 VS 诊断工具
```

**解决方案：**

```bash
# 限制连接池大小
easyssh config set ssh.pool-max-connections 10

# 启用自动清理
easyssh config set session.auto-cleanup 1h

# 定期重启（临时方案）
# 配置 Supervisor/Systemd 自动重启
```

### 高 CPU 使用

**诊断：**

```bash
# 找出占用 CPU 的进程
top -o cpu

# 查看详细日志
easyssh --verbose --log-level debug
```

**解决方案：**

```bash
# 降低轮询频率
easyssh config set ui.refresh-rate 5000  # 5秒

# 禁用不必要功能
easyssh config set terminal.cursor-blink false
easyssh config set server.health-check false
```

## 数据问题

### 数据库损坏 (E007)

**症状：**
```
Error E007: Database corruption detected
SQLite error: database disk image is malformed
```

**诊断：**

```bash
# 检查数据库完整性
cd ~/.easyssh
sqlite3 data.enc "PRAGMA integrity_check;"

# 查看日志
less ~/.easyssh/logs/easyssh.log
```

**解决方案：**

```bash
# 方法 1: 从备份恢复
cp ~/.easyssh/backups/data-2026-01-15.enc ~/.easyssh/data.enc

# 方法 2: 使用 SQLite 修复
cp data.enc data.corrupted
sqlite3 data.corrupted ".dump" | sqlite3 data.new.enc

# 方法 3: 导出并重建
easyssh export --format json --output backup.json
# 删除损坏的数据库
rm ~/.easyssh/data.enc
# 重新初始化
easyssh --init
# 导入数据
easyssh import --source json --file backup.json
```

### 主密码问题 (E006)

**症状：**
```
Error E006: Invalid master password
Failed to decrypt database
```

**场景：**

**1. 忘记主密码**

::: danger 数据无法恢复
Lite/Standard 使用本地加密，忘记主密码意味着数据永久丢失。
:::

**预防措施：**
```bash
# 定期导出备份
easyssh export --format json --output backup-$(date +%Y%m%d).json

# 将备份存储在安全位置（密码管理器、加密 U 盘）
```

**2. 主密码正确但无法解密**

可能原因：
- 数据库文件损坏
- 使用了不同的数据库文件

**解决方案：**
```bash
# 确认数据文件位置
easyssh config get database.path

# 检查文件是否存在
ls -la ~/.easyssh/data.enc

# 尝试修复（专业支持）
# 联系 support@easyssh.dev
```

## 日志收集

### 启用详细日志

```bash
# 临时启用
easyssh --verbose --log-level trace

# 永久配置
easyssh config set log.level trace
easyssh config set log.file true
```

### 日志位置

```
macOS: ~/Library/Application Support/EasySSH/logs/easyssh.log
Windows: %APPDATA%\EasySSH\logs\easyssh.log
Linux: ~/.config/easyssh/logs/easyssh.log
```

### 日志内容

```log
2026-01-15 10:30:15 [INFO] Starting EasySSH v1.2.0
2026-01-15 10:30:15 [DEBUG] Loading configuration from ~/.easyssh/config.toml
2026-01-15 10:30:15 [DEBUG] Initializing database
2026-01-15 10:30:15 [ERROR] Failed to connect to 192.168.1.100:22
2026-01-15 10:30:15 [ERROR] Error: Connection refused (os error 111)
```

### 导出日志用于支持

```bash
# 打包日志
tar -czf easyssh-logs-$(date +%Y%m%d).tar.gz ~/.easyssh/logs/

# 包含系统信息
easyssh --system-info > system-info.txt
```

## 重置与恢复

### 软重置（保留数据）

```bash
# 重置配置（恢复默认）
easyssh config reset

# 重置布局（Standard）
easyssh layout reset
```

### 硬重置（删除所有数据）

::: danger 警告
这将永久删除所有数据！
:::

```bash
# 1. 导出备份（如需要）
easyssh export --format json --output final-backup.json

# 2. 关闭应用

# 3. 删除数据目录
# macOS
rm -rf ~/Library/Application\ Support/EasySSH/

# Windows
rmdir /s %APPDATA%\EasySSH

# Linux
rm -rf ~/.config/easyssh/
rm -rf ~/.local/share/easyssh/

# 4. 重启应用，重新初始化
```

## 平台特定问题

### macOS

**无法打开应用：**
```bash
# 移除隔离属性
xattr -dr com.apple.quarantine /Applications/EasySSH.app

# 或系统设置中允许
# 系统偏好设置 → 安全性与隐私 → 通用
```

**权限弹窗频繁：**
```bash
# 重置权限
tccutil reset All com.anixops.easyssh

# 或使用系统偏好设置手动添加
```

### Windows

**Windows 保护提示：**
1. 点击「更多信息」
2. 点击「仍要运行」

**路径问题（空格或中文）：**
```powershell
# 使用短路径名
cd "C:\Users\Username\AppData\Roaming\EasySSH"

# 或移动安装位置到无空格路径
```

**防病毒软件拦截：**
- 将 EasySSH 添加到白名单
- 或暂时禁用实时保护进行测试

### Linux

**GTK 主题冲突：**
```bash
# 使用不同主题运行
GTK_THEME=Adwaita easyssh

# 或禁用客户端装饰
GTK_CSD=0 easyssh
```

**Wayland 问题：**
```bash
# 强制使用 XWayland
easyssh --enable-features=UseOzonePlatform --ozone-platform=x11

# 或使用原生 Wayland
easyssh --ozone-platform=wayland
```

## 报告问题

### 提交 Bug 报告

请提供以下信息：

```markdown
**环境信息**
- EasySSH 版本:
- 操作系统:
- 安装方式:

**问题描述**
发生了什么？

**复现步骤**
1.
2.
3.

**期望结果**
应该发生什么？

**实际结果**
实际发生了什么？

**日志**
```
粘贴相关日志
```

**截图**
如有适用，添加截图
```

### 提交渠道

- GitHub Issues: https://github.com/anixops/easyssh/issues
- 邮件: support@easyssh.dev
- Discord: https://discord.gg/easyssh

## 快速修复清单

遇到问题时按顺序尝试：

1. [ ] 重启应用
2. [ ] 检查网络连接
3. [ ] 验证服务器状态
4. [ ] 检查凭据/密钥
5. [ ] 启用详细日志查看错误
6. [ ] 更新到最新版本
7. [ ] 重置配置
8. [ ] 从备份恢复
9. [ ] 联系支持
