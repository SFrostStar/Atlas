param([Parameter(Mandatory)][string]$Directory)
$ErrorActionPreference = 'Stop'

$temp = [IO.Path]::GetFullPath([IO.Path]::GetTempPath()).TrimEnd([IO.Path]::DirectorySeparatorChar)
$path = [IO.Path]::GetFullPath($Directory)
if (
    [IO.Path]::GetDirectoryName($path) -ne $temp -or
    [IO.Path]::GetFileName($path) -notmatch '^atlas-(native-tests|setup-build)-[a-f0-9-]{36}$'
) {
    throw "Refusing to remove a directory outside Atlas scratch space: $path"
}
if (-not (Test-Path -LiteralPath $path -PathType Container)) { return }
$scratchItem = Get-Item -LiteralPath $path -Force
if ($scratchItem.Attributes -band [IO.FileAttributes]::ReparsePoint) {
    throw "Atlas scratch directory is a link: $path"
}
$pending = [Collections.Generic.Stack[IO.DirectoryInfo]]::new()
$pending.Push($scratchItem)
while ($pending.Count -gt 0) {
    foreach ($entry in $pending.Pop().EnumerateFileSystemInfos()) {
        if ($entry.Attributes -band [IO.FileAttributes]::ReparsePoint) {
            throw "Atlas scratch directory contains a link: $($entry.FullName)"
        }
        if ($entry -is [IO.DirectoryInfo]) { $pending.Push($entry) }
    }
}
[IO.Directory]::Delete($scratchItem.FullName, $true)
