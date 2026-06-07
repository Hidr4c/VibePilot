@echo off
title VibePilot Rust Tool
cd /d "%~dp0\vibe_pilot_rust"
cls

:menu
cls
echo ===================================================
echo             VibePilot Rust Build Tool
echo ===================================================
echo.
echo Please select an option:
echo.
echo [1] Run Automated Tests (cargo test)
echo [2] Build and Run in Debug Mode (cargo run)
echo [3] Build Release Version (cargo build --release)
echo [4] Run Release Version (target\release\vibepilot.exe)
echo [5] Exit
echo.
set /p opt="Enter choice (1-5): "

if "%opt%"=="1" (
    echo.
    echo Running tests...
    cargo test
    echo.
    pause
    goto menu
)
if "%opt%"=="2" (
    echo.
    echo Launching in Debug mode...
    cargo run
    goto menu
)
if "%opt%"=="3" (
    echo.
    echo Building release version...
    cargo build --release
    if %ERRORLEVEL% equ 0 (
        if not exist "target\release\dist" mkdir "target\release\dist"
        copy /Y "target\release\vibepilot.exe" "target\release\dist\vibepilot.exe" >nul
        echo.
        echo Build successful! Clean release executable is at: target\release\dist\vibepilot.exe
    ) else (
        echo.
        echo Build failed. Make sure the application is closed.
    )
    echo.
    pause
    goto menu
)
if "%opt%"=="4" (
    if exist "target\release\dist\vibepilot.exe" (
        echo.
        echo Starting release version...
        start "" "target\release\dist\vibepilot.exe"
    ) else (
        echo.
        echo Release binary not found in dist. Please build it first (option 3).
    )
    echo.
    pause
    goto menu
)
if "%opt%"=="5" (
    goto :eof
)

echo.
echo Invalid choice.
pause
goto menu
