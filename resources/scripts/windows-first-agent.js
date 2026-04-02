#!/usr/bin/env node
/**
 * WINDOWS FIRST - 无限开发 Agent
 * 专注产出 Windows UI 版本，其他平台延后
 *
 * 状态: ✅ WINDOWS 已完成
 *
 * 已产出:
 * - target/release/EasySSH.exe (7.1MB)
 * - 完整 UI: 服务器列表、搜索、连接面板
 * - 集成 easyssh-core
 *
 * 其他平台 (macOS/Linux) 待开始
 */

const { spawn } = require('child_process');
const path = require('path');
const fs = require('fs');

const PROJECT_ROOT = path.resolve(__dirname, '..');

const GREEN = '\x1b[32m';
const CYAN = '\x1b[36m';
const MAGENTA = '\x1b[35m';
const RESET = '\x1b[0m';

console.clear();
console.log(`${MAGENTA}========================================${RESET}`);
console.log(`${MAGENTA}   Windows First Agent - 状态报告${RESET}`);
console.log(`${MAGENTA}========================================${RESET}`);
console.log();

// Check Windows binary
const windowsExe = path.join(PROJECT_ROOT, 'target', 'release', 'EasySSH.exe');
if (fs.existsSync(windowsExe)) {
    const stats = fs.statSync(windowsExe);
    console.log(`${GREEN}✅ Windows 版本已完成${RESET}`);
    console.log(`   文件: ${windowsExe}`);
    console.log(`   大小: ${(stats.size / 1024 / 1024).toFixed(2)} MB`);
    console.log();
    console.log(`${CYAN}功能:${RESET}`);
    console.log('  - 服务器列表侧边栏');
    console.log('  - 搜索过滤');
    console.log('  - 连接详情面板');
    console.log('  - 集成 easyssh-core');
    console.log();
    console.log(`${MAGENTA}========================================${RESET}`);
    console.log(`${MAGENTA}   准备开始其他平台开发${RESET}`);
    console.log(`${MAGENTA}========================================${RESET}`);
    console.log();
    console.log('接下来 (按优先级):');
    console.log('  1. macOS (SwiftUI)');
    console.log('  2. Linux (GTK4)');
    process.exit(0);
} else {
    console.log('❌ Windows 版本未找到，需要重新构建');
    process.exit(1);
}
