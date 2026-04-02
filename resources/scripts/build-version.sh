#!/bin/bash
# EasySSH 版本化构建脚本
# 用法: ./scripts/build-version.sh [lite|standard|pro] [debug|release]

set -e

# 颜色定义
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# 默认值
VERSION=${1:-standard}
PROFILE=${2:-release}

# 验证版本参数
case $VERSION in
    lite|standard|pro)
        ;;
    *)
        echo -e "${RED}错误: 无效版本 '$VERSION'${NC}"
        echo "用法: $0 [lite|standard|pro] [debug|release]"
        exit 1
        ;;
esac

# 验证profile参数
case $PROFILE in
    debug|release)
        ;;
    *)
        echo -e "${RED}错误: 无效profile '$PROFILE'${NC}"
        echo "用法: $0 [lite|standard|pro] [debug|release]"
        exit 1
        ;;
esac

# 设置target目录
export CARGO_TARGET_DIR="target/$VERSION"
export CARGO_EASYSSH_VERSION="$VERSION"

echo -e "${BLUE}========================================${NC}"
echo -e "${BLUE}  EasySSH $VERSION 版本构建${NC}"
echo -e "${BLUE}  Profile: $PROFILE${NC}"
echo -e "${BLUE}  Target: $CARGO_TARGET_DIR${NC}"
echo -e "${BLUE}========================================${NC}"

# 创建target目录
mkdir -p "$CARGO_TARGET_DIR"

# 根据版本选择feature
case $VERSION in
    lite)
        FEATURES="lite"
        RUSTFLAGS=""
        ;;
    standard)
        FEATURES="standard"
        RUSTFLAGS=""
        ;;
    pro)
        FEATURES="pro"
        RUSTFLAGS=""
        ;;
esac

# 构建命令
if [ "$PROFILE" = "release" ]; then
    echo -e "${YELLOW}执行Release构建 (optimized)...${NC}"
    PROFILE_NAME="release-$VERSION"
    cargo build --profile "$PROFILE_NAME" --features "$FEATURES" ${RUSTFLAGS:+--} $RUSTFLAGS
else
    echo -e "${YELLOW}执行Debug构建...${NC}"
    cargo build --features "$FEATURES"
fi

# 检查构建结果
if [ $? -eq 0 ]; then
    echo -e "${GREEN}✓ 构建成功!${NC}"
    echo ""
    echo -e "${BLUE}构建产物:${NC}"
    find "$CARGO_TARGET_DIR" -maxdepth 2 -name "*.exe" -o -maxdepth 2 -name "easyssh*" -type f 2>/dev/null | head -5
    echo ""
    echo -e "${BLUE}目录大小:${NC}"
    du -sh "$CARGO_TARGET_DIR" 2>/dev/null || echo "目录为空"
else
    echo -e "${RED}✗ 构建失败${NC}"
    exit 1
fi
