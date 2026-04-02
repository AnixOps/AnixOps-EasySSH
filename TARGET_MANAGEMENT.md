# EasySSH Target目录管理指南

## 目录结构

```
target/
├── lite/
│   ├── debug/      # Lite版本Debug构建
│   └── release/    # Lite版本Release构建
├── standard/
│   ├── debug/      # Standard版本Debug构建
│   └── release/    # Standard版本Release构建
├── pro/
│   ├── debug/      # Pro版本Debug构建
│   └── release/    # Pro版本Release构建
├── shared/         # 共享依赖缓存
├── debug/          # 旧版Debug构建 (保留兼容)
└── release/        # 旧版Release构建 (保留兼容)
```

## 使用方法

### 1. 构建特定版本

**Linux/macOS:**
```bash
# 构建Standard版本 (Release)
./scripts/build-version.sh standard release

# 构建Lite版本 (Debug)
./scripts/build-version.sh lite debug

# 构建Pro版本
./scripts/build-version.sh pro release
```

**Windows:**
```cmd
# 构建Standard版本 (Release)
scripts\build-version.bat standard release

# 构建Lite版本
scripts\build-version.bat lite debug
```

### 2. 手动构建 (使用Cargo)

```bash
# 构建Lite版本
CARGO_TARGET_DIR=target/lite cargo build --release --features lite

# 构建Standard版本
CARGO_TARGET_DIR=target/standard cargo build --release --features standard

# 构建Pro版本
CARGO_TARGET_DIR=target/pro cargo build --release --features pro
```

### 3. 清理构建缓存

```bash
# 交互式清理工具
./scripts/clean-target.sh
```

## 配置说明

### `.cargo/config.toml`

- 定义了版本特定的优化配置 (release-lite, release-standard, release-pro)
- 支持跨平台编译优化
- 设置共享缓存路径

### 环境变量

| 变量 | 说明 | 示例 |
|------|------|------|
| `CARGO_TARGET_DIR` | 指定构建输出目录 | `target/lite` |
| `CARGO_EASYSSH_VERSION` | 版本标识 | `standard` |
| `CARGO_SHARED_TARGET` | 共享缓存路径 | `target/shared` |

## 磁盘空间管理

### 各版本典型占用

| 版本 | Debug | Release | 说明 |
|------|-------|---------|------|
| Lite | ~500MB | ~100MB | 最小体积 |
| Standard | ~1.5GB | ~300MB | 包含终端组件 |
| Pro | ~2GB | ~400MB | 完整功能 |

### 清理建议

1. **开发时**: 保留当前开发版本的debug构建
2. **发布前**: 清理所有debug，仅保留release
3. **CI/CD**: 每次构建后清理，避免累积

## 迁移说明

从旧结构迁移:
1. 旧 `target/release-lite/` → 新 `target/lite/release/`
2. 旧 `target/release/` → 新 `target/standard/release/`
3. 旧嵌套 `platforms/*/target/` → 已删除，统一使用根目录target

## 注意事项

1. **不要手动创建嵌套target目录**，所有构建产物必须输出到根目录target
2. **定期运行清理脚本**，防止磁盘空间耗尽
3. **CI/CD配置**需要更新以使用新的构建脚本
4. **共享缓存** (target/shared) 可用于加速多版本构建
