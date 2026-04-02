@echo off
chcp 65001 >nul
echo ============================================
echo EasySSH Windows Local Build
echo ============================================
echo.

REM Check prerequisites
where rustc >nul 2>nul
if %errorlevel% neq 0 (
    echo [ERROR] Rust not found. Please install Rust from https://rustup.rs/
    exit /b 1
)

echo [OK] Rust found
rustc --version
echo.

REM Build Core Library
echo ============================================
echo Step 1: Building Core Library
echo ============================================
cd /d %~dp0\..\..\..\..
cargo build --release -p easyssh-core
if %errorlevel% neq 0 (
    echo [ERROR] Core library build failed
    exit /b 1
)
echo [OK] Core library built successfully
echo.

REM Build TUI (Cross-platform CLI)
echo ============================================
echo Step 2: Building TUI (Terminal UI)
echo ============================================
cargo build --release -p easyssh-tui
if %errorlevel% neq 0 (
    echo [ERROR] TUI build failed
    exit /b 1
)
echo [OK] TUI built successfully
echo.

REM Test TUI
echo ============================================
echo Step 3: Testing TUI
echo ============================================
.\target\release\easyssh.exe --version
if %errorlevel% neq 0 (
    echo [ERROR] TUI test failed
    exit /b 1
)
.\target\release\easyssh.exe --help
echo [OK] TUI works
echo.

REM Build Windows Native (simplified for now)
echo ============================================
echo Step 4: Checking Windows Native Project
echo ============================================
cd /d %~dp0\..\..\..\..\platforms\windows\easyssh-winui

echo Windows native app requires:
echo   - Visual Studio 2022 with C++ build tools
echo   - Windows App SDK
echo   - WinUI 3
echo.
echo For now, using TUI as the Windows interface.
echo.
echo To build native app (requires full Windows dev environment):
echo   cd platforms\windows\easyssh-winui
echo   cargo build --release
echo.

REM Create desktop shortcut for TUI
echo ============================================
echo Step 5: Creating Desktop Shortcut
echo ============================================
set TARGET_DIR=%~dp0\..\..\..\..\target\release
set SHORTCUT_PATH=%USERPROFILE%\Desktop\EasySSH.lnk

powershell -Command "$WshShell = New-Object -comObject WScript.Shell; $Shortcut = $WshShell.CreateShortcut('%SHORTCUT_PATH%'); $Shortcut.TargetPath = '%TARGET_DIR%\easyssh.exe'; $Shortcut.WorkingDirectory = '%TARGET_DIR%'; $Shortcut.Description = 'EasySSH Terminal UI'; $Shortcut.Save()"

echo [OK] Desktop shortcut created: %SHORTCUT_PATH%
echo.

echo ============================================
echo Build Summary
echo ============================================
echo Core Library: %TARGET_DIR%\easyssh_core.dll
echo TUI Binary:   %TARGET_DIR%\easyssh.exe
echo.
echo To use EasySSH:
echo   1. Double-click EasySSH icon on desktop
echo   2. Or open terminal and run: easyssh --help
echo.
pause
