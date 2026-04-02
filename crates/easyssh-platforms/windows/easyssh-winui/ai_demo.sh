#!/bin/bash
# AI终端功能演示脚本
# 用于展示EasySSH AI智能终端的各项功能

echo "================================================"
echo "  EasySSH AI智能终端功能演示"
echo "================================================"
echo ""

# 演示1: 自然语言转命令
demo_natural_language() {
    echo "演示1: 自然语言转命令"
    echo "------------------------"
    echo "用户输入: '查看当前目录下所有文件的大小'"
    echo "AI输出:   du -sh *"
    echo ""
    echo "用户输入: '查找最近修改过的日志文件'"
    echo "AI输出:   find /var/log -name '*.log' -mtime -7"
    echo ""
    echo "用户输入: '显示系统启动时间'"
    echo "AI输出:   uptime"
    echo ""
    sleep 2
}

# 演示2: 命令补全
demo_completion() {
    echo "演示2: 智能命令补全"
    echo "------------------------"
    echo "输入: 'gi'"
    echo "建议:"
    echo "  1. git status - 显示工作树状态"
    echo "  2. git log --oneline - 简洁日志"
    echo "  3. git add . - 添加所有文件"
    echo ""
    echo "输入: 'docker ps'"
    echo "建议:"
    echo "  1. docker ps -a - 显示所有容器"
    echo "  2. docker ps --format - 自定义格式"
    echo ""
    sleep 2
}

# 演示3: 错误诊断
demo_error_diagnosis() {
    echo "演示3: 智能错误诊断"
    echo "------------------------"
    echo "错误: 'Permission denied'"
    echo "AI诊断:"
    echo "  问题: 权限不足"
    echo "  解决方案:"
    echo "    1. 使用 sudo 提升权限"
    echo "    2. 检查文件所有者: ls -la"
    echo "    3. 修改文件权限: chmod 755"
    echo ""
    echo "错误: 'Command not found: docker-compose'"
    echo "AI诊断:"
    echo "  问题: 命令未安装"
    echo "  解决方案:"
    echo "    1. Ubuntu/Debian: sudo apt install docker-compose"
    echo "    2. macOS: brew install docker-compose"
    echo ""
    sleep 2
}

# 演示4: 命令解释
demo_explanation() {
    echo "演示4: 命令解释器"
    echo "------------------------"
    echo "命令: find /var/log -name '*.log' -mtime -7 -exec gzip {} \;"
    echo ""
    echo "解释:"
    echo "  find    - 查找文件的命令"
    echo "  /var/log - 搜索的起始目录"
    echo "  -name '*.log' - 匹配.log后缀的文件"
    echo "  -mtime -7 - 7天内修改的文件"
    echo "  -exec gzip {} \; - 对每个文件执行gzip压缩"
    echo ""
    echo "总结: 查找并压缩最近7天内修改的所有日志文件"
    echo ""
    sleep 2
}

# 演示5: 安全审计
demo_security() {
    echo "演示5: AI安全审计"
    echo "------------------------"
    echo "命令: 'rm -rf /'"
    echo "审计结果:"
    echo "  风险等级: 🔴 严重 (Critical)"
    echo "  威胁: 将递归删除整个文件系统"
    echo "  建议: 不要执行此命令！"
    echo ""
    echo "命令: 'chmod 777 /etc'"
    echo "审计结果:"
    echo "  风险等级: 🟠 高风险 (High)"
    echo "  威胁: 过度开放系统目录权限"
    echo "  建议: 使用 chmod 755 替代"
    echo ""
    sleep 2
}

# 演示6: 日志分析
demo_logs() {
    echo "演示6: 智能日志分析"
    echo "------------------------"
    echo "日志内容示例:"
    echo "  [ERROR] Connection refused: localhost:3306"
    echo "  [WARN]  High memory usage: 87%"
    echo "  [ERROR] Disk full: /var partition"
    echo ""
    echo "分析结果:"
    echo "  发现3个问题:"
    echo "  1. 数据库连接失败 (严重)"
    echo "  2. 内存使用率过高 (中等)"
    echo "  3. 磁盘空间不足 (严重)"
    echo ""
    echo "建议操作:"
    echo "  1. 检查MySQL服务状态"
    echo "  2. 清理不必要的进程"
    echo "  3. 清理日志文件或扩展存储"
    echo ""
    sleep 2
}

# 演示7: 快捷操作
demo_quick_actions() {
    echo "演示7: 快捷操作"
    echo "------------------------"
    echo "快捷按钮功能:"
    echo "  [Explain this command]  - 解释当前命令"
    echo "  [Find errors in output] - 分析输出中的错误"
    echo "  [Is this safe?]         - 审计命令安全性"
    echo "  [Better alternatives?]  - 获取更好的替代命令"
    echo ""
    echo "一键操作，无需手动输入！"
    echo ""
    sleep 2
}

# 主程序
main() {
    echo "欢迎使用 EasySSH AI智能终端"
    echo ""

    demo_natural_language
    demo_completion
    demo_error_diagnosis
    demo_explanation
    demo_security
    demo_logs
    demo_quick_actions

    echo "================================================"
    echo "  演示结束"
    echo "================================================"
    echo ""
    echo "使用方法:"
    echo "1. 启动EasySSH"
    echo "2. 点击工具栏 '🧐' AI Assistant按钮"
    echo "3. 配置AI提供商(Claude/OpenAI/本地模型)"
    echo "4. 开始使用AI智能功能!"
    echo ""
    echo "更多信息请查看:"
    echo "- AI_TERMINAL.md - 完整文档"
    echo "- AI_QUICKSTART.md - 快速启动指南"
}

main
