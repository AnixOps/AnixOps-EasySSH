@echo off
REM EasySSH Windows Installer Build Batch Script
REM Wrapper for the bash script on Windows

setlocal EnableDelayedExpansion

REM Configuration
set VERSION=%1
if "%VERSION%"=="" set VERSION=0.3.0

set SOURCE_DIR=%2
if "%SOURCE_DIR%"=="" set SOURCE_DIR=..\..\target\release

echo ============================================
echo EasySSH Windows Installer Build
echo ============================================
echo Version: %VERSION%
echo Source: %SOURCE_DIR%
echo.

REM Check for prerequisites
echo Checking prerequisites...

where candle.exe >nul 2>nul
if %errorlevel% neq 0 (
    if exist "C:\Program Files (x86)\WiX Toolset v3.11\bin\candle.exe" (
        set "PATH=%PATH%;C:\Program Files (x86)\WiX Toolset v3.11\bin"
    ) else (
        echo [ERROR] WiX Toolset not found
        echo Please install from: https://wixtoolset.org/
        exit /b 1
    )
)

where makensis.exe >nul 2>nul
if %errorlevel% neq 0 (
    if exist "C:\Program Files (x86)\NSIS\makensis.exe" (
        set "PATH=%PATH%;C:\Program Files (x86)\NSIS"
    ) else (
        echo [ERROR] NSIS not found
        echo Please install from: https://nsis.sourceforge.io/
        exit /b 1
    )
)

echo [OK] Prerequisites found

REM Check for source binary
if not exist "%SOURCE_DIR%\EasySSH.exe" (
    echo [ERROR] Source binary not found: %SOURCE_DIR%\EasySSH.exe
    echo Please build first:
    echo   cd platforms\windows\easyssh-winui
    echo   cargo build --release
    exit /b 1
)

REM Set up directories
set OUTPUT_DIR=..\..\releases\v%VERSION%\windows
if not exist "%OUTPUT_DIR%" mkdir "%OUTPUT_DIR%"

set RESOURCES_DIR=resources
mkdir "%RESOURCES_DIR%" 2>nul

REM Copy resources
copy "%SOURCE_DIR%\EasySSH.exe" "%RESOURCES_DIR%\" >nul
copy "..\..\core\icons\icon.ico" "%RESOURCES_DIR%\" >nul
copy "..\..\LICENSE" "%RESOURCES_DIR%\LICENSE.txt" >nul

REM Create README
echo EasySSH v%VERSION% > "%RESOURCES_DIR%\README.txt"
echo ================= >> "%RESOURCES_DIR%\README.txt"
echo. >> "%RESOURCES_DIR%\README.txt"
echo Quick Start: >> "%RESOURCES_DIR%\README.txt"
echo 1. Run EasySSH.exe >> "%RESOURCES_DIR%\README.txt"
echo 2. Add your SSH servers via the UI >> "%RESOURCES_DIR%\README.txt"
echo 3. Connect using password or key authentication >> "%RESOURCES_DIR%\README.txt"
echo. >> "%RESOURCES_DIR%\README.txt"
echo System Requirements: >> "%RESOURCES_DIR%\README.txt"
echo - Windows 10/11 64-bit >> "%RESOURCES_DIR%\README.txt"
echo - No additional dependencies required >> "%RESOURCES_DIR%\README.txt"
echo. >> "%RESOURCES_DIR%\README.txt"
echo For support, visit: https://github.com/anixops/easyssh >> "%RESOURCES_DIR%\README.txt"

echo [OK] Resources prepared

REM ============================================
REM Build WiX MSI
REM ============================================
echo.
echo ============================================
echo Building WiX MSI Installer
echo ============================================

set WIX_DIR=..\wix
if not exist "%WIX_DIR%\build" mkdir "%WIX_DIR%\build"

REM Copy resources to WiX dir
copy "%RESOURCES_DIR%\*" "%WIX_DIR%\" >nul 2>nul

echo Compiling WiX source...
candle.exe -arch x64 -dVersion=%VERSION% -dSourceDir="%CD%\%WIX_DIR%" -out "%WIX_DIR%\build\" "%WIX_DIR%\EasySSH.wxs"
if %errorlevel% neq 0 (
    echo [ERROR] WiX compilation failed
    exit /b 1
)

echo Linking MSI...
light.exe -ext WixUIExtension -ext WixUtilExtension -cultures:en-US -out "%WIX_DIR%\build\EasySSH-%VERSION%-x64.msi" "%WIX_DIR%\build\EasySSH.wixobj"
if %errorlevel% neq 0 (
    echo [ERROR] WiX linking failed
    exit /b 1
)

REM Sign MSI if certificate is available
if defined SIGN_CERT (
    echo Signing MSI...
    signtool.exe sign /f "%SIGN_CERT%" /tr "http://timestamp.digicert.com" /td sha256 /fd sha256 /d "EasySSH Installer" "%WIX_DIR%\build\EasySSH-%VERSION%-x64.msi"
)

REM Copy MSI to output
copy "%WIX_DIR%\build\EasySSH-%VERSION%-x64.msi" "%OUTPUT_DIR%\" >nul
echo [OK] MSI installer created

REM ============================================
REM Build NSIS Installer
REM ============================================
echo.
echo ============================================
echo Building NSIS Installer
echo ============================================

set NSIS_DIR=..\nsis
if not exist "%NSIS_DIR%\images" mkdir "%NSIS_DIR%\images"

echo Compiling NSIS script...
makensis.exe /DPRODUCT_VERSION=%VERSION% /DOUTPUT_NAME="EasySSH-%VERSION%-x64.exe" "%NSIS_DIR%\easyssh.nsi"
if %errorlevel% neq 0 (
    echo [ERROR] NSIS compilation failed
    exit /b 1
)

REM Sign EXE if certificate is available
if defined SIGN_CERT (
    echo Signing installer...
    signtool.exe sign /f "%SIGN_CERT%" /tr "http://timestamp.digicert.com" /td sha256 /fd sha256 /d "EasySSH Setup" "%NSIS_DIR%\EasySSH-%VERSION%-x64.exe"
)

REM Copy EXE to output
copy "%NSIS_DIR%\EasySSH-%VERSION%-x64.exe" "%OUTPUT_DIR%\" >nul
echo [OK] NSIS installer created

REM ============================================
REM Create Portable Package
REM ============================================
echo.
echo ============================================
echo Creating Portable Package
echo ============================================

set PORTABLE_DIR=EasySSH-%VERSION%-portable
mkdir "%PORTABLE_DIR%"

copy "%SOURCE_DIR%\EasySSH.exe" "%PORTABLE_DIR%\" >nul
copy "..\..\core\icons\icon.ico" "%PORTABLE_DIR%\" >nul
copy "..\..\LICENSE" "%PORTABLE_DIR%\LICENSE.txt" >nul

echo EasySSH v%VERSION% Portable > "%PORTABLE_DIR%\README.txt"
echo =========================== >> "%PORTABLE_DIR%\README.txt"
echo. >> "%PORTABLE_DIR%\README.txt"
echo This is a portable version of EasySSH. >> "%PORTABLE_DIR%\README.txt"
echo No installation required. >> "%PORTABLE_DIR%\README.txt"
echo. >> "%PORTABLE_DIR%\README.txt"
echo Quick Start: >> "%PORTABLE_DIR%\README.txt"
echo 1. Run EasySSH.exe directly >> "%PORTABLE_DIR%\README.txt"
echo 2. Your data will be stored in: >> "%PORTABLE_DIR%\README.txt"
echo    %%LOCALAPPDATA%%\AnixOps\EasySSH >> "%PORTABLE_DIR%\README.txt"

echo @echo off > "%PORTABLE_DIR%\EasySSH.bat"
echo start "" "%%~dp0EasySSH.exe" >> "%PORTABLE_DIR%\EasySSH.bat"

REM Create ZIP using PowerShell
powershell -Command "Compress-Archive -Path '%PORTABLE_DIR%' -DestinationPath '%OUTPUT_DIR%\EasySSH-%VERSION%-windows-x64-portable.zip' -Force"

rmdir /s /q "%PORTABLE_DIR%"
echo [OK] Portable package created

REM ============================================
REM Generate Checksums
REM ============================================
echo.
echo ============================================
echo Generating Checksums
echo ============================================

set CHECKSUM_FILE=%OUTPUT_DIR%\SHA256SUMS.txt
echo EasySSH v%VERSION% Windows Installer Checksums > "%CHECKSUM_FILE%"
echo =============================================== >> "%CHECKSUM_FILE%"
echo. >> "%CHECKSUM_FILE%"
echo Generated: %date% %time% >> "%CHECKSUM_FILE%"
echo. >> "%CHECKSUM_FILE%"

for %%f in ("%OUTPUT_DIR%\*.msi" "%OUTPUT_DIR%\*.exe" "%OUTPUT_DIR%\*.zip") do (
    powershell -Command "$hash = (Get-FileHash '%%f' -Algorithm SHA256).Hash; Write-Host \"$hash  %%~nxf\"" >> "%CHECKSUM_FILE%"
)

echo [OK] Checksums generated

REM ============================================
REM Summary
REM ============================================
echo.
echo ============================================
echo Build Complete!
echo ============================================
echo.
echo Output files:
dir /b "%OUTPUT_DIR%"
echo.
echo Next steps:
echo   1. Test installers on clean Windows VMs
echo   2. Verify digital signatures (if configured)
echo   3. Upload to GitHub releases
echo.
pause
