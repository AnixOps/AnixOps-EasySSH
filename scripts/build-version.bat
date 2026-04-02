@echo off
REM EasySSH 版本化构建脚本 (Windows)
REM 用法: scripts\build-version.bat [lite|standard|pro] [debug|release]

setlocal EnableDelayedExpansion

REM 默认值
set "VERSION=%~1"
if "!VERSION!"=="" set "VERSION=standard"

set "PROFILE=%~2"
if "!PROFILE!"=="" set "PROFILE=release"

REM 验证版本参数
if /I not "!VERSION!"=="lite" if /I not "!VERSION!"=="standard" if /I not "!VERSION!"=="pro" (
    echo [ERROR] 无效版本: !VERSION!
    echo 用法: %0 [lite^|standard^|pro] [debug^|release]
    exit /b 1
)

REM 验证profile参数
if /I not "!PROFILE!"=="debug" if /I not "!PROFILE!"=="release" (
    echo [ERROR] 无效profile: !PROFILE!
    echo 用法: %0 [lite^|standard^|pro] [debug^|release]
    exit /b 1
)

REM 设置target目录
set "CARGO_TARGET_DIR=target\!VERSION!"
set "CARGO_EASYSSH_VERSION=!VERSION!"

echo ========================================
echo   EasySSH !VERSION! 版本构建
echo   Profile: !PROFILE!
echo   Target: !CARGO_TARGET_DIR!
echo ========================================

REM 创建target目录
if not exist "!CARGO_TARGET_DIR!" mkdir "!CARGO_TARGET_DIR!"

REM 根据版本选择feature
if /I "!VERSION!"=="lite" (
    set "FEATURES=lite"
) else if /I "!VERSION!"=="standard" (
    set "FEATURES=standard"
) else if /I "!VERSION!"=="pro" (
    set "FEATURES=pro"
)

REM 构建命令
if /I "!PROFILE!"=="release" (
    echo [INFO] 执行Release构建 ^(optimized^)...
    set "PROFILE_NAME=release-!VERSION!"
    cargo build --profile !PROFILE_NAME! --features !FEATURES!
) else (
    echo [INFO] 执行Debug构建...
    cargo build --features !FEATURES!
)

REM 检查构建结果
if %ERRORLEVEL% equ 0 (
    echo [SUCCESS] 构建成功!
    echo.
    echo 构建产物:
    dir /b /s "!CARGO_TARGET_DIR!\*.exe" 2>nul | findstr /i "easyssh" | head -5
    echo.
    echo 目录大小:
    for /f "tokens=*" %%a in ('dir /s "!CARGO_TARGET_DIR!" 2^>nul ^| findstr "字节"') do (
        echo %%a
        goto :size_done
    )
    :size_done
) else (
    echo [ERROR] 构建失败
    exit /b 1
)

endlocal
