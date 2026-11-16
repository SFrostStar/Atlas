$ErrorActionPreference = 'Stop'
$atlasProject = Split-Path -Parent $PSScriptRoot
$atlasTemp = Join-Path ([IO.Path]::GetTempPath()) ('atlas-native-tests-' + [guid]::NewGuid())
$atlasCompiler = Join-Path $env:WINDIR 'Microsoft.NET\Framework64\v4.0.30319\csc.exe'
New-Item -ItemType Directory -Path $atlasTemp | Out-Null
try {
    $atlasTest = Join-Path $atlasTemp 'BootstrapTests.exe'
    $atlasArguments = @(
        '/nologo', '/utf8output', '/langversion:5', '/target:exe', "/out:$atlasTest",
        '/r:System.Web.Extensions.dll', '/r:System.IO.Compression.dll', '/r:Microsoft.CSharp.dll',
        (Join-Path $atlasProject 'bootstrap\UpdateCore.cs'), (Join-Path $atlasProject 'bootstrap\DesktopEntry.cs'),
        (Join-Path $atlasProject 'tests\bootstrap-core.cs')
    )
    & $atlasCompiler @atlasArguments
    if ($LASTEXITCODE -ne 0) { throw 'Could not compile the bootstrap regression test.' }
    & $atlasTest $atlasTemp
    if ($LASTEXITCODE -ne 0) { throw 'Bootstrap regression failed.' }
} finally {
    & (Join-Path $PSScriptRoot 'Remove-AtlasScratch.ps1') -Directory $atlasTemp
}
