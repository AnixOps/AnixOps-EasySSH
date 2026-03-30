#!/bin/bash
# EasySSH 全自动开发脚本
# 使用 babysitter 进行自动化测试和迭代

set -e

echo "🚀 EasySSH Autonomous Development"
echo "=================================="

# 检查 babysitter 是否可用
if ! command -v a5c &> /dev/null; then
    echo "⚠️  Babysitter CLI (a5c) 未安装"
    echo "请先安装: npm install -g @a5c-ai/babysitter-cli"
    exit 1
fi

# 默认参数
MODE="${1:-core}"
ITERATIONS="${2:-50}"
AUTO_FIX="${3:-true}"

echo ""
echo "📋 配置:"
echo "   模式: $MODE"
echo "   迭代: $ITERATIONS"
echo "   自动修复: $AUTO_FIX"
echo ""

# 运行 babysitter 流程
echo "🏃 启动自动化流程..."
a5c run easyssh-autonomous-dev \
    --input "{\"maxIterations\": $ITERATIONS, \"testMode\": \"$MODE\", \"autoFix\": $AUTO_FIX}" \
    --output "runs/autonomous-$(date +%Y%m%d-%H%M%S).json"

echo ""
echo "✅ 流程完成！"
echo "查看详细报告: runs/autonomous-*.json"
