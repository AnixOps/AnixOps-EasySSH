# EasySSH 安装指南
# EasySSH Installation Guide

> **English Version**: [Jump to English Section](#installation-guide)

---

## 系统要求 / System Requirements

| 平台 | 最低版本 | 架构 | 存储空间 |
|------|----------|------|----------|
| Windows | Windows 10/11 | x64, ARM64 | 50 MB |
| Linux | Ubuntu 20.04+, Fedora 35+ | x64, ARM64 | 50 MB |
| macOS | macOS 12+ (Monterey) | Intel, Apple Silicon | 50 MB |

---

## Windows 安装

### 方法一: 安装包 (.msi/.exe) - 推荐

1. **下载安装包**
   ```powershell
   # 从 GitHub Releases 下载
   Invoke-WebRequest -Uri "https://github.com/anixops/easyssh/releases/download/v0.3.0/EasySSH-Lite-0.3.0-x64.msi" -OutFile "EasySSH-Lite.msi"
   ```

2. **运行安装程序**
   ```powershell
   # 静默安装
   msiexec /i EasySSH-Lite.msi /quiet /norestart

   # 或双击运行安装向导
   ```

3. **验证安装**
   ```powershell
   # 检查安装路径
   Get-Command easyssh-lite

   # 查看版本
   easyssh-lite --version
   # 输出: EasySSH Lite v0.3.0
   ```

### 方法二: Scoop 包管理器

```powershell
# 添加 bucket (首次使用)
scoop bucket add anixops https://github.com/anixops/scoop-bucket

# 安装 EasySSH Lite
scoop install easyssh-lite

# 更新
scoop update easyssh-lite
```

### 方法三: 便携版 (.zip)

```powershell
# 1. 下载便携版
Invoke-WebRequest -Uri "https://github.com/anixops/easyssh/releases/download/v0.3.0/EasySSH-Lite-0.3.0-x64-portable.zip" -OutFile "easyssh-portable.zip"

# 2. 解压到任意目录
Expand-Archive -Path "easyssh-portable.zip" -DestinationPath "C:\Tools\EasySSH"

# 3. 添加到 PATH (可选)
[Environment]::SetEnvironmentVariable("Path", $env:Path + ";C:\Tools\EasySSH", "User")
```

### Windows 终端集成

```powershell
# 在 Windows Terminal 中添加配置
# 打开 Windows Terminal 设置 (Ctrl+,)
# 添加新配置文件:

{
    "guid": "{12345678-1234-1234-1234-123456789012}",
    "name": "EasySSH Lite",
    "commandline": "C:\\Program Files\\EasySSH Lite\\easyssh-lite.exe",
    "icon": "C:\\Program Files\\EasySSH Lite\\icon.png",
    "startingDirectory": "%USERPROFILE%"
}
```

---

## Linux 安装

### Ubuntu / Debian (.deb)

```bash
# 1. 下载 .deb 包
wget https://github.com/anixops/easyssh/releases/download/v0.3.0/easyssh-lite_0.3.0_amd64.deb

# 2. 安装
sudo dpkg -i easyssh-lite_0.3.0_amd64.deb

# 3. 修复依赖 (如有需要)
sudo apt-get install -f

# 4. 验证安装
easyssh-lite --version
```

### Fedora / RHEL / CentOS (.rpm)

```bash
# 1. 下载 .rpm 包
wget https://github.com/anixops/easyssh/releases/download/v0.3.0/easyssh-lite-0.3.0-1.x86_64.rpm

# 2. 安装
sudo rpm -i easyssh-lite-0.3.0-1.x86_64.rpm

# 或使用 dnf
sudo dnf install easyssh-lite-0.3.0-1.x86_64.rpm

# 3. 验证
easyssh-lite --version
```

### Arch Linux (AUR)

```bash
# 使用 yay 安装
yay -S easyssh-lite

# 或使用 paru
paru -S easyssh-lite

# 从源码构建
git clone https://aur.archlinux.org/easyssh-lite.git
cd easyssh-lite
makepkg -si
```

### 通用方法: AppImage

```bash
# 1. 下载 AppImage
wget https://github.com/anixops/easyssh/releases/download/v0.3.0/EasySSH-Lite-0.3.0-x86_64.AppImage

# 2. 添加执行权限
chmod +x EasySSH-Lite-0.3.0-x86_64.AppImage

# 3. 运行
./EasySSH-Lite-0.3.0-x86_64.AppImage

# 4. 可选: 添加到系统
sudo mv EasySSH-Lite-0.3.0-x86_64.AppImage /usr/local/bin/easyssh-lite
```

### 从源码编译

```bash
# 1. 安装依赖
# Ubuntu/Debian:
sudo apt-get install -y libgtk-4-dev libadwaita-1-dev build-essential

# Fedora:
sudo dnf install gtk4-devel libadwaita-devel gcc gcc-c++

# 2. 克隆仓库
git clone https://github.com/anixops/easyssh.git
cd easyssh/crates/lite-gtk

# 3. 编译
cargo build --release

# 4. 安装
sudo cp target/release/easyssh-lite /usr/local/bin/
sudo cp resources/easyssh-lite.desktop /usr/share/applications/
```

---

## macOS 安装

### 方法一: Homebrew (推荐)

```bash
# 添加 tap
brew tap anixops/tap

# 安装 EasySSH Lite
brew install --cask easyssh-lite

# 或安装命令行版本
brew install easyssh-lite

# 更新
brew upgrade easyssh-lite
```

### 方法二: DMG 安装包

```bash
# 1. 下载 DMG
curl -LO https://github.com/anixops/easyssh/releases/download/v0.3.0/EasySSH-Lite-0.3.0.dmg

# 2. 挂载 DMG
hdiutil attach EasySSH-Lite-0.3.0.dmg

# 3. 复制到应用程序文件夹
cp -R "/Volumes/EasySSH Lite/EasySSH Lite.app" /Applications/

# 4. 卸载 DMG
hdiutil detach "/Volumes/EasySSH Lite"
```

### 方法三: MacPorts

```bash
sudo port install easyssh-lite
```

### 方法四: 从源码编译

```bash
# 1. 安装 Xcode 命令行工具
xcode-select --install

# 2. 安装 Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# 3. 克隆并编译
git clone https://github.com/anixops/easyssh.git
cd easyssh/crates/lite-swift
swift build -c release

# 4. 安装到应用程序
cp -R .build/release/EasySSH\ Lite.app /Applications/
```

### macOS 安全设置

首次运行时，macOS 可能会阻止应用：

```bash
# 方法 1: 系统设置
# 系统设置 → 隐私与安全性 → 安全性 → 允许 EasySSH Lite

# 方法 2: 命令行
xattr -dr com.apple.quarantine /Applications/EasySSH\ Lite.app

# 方法 3: 右键打开
# 右键点击应用 → 打开 → 确认
```

---

## Installation Guide (English)

### Windows

#### Option 1: MSI Installer (Recommended)

```powershell
# Download
Invoke-WebRequest -Uri "https://github.com/anixops/easyssh/releases/download/v0.3.0/EasySSH-Lite-0.3.0-x64.msi" -OutFile "EasySSH-Lite.msi"

# Silent install
msiexec /i EasySSH-Lite.msi /quiet /norestart
```

#### Option 2: Scoop

```powershell
scoop bucket add anixops https://github.com/anixops/scoop-bucket
scoop install easyssh-lite
```

#### Option 3: Portable ZIP

```powershell
Invoke-WebRequest -Uri "https://github.com/anixops/easyssh/releases/download/v0.3.0/EasySSH-Lite-0.3.0-x64-portable.zip" -OutFile "easyssh.zip"
Expand-Archive -Path "easyssh.zip" -DestinationPath "C:\Tools\EasySSH"
```

### Linux

#### Ubuntu/Debian (.deb)

```bash
wget https://github.com/anixops/easyssh/releases/download/v0.3.0/easyssh-lite_0.3.0_amd64.deb
sudo dpkg -i easyssh-lite_0.3.0_amd64.deb
sudo apt-get install -f
```

#### Fedora/RHEL (.rpm)

```bash
wget https://github.com/anixops/easyssh/releases/download/v0.3.0/easyssh-lite-0.3.0-1.x86_64.rpm
sudo dnf install easyssh-lite-0.3.0-1.x86_64.rpm
```

#### Arch Linux (AUR)

```bash
yay -S easyssh-lite
```

#### Universal: AppImage

```bash
wget https://github.com/anixops/easyssh/releases/download/v0.3.0/EasySSH-Lite-0.3.0-x86_64.AppImage
chmod +x EasySSH-Lite-0.3.0-x86_64.AppImage
./EasySSH-Lite-0.3.0-x86_64.AppImage
```

### macOS

#### Option 1: Homebrew (Recommended)

```bash
brew tap anixops/tap
brew install --cask easyssh-lite
```

#### Option 2: DMG

```bash
curl -LO https://github.com/anixops/easyssh/releases/download/v0.3.0/EasySSH-Lite-0.3.0.dmg
hdiutil attach EasySSH-Lite-0.3.0.dmg
cp -R "/Volumes/EasySSH Lite/EasySSH Lite.app" /Applications/
hdiutil detach "/Volumes/EasySSH Lite"
```

#### Security Override (if blocked)

```bash
xattr -dr com.apple.quarantine /Applications/EasySSH\ Lite.app
```

---

## 安装后配置 / Post-Installation

### 1. 验证安装 / Verify Installation

```bash
# 所有平台通用 / All platforms
easyssh-lite --version
# EasySSH Lite v0.3.0

easyssh-lite --help
# 显示帮助信息 / Show help
```

### 2. 配置自动启动 / Auto-Start (可选)

#### Windows
```powershell
# 添加到启动文件夹
$startup = "$env:APPDATA\Microsoft\Windows\Start Menu\Programs\Startup"
$wsh = New-Object -ComObject WScript.Shell
$shortcut = $wsh.CreateShortcut("$startup\EasySSH Lite.lnk")
$shortcut.TargetPath = "C:\Program Files\EasySSH Lite\easyssh-lite.exe"
$shortcut.Save()
```

#### Linux (systemd)
```bash
# 创建 systemd 用户服务
mkdir -p ~/.config/systemd/user
cat > ~/.config/systemd/user/easyssh-lite.service << 'EOF'
[Unit]
Description=EasySSH Lite
After=graphical-session.target

[Service]
Type=simple
ExecStart=/usr/local/bin/easyssh-lite
Restart=on-failure

[Install]
WantedBy=default.target
EOF

systemctl --user enable easyssh-lite
systemctl --user start easyssh-lite
```

#### macOS
```bash
# 使用 launchd
mkdir -p ~/Library/LaunchAgents
cat > ~/Library/LaunchAgents/com.anixops.easyssh-lite.plist << 'EOF'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>com.anixops.easyssh-lite</string>
    <key>ProgramArguments</key>
    <array>
        <string>/Applications/EasySSH Lite.app/Contents/MacOS/easyssh-lite</string>
    </array>
    <key>RunAtLoad</key>
    <true/>
</dict>
</plist>
EOF

launchctl load ~/Library/LaunchAgents/com.anixops.easyssh-lite.plist
```

### 3. 配置 SSH 密钥路径 / SSH Key Path

EasySSH Lite 自动检测常见 SSH 密钥位置：

```
Windows: C:\Users\<用户名>\.ssh\
Linux:   ~/.ssh/
macOS:   ~/.ssh/
```

如需自定义路径，在应用设置中配置：
```
设置 → SSH → 默认密钥路径
```

---

## 故障排除 / Troubleshooting

### 安装问题

| 问题 | 解决方案 |
|------|----------|
| "无法验证发布者" (Windows) | 点击"更多信息" → "仍要运行" |
| "无法打开应用" (macOS) | 参见上方 macOS 安全设置 |
| 依赖缺失 (Linux) | `sudo apt-get install -f` |
| 字体显示异常 | 安装系统默认等宽字体 |

### 卸载

#### Windows
```powershell
# MSI 安装
msiexec /x {ProductCode} /quiet

# 或控制面板 → 程序和功能 → 卸载
```

#### Linux
```bash
# Debian/Ubuntu
sudo dpkg -r easyssh-lite

# Fedora/RHEL
sudo rpm -e easyssh-lite

# 或通用方法
sudo rm /usr/local/bin/easyssh-lite
```

#### macOS
```bash
# Homebrew
brew uninstall --cask easyssh-lite

# 手动
rm -rf /Applications/EasySSH\ Lite.app
rm -rf ~/Library/Application\ Support/EasySSH\ Lite
```

---

## 截图占位符 / Screenshots

### Windows 安装向导 / Windows Setup Wizard
```
[截图占位符: Windows MSI 安装向导界面]
[Screenshot placeholder: Windows MSI installer wizard]
```

### macOS 应用 / macOS Application
```
[截图占位符: macOS 上的 EasySSH Lite 应用]
[Screenshot placeholder: EasySSH Lite on macOS]
```

### Linux GTK 界面 / Linux GTK Interface
```
[截图占位符: Linux GTK4 原生界面]
[Screenshot placeholder: Linux GTK4 native interface]
```

---

**文档版本**: v0.3.0
**最后更新**: 2026-04-02
