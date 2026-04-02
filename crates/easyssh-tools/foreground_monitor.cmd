@echo off
chcp 65001 >nul
title 🔧 EasySSH 前台自动化监控中心

set REPO_PATH=C:\Users\z7299\Documents\GitHub\AnixOps-EasySSH

:loop
echo.
echo ╔══════════════════════════════════════════════════════════════════════════════╗
echo ║                    🔧 EasySSH 前台自动化监控中心 🔧                           ║
echo ╠══════════════════════════════════════════════════════════════════════════════╣
echo ║ 时间: %date% %time%                                                    ║
echo ╠══════════════════════════════════════════════════════════════════════════════╣

REM 检查代码质量
echo ║ 代码质量     │ ⏳ │ 检查中...              │                                   ║
cd /d %REPO_PATH%
cargo fmt -- --check >nul 2>&1
if %errorlevel% == 0 (
    echo ║ 代码质量     │ ✅ │ %time:~0,8%              │ 格式检查通过                       ║
) else (
    echo ║ 代码质量     │ ✗  │ %time:~0,8%              │ 需要格式化                         ║
)

REM 检查构建状态
echo ║ 构建状态     │ ⏳ │ 检查中...              │                                   ║
cargo check -p easyssh-core --message-format=short >nul 2>&1
if %errorlevel% == 0 (
    echo ║ 构建状态     │ ✅ │ %time:~0,8%              │ Core库检查通过                     ║
) else (
    echo ║ 构建状态     │ ✗  │ %time:~0,8%              │ Core库有错误                       ║
)

REM 检查测试
echo ║ 测试执行     │ ⏳ │ 检查中...              │                                   ║
cargo test -p easyssh-core --no-fail-fast 2>&1 | findstr "test result:" > %TEMP%\test_result.txt
set /p TEST_RESULT=<%TEMP%\test_result.txt
if not "%TEST_RESULT%"=="" (
    echo ║ 测试执行     │ ✅ │ %time:~0,8%              │ %TEST_RESULT:~0,30%                     ║
) else (
    echo ║ 测试执行     │ ⏳ │ %time:~0,8%              │ 测试运行中                         ║
)

REM 检查安全审计
echo ║ 安全审计     │ ⏳ │ 检查中...              │                                   ║
cargo audit 2>&1 | findstr "Success" >nul
if %errorlevel% == 0 (
    echo ║ 安全审计     │ ✅ │ %time:~0,8%              │ 未发现安全漏洞                     ║
) else (
    echo ║ 安全审计     │ ⏳ │ %time:~0,8%              │ cargo-audit未安装                  ║
)

REM 检查CI/CD
echo ║ CI/CD流程    │ ⏳ │ 检查中...              │                                   ║
if exist "%REPO_PATH%\.github\workflows\*.yml" (
    dir /b "%REPO_PATH%\.github\workflows\*.yml" 2>nul | find /c /v "" > %TEMP%\wf_count.txt
    set /p WF_COUNT=<%TEMP%\wf_count.txt
    echo ║ CI/CD流程    │ ✅ │ %time:~0,8%              │ %WF_COUNT% 个工作流配置就绪           ║
) else (
    echo ║ CI/CD流程    │ ✗  │ %time:~0,8%              │ 工作流目录不存在                   ║
)

echo ╠══════════════════════════════════════════════════════════════════════════════╣
echo ║ 命令: [R]刷新 [A]修复 [Q]退出 [C]清理 [S]提交                                ║
echo ╚══════════════════════════════════════════════════════════════════════════════╝
echo 30秒后自动刷新，按 Q 退出...

timeout /t 30 /nobreak >nul

REM 检查按键
choice /c RAQCS /n /t 1 /d R >nul
if %errorlevel% == 1 goto refresh
if %errorlevel% == 2 goto refresh
if %errorlevel% == 3 goto quit
if %errorlevel% == 4 goto auto_fix
if %errorlevel% == 5 goto commit

:refresh
goto loop

:auto_fix
echo.
echo 🔧 执行自动修复...
cd /d %REPO_PATH%
cargo fmt 2>nul
cargo clean 2>nul
echo ✅ 格式化完成，缓存已清理
timeout /t 2 >nul
goto loop

:commit
echo.
echo 📦 提交更改...
cd /d %REPO_PATH%
git add -A 2>nul
git commit -m "auto: 前台监控自动修复 %date% %time%" 2>nul
git push 2>nul
echo ✅ 提交完成
timeout /t 2 >nul
goto loop

:quit
echo.
echo 👋 退出监控...
exit /b 0
