#!/bin/bash
# EasySSH 三版本构建脚本
# 确保每个版本输出到独立的target目录

set -e

# 获取项目根目录
PROJECT_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$PROJECT_ROOT"

# 显示帮助
show_help() {
    echo "EasySSH 三版本构建脚本"
    echo ""
    echo "用法: $0 <edition> [command] [options]"
    echo ""
    echo "版本 (edition):"
    echo "  lite      - Lite版本 (最小体积)"
    echo "  standard  - Standard版本 (平衡) [默认]"
    echo "  pro       - Pro版本 (完整功能)"
    echo ""
    echo "命令 (command):"
    echo "  build     - 构建 [默认]"
    echo "  check     - 检查"
    echo "  test      - 测试"
    echo "  clean     - 清理构建目录"
    echo "  run       - 运行"
    echo ""
    echo "选项 (options):"
    echo "  --release - 发布模式"
    echo "  --target  - 指定目标平台"
    echo ""
    echo "示例:"
    echo "  $0 lite build --release"
    echo "  $0 standard test"
    echo "  $0 pro check --target x86_64-pc-windows-msvc"
}

# 解析参数
EDITION="${1:-standard}"
COMMAND="${2:-build}"
shift 2 || true

# 验证版本
if [[ ! "$EDITION" =~ ^(lite|standard|pro)$ ]]; then
    echo "错误: 未知的版本 '$EDITION'"
    echo "可用版本: lite, standard, pro"
    exit 1
fi

# 设置环境变量
export CARGO_EASYSSH_EDITION="$EDITION"
export CARGO_TARGET_DIR="target/$EDITION"

# 设置发布配置文件
PROFILE_FLAG=""
if [[ "$*" == *"--release"* ]]; then
    PROFILE_FLAG="--profile=release-$EDITION"
    # 移除--release，因为我们要用自定义profile
    set -- "${@/--release/}"
fi

# 颜色输出
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}========================================${NC}"
echo -e "${BLUE}  EasySSH Builder${NC}"
echo -e "${BLUE}========================================${NC}"
echo ""
echo -e "版本 (Edition): ${GREEN}$EDITION${NC}"
echo -e "命令 (Command): ${GREEN}$COMMAND${NC}"
echo -e "Target目录: ${YELLOW}$CARGO_TARGET_DIR${NC}"
echo ""

# 执行命令
case "$COMMAND" in
    build)
        echo -e "${BLUE}开始构建...${NC}"
        if [[ -n "$PROFILE_FLAG" ]]; then
            cargo build $PROFILE_FLAG --features "$EDITION" "$@"
        else
            cargo build --features "$EDITION" "$@"
        fi
        ;;
    check)
        echo -e "${BLUE}开始检查...${NC}"
        cargo check --features "$EDITION" "$@"
        ;;
    test)
        echo -e "${BLUE}开始测试...${NC}"
        cargo test --features "$EDITION" "$@"
        ;;
    clean)
        echo -e "${YELLOW}清理 $EDITION 构建目录...${NC}"
        rm -rf "$CARGO_TARGET_DIR"
        echo -e "${GREEN}清理完成${NC}"
        ;;
    run)
        echo -e "${BLUE}运行...${NC}"
        # 确定运行哪个包
        if [[ "$EDITION" == "lite" ]]; then
            # Lite版本运行TUI或平台UI
            if [[ -d "crates/easyssh-tui" ]]; then
                cargo run -p easyssh-tui --features "$EDITION" "$@"
            else
                echo "错误: 未找到Lite版本的运行目标"
                exit 1
            fi
        else
            # Standard/Pro版本运行对应平台UI
            case "$(uname -s)" in
                Linux*)     PKG="easyssh-gtk4" ;;
                Darwin*)    PKG="easyssh-swift" ;;
                CYGWIN*|MINGW*|MSYS*) PKG="easyssh-winui" ;;
                *)          echo "未知平台"; exit 1 ;;
            esac
            cargo run -p "$PKG" --features "$EDITION" "$@"
        fi
        ;;
    *)
        echo "错误: 未知命令 '$COMMAND'"
        show_help
        exit 1
        ;;
esac

echo ""
echo -e "${GREEN}✓ 操作完成${NC}"
echo -e "构建产物位置: ${YELLOW}$CARGO_TARGET_DIR${NC}"
