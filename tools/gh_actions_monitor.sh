#!/bin/bash
# EasySSH GitHub Actions 持续监视脚本

REPO="AnixOps/AnixOps-EasySSH"
BRANCH="main"
INTERVAL=30  # 30秒刷新一次

clear_screen() {
    printf '\033[2J\033[H'
}

show_header() {
    echo "╔══════════════════════════════════════════════════════════════════════════════╗"
    echo "║                    🔍 GitHub Actions 持续监视中心 🔍                          ║"
    echo "╠══════════════════════════════════════════════════════════════════════════════╣"
    echo "║ 仓库: $REPO"
    echo "║ 分支: $BRANCH                                                              ║"
    echo "║ 时间: $(date '+%Y-%m-%d %H:%M:%S')                                                    ║"
    echo "╠══════════════════════════════════════════════════════════════════════════════╣"
}

show_workflows() {
    local runs=$(gh run list -R "$REPO" -b "$BRANCH" -L 10 2>/dev/null)

    if [ -z "$runs" ]; then
        echo "║ 状态: ⚠️  无法获取工作流状态 (检查 gh CLI 认证)                              ║"
        return
    fi

    # 解析最近的运行状态
    local completed=$(echo "$runs" | grep -c "completed")
    local in_progress=$(echo "$runs" | grep -c "in_progress")
    local success=$(echo "$runs" | grep "completed" | grep -c "success")
    local failure=$(echo "$runs" | grep "completed" | grep -c "failure")

    echo "║ 最近工作流运行状态:                                                          ║"
    echo "║   ✅ 成功: $success  │  ❌ 失败: $failure  │  ⏳ 运行中: $in_progress                      ║"
    echo "╠══════════════════════════════════════════════════════════════════════════════╣"

    # 显示详细列表
    echo "$runs" | head -5 | while read line; do
        local status=$(echo "$line" | awk '{print $1}')
        local workflow=$(echo "$line" | awk '{print $2}')
        local event=$(echo "$line" | awk '{print $3}')
        local time=$(echo "$line" | awk '{print $4}')

        local icon="⏳"
        if [[ "$status" == *"success"* ]]; then icon="✅"; fi
        if [[ "$status" == *"failure"* ]]; then icon="❌"; fi
        if [[ "$status" == *"in_progress"* ]]; then icon="🔄"; fi

        printf "║ %s %-20s │ %-10s │ %-20s ║\n" "$icon" "$workflow" "$event" "$time"
    done
}

show_footer() {
    echo "╠══════════════════════════════════════════════════════════════════════════════╣"
    echo "║ 操作: [R]刷新  [O]打开浏览器  [L]查看日志  [Q]退出                          ║"
    echo "║ 自动刷新: 每30秒                                                            ║"
    echo "╚══════════════════════════════════════════════════════════════════════════════╝"
}

# 主循环
main() {
    # 检查 gh CLI
    if ! command -v gh &> /dev/null; then
        echo "❌ 需要先安装 GitHub CLI: https://cli.github.com/"
        exit 1
    fi

    # 检查认证
    if ! gh auth status &>/dev/null; then
        echo "❌ 请先登录: gh auth login"
        exit 1
    fi

    while true; do
        clear_screen
        show_header
        show_workflows
        show_footer

        # 等待按键或30秒
        read -t 30 -n 1 key || true

        case "$key" in
            r|R) continue ;;
            o|O) gh repo view "$REPO" --web ;;
            l|L) gh run list -R "$REPO" -b "$BRANCH" -L 1 --json databaseId -q '.[0].databaseId' | xargs gh run view -R "$REPO" --web ;;
            q|Q) echo -e "\n👋 退出监视"; exit 0 ;;
        esac
    done
}

main "$@"
