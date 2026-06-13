@echo off
setlocal
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
echo [2] Build and Run Worker in Debug Mode (cargo run --bin vibepilot)
echo [3] Build and Run Master in Debug Mode (cargo run --bin vibepilot_master)
echo [4] Build Release Versions (cargo build --release)
echo [5] Run Release Worker (target\release\dist\vibepilot.exe)
echo [6] Run Release Master (target\release\dist\vibepilot_master.exe)
echo [7] Deploy Draft Release (deploy_draft.ps1)
echo [8] Create Release (create_release.ps1)
echo [9] Exit
echo.
set "opt="
set /p opt="Enter choice (1-9): "

if "%opt%"=="1" goto opt_test
if "%opt%"=="2" goto opt_debug_worker
if "%opt%"=="3" goto opt_debug_master
if "%opt%"=="4" goto opt_build_release
if "%opt%"=="5" goto opt_run_worker
if "%opt%"=="6" goto opt_run_master
if "%opt%"=="7" goto opt_deploy_draft
if "%opt%"=="8" goto opt_create_release
if "%opt%"=="9" exit /b 0
if "%opt%"=="exit" exit /b 0

echo.
echo Invalid choice.
pause
goto menu

:: === Option 1: Run Tests =====================================================

:opt_test
echo.
echo Running tests...
cargo test
echo.
pause
goto menu

:: === Option 2: Debug Worker ==================================================

:opt_debug_worker
echo.
echo Launching Worker in Debug mode...
cargo run --bin vibepilot
goto menu

:: === Option 3: Debug Master ==================================================

:opt_debug_master
echo.
echo Launching Master in Debug mode...
cargo run --bin vibepilot_master
goto menu

:: === Option 4: Build Release =================================================

:opt_build_release
echo.
echo Building release versions...
cargo build --release
if %ERRORLEVEL% neq 0 goto build_failed
if not exist "target\release\dist" mkdir "target\release\dist"
copy /Y "target\release\vibepilot.exe" "target\release\dist\vibepilot.exe" >nul
copy /Y "target\release\vibepilot_master.exe" "target\release\dist\vibepilot_master.exe" >nul
echo.
echo Build successful! Clean release executables are at:
echo   - target\release\dist\vibepilot.exe (Worker)
echo   - target\release\dist\vibepilot_master.exe (Master)
echo.
pause
goto menu

:build_failed
echo.
echo Build failed. Make sure the applications are closed.
echo.
pause
goto menu

:: === Option 5: Run Release Worker ============================================

:opt_run_worker
if not exist "target\release\dist\vibepilot.exe" goto no_worker_binary
echo.
echo Starting release worker...
start "" "target\release\dist\vibepilot.exe"
echo.
pause
goto menu

:no_worker_binary
echo.
echo Release worker binary not found in dist. Please build first [option 4].
echo.
pause
goto menu

:: === Option 6: Run Release Master ============================================

:opt_run_master
if not exist "target\release\dist\vibepilot_master.exe" goto no_master_binary
echo.
echo Starting release master...
start "" "target\release\dist\vibepilot_master.exe"
echo.
pause
goto menu

:no_master_binary
echo.
echo Release master binary not found in dist. Please build first [option 4].
echo.
pause
goto menu

:: === Option 7: Deploy Draft ==================================================

:opt_deploy_draft
echo.
set "draft_ver="
set /p draft_ver="Enter draft version (or press Enter for default 'draft'): "
echo.
echo Deploying draft release...
cd /d "%~dp0"
if "%draft_ver%"=="" goto draft_no_version
powershell -NoProfile -ExecutionPolicy Bypass -File "%~dp0scripts\deploy_draft.ps1" -Version "%draft_ver%"
goto draft_done

:draft_no_version
powershell -NoProfile -ExecutionPolicy Bypass -File "%~dp0scripts\deploy_draft.ps1"

:draft_done
cd /d "%~dp0\vibe_pilot_rust"
echo.
pause
goto menu

:: === Option 8: Create Release ================================================

:opt_create_release
echo.
set "latest_tag="
for /f "usebackq tokens=*" %%t in (`git tag -l "v*" --sort=-v:refname`) do if not defined latest_tag set "latest_tag=%%t"
if not defined latest_tag set "latest_tag=none"
echo Current latest version: %latest_tag%
echo.

:release_prompt
set "rel_ver="
set /p rel_ver="Enter release version (e.g. 1.0.2): "
if "%rel_ver%"=="" goto release_no_version

:: Validate format: strip optional leading 'v', then check X.Y.Z with numbers only
set "ver_clean=%rel_ver%"
if "%ver_clean:~0,1%"=="v" set "ver_clean=%ver_clean:~1%"
if "%ver_clean:~0,1%"=="V" set "ver_clean=%ver_clean:~1%"
echo %ver_clean%| findstr /R "^[0-9][0-9]*[.][0-9][0-9]*[.][0-9][0-9]*$" >nul 2>nul
if errorlevel 1 goto release_invalid
goto release_valid

:release_invalid
echo.
echo [ERROR] Invalid version format: "%rel_ver%". Expected: 1.0.2 (numbers separated by dots)
echo.
goto release_prompt

:release_valid
echo.
echo Creating release...
cd /d "%~dp0"
powershell -NoProfile -ExecutionPolicy Bypass -File "%~dp0scripts\create_release.ps1" -Version "%rel_ver%"
goto release_done

:release_no_version
cd /d "%~dp0"
powershell -NoProfile -ExecutionPolicy Bypass -File "%~dp0scripts\create_release.ps1"

:release_done
cd /d "%~dp0\vibe_pilot_rust"
echo.
pause
goto menu

