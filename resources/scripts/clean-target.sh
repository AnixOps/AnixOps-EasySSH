#!/bin/bash
# EasySSH Target目录清理脚本
# 用于清理构建缓存，释放磁盘空间

set -e

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

echo -e "${BLUE}EasySSH Target目录清理工具${NC}"
echo ""

# 显示当前磁盘使用情况
echo -e "${YELLOW}当前target目录使用情况:${NC}"
du -sh target/* 2>/dev/null || echo "target目录为空"
echo ""

# 显示选项
echo "请选择清理选项:"
echo "  1. 清理所有构建缓存 (保留目录结构)"
echo "  2. 仅清理Debug构建"
echo "  3. 仅清理Release构建"
echo "  4. 清理特定版本 (lite/standard/pro)"
echo "  5. 完全删除target目录 (危险!)"
echo "  0. 取消"
echo ""

read -p "选择 [0-5]: " choice

case $choice in
    1)
        echo -e "${YELLOW}清理所有构建缓存...${NC}"
        for version in lite standard pro; do
            if [ -d "target/$version" ]; then
                rm -rf "target/$version/debug"/* "target/$version/release"/* 2>/dev/null
                echo "  ✓ 清理 target/$version"
            fi
        done
        rm -rf target/debug/* target/release/* 2>/dev/null
        echo -e "${GREEN}✓ 清理完成${NC}"
        ;;
    2)
        echo -e "${YELLOW}清理所有Debug构建...${NC}"
        for version in lite standard pro; do
            rm -rf "target/$version/debug"/* 2>/dev/null
        done
        rm -rf target/debug/* 2>/dev/null
        echo -e "${GREEN}✓ Debug构建已清理${NC}"
        ;;
    3)
        echo -e "${YELLOW}清理所有Release构建...${NC}"
        for version in lite standard pro; do
            rm -rf "target/$version/release"/* 2>/dev/null
        done
        rm -rf target/release/* 2>/dev/null
        echo -e "${GREEN}✓ Release构建已清理${NC}"
        ;;
    4)
        read -p "输入要清理的版本 (lite/standard/pro): " ver
        if [ -d "target/$ver" ]; then
            echo -e "${YELLOW}清理 target/$ver ...${NC}"
            rm -rf "target/$ver"/*
            echo -e "${GREEN}✓ $ver 版本已清理${NC}"
        else
            echo -e "${RED}版本 '$ver' 不存在${NC}"
        fi
        ;;
    5)
        echo -e "${RED}警告: 这将完全删除target目录!${NC}"
        read -p "确认? [y/N]: " confirm
        if [[ $confirm == [yY] ]]; then
            rm -rf target
            echo -e "${GREEN}✓ target目录已删除${NC}"
        else
            echo "已取消"
        fi
        ;;
    0)
        echo "已取消"
        exit 0
        ;;
    *)
        echo -e "${RED}无效选项${NC}"
        exit 1
        ;;
esac

# 显示清理后的使用情况
echo ""
echo -e "${YELLOW}清理后target目录使用情况:${NC}"
du -sh target 2>/dev/null || echo "target目录不存在"
