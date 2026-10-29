@echo off
powershell.exe -NoProfile -File "%~dp0Build-Atlas.ps1" %*
pause
