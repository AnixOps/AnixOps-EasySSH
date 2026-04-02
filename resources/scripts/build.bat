@echo off
setlocal EnableDelayedExpansion

rem EasySSH Unified Build Script for Windows
rem Supports building all three editions: Lite, Standard, Pro
rem Usage: build.bat [lite|standard|pro] [--sign] [--release]

title EasySSH Build System

set VERSION=0.3.0
set EDITION=standard
set PROFILE=release-standard
set SHOULD_SIGN=false
set SKIP_TESTS=false

rem Parse arguments
:parse_args
if "%~1"=="" goto :done_parsing

if /I "%~1"=="lite" (
    set EDITION=lite
    set PROFILE=release-lite
    shift
    goto :parse_args
)

if /I "%~1"=="standard" (
    set EDITION=standard
    set PROFILE=release-standard
    shift
    goto :parse_args
)

if /I "%~1"=="pro" (
    set EDITION=pro
    set PROFILE=release-pro
    shift
    goto :parse_args
)

if /I "%~1"=="--sign" (
    set SHOULD_SIGN=true
    shift
    goto :parse_args
)

if /I "%~1"=="--release" (
    rem Already set
    shift
    goto :parse_args
)

if /I "%~1"=="--dev" (
    set PROFILE=dev
    shift
    goto :parse_args
)

if /I "%~1"=="--skip-tests" (
    set SKIP_TESTS=true
    shift
    goto :parse_args
)

if /I "%~1"=="--help" (
    call :show_help
    exit /b 0
)

echo [ERROR] Unknown argument: %~1
call :show_help
exit /b 1

:done_parsing

echo ========================================
echo   EasySSH Build System v%VERSION%
echo ========================================
echo.
echo Edition: %EDITION%
echo Profile: %PROFILE%
echo Sign:    %SHOULD_SIGN%
echo.

rem Set environment variables
set RUSTFLAGS=-C target-feature=+crt-static
set CARGO_EASYSSH_EDITION=%EDITION%

rem Create release directory
set RELEASE_DIR=%~dp0..\releases\%EDITION%-v%VERSION%
if not exist "%RELEASE_DIR%" mkdir "%RELEASE_DIR%"

echo [INFO] Building EasySSH %EDITION% for Windows x64...
echo.

cd %~dp0..\crates\easyssh-platforms\windows\easyssh-winui

cargo build --profile %PROFILE% --features %EDITION% --no-default-features
if errorlevel 1 (
    echo [ERROR] Build failed!
    exit /b 1
)

echo.
echo [SUCCESS] Build complete!
echo.

rem Package
set BINARY_PATH=target\%PROFILE%\EasySSH.exe
set PKG_DIR=%RELEASE_DIR%\easyssh-%EDITION%-%VERSION%-windows-x64

if not exist "%PKG_DIR%" mkdir "%PKG_DIR%"
copy "%BINARY_PATH%" "%PKG_DIR%\EasySSH.exe"

rem Create README
echo EasySSH %EDITION% v%VERSION% for Windows x64 > "%PKG_DIR%\README.txt"
echo ================================================ >> "%PKG_DIR%\README.txt"
echo. >> "%PKG_DIR%\README.txt"
echo Quick Start: >> "%PKG_DIR%\README.txt"
echo 1. Run EasySSH.exe >> "%PKG_DIR%\README.txt"
echo 2. Add your SSH servers via the UI >> "%PKG_DIR%\README.txt"
echo 3. Connect using password or key authentication >> "%PKG_DIR%\README.txt"
echo. >> "%PKG_DIR%\README.txt"
echo System Requirements: >> "%PKG_DIR%\README.txt"
echo - Windows 10/11 64-bit >> "%PKG_DIR%\README.txt"
echo - No additional dependencies required >> "%PKG_DIR%\README.txt"
echo. >> "%PKG_DIR%\README.txt"
echo For support: https://github.com/anixops/easyssh >> "%PKG_DIR%\README.txt"

rem Create ZIP
cd %RELEASE_DIR%
powershell -Command "Compress-Archive -Path '%PKG_DIR%' -DestinationPath 'easyssh-%EDITION%-%VERSION%-windows-x64.zip' -Force"

echo [SUCCESS] Package created: easyssh-%EDITION%-%VERSION%-windows-x64.zip

rem Sign if requested
if "%SHOULD_SIGN%"=="true" (
    call :sign_binary "%PKG_DIR%\EasySSH.exe"
)

echo.
echo ========================================
echo   Build Complete: EasySSH %EDITION% v%VERSION%
echo ========================================
echo.
echo Output: %RELEASE_DIR%
echo.

rem Show sizes
dir /s /-c "%RELEASE_DIR%" | findstr "total"

exit /b 0

:sign_binary
set BINARY=%~1
echo [INFO] Signing binary: %BINARY%

if not defined WINDOWS_CERTIFICATE_THUMBPRINT (
    echo [WARN] WINDOWS_CERTIFICATE_THUMBPRINT not set, skipping signing
    exit /b 0
)

if exist "%ProgramFiles(x86)%\Windows Kits\10\bin\10.0.19041.0\x64\signtool.exe" (
    "%ProgramFiles(x86)%\Windows Kits\10\bin\10.0.19041.0\x64\signtool.exe" sign /sha1 %WINDOWS_CERTIFICATE_THUMBPRINT% /tr http://timestamp.digicert.com /td sha256 /fd sha256 "%BINARY%"
    echo [SUCCESS] Binary signed
) else (
    echo [WARN] signtool.exe not found, skipping signing
)
exit /b 0

:show_help
echo Usage: build.bat [OPTIONS] [EDITION]
echo.
echo Build EasySSH for Windows.echo.
echo Arguments:
echo   EDITION      Build edition: lite, standard, pro (default: standard)
echo.
echo Options:
echo   --sign       Sign the resulting binaries
echo   --release    Use release profile (default)
echo   --dev        Use dev profile
echo   --skip-tests Skip running tests
echo   --help       Show this help message
echo.
echo Examples:
echo   build.bat lite              # Build Lite edition
echo   build.bat standard --sign   # Build Standard with signing
echo   build.bat pro --release     # Build Pro release
echo.
exit /b 0
