param(
    [string]$RuntimeDirectory,
    [string]$MinecraftDirectory,
    [string]$ServerJar,
    [string]$CompiledExporters
)
$ErrorActionPreference = 'Stop'
$atlasProject = Split-Path -Parent $PSScriptRoot
if (-not $RuntimeDirectory) { $RuntimeDirectory = Join-Path $atlasProject 'runtime' }
$atlasSource = Join-Path $atlasProject 'engine\atlas'
$atlasBundle = Join-Path $atlasProject 'bundle\game'
$atlasGameRoot = Join-Path $atlasBundle 'game-root'
$atlasScratch = Join-Path $atlasProject ('work\setup-build-' + [guid]::NewGuid())
$atlasOldPath = $env:PATH
$atlasOldCargo = $env:CARGO_HOME
$atlasOldRustup = $env:RUSTUP_HOME
try {
    $env:CARGO_HOME = Join-Path $atlasProject 'tools\cargo'
    $env:RUSTUP_HOME = Join-Path $atlasProject 'tools\rustup'
    $atlasLlvm = Get-ChildItem (Join-Path $atlasProject 'tools') -Directory -Filter 'llvm-mingw-*-ucrt-x86_64' | Select-Object -First 1
    $env:PATH = (Join-Path $atlasLlvm.FullName 'bin') + ';' + (Join-Path $env:CARGO_HOME 'bin') + ';' + $RuntimeDirectory + ';' + $env:PATH
    Push-Location $atlasSource
    try { & cargo build --locked --profile fast -p atlas-prepare; if ($LASTEXITCODE -ne 0) {throw 'Cannot build atlas-prepare'} } finally {Pop-Location}
    New-Item -ItemType Directory -Path $atlasBundle,$atlasGameRoot,$atlasScratch -Force | Out-Null
    foreach ($atlasPart in @('research-lock.json','mods\atlas\src\test\java\dev\atlaslab\parity','crates\atlas-prepare\src\exporters.rs')) {
        $atlasTo = Join-Path $atlasGameRoot $atlasPart
        New-Item -ItemType Directory -Path (Split-Path -Parent $atlasTo) -Force | Out-Null
        if (Test-Path -LiteralPath (Join-Path $atlasSource $atlasPart) -PathType Container) {
            New-Item -ItemType Directory -Path $atlasTo -Force | Out-Null
            Copy-Item -Path ((Join-Path $atlasSource $atlasPart) + '\*') -Destination $atlasTo -Recurse -Force
        } else { Copy-Item -LiteralPath (Join-Path $atlasSource $atlasPart) -Destination $atlasTo -Force }
    }
    $atlasPreparer = Join-Path $atlasSource 'target\fast\atlas-prepare.exe'
    if (-not $CompiledExporters) {
        # Only developer-supplied local Minecraft inputs are used while building.
        # End users receive the original exporter classes, not Minecraft files.
        foreach ($atlasPart in @('research-lock.json','mods','crates')) {
            Copy-Item -LiteralPath (Join-Path $atlasGameRoot $atlasPart) -Destination (Join-Path $atlasScratch $atlasPart) -Recurse -Force
        }
        $atlasArgs = @('compile-exporters','--prepare','--root',$atlasScratch)
        if ($MinecraftDirectory) { $atlasArgs += @('--minecraft-dir',$MinecraftDirectory) }
        if ($ServerJar) { $atlasArgs += @('--server-jar',$ServerJar) }
        $atlasLines = & $atlasPreparer @atlasArgs
        if ($LASTEXITCODE -ne 0) {throw 'Cannot compile original exporters; supply your pinned Minecraft files and JDK 25'}
        $CompiledExporters = $atlasLines | Select-Object -Last 1
    }
    if (-not (Test-Path -LiteralPath (Join-Path $CompiledExporters 'manifest.json'))) {throw 'Exporter bundle manifest missing'}
    $atlasExporterTarget = Join-Path $atlasGameRoot 'exporters'
    New-Item -ItemType Directory -Path $atlasExporterTarget -Force | Out-Null
    Copy-Item -Path ($CompiledExporters + '\*') -Destination $atlasExporterTarget -Recurse -Force
    # Exporter compiler logs are local diagnostics, not part of the distributable.
    Get-ChildItem -LiteralPath $atlasExporterTarget -Recurse -File -Filter '*.log' |
        Remove-Item -Force
    foreach ($atlasBinary in @('atlas-client.exe','atlas-ctl.exe','libc++.dll','libunwind.dll','libwinpthread-1.dll')) {
        Copy-Item -LiteralPath (Join-Path $RuntimeDirectory $atlasBinary) -Destination (Join-Path $atlasBundle $atlasBinary) -Force
    }
    Copy-Item -LiteralPath $atlasPreparer -Destination (Join-Path $atlasBundle 'atlas-prepare.exe') -Force
    Get-ChildItem -LiteralPath $RuntimeDirectory -Filter 'LICENSE*.txt' -File | Copy-Item -Destination $atlasBundle -Force
    $atlasNotices = @{
        'crates\atlas-theme\fonts\OFL-Geist.txt' = 'LICENSE-Geist.txt'
        'crates\atlas-theme\fonts\OFL-PixelifySans.txt' = 'LICENSE-PixelifySans.txt'
        'crates\atlas-theme\icons\pixel\LICENSE-pixelarticons.txt' = 'LICENSE-Pixelarticons.txt'
        'third_party\wgpu-hal\LICENSE.MIT' = 'LICENSE-wgpu-hal-MIT.txt'
        'third_party\wgpu-hal\LICENSE.APACHE' = 'LICENSE-wgpu-hal-Apache.txt'
    }
    foreach ($atlasNotice in $atlasNotices.GetEnumerator()) {
        Copy-Item -LiteralPath (Join-Path $atlasSource $atlasNotice.Key) -Destination (Join-Path $atlasBundle $atlasNotice.Value) -Force
    }
    Write-Host 'Atlas first-run payload ready in bundle/game. No Minecraft jars or prepared resources are included.'
} finally {
    $env:PATH = $atlasOldPath
    $env:CARGO_HOME = $atlasOldCargo
    $env:RUSTUP_HOME = $atlasOldRustup
}
