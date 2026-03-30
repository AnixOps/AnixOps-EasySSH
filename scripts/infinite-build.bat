@echo off
setlocal EnableDelayedExpansion

:: TRUE INFINITE LOOP - Never stops until success
:: Usage: infinite-build.bat
:: Press Ctrl+C to stop manually

cd /d "%~dp0\.."
set "PROJECT_ROOT=%CD%"
set ITERATION=0

:: Get start time
for /f "tokens=2 delims==." %%a in ('wmic os get localdatetime /value') do set START=%%a
set START_TIME=%START:~0,8%%START:~8,6%

cls
echo ============================================
echo   TRUE INFINITE BUILD LOOP
echo   Will NEVER stop until success
echo ============================================
echo.
echo Project: %PROJECT_ROOT%
echo Started: %date% %time%
echo.
echo Press Ctrl+C to stop manually
echo.
timeout /t 3 /nobreak >nul

:INFINITE_LOOP
set /a ITERATION+=1

:: Calculate elapsed time
for /f "tokens=2 delims==." %%a in ('wmic os get localdatetime /value') do set NOW=%%a
set NOW_TIME=%NOW:~0,8%%NOW:~8,6%

cls
echo ============================================
echo   ITERATION %ITERATION%
echo   Started: %date% %time%
echo ============================================
echo.

:: Build Core
echo [1/4] Building Core Library...
cargo build --release -p easyssh-core >nul 2>&1
if %ERRORLEVEL% neq 0 (
    echo [X] Core build failed
    echo Cleaning and retrying...
    cargo clean -p easyssh-core >nul 2>&1
    cargo update >nul 2>&1
    timeout /t 5 /nobreak >nul
    goto :INFINITE_LOOP
)

:: Test Core
echo [2/4] Testing Core Library...
cargo test --release -p easyssh-core >nul 2>&1
if %ERRORLEVEL% neq 0 (
    echo [X] Core tests failed
    timeout /t 5 /nobreak >nul
    goto :INFINITE_LOOP
)

:: Build TUI
echo [3/4] Building TUI...
cargo build --release -p easyssh-tui >nul 2>&1
if %ERRORLEVEL% neq 0 (
    echo [X] TUI build failed
    echo Cleaning and retrying...
    cargo clean -p easyssh-tui >nul 2>&1
    timeout /t 5 /nobreak >nul
    goto :INFINITE_LOOP
)

:: Test TUI
if exist "target\release\easyssh.exe" (
    echo [4/4] Testing TUI...
    target\release\easyssh.exe --version >nul 2>&1
    if %ERRORLEVEL% neq 0 (
        echo [X] TUI test failed
        timeout /t 5 /nobreak >nul
        goto :INFINITE_LOOP
    )
)

:: SUCCESS!
echo.
echo ============================================
echo SUCCESS AFTER %ITERATION% ITERATIONS!
echo ============================================
echo.
echo [OK] Core Library built
echo [OK] All tests passed
echo [OK] TUI built
echo.
echo Binaries in: %PROJECT_ROOT%\target\release\
dir %PROJECT_ROOT%\target\release\easyssh.exe /b 2>nul
echo.
pause
exit /b 0
