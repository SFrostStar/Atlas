@echo off
if exist "%~dp0..\dist\Atlas Launcher-win32-x64\Atlas Launcher.exe" (
  start "" "%~dp0..\dist\Atlas Launcher-win32-x64\Atlas Launcher.exe"
) else (
  cd /d "%~dp0.."
  call npm.cmd start
)
