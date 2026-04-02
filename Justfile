# EasySSH Justfile - 简化构建命令
# 安装just: cargo install just
# 使用方法: just <command>

# 默认显示帮助
default:
    @just --list

# 构建命令
build-lite:
    @echo "Building Lite edition..."
    ./resources/scripts/build-edition.sh lite build --release

build-standard:
    @echo "Building Standard edition..."
    ./resources/scripts/build-edition.sh standard build --release

build-pro:
    @echo "Building Pro edition..."
    ./resources/scripts/build-edition.sh pro build --release

# 检查命令
check-lite:
    @echo "Checking Lite edition..."
    CARGO_TARGET_DIR=target/lite cargo check --features lite

check-standard:
    @echo "Checking Standard edition..."
    CARGO_TARGET_DIR=target/standard cargo check --features standard

check-pro:
    @echo "Checking Pro edition..."
    CARGO_TARGET_DIR=target/pro cargo check --features pro

# 测试命令
test-lite:
    @echo "Testing Lite edition..."
    CARGO_TARGET_DIR=target/lite cargo test --features lite

test-standard:
    @echo "Testing Standard edition..."
    CARGO_TARGET_DIR=target/standard cargo test --features standard

test-pro:
    @echo "Testing Pro edition..."
    CARGO_TARGET_DIR=target/pro cargo test --features pro

# 清理命令
clean-lite:
    @echo "Cleaning Lite edition..."
    rm -rf target/lite

clean-standard:
    @echo "Cleaning Standard edition..."
    rm -rf target/standard

clean-pro:
    @echo "Cleaning Pro edition..."
    rm -rf target/pro

clean-all:
    @echo "Cleaning all editions..."
    rm -rf target/lite target/standard target/pro

# 运行命令
run-lite:
    @echo "Running Lite edition (TUI)..."
    CARGO_TARGET_DIR=target/lite cargo run -p easyssh-tui --features lite

run-standard:
    @echo "Running Standard edition..."
    # 根据平台选择对应的包
    CARGO_TARGET_DIR=target/standard cargo run -p easyssh-winui --features standard

# 开发命令
dev-lite:
    @echo "Starting Lite development..."
    CARGO_TARGET_DIR=target/lite cargo watch -x "check --features lite" -x "test --features lite"

dev-standard:
    @echo "Starting Standard development..."
    CARGO_TARGET_DIR=target/standard cargo watch -x "check --features standard"

# 发布构建
release-lite:
    @echo "Building Lite release..."
    CARGO_TARGET_DIR=target/lite cargo build --profile=release-lite --features lite

release-standard:
    @echo "Building Standard release..."
    CARGO_TARGET_DIR=target/standard cargo build --profile=release-standard --features standard

release-pro:
    @echo "Building Pro release..."
    CARGO_TARGET_DIR=target/pro cargo build --profile=release-pro --features pro

# 完整构建所有版本
build-all: build-lite build-standard build-pro
    @echo "All editions built successfully!"
    @echo "Artifacts:"
    @ls -lh target/lite/release-lite/easyssh* 2>/dev/null || true
    @ls -lh target/standard/release-standard/easyssh* 2>/dev/null || true
    @ls -lh target/pro/release-pro/easyssh* 2>/dev/null || true

# 统计代码行数
loc:
    @echo "Lines of code:"
    find crates -name "*.rs" | xargs wc -l | tail -1

# 格式化代码
fmt:
    cargo fmt --all

# 检查所有版本
check-all: check-lite check-standard check-pro
    @echo "All editions checked!"
