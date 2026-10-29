param()
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Drawing
$atlasProject = Split-Path -Parent $PSScriptRoot
$atlasAssets = Join-Path $atlasProject 'assets'
$atlasLocalIcons = Join-Path $atlasProject 'engine\atlas\.local\atlas-icons'
New-Item -ItemType Directory -Path $atlasAssets,$atlasLocalIcons -Force | Out-Null

function Write-AtlasIco([string]$Destination, [object[]]$Images) {
    $stream = [System.IO.File]::Create($Destination)
    $writer = [System.IO.BinaryWriter]::new($stream)
    try {
        $writer.Write([uint16]0); $writer.Write([uint16]1); $writer.Write([uint16]$Images.Count)
        $offset = 6 + 16 * $Images.Count
        foreach ($item in $Images) {
            $dimension = if ($item.Size -eq 256) { 0 } else { $item.Size }
            $writer.Write([byte]$dimension); $writer.Write([byte]$dimension)
            $writer.Write([byte]0); $writer.Write([byte]0)
            $writer.Write([uint16]1); $writer.Write([uint16]32)
            $writer.Write([uint32]$item.Bytes.Length); $writer.Write([uint32]$offset)
            $offset += $item.Bytes.Length
        }
        foreach ($item in $Images) { $writer.Write([byte[]]$item.Bytes) }
    } finally { $writer.Dispose(); $stream.Dispose() }
}

function Get-AtlasPngBytes([System.Drawing.Bitmap]$Bitmap) {
    $stream = [System.IO.MemoryStream]::new()
    try { $Bitmap.Save($stream,[System.Drawing.Imaging.ImageFormat]::Png); return ,$stream.ToArray() }
    finally { $stream.Dispose() }
}

$launcherImages = @()
foreach ($size in @(16,24,32,48,64,128,256,1024)) {
    $bitmap = [System.Drawing.Bitmap]::new($size,$size,[System.Drawing.Imaging.PixelFormat]::Format32bppArgb)
    $graphics = [System.Drawing.Graphics]::FromImage($bitmap)
    $brush = [System.Drawing.SolidBrush]::new([System.Drawing.ColorTranslator]::FromHtml('#ff8a3d'))
    try {
        $graphics.Clear([System.Drawing.Color]::Transparent)
        $graphics.SmoothingMode = [System.Drawing.Drawing2D.SmoothingMode]::AntiAlias
        $scale = $size / 256.0
        $points = [System.Drawing.PointF[]]@(
            [System.Drawing.PointF]::new(110*$scale,43*$scale), [System.Drawing.PointF]::new(146*$scale,43*$scale),
            [System.Drawing.PointF]::new(221*$scale,210*$scale), [System.Drawing.PointF]::new(174*$scale,210*$scale),
            [System.Drawing.PointF]::new(128*$scale,103*$scale), [System.Drawing.PointF]::new(81*$scale,210*$scale),
            [System.Drawing.PointF]::new(35*$scale,210*$scale))
        $graphics.FillPolygon($brush,$points)
        $bytes = Get-AtlasPngBytes $bitmap
        if ($size -le 256) { $launcherImages += [pscustomobject]@{ Size=$size; Bytes=$bytes } }
        if ($size -eq 256) { [System.IO.File]::WriteAllBytes((Join-Path $atlasAssets 'icon.png'),$bytes) }
        if ($size -eq 1024) { [System.IO.File]::WriteAllBytes((Join-Path $atlasAssets 'discord-logo.png'),$bytes) }
    } finally { $brush.Dispose(); $graphics.Dispose(); $bitmap.Dispose() }
}
Write-AtlasIco (Join-Path $atlasAssets 'icon.ico') $launcherImages
Copy-Item -LiteralPath (Join-Path $atlasAssets 'icon.ico') -Destination (Join-Path $atlasAssets 'atlas-launcher.ico') -Force

# These official game icons stay local, alongside the player's own prepared resources.
$cachedGameIcon = Join-Path $atlasAssets 'game-icon.ico'
$cachedGamePng = Join-Path $atlasAssets 'game-icon.png'
if ((Test-Path -LiteralPath $cachedGameIcon) -and (Test-Path -LiteralPath $cachedGamePng)) {
    Copy-Item -LiteralPath $cachedGameIcon -Destination (Join-Path $atlasLocalIcons 'game-icon.ico') -Force
    Copy-Item -LiteralPath $cachedGamePng -Destination (Join-Path $atlasLocalIcons 'game-icon.png') -Force
    Write-Host 'Atlas launcher and game icons are ready.'
    return
}
$minecraftAssets = @((Join-Path $env:APPDATA 'horizon-launcher\minecraft\assets'),(Join-Path $env:APPDATA '.minecraft\assets')) |
    Where-Object { Test-Path -LiteralPath (Join-Path $_ 'indexes') } | Select-Object -First 1
if (-not $minecraftAssets) { throw 'The local Minecraft asset store was not found.' }
$indexFile = Get-ChildItem -LiteralPath (Join-Path $minecraftAssets 'indexes') -File -Filter '*.json' | Sort-Object LastWriteTime -Descending | Select-Object -First 1
$index = Get-Content -LiteralPath $indexFile.FullName -Raw | ConvertFrom-Json
$gameImages = @()
foreach ($size in @(16,32,48,128,256)) {
    $entry = $index.objects.PSObject.Properties["icons/icon_${size}x${size}.png"].Value
    if (-not $entry) { throw "Minecraft icon ${size}px is not present in the local asset index." }
    $assetPath = Join-Path $minecraftAssets ("objects\" + $entry.hash.Substring(0,2) + '\' + $entry.hash)
    if ((Get-FileHash -LiteralPath $assetPath -Algorithm SHA1).Hash.ToLowerInvariant() -ne $entry.hash) { throw 'A Minecraft icon failed its asset checksum.' }
    $bytes = [System.IO.File]::ReadAllBytes($assetPath)
    $gameImages += [pscustomobject]@{ Size=$size; Bytes=$bytes }
    if ($size -eq 256) {
        [System.IO.File]::WriteAllBytes((Join-Path $atlasAssets 'game-icon.png'),$bytes)
        [System.IO.File]::WriteAllBytes((Join-Path $atlasLocalIcons 'game-icon.png'),$bytes)
    }
}
Write-AtlasIco (Join-Path $atlasAssets 'game-icon.ico') $gameImages
Copy-Item -LiteralPath (Join-Path $atlasAssets 'game-icon.ico') -Destination (Join-Path $atlasLocalIcons 'game-icon.ico') -Force
Write-Host 'Atlas launcher: transparent orange A. Game: local Minecraft grass-block icon.'
