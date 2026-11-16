param(
    [ValidateSet('fast', 'release')]
    [string]$Profile = 'fast',
    [switch]$CheckOnly,
    [string]$OutputDirectory
)

$ErrorActionPreference = 'Stop'
$atlasProject = Split-Path -Parent $PSScriptRoot
$sourcePath = Join-Path $atlasProject 'engine\atlas'
$portableCargo = Join-Path $atlasProject 'tools\cargo\bin\cargo.exe'
$oldCargoHome = $env:CARGO_HOME
$oldRustupHome = $env:RUSTUP_HOME
$oldBuildPath = $env:PATH
$oldAtlasVersion = $env:ATLAS_VERSION
$oldAtlasNightly = $env:ATLAS_NIGHTLY
$targetPath = Join-Path $sourcePath 'target'

function Limit-AtlasBuildCache {
    $profilePath = Join-Path $targetPath $Profile
    if (-not (Test-Path -LiteralPath $profilePath -PathType Container)) { return }

    foreach ($directory in @($profilePath, $targetPath, $sourcePath, (Split-Path $sourcePath), $atlasProject)) {
        if ((Get-Item -LiteralPath $directory -Force).Attributes -band [IO.FileAttributes]::ReparsePoint) {
            throw "Refusing to clean a build cache reached through a link: $directory"
        }
    }
    $profileDirectory = Get-Item -LiteralPath $profilePath -Force
    if ($profileDirectory.Parent.FullName -ne [IO.Path]::GetFullPath($targetPath)) {
        throw "Unexpected build cache location: $profilePath"
    }
    $bytes = 0L
    $pending = [Collections.Generic.Stack[IO.DirectoryInfo]]::new()
    $pending.Push($profileDirectory)
    while ($pending.Count -gt 0) {
        foreach ($entry in $pending.Pop().EnumerateFileSystemInfos()) {
            if ($entry.Attributes -band [IO.FileAttributes]::ReparsePoint) {
                throw "Refusing to clean a build cache containing a link: $($entry.FullName)"
            }
            if ($entry -is [IO.DirectoryInfo]) { $pending.Push($entry) }
            else { $bytes += $entry.Length }
        }
    }
    if ($bytes -gt 12GB) {
        Write-Host "Atlas: clearing the $Profile Rust build cache (over 12 GiB)."
        & $cargoPath clean --target-dir $targetPath --profile $Profile
        if ($LASTEXITCODE -ne 0) { throw 'Could not clear the Atlas build cache.' }
    }
}

try {
    & node (Join-Path $PSScriptRoot 'atlas-server-policy.cjs')
    if ($LASTEXITCODE -ne 0) { throw 'Atlas server policy needs review before building this upstream version.' }
    $launcherManifest = Get-Content -LiteralPath (Join-Path $atlasProject 'package.json') -Raw | ConvertFrom-Json
    $env:ATLAS_VERSION = $launcherManifest.version
    $env:ATLAS_NIGHTLY = ''
    & (Join-Path $PSScriptRoot 'Generate-Icons.ps1')
    # Upgrade the manifest name in Atlas's own previously prepared resource copy.
    $prepared = Join-Path $atlasProject 'runtime\.local\atlas-client\26.3'
    $oldManifest = Join-Path $prepared 'horizon-resources.json'
    $atlasManifest = Join-Path $prepared 'atlas-resources.json'
    if ((Test-Path -LiteralPath $oldManifest) -and -not (Test-Path -LiteralPath $atlasManifest)) {
        Copy-Item -LiteralPath $oldManifest -Destination $atlasManifest
    }
    if (Test-Path -LiteralPath $portableCargo) {
        $env:CARGO_HOME = Join-Path $atlasProject 'tools\cargo'
        $env:RUSTUP_HOME = Join-Path $atlasProject 'tools\rustup'
        $llvmDirectory = Get-ChildItem -LiteralPath (Join-Path $atlasProject 'tools') -Directory -Filter 'llvm-mingw-*-ucrt-x86_64' | Select-Object -First 1
        if (-not $llvmDirectory) { throw 'The local LLVM-MinGW toolchain is missing from tools/.' }
        $env:PATH = (Join-Path $llvmDirectory.FullName 'bin') + ';' + (Split-Path -Parent $portableCargo) + ';' + $env:PATH
        $cargoPath = $portableCargo
    } else {
        $cargoCommand = Get-Command cargo -ErrorAction SilentlyContinue
        if (-not $cargoCommand) { throw 'Rust/Cargo is missing. Restore the local tools/ folder or install Rust and C++ Build Tools.' }
        $cargoPath = $cargoCommand.Source
    }
    if (-not (Test-Path -LiteralPath (Join-Path $sourcePath 'Cargo.toml'))) { throw "Atlas source was not found: $sourcePath" }
    Push-Location -LiteralPath $sourcePath
    try {
        Limit-AtlasBuildCache
        if ($CheckOnly) {
            & $cargoPath check --locked --target-dir $targetPath --profile $Profile -p atlas-client -p atlas-client-lib --bin atlas-client --bin atlas-ctl
        } else {
            & $cargoPath build --locked --target-dir $targetPath --profile $Profile -p atlas-client -p atlas-client-lib --bin atlas-client --bin atlas-ctl
        }
        if ($LASTEXITCODE -ne 0) { throw 'Atlas build failed. Read the compiler output above.' }
        if (-not $CheckOnly) {
            $runtimePath = if ($OutputDirectory) { [System.IO.Path]::GetFullPath($OutputDirectory) } else { Join-Path $atlasProject 'runtime' }
            New-Item -ItemType Directory -Path $runtimePath -Force | Out-Null
            foreach ($binary in @('client', 'ctl')) {
                $builtPath = Join-Path $sourcePath "target\$Profile\atlas-$binary.exe"
                Copy-Item -LiteralPath $builtPath -Destination (Join-Path $runtimePath "atlas-$binary.exe") -Force
            }
            if ($llvmDirectory) {
                foreach ($dll in @('libc++.dll', 'libunwind.dll', 'libwinpthread-1.dll')) {
                    $dllPath = Join-Path $llvmDirectory.FullName "bin\$dll"
                    if (Test-Path -LiteralPath $dllPath) { Copy-Item -LiteralPath $dllPath -Destination $runtimePath -Force }
                }
            }
            [IO.File]::WriteAllText(
                (Join-Path $runtimePath 'atlas-runtime.json'),
                (@{ version = $env:ATLAS_VERSION } | ConvertTo-Json)
            )
            Write-Host "Built Atlas: $runtimePath\atlas-client.exe" -ForegroundColor Green
            Write-Host 'Restart Atlas Launcher to use the new client. Close the game before building again.'
        }
    } finally { Pop-Location }
} finally {
    $env:CARGO_HOME = $oldCargoHome
    $env:RUSTUP_HOME = $oldRustupHome
    $env:PATH = $oldBuildPath
    $env:ATLAS_VERSION = $oldAtlasVersion
    $env:ATLAS_NIGHTLY = $oldAtlasNightly
}
