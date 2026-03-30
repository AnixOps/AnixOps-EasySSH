@echo off
chcp 65001 >nul
echo 🚀 EasySSH Autonomous Development
echo ===================================

REM 检查 babysitter 是否可用
where a5c >nul 2>nul
if %errorlevel% neq 0 (
    echo ⚠️  Babysitter CLI ^(a5c^) 未安装
    echo 请先安装: npm install -g @a5c-ai/babysitter-cli
    exit /b 1
)

REM 默认参数
set "MODE=%~1"
if "%MODE%"=="" set "MODE=core"

set "ITERATIONS=%~2"
if "%ITERATIONS%"=="" set "ITERATIONS=50"

set "AUTO_FIX=%~3"
if "%AUTO_FIX%"=="" set "AUTO_FIX=true"

echo.
echo 📋 配置:
echo    模式: %MODE%
echo    迭代: %ITERATIONS%
echo    自动修复: %AUTO_FIX%
echo.

REM 运行 babysitter 流程
echo 🏃 启动自动化流程...
a5c run easyssh-autonomous-dev --input "{\"maxIterations\": %ITERATIONS%, \"testMode\": \"%MODE%\", \"autoFix\": %AUTO_FIX%}" --output "runs/autonomous-%date:~-4,4%%date:~-10,2%%date:~-7,2%-%time:~0,2%%time:~3,2%%time:~6,2%.json"

echo.
echo ✅ 流程完成！
pause
