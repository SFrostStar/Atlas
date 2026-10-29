param(
    [string]$LauncherDirectory,
    [string]$OutputDirectory,
    [switch]$Zip
)
$ErrorActionPreference = 'Stop'
$atlasRoot = Split-Path -Parent $PSScriptRoot
if (-not $LauncherDirectory) {
    $LauncherDirectory = Join-Path $atlasRoot 'work\package-fixture\dist\Atlas Launcher-win32-x64'
}
$atlasVersion = (Get-Content -LiteralPath (Join-Path $atlasRoot 'package.json') -Raw | ConvertFrom-Json).version -replace '\.0$', ''
if (-not $OutputDirectory) { $OutputDirectory = Join-Path $atlasRoot "dist\Atlas-$atlasVersion-Portable" }
$atlasDestination = [IO.Path]::GetFullPath($OutputDirectory)
$atlasLauncher = [IO.Path]::GetFullPath($LauncherDirectory)
$atlasPayload = Join-Path $atlasRoot 'bundle\game'
if (Test-Path -LiteralPath $atlasDestination) { throw 'Output already exists; choose a new OutputDirectory.' }
foreach ($atlasFile in @('Atlas Launcher.exe','resources\app.asar')) {
    if (-not (Test-Path -LiteralPath (Join-Path $atlasLauncher $atlasFile) -PathType Leaf)) { throw "Missing launcher file: $atlasFile" }
}
foreach ($atlasFile in @('atlas-client.exe','atlas-ctl.exe','atlas-prepare.exe','libc++.dll','libunwind.dll','libwinpthread-1.dll','game-root\research-lock.json','game-root\exporters\manifest.json')) {
    if (-not (Test-Path -LiteralPath (Join-Path $atlasPayload $atlasFile) -PathType Leaf)) { throw "Missing setup payload: $atlasFile" }
}
if (Get-ChildItem -LiteralPath (Join-Path $atlasPayload 'game-root') -Recurse -File -Filter '*.jar') { throw 'Minecraft jars must not be included in the portable payload.' }
foreach ($atlasForbidden in @('data','runtime','engine','tools','bundle')) {
    if (Test-Path -LiteralPath (Join-Path $atlasLauncher $atlasForbidden)) { throw "Unexpected folder in staged launcher: $atlasForbidden" }
}
New-Item -ItemType Directory -Path $atlasDestination -Force | Out-Null
Copy-Item -Path (Join-Path $atlasLauncher '*') -Destination $atlasDestination -Recurse
$atlasGameDestination = Join-Path $atlasDestination 'bundle\game'
New-Item -ItemType Directory -Path $atlasGameDestination -Force | Out-Null
foreach ($atlasFile in @('atlas-client.exe','atlas-ctl.exe','atlas-prepare.exe','libc++.dll','libunwind.dll','libwinpthread-1.dll','game-root')) {
    Copy-Item -LiteralPath (Join-Path $atlasPayload $atlasFile) -Destination $atlasGameDestination -Recurse
}
Get-ChildItem -LiteralPath (Join-Path $atlasRoot 'runtime') -File -Filter 'LICENSE*.txt' |
    ForEach-Object { Copy-Item -LiteralPath $_.FullName -Destination $atlasGameDestination }
Get-ChildItem -LiteralPath $atlasPayload -File -Filter 'LICENSE*.txt' |
    ForEach-Object { Copy-Item -LiteralPath $_.FullName -Destination $atlasGameDestination -Force }
@{ product = 'Atlas'; version = $atlasVersion; portable = $true } | ConvertTo-Json | Set-Content -LiteralPath (Join-Path $atlasDestination 'atlas-portable.json') -Encoding UTF8
@'
Atlas Launcher

RU: Запустите Atlas Launcher.exe. Создайте локальный профиль и нажмите «Играть».
При первом запуске подтвердите скачивание файлов игры. Нужен интернет.
Java и игровые ресурсы установятся автоматически. Microsoft-вход настраивается отдельно.
Миры, профили и настройки хранятся в папке data рядом с лаунчером.
Сохраняйте весь комплект файлов вместе; для обновления сохраняйте свою папку data.

EN: Open Atlas Launcher.exe, create a local profile, and press Play.
Approve the game-file download on first launch. Internet access is required.
Java and game resources install automatically. Microsoft sign-in is configured separately.
Worlds, profiles and settings are stored in data next to the launcher.
Keep the complete folder together, and preserve your data folder when updating.
'@ | Set-Content -LiteralPath (Join-Path $atlasDestination 'START-HERE.txt') -Encoding UTF8
if ($Zip) {
    Add-Type -AssemblyName System.IO.Compression.FileSystem
    $atlasArchive = $atlasDestination + '.zip'
    if (Test-Path -LiteralPath $atlasArchive) { throw 'Zip already exists; choose a new output path.' }
    [IO.Compression.ZipFile]::CreateFromDirectory($atlasDestination, $atlasArchive, [IO.Compression.CompressionLevel]::Fastest, $true)
    Write-Output $atlasArchive
}
Write-Output $atlasDestination
