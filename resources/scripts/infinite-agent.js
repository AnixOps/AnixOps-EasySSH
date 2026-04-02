#!/usr/bin/env node
/**
 * INFINITE BUILD AGENT
 * 真正的无限循环 - 不做好就不停
 *
 * 这个Agent会持续运行，直到构建成功
 * 没有最大迭代次数限制
 */

const { spawn } = require('child_process');
const path = require('path');

const PROJECT_ROOT = path.resolve(__dirname, '..');
let ITERATION = 0;
const START_TIME = Date.now();

// Colors
const RED = '\x1b[31m';
const GREEN = '\x1b[32m';
const YELLOW = '\x1b[33m';
const CYAN = '\x1b[36m';
const RESET = '\x1b[0m';

function log(msg, color = RESET) {
    console.log(`${color}[${new Date().toISOString()}] ${msg}${RESET}`);
}

function formatElapsed() {
    const elapsed = Math.floor((Date.now() - START_TIME) / 1000);
    const h = Math.floor(elapsed / 3600);
    const m = Math.floor((elapsed % 3600) / 60);
    const s = elapsed % 60;
    return `${h.toString().padStart(2, '0')}:${m.toString().padStart(2, '0')}:${s.toString().padStart(2, '0')}`;
}

function runCommand(cmd, args, options = {}) {
    return new Promise((resolve, reject) => {
        const child = spawn(cmd, args, {
            cwd: PROJECT_ROOT,
            stdio: options.silent ? 'pipe' : 'inherit',
            ...options
        });

        let stdout = '';
        let stderr = '';

        if (options.silent) {
            child.stdout?.on('data', (data) => { stdout += data; });
            child.stderr?.on('data', (data) => { stderr += data; });
        }

        child.on('close', (code) => {
            if (code === 0) {
                resolve({ stdout, stderr, code });
            } else {
                reject({ stdout, stderr, code });
            }
        });

        child.on('error', reject);
    });
}

async function applyFixes() {
    log('应用修复...', YELLOW);

    try {
        await runCommand('cargo', ['clean', '-p', 'easyssh-core'], { silent: true });
    } catch (e) { /* ignore */ }

    try {
        await runCommand('cargo', ['update'], { silent: true });
    } catch (e) { /* ignore */ }

    try {
        await runCommand('cargo', ['fmt'], { silent: true });
    } catch (e) { /* ignore */ }

    log('修复完成，5秒后重试...', YELLOW);
    await new Promise(r => setTimeout(r, 5000));
}

async function buildCore() {
    log('[1/4] 构建 Core Library...', CYAN);
    try {
        await runCommand('cargo', ['build', '--release', '-p', 'easyssh-core'], { silent: true });
        log('[OK] Core 构建成功', GREEN);
        return true;
    } catch (e) {
        log('[X] Core 构建失败', RED);
        return false;
    }
}

async function testCore() {
    log('[2/4] 测试 Core Library...', CYAN);
    try {
        await runCommand('cargo', ['test', '--release', '-p', 'easyssh-core'], { silent: true });
        log('[OK] Core 测试通过', GREEN);
        return true;
    } catch (e) {
        log('[X] Core 测试失败', RED);
        return false;
    }
}

async function buildTUI() {
    log('[3/4] 构建 TUI...', CYAN);
    try {
        await runCommand('cargo', ['build', '--release', '-p', 'easyssh-tui'], { silent: true });
        log('[OK] TUI 构建成功', GREEN);
        return true;
    } catch (e) {
        log('[X] TUI 构建失败', RED);
        return false;
    }
}

async function testTUI() {
    log('[4/4] 测试 TUI...', CYAN);
    const platform = process.platform;
    const binaryName = platform === 'win32' ? 'easyssh.exe' : 'easyssh';
    const binaryPath = path.join(PROJECT_ROOT, 'target', 'release', binaryName);

    try {
        await runCommand(binaryPath, ['--version'], { silent: true });
        log('[OK] TUI 测试通过', GREEN);
        return true;
    } catch (e) {
        log('[X] TUI 测试失败', RED);
        return false;
    }
}

async function iteration() {
    ITERATION++;

    console.clear();
    console.log(`${CYAN}========================================${RESET}`);
    console.log(`${CYAN}   迭代 ${ITERATION} - 已运行 ${formatElapsed()}${RESET}`);
    console.log(`${CYAN}========================================${RESET}`);
    console.log();

    // Step 1: Build Core
    if (!await buildCore()) {
        await applyFixes();
        return false;
    }

    // Step 2: Test Core
    if (!await testCore()) {
        await new Promise(r => setTimeout(r, 5000));
        return false;
    }

    // Step 3: Build TUI
    if (!await buildTUI()) {
        await applyFixes();
        return false;
    }

    // Step 4: Test TUI
    if (!await testTUI()) {
        await new Promise(r => setTimeout(r, 5000));
        return false;
    }

    // SUCCESS!
    console.log();
    console.log(`${GREEN}========================================${RESET}`);
    console.log(`${GREEN}   🎉 成功! 第 ${ITERATION} 次迭代${RESET}`);
    console.log(`${GREEN}========================================${RESET}`);
    console.log();
    console.log(`${GREEN}✅ Core Library 构建成功${RESET}`);
    console.log(`${GREEN}✅ 所有测试通过${RESET}`);
    console.log(`${GREEN}✅ TUI 构建成功${RESET}`);
    console.log();
    console.log(`总用时: ${formatElapsed()}`);
    console.log();

    return true;
}

async function main() {
    console.clear();
    console.log(`${CYAN}========================================${RESET}`);
    console.log(`${CYAN}   无限构建 Agent - 永不停止${RESET}`);
    console.log(`${CYAN}========================================${RESET}`);
    console.log();
    console.log(`项目: ${PROJECT_ROOT}`);
    console.log(`开始: ${new Date().toLocaleString()}`);
    console.log();
    console.log(`${YELLOW}按 Ctrl+C 停止${RESET}`);
    console.log();

    await new Promise(r => setTimeout(r, 3000));

    // TRUE INFINITE LOOP
    while (true) {
        const success = await iteration();
        if (success) {
            process.exit(0);
        }
        // 否则继续循环，永不停止
    }
}

main().catch(err => {
    log(`错误: ${err.message}`, RED);
    process.exit(1);
});
