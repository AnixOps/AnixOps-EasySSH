@echo off
REM Build Windows Installers for all EasySSH versions
REM Usage: build-windows.bat [version]
REM Example: build-windows.bat 0.3.0

setlocal EnableDelayedExpansion

set "VERSION=%~1"
if "%VERSION%"=="" set "VERSION=0.3.0"

echo ========================================
echo EasySSH Windows Installer Build
echo Version: %VERSION%
echo ========================================

set "SCRIPT_DIR=%~dp0"
set "PROJECT_ROOT=%SCRIPT_DIR%../../.."
set "INSTALLER_DIR=%PROJECT_ROOT%\resources\installer\windows"
set "RELEASE_DIR=%PROJECT_ROOT%\releases\v%VERSION%\windows"

REM Check prerequisites
echo Checking prerequisites...

where candle.exe >nul 2>nul
if %ERRORLEVEL% neq 0 (
    if not exist "%WIX%\bin\candle.exe" (
        echo Error: WiX Toolset not found.
        echo Please install WiX v3.11+ from https://wixtoolset.org/releases/
        exit /b 1
    )
    set "WIX_CANDLE=%WIX%\bin\candle.exe"
    set "WIX_LIGHT=%WIX%\bin\light.exe"
) else (
    set "WIX_CANDLE=candle.exe"
    set "WIX_LIGHT=light.exe"
)

where makensis.exe >nul 2>nul
if %ERRORLEVEL% neq 0 (
    echo Error: NSIS not found.
    echo Please install NSIS 3.0+ from https://nsis.sourceforge.io/Download
    exit /b 1
)

REM Create release directory
if not exist "%RELEASE_DIR%" mkdir "%RELEASE_DIR%"

REM Build Lite
echo.
echo Building EasySSH Lite...
if exist "%PROJECT_ROOT%\target\release-lite" (
    call :build_wix "lite" "%INSTALLER_DIR%\wix\lite\EasySSH-Lite.wxs" "%PROJECT_ROOT%\target\release-lite"
    call :build_nsis "lite" "%INSTALLER_DIR%\nsis\easyssh-lite.nsi"
    call :create_portable "lite" "%PROJECT_ROOT%\target\release-lite" "easyssh-lite.exe"
) else (
    echo Warning: Lite build not found, skipping...
)

REM Build Standard
echo.
echo Building EasySSH Standard...
if exist "%PROJECT_ROOT%\target\release-standard" (
    call :build_wix "standard" "%INSTALLER_DIR%\wix\standard\EasySSH-Standard.wxs" "%PROJECT_ROOT%\target\release-standard"
    call :build_nsis "standard" "%INSTALLER_DIR%\nsis\easyssh-standard.nsi"
    call :create_portable "standard" "%PROJECT_ROOT%\target\release-standard" "easyssh-standard.exe"
) else (
    echo Warning: Standard build not found, skipping...
)

REM Build Pro
echo.
echo Building EasySSH Pro...
if exist "%PROJECT_ROOT%\target\release-pro" (
    call :build_wix "pro" "%INSTALLER_DIR%\wix\pro\EasySSH-Pro.wxs" "%PROJECT_ROOT%\target\release-pro"
    call :build_nsis "pro" "%INSTALLER_DIR%\nsis\easyssh-pro.nsi"
    call :create_portable "pro" "%PROJECT_ROOT%\target\release-pro" "easyssh-pro.exe"
) else (
    echo Warning: Pro build not found, skipping...
)

REM Generate checksums
echo.
echo Generating checksums...
cd /d "%RELEASE_DIR%"
certutil -hashfile "EasySSH-lite-%VERSION%-x64.msi" SHA256 > SHA256SUMS.txt 2>nul
certutil -hashfile "EasySSH-standard-%VERSION%-x64.msi" SHA256 >> SHA256SUMS.txt 2>nul
certutil -hashfile "EasySSH-pro-%VERSION%-x64.msi" SHA256 >> SHA256SUMS.txt 2>nul

echo.
echo ========================================
echo Build Complete!
echo ========================================
echo.
echo Output directory: %RELEASE_DIR%
echo.
dir "%RELEASE_DIR%"

goto :eof

:build_wix
set "VERSION_NAME=%~1"
set "WXS_FILE=%~2"
set "SOURCE_DIR=%~3"

echo   Compiling WiX for %VERSION_NAME%...

set "BUILD_DIR=%INSTALLER_DIR%\wix\%VERSION_NAME%\build"
if not exist "%BUILD_DIR%" mkdir "%BUILD_DIR%"

"%WIX_CANDLE%" -arch x64 ^
    -dVersion="%VERSION%" ^
    -dSourceDir="%SOURCE_DIR%" ^
    -out "%BUILD_DIR%\" ^
    "%WXS_FILE%"

if %ERRORLEVEL% neq 0 (
    echo Error: WiX compilation failed for %VERSION_NAME%
    exit /b 1
)

echo   Linking MSI for %VERSION_NAME%...
"%WIX_LIGHT%" -ext WixUIExtension ^
    -out "%RELEASE_DIR%\EasySSH-%VERSION_NAME%-%VERSION%-x64.msi" ^
    "%BUILD_DIR%\EasySSH-%VERSION_NAME%.wixobj"

if %ERRORLEVEL% neq 0 (
    echo Error: WiX linking failed for %VERSION_NAME%
    exit /b 1
)

echo   MSI created: EasySSH-%VERSION_NAME%-%VERSION%-x64.msi
goto :eof

:build_nsis
set "VERSION_NAME=%~1"
set "NSI_FILE=%~2"

echo   Building NSIS for %VERSION_NAME%...

cd /d "%~dp2"
makensis.exe /DPRODUCT_VERSION="%VERSION%" "%~nx2"

if %ERRORLEVEL% neq 0 (
    echo Error: NSIS build failed for %VERSION_NAME%
    exit /b 1
)

REM Move output to releases
move "EasySSH-%VERSION_NAME%-%VERSION%-x64.exe" "%RELEASE_DIR%\" >nul

echo   NSIS installer created: EasySSH-%VERSION_NAME%-%VERSION%-x64.exe
goto :eof

:create_portable
set "VERSION_NAME=%~1"
set "SOURCE_DIR=%~2"
set "EXE_NAME=%~3"

echo   Creating portable ZIP for %VERSION_NAME%...

set "PORTABLE_DIR=%RELEASE_DIR%\portable-%VERSION_NAME%"
if not exist "%PORTABLE_DIR%" mkdir "%PORTABLE_DIR%"

REM Copy files
copy "%SOURCE_DIR%\%EXE_NAME%" "%PORTABLE_DIR%\" >nul
copy "%SOURCE_DIR%\icon.ico" "%PORTABLE_DIR%\" >nul 2>nul
copy "%PROJECT_ROOT%\LICENSE" "%PORTABLE_DIR%\" >nul

REM Create README
echo EasySSH %VERSION_NAME% v%VERSION% (Portable)> "%PORTABLE_DIR%\README.txt"
echo ============================================>> "%PORTABLE_DIR%\README.txt"
echo.>> "%PORTABLE_DIR%\README.txt"
echo This is a portable version of EasySSH %VERSION_NAME%.>> "%PORTABLE_DIR%\README.txt"
echo No installation required - just extract and run.>> "%PORTABLE_DIR%\README.txt"
echo.>> "%PORTABLE_DIR%\README.txt"
echo Usage:>> "%PORTABLE_DIR%\README.txt"
echo 1. Extract this ZIP to any location (e.g., USB drive)>> "%PORTABLE_DIR%\README.txt"
echo 2. Run %EXE_NAME%>> "%PORTABLE_DIR%\README.txt"
echo 3. Your data is stored in the same directory>> "%PORTABLE_DIR%\README.txt"
echo.>> "%PORTABLE_DIR%\README.txt"
echo For support, visit: https://github.com/anixops/easyssh>> "%PORTABLE_DIR%\README.txt"

REM Create ZIP
cd /d "%RELEASE_DIR%"
7z a -tzip "EasySSH-%VERSION_NAME%-%VERSION%-portable.zip" "portable-%VERSION_NAME%\" >nul
rmdir /s /q "portable-%VERSION_NAME%"

echo   Portable ZIP created: EasySSH-%VERSION_NAME%-%VERSION%-portable.zip
goto :eof
