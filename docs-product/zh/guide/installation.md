# 安装指南

本文档详细介绍 EasySSH 各版本在不同平台的安装方法。

## 系统要求

### Lite 版

| 平台 | 最低版本 | 架构 |
|------|----------|------|
| macOS | 10.15 (Catalina) | Intel, Apple Silicon |
| Windows | Windows 10 (1809) | x64, ARM64 |
| Linux | Ubuntu 20.04, Fedora 35 | x64, ARM64 |

### Standard 版

| 平台 | 最低版本 | 架构 |
|------|----------|------|
| macOS | 11.0 (Big Sur) | Intel, Apple Silicon |
| Windows | Windows 10 (2004) | x64, ARM64 |
| Linux | Ubuntu 22.04, Fedora 36 | x64, ARM64 |
| 依赖 | WebGL 2.0 | GPU 加速 |

### Pro 版服务端

| 环境 | 要求 |
|------|------|
| Docker | 20.10+ |
| Kubernetes | 1.24+ |
| 数据库 | PostgreSQL 14+ |
| 缓存 | Redis 7+ |

## 各平台安装

### macOS

#### Homebrew 安装（推荐）

::: tabs
@tab Lite
```bash
brew tap anixops/easyssh
brew install easyssh-lite
```
@tab Standard
```bash
brew tap anixops/easyssh
brew install easyssh
```
:::

#### 手动安装

1. 下载对应版本的 `.dmg` 文件
   - Lite: `EasySSH-Lite-1.x.x.dmg`
   - Standard: `EasySSH-1.x.x.dmg`

2. 双击打开 DMG，将应用拖到 Applications 文件夹

3. 首次启动可能需要：
   - 右键点击应用 → 打开
   - 或在系统偏好设置 → 安全性与隐私中允许

### Windows

#### Winget 安装（推荐）

::: tabs
@tab Lite
```powershell
winget install EasySSH.Lite
```
@tab Standard
```powershell
winget install EasySSH
```
:::

#### Microsoft Store

搜索 "EasySSH" 直接安装 Lite 版。

#### 手动安装

1. 下载对应版本的安装程序
   - Lite: `EasySSH-Lite-Setup-1.x.x.exe`
   - Standard: `EasySSH-Setup-1.x.x.exe`

2. 运行安装程序，按向导完成安装

3. Windows Defender 可能会提示，点击「更多信息」→「仍要运行」

### Linux

#### Ubuntu/Debian

```bash
# 添加仓库
curl -fsSL https://easyssh.dev/apt/gpg.key | sudo gpg --dearmor -o /usr/share/keyrings/easyssh.gpg
echo "deb [signed-by=/usr/share/keyrings/easyssh.gpg] https://easyssh.dev/apt stable main" | sudo tee /etc/apt/sources.list.d/easyssh.list

# 更新并安装
sudo apt update
sudo apt install easyssh-lite  # Lite 版
# 或
sudo apt install easyssh       # Standard 版
```

#### Fedora

```bash
# 添加 COPR 仓库
sudo dnf copr enable easyssh/easyssh

# 安装
sudo dnf install easyssh-lite  # Lite 版
# 或
sudo dnf install easyssh       # Standard 版
```

#### Arch Linux

```bash
# 使用 AUR 助手
yay -S easyssh
# 或
paru -S easyssh

# 手动安装
git clone https://aur.archlinux.org/easyssh.git
cd easyssh
makepkg -si
```

#### 通用 AppImage

```bash
# 下载 AppImage
wget https://easyssh.dev/download/linux/EasySSH-Lite-1.x.x.AppImage
chmod +x EasySSH-Lite-1.x.x.AppImage

# 运行
./EasySSH-Lite-1.x.x.AppImage

# 可选：集成到系统
./EasySSH-Lite-1.x.x.AppImage --appimage-extract-and-run
```

## Pro 版部署

Pro 版需要部署服务端，详见 [企业部署指南](/zh/deploy/enterprise)。

### 快速启动（Docker）

```bash
# 创建 docker-compose.yml
cat > docker-compose.yml << 'EOF'
version: '3.8'
services:
  easyssh:
    image: easyssh/pro:latest
    ports:
      - "8080:8080"
    environment:
      - DATABASE_URL=postgres://easyssh:password@db:5432/easyssh
      - REDIS_URL=redis://redis:6379
    depends_on:
      - db
      - redis

  db:
    image: postgres:15
    environment:
      POSTGRES_USER: easyssh
      POSTGRES_PASSWORD: password
      POSTGRES_DB: easyssh
    volumes:
      - postgres_data:/var/lib/postgresql/data

  redis:
    image: redis:7-alpine

volumes:
  postgres_data:
EOF

# 启动
docker-compose up -d
```

## 验证安装

### 命令行验证

```bash
# 检查版本
easyssh --version

# 预期输出：
# EasySSH Lite 1.x.x
# 或
# EasySSH Standard 1.x.x

# 检查健康状态
easyssh --health-check
```

### 图形界面验证

1. 启动应用
2. 首次启动应显示设置向导
3. 完成向导后应能看到主界面

## 卸载

### macOS

```bash
# Homebrew 安装
brew uninstall easyssh-lite
# 或
brew uninstall easyssh

# 手动删除
rm -rf /Applications/EasySSH.app
rm -rf ~/Library/Application\ Support/EasySSH
```

### Windows

```powershell
# Winget 安装
winget uninstall EasySSH.Lite
# 或
winget uninstall EasySSH

# 手动删除
# 设置 → 应用 → 应用和功能 → EasySSH → 卸载
```

### Linux

```bash
# Ubuntu/Debian
sudo apt remove easyssh-lite
sudo apt autoremove

# Fedora
sudo dnf remove easyssh-lite

# Arch
sudo pacman -R easyssh

# 清理数据
rm -rf ~/.config/easyssh
rm -rf ~/.local/share/easyssh
```

## 故障排查

### 安装失败

**问题：macOS "无法打开，因为无法验证开发者"**

解决：
```bash
# 临时允许
xattr -dr com.apple.quarantine /Applications/EasySSH.app

# 或前往 系统偏好设置 → 安全性与隐私 → 通用 → 仍要打开
```

**问题：Windows "Windows 已保护你的电脑"**

解决：点击「更多信息」→「仍要运行」。这是因为我们尚未完成昂贵的代码签名证书申请。

**问题：Linux 缺少依赖**

解决：
```bash
# Ubuntu/Debian
sudo apt install libgtk-3-0 libwebkit2gtk-4.0-37

# Fedora
sudo dnf install gtk3 webkit2gtk3
```

### 启动失败

详见 [故障排查指南](/zh/troubleshooting/)。

## 更新

### 自动更新

- **macOS**: Homebrew `brew upgrade`
- **Windows**: Winget 或应用内自动更新
- **Linux**: 包管理器 `apt upgrade` / `dnf upgrade`

### 手动更新

下载新版本安装包直接覆盖安装，数据会自动保留。

## 获取旧版本

```bash
# 查看所有版本
easyssh --list-versions

# 下载指定版本
curl -O https://easyssh.dev/download/1.0.0/EasySSH-1.0.0.dmg
```
