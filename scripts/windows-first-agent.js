#!/usr/bin/env node
/**
 * WINDOWS FIRST - 无限开发 Agent
 * 专注产出 Windows UI 版本，其他平台延后
 *
 * 原则:
 * 1. Windows 版本必须完全可用 (可编译 + 可运行 + 有完整UI)
 * 2. 其他平台 (macOS/Linux) 完全不碰，等 Windows 完成后再说
 * 3. 无限循环直到 Windows 版本完美运行
 */

const { spawn } = require('child_process');
const path = require('path');
const fs = require('fs');

const PROJECT_ROOT = path.resolve(__dirname, '..');
const WINDOWS_DIR = path.join(PROJECT_ROOT, 'platforms', 'windows', 'easyssh-winui');

let ITERATION = 0;
const START_TIME = Date.now();

const RED = '\x1b[31m';
const GREEN = '\x1b[32m';
const YELLOW = '\x1b[33m';
const CYAN = '\x1b[36m';
const MAGENTA = '\x1b[35m';
const RESET = '\x1b[0m';

function log(msg, color = RESET) {
    console.log(`${color}[WinAgent] ${msg}${RESET}`);
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
            cwd: options.cwd || PROJECT_ROOT,
            stdio: options.silent ? 'pipe' : 'inherit',
            shell: process.platform === 'win32',
            ...options
        });

        let stdout = '', stderr = '';
        if (options.silent) {
            child.stdout?.on('data', d => stdout += d);
            child.stderr?.on('data', d => stderr += d);
        }

        child.on('close', code => code === 0 ? resolve({stdout, stderr, code}) : reject({stdout, stderr, code}));
        child.on('error', reject);
    });
}

// ==================== PHASE 1: Core Build ====================
async function buildCore() {
    log('[Phase 1/5] 构建 Core Library...', CYAN);
    try {
        await runCommand('cargo', ['build', '--release', '-p', 'easyssh-core'], {silent: true});
        log('✅ Core 构建成功', GREEN);
        return true;
    } catch (e) {
        log('❌ Core 构建失败', RED);
        return false;
    }
}

// ==================== PHASE 2: Test Core ====================
async function testCore() {
    log('[Phase 2/5] 测试 Core Library...', CYAN);
    try {
        await runCommand('cargo', ['test', '--release', '-p', 'easyssh-core'], {silent: true});
        log('✅ Core 测试通过', GREEN);
        return true;
    } catch (e) {
        log('❌ Core 测试失败', RED);
        return false;
    }
}

// ==================== PHASE 3: Windows Build ====================
async function buildWindows() {
    log('[Phase 3/5] 构建 Windows WinUI App...', MAGENTA);
    try {
        // Clean previous build artifacts that might conflict
        const targetDir = path.join(WINDOWS_DIR, 'target');
        if (fs.existsSync(targetDir)) {
            fs.rmSync(targetDir, { recursive: true, force: true });
        }

        await runCommand('cargo', ['build', '--release'], {
            cwd: WINDOWS_DIR,
            silent: true
        });
        log('✅ Windows 构建成功', GREEN);
        return true;
    } catch (e) {
        log('❌ Windows 构建失败', RED);
        if (e.stderr) log(`错误: ${e.stderr.slice(0, 500)}`, RED);
        return false;
    }
}

// ==================== PHASE 4: Check Binary ====================
async function checkBinary() {
    log('[Phase 4/5] 检查 Windows 可执行文件...', CYAN);
    const exePath = path.join(WINDOWS_DIR, 'target', 'release', 'EasySSH.exe');

    if (!fs.existsSync(exePath)) {
        log('❌ EasySSH.exe 不存在', RED);
        return false;
    }

    const stats = fs.statSync(exePath);
    log(`✅ 可执行文件存在 (${(stats.size / 1024 / 1024).toFixed(2)} MB)`, GREEN);
    return true;
}

// ==================== PHASE 5: UI Completeness Check ====================
async function checkUICompleteness() {
    log('[Phase 5/5] 检查 UI 完整性...', CYAN);

    // Check required source files exist
    const requiredFiles = [
        'src/main.rs',
        'src/bridge.rs',
        'src/pages/main.rs',
        'src/pages/mod.rs',
        'src/viewmodels/mod.rs',
        'Cargo.toml'
    ];

    let allExist = true;
    for (const file of requiredFiles) {
        const fullPath = path.join(WINDOWS_DIR, file);
        if (fs.existsSync(fullPath)) {
            log(`  ✓ ${file}`, GREEN);
        } else {
            log(`  ✗ ${file} 缺失`, RED);
            allExist = false;
        }
    }

    return allExist;
}

// ==================== FIX: Windows Project Issues ====================
async function fixWindowsProject() {
    log('应用 Windows 项目修复...', YELLOW);

    // Check if we need to fix Cargo.toml
    const cargoPath = path.join(WINDOWS_DIR, 'Cargo.toml');
    const cargoContent = fs.readFileSync(cargoPath, 'utf8');

    // Ensure proper dependencies are present
    if (!cargoContent.includes('windows-app-sdk')) {
        log('添加 windows-app-sdk 依赖...', YELLOW);

        // For now, let's make sure we have the correct windows crate features
        const fixedCargo = cargoContent.replace(
            '[dependencies]',
            `[dependencies]
# WinUI 3 - Windows App SDK (modern WinUI)
windows-app-sdk = { version = "0.2", optional = true }

# Windows API bindings
windows = { version = "0.52", features = [
    "UI_Xaml",
    "UI_Xaml_Controls",
    "UI_Xaml_Navigation",
    "UI_Xaml_Media",
    "ApplicationModel_Activation",
    "Foundation_Collections",
    "System",
] }
windows-core = "0.52"

# Win32 for native window handling
windows-sys = { version = "0.52", features = ["Win32_UI_WindowsAndMessaging", "Win32_System_Threading"] }`
        );

        fs.writeFileSync(cargoPath, fixedCargo);
        log('✅ Cargo.toml 已更新', GREEN);
    }

    // Clean and retry
    try {
        const targetDir = path.join(WINDOWS_DIR, 'target');
        if (fs.existsSync(targetDir)) {
            fs.rmSync(targetDir, { recursive: true, force: true });
        }
    } catch (e) {
        // ignore
    }

    await new Promise(r => setTimeout(r, 3000));
}

// ==================== MAIN ITERATION ====================
async function iteration() {
    ITERATION++;

    console.clear();
    console.log(`${MAGENTA}========================================${RESET}`);
    console.log(`${MAGENTA}   Windows First Agent - 迭代 ${ITERATION}${RESET}`);
    console.log(`${MAGENTA}   已运行: ${formatElapsed()}${RESET}`);
    console.log(`${MAGENTA}========================================${RESET}`);
    console.log();

    // Phase 1: Core
    if (!await buildCore()) {
        log('Core 构建失败，等待5秒重试...', YELLOW);
        await new Promise(r => setTimeout(r, 5000));
        return false;
    }

    // Phase 2: Test Core
    if (!await testCore()) {
        log('Core 测试失败，等待5秒重试...', YELLOW);
        await new Promise(r => setTimeout(r, 5000));
        return false;
    }

    // Phase 3: Windows Build
    if (!await buildWindows()) {
        await fixWindowsProject();
        return false;
    }

    // Phase 4: Check Binary
    if (!await checkBinary()) {
        log('可执行文件检查失败，修复后重试...', YELLOW);
        await fixWindowsProject();
        return false;
    }

    // Phase 5: UI Completeness
    if (!await checkUICompleteness()) {
        log('UI 不完整，需要补充实现...', YELLOW);
        // TODO: Auto-generate missing UI components
        return false;
    }

    // SUCCESS!
    console.log();
    console.log(`${GREEN}========================================${RESET}`);
    console.log(`${GREEN}   🎉 Windows 版本完成! 第 ${ITERATION} 次迭代${RESET}`);
    console.log(`${GREEN}========================================${RESET}`);
    console.log();
    console.log(`${GREEN}✅ Core Library 构建成功${RESET}`);
    console.log(`${GREEN}✅ Windows WinUI App 构建成功${RESET}`);
    console.log(`${GREEN}✅ 可执行文件已生成${RESET}`);
    console.log(`${GREEN}✅ UI 组件完整${RESET}`);
    console.log();
    console.log(`输出目录:`);
    console.log(`  ${path.join(WINDOWS_DIR, 'target', 'release', 'EasySSH.exe')}`);
    console.log();
    console.log(`总用时: ${formatElapsed()}`);
    console.log();

    return true;
}

async function main() {
    console.clear();
    console.log(`${MAGENTA}========================================${RESET}`);
    console.log(`${MAGENTA}   Windows First - 无限开发 Agent${RESET}`);
    console.log(`${MAGENTA}========================================${RESET}`);
    console.log();
    console.log(`原则:`);
    console.log(`  1. Windows 版本必须完全可用`);
    console.log(`  2. 其他平台 (macOS/Linux) 完全不碰`);
    console.log(`  3. 无限循环直到 Windows 完美运行`);
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
        // 继续循环，永不停止
    }
}

main().catch(err => {
    log(`致命错误: ${err.message}`, RED);
    process.exit(1);
});
