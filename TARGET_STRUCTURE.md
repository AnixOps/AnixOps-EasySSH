# EasySSH Target目录结构

## 三版本分离设计

```
target/
├── shared/                    # 共享依赖缓存
│   ├── .cargo-lock
│   ├── .rustc_info.json
│   └── CACHEDIR.TAG
│
├── lite/                      # Lite版本构建目录
│   ├── debug/
│   ├── release-lite/
│   ├── wix/                   # Windows installer
│   └── *.exe / *.dmg / *.AppImage
│
├── standard/                  # Standard版本构建目录 (默认)
│   ├── debug/
│   ├── release-standard/
│   ├── wix/
│   └── *.exe / *.dmg / *.AppImage
│
└── pro/                       # Pro版本构建目录
    ├── debug/
    ├── release-pro/
    ├── wix/
    └── *.exe / *.dmg / *.AppImage
```

## 使用方法

### 方式1: 使用 build-edition.sh 脚本 (推荐)

```bash
# 构建Lite版本
./resources/scripts/build-edition.sh lite build --release

# 构建Standard版本
./resources/scripts/build-edition.sh standard build --release

# 构建Pro版本
./resources/scripts/build-edition.sh pro build --release

# 测试Lite版本
./resources/scripts/build-edition.sh lite test

# 清理Pro版本
./resources/scripts/build-edition.sh pro clean
```

### 方式2: 直接使用 Cargo

```bash
# 设置环境变量后使用cargo
export CARGO_TARGET_DIR=target/lite
cargo build --features lite --profile=release-lite

export CARGO_TARGET_DIR=target/standard
cargo build --features standard --profile=release-standard

export CARGO_TARGET_DIR=target/pro
cargo build --features pro --profile=release-pro
```

### 方式3: 使用 Just 命令运行器

```bash
just build-lite
just build-standard
just build-pro
```

## 版本标识

构建产物命名规范:
- Lite: `easyssh-lite-v{version}-{platform}.{ext}`
- Standard: `easyssh-standard-v{version}-{platform}.{ext}`
- Pro: `easyssh-pro-v{version}-{platform}.{ext}`

示例:
- `easyssh-lite-v0.3.0-windows-x64.exe`
- `easyssh-standard-v0.4.0-macos-universal.dmg`
- `easyssh-pro-v0.5.0-linux-x64.AppImage`

## 清理策略

```bash
# 清理特定版本
rm -rf target/lite
rm -rf target/standard
rm -rf target/pro

# 清理所有 (保留共享缓存)
rm -rf target/*/debug
rm -rf target/*/release-*

# 完全清理 (包括共享缓存)
rm -rf target
```

## CI/CD集成

GitHub Actions中:

```yaml
- name: Build Lite
  run: ./resources/scripts/build-edition.sh lite build --release

- name: Build Standard
  run: ./resources/scripts/build-edition.sh standard build --release

- name: Build Pro
  run: ./resources/scripts/build-edition.sh pro build --release

- name: Upload artifacts
  uses: actions/upload-artifact@v4
  with:
    name: easyssh-${{ matrix.edition }}-${{ matrix.target }}
    path: target/${{ matrix.edition }}/release-${{ matrix.edition }}/easyssh*
```
