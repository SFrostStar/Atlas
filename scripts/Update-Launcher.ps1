$ErrorActionPreference = 'Stop'

$projectRoot = Split-Path -Parent $PSScriptRoot
Push-Location -LiteralPath $projectRoot
try {
    Write-Host "Checking for running Atlas Launcher processes..." -ForegroundColor Cyan
    $runningProcesses = Get-Process -Name "Atlas Launcher" -ErrorAction SilentlyContinue
    if ($runningProcesses) {
        Write-Host "Closing active Atlas Launcher before repackaging..." -ForegroundColor Yellow
        $runningProcesses | Stop-Process -Force -ErrorAction SilentlyContinue
        Start-Sleep -Milliseconds 800
    }

    Write-Host "Packaging launcher (npm run package)..." -ForegroundColor Cyan
    & npm.cmd run package
    if ($LASTEXITCODE -ne 0) {
        throw "npm run package failed with code $LASTEXITCODE."
    }

    $distExe = Join-Path $projectRoot "dist\Atlas Launcher-win32-x64\Atlas Launcher.exe"
    if (-not (Test-Path -LiteralPath $distExe)) {
        throw "Packaged binary not found: $distExe"
    }

    Write-Host "Updating shortcut Atlas Launcher.lnk..." -ForegroundColor Cyan
    $shortcutPath = Join-Path $projectRoot "Atlas Launcher.lnk"
    $iconPath = Join-Path $projectRoot "assets\atlas-launcher.ico"

    $wsh = New-Object -ComObject WScript.Shell
    $shortcut = $wsh.CreateShortcut($shortcutPath)
    $shortcut.TargetPath = $distExe
    $shortcut.WorkingDirectory = $projectRoot
    if (Test-Path -LiteralPath $iconPath) {
        $shortcut.IconLocation = "$iconPath,0"
    }
    $shortcut.Description = "Atlas Launcher"
    $shortcut.Save()

    Write-Host "`nГотово! Лаунчер успешно обновлен. Ярлык 'Atlas Launcher.lnk' актуален.`n" -ForegroundColor Green
}
finally {
    Pop-Location
}