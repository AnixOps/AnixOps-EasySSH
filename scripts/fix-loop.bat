@echo off
setlocal EnableDelayedExpansion

echo ============================================
echo EASYSSH CONTINUOUS FIX LOOP (Windows)
echo ============================================
echo.
echo This script will build until success
echo Press Ctrl+C to stop
echo.

set "ITERATION=0"
set "MAX_ITERATIONS=%MAX_ITERATIONS%"
if "%MAX_ITERATIONS%"=="" set "MAX_ITERATIONS=100"

cd /d "%~dp0\.."
set "PROJECT_ROOT=%CD%"

echo Project: %PROJECT_ROOT%
echo Max Iterations: %MAX_ITERATIONS%
echo.
timeout /t 3 /nobreak >nul

:MAIN_LOOP
set /a ITERATION+=1

echo.
echo ============================================
echo ITERATION %ITERATION%
echo ============================================

:: Build Core
echo [1/5] Building Core Library...
cargo build --release -p easyssh-core >"%TEMP%\easyssh_build_%ITERATION%.log" 2>&1
if %ERRORLEVEL% neq 0 (
    echo [ERROR] Core build failed
    echo Attempting fix...
    cargo update >>"%TEMP%\easyssh_build_%ITERATION%.log" 2>&1
    goto :BUILD_FAILED
)

:: Test Core
echo [2/5] Testing Core Library...
cargo test --release -p easyssh-core >>"%TEMP%\easyssh_build_%ITERATION%.log" 2>&1
if %ERRORLEVEL% neq 0 (
    echo [ERROR] Core tests failed
    goto :BUILD_FAILED
)

:: Clippy
echo [3/5] Running Clippy...
cargo clippy -p easyssh-core -- -D warnings >>"%TEMP%\easyssh_build_%ITERATION%.log" 2>&1
if %ERRORLEVEL% neq 0 (
    echo [WARNING] Clippy warnings found, attempting auto-fix...
    rustup component add clippy 2>nul
    cargo clippy --fix --allow-dirty --allow-staged -p easyssh-core 2>&1 | findstr /C:"Fixing" && (
        echo Fix applied, retrying...
        goto :MAIN_LOOP
    )
    goto :BUILD_FAILED
)

:: Build TUI
echo [4/5] Building TUI...
cargo build --release -p easyssh-tui >>"%TEMP%\easyssh_build_%ITERATION%.log" 2>&1
if %ERRORLEVEL% neq 0 (
    echo [ERROR] TUI build failed
    goto :BUILD_FAILED
)

:: Test TUI
echo [5/5] Testing TUI...
if exist "target\release\easyssh.exe" (
    target\release\easyssh.exe --version >nul 2>&1
    if %ERRORLEVEL% neq 0 (
        echo [ERROR] TUI test failed
        goto :BUILD_FAILED
    )
) else (
    echo [ERROR] TUI binary not found
    goto :BUILD_FAILED
)

:: Success!
echo.
echo ============================================
echo SUCCESS AFTER %ITERATION% ITERATION(S)
echo ============================================
echo.
echo All builds passing:
echo   [OK] Core Library
echo   [OK] TUI CLI
echo.
echo Binaries available in:
echo   %PROJECT_ROOT%\target\release\
echo.
pause
exit /b 0

:BUILD_FAILED
echo.
echo Build failed. Retrying in 3 seconds...
echo (Press Ctrl+C to stop)
timeout /t 3 /nobreak >nul

:: Check max iterations
if %MAX_ITERATIONS% gtr 0 (
    if %ITERATION% geq %MAX_ITERATIONS% (
        echo.
        echo ============================================
        echo Reached maximum iterations (%MAX_ITERATIONS%)
        echo Stopping.
        echo ============================================
        exit /b 1
    )
)

goto :MAIN_LOOP
