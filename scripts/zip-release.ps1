param([Parameter(Mandatory=$true)][string]$SourceDirectory, [Parameter(Mandatory=$true)][string]$ArchivePath)
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.IO.Compression.FileSystem
Add-Type -AssemblyName System.IO.Compression
if (Test-Path -LiteralPath $ArchivePath) { throw 'Release archive already exists.' }
# Flat entries match atlas-update.json exactly. Directory entries are unnecessary.
$atlasSource = [IO.Path]::GetFullPath($SourceDirectory).TrimEnd('\')
$atlasZip = [IO.Compression.ZipFile]::Open($ArchivePath, [IO.Compression.ZipArchiveMode]::Create)
try {
    Get-ChildItem -LiteralPath $atlasSource -File -Recurse | ForEach-Object {
        $atlasName = $_.FullName.Substring($atlasSource.Length + 1).Replace('\', '/')
        [IO.Compression.ZipFileExtensions]::CreateEntryFromFile($atlasZip, $_.FullName, $atlasName, [IO.Compression.CompressionLevel]::Fastest) | Out-Null
    }
} finally { $atlasZip.Dispose() }
