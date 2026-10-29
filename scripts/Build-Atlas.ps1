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
try {
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
        if ($CheckOnly) {
            & $cargoPath check --locked -p atlas-client --bin atlas-client --bin atlas-ctl --features mods
        } else {
            & $cargoPath build --locked --profile $Profile -p atlas-client --bin atlas-client --bin atlas-ctl --features mods
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
            Write-Host "Built Atlas: $runtimePath\atlas-client.exe" -ForegroundColor Green
            Write-Host 'Restart Atlas Launcher to use the new client. Close the game before building again.'
        }
    } finally { Pop-Location }
} finally {
    $env:CARGO_HOME = $oldCargoHome
    $env:RUSTUP_HOME = $oldRustupHome
    $env:PATH = $oldBuildPath
}
