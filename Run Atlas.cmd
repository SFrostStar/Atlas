@echo off
if exist "%~dp0dist\Atlas Launcher-win32-x64\Atlas Launcher.exe" (
  start "" "%~dp0dist\Atlas Launcher-win32-x64\Atlas Launcher.exe"
) else (
  cd /d "%~dp0"
  call npm.cmd start
)
