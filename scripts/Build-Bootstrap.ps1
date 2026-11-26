param([Parameter(Mandatory = $true)][string]$OutputDirectory, [switch]$Preview)

$ErrorActionPreference = 'Stop'
$atlasRoot = Split-Path -Parent $PSScriptRoot
$atlasFramework = Join-Path $env:WINDIR 'Microsoft.NET\Framework64\v4.0.30319'
$atlasCompiler = Join-Path $atlasFramework 'csc.exe'
if (-not (Test-Path -LiteralPath $atlasCompiler -PathType Leaf)) {
    throw 'The Windows .NET Framework C# compiler is required to build the Atlas loader.'
}

$atlasVersion = (Get-Content -LiteralPath (Join-Path $atlasRoot 'package.json') -Raw | ConvertFrom-Json).version
if ($atlasVersion -notmatch '\A(0|[1-9][0-9]{0,4})\.(0|[1-9][0-9]{0,4})\.(0|[1-9][0-9]{0,4})\z' -or
    @($atlasVersion.Split('.') | Where-Object { [int]$_ -gt 65534 }).Count) {
    throw 'Atlas needs a three-part numeric version for its Windows executable.'
}

$atlasOutput = [IO.Path]::GetFullPath($OutputDirectory)
$atlasExecutable = Join-Path $atlasOutput 'Atlas-Launcher.exe'
if (Test-Path -LiteralPath $atlasExecutable) { throw "Atlas loader already exists: $atlasExecutable" }
$atlasSources = @(Get-ChildItem -LiteralPath (Join-Path $atlasRoot 'bootstrap') -File -Filter '*.cs' | Sort-Object Name)
if (-not $atlasSources.Count) { throw 'Atlas loader sources were not found.' }
$atlasIcon = Join-Path $atlasRoot 'assets\atlas-launcher.ico'

$atlasScratch = Join-Path ([IO.Path]::GetTempPath()) ('atlas-bootstrap-build-' + [guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $atlasScratch | Out-Null
$atlasMetadata = Join-Path $atlasScratch 'Version.cs'
$atlasManifest = Join-Path $atlasScratch 'Atlas-Launcher.manifest'
$atlasCompiled = Join-Path $atlasScratch 'Atlas-Launcher.exe'
try {
    @"
using System.Reflection;
[assembly: System.Runtime.Versioning.TargetFramework(".NETFramework,Version=v4.6.2")]
[assembly: AssemblyTitle("Atlas Launcher")]
[assembly: AssemblyDescription("Atlas launcher setup and updates")]
[assembly: AssemblyCompany("SFrostStar")]
[assembly: AssemblyProduct("Atlas")]
[assembly: AssemblyVersion("$atlasVersion.0")]
[assembly: AssemblyFileVersion("$atlasVersion.0")]
[assembly: AssemblyInformationalVersion("$atlasVersion")]
"@ | Set-Content -LiteralPath $atlasMetadata -Encoding UTF8
    @'
<?xml version="1.0" encoding="utf-8"?>
<assembly xmlns="urn:schemas-microsoft-com:asm.v1" manifestVersion="1.0">
  <assemblyIdentity version="1.0.0.0" processorArchitecture="amd64" name="SFrostStar.AtlasLauncher" type="win32"/>
  <trustInfo xmlns="urn:schemas-microsoft-com:asm.v3">
    <security>
      <requestedPrivileges>
        <requestedExecutionLevel level="asInvoker" uiAccess="false"/>
      </requestedPrivileges>
    </security>
  </trustInfo>
  <compatibility xmlns="urn:schemas-microsoft-com:compatibility.v1">
    <application><supportedOS Id="{8e0f7a12-bfb3-4fe8-b9a5-48fd50a15a9a}"/></application>
  </compatibility>
  <application xmlns="urn:schemas-microsoft-com:asm.v3">
    <windowsSettings>
      <longPathAware xmlns="http://schemas.microsoft.com/SMI/2016/WindowsSettings">true</longPathAware>
      <dpiAwareness xmlns="http://schemas.microsoft.com/SMI/2016/WindowsSettings">PerMonitorV2</dpiAwareness>
    </windowsSettings>
  </application>
</assembly>
'@ | Set-Content -LiteralPath $atlasManifest -Encoding UTF8
    $atlasReferences = @('System.Windows.Forms', 'System.Drawing', 'System.Web.Extensions', 'System.IO.Compression', 'Microsoft.CSharp')
    $atlasArguments = @('/nologo', '/target:winexe', '/platform:x64', '/langversion:5', '/codepage:65001', '/optimize+',
        "/win32icon:$atlasIcon", "/win32manifest:$atlasManifest", "/out:$atlasCompiled",
        "/resource:$(Join-Path $atlasRoot 'assets\atlas-landscape.png'),atlas.landscape",
        "/resource:$(Join-Path $atlasRoot 'assets\icon.png'),atlas.mark",
        "/resource:$(Join-Path $atlasRoot 'assets\fonts\Geist-Regular.ttf'),atlas.geist.regular",
        "/resource:$(Join-Path $atlasRoot 'assets\fonts\Geist-SemiBold.ttf'),atlas.geist.semibold",
        "/resource:$(Join-Path $atlasRoot 'assets\fonts\OFL-Geist.txt'),atlas.geist.license")
    if ($Preview) { $atlasArguments += '/define:ATLAS_PREVIEW' }
    $atlasArguments += $atlasReferences | ForEach-Object { "/reference:$(Join-Path $atlasFramework "$_.dll")" }
    $atlasArguments += $atlasMetadata
    $atlasArguments += $atlasSources.FullName
    & $atlasCompiler @atlasArguments
    if ($LASTEXITCODE -ne 0) { throw 'Atlas loader compilation failed.' }
    $atlasFile = Get-Item -LiteralPath $atlasCompiled
    if ($atlasFile.Length -le 0 -or $atlasFile.Length -gt 50000000 -or $atlasFile.VersionInfo.ProductVersion -ne $atlasVersion) {
        throw 'Atlas loader failed its size or version check.'
    }
    New-Item -ItemType Directory -Path $atlasOutput -Force | Out-Null
    [IO.File]::Move($atlasCompiled, $atlasExecutable)
    Write-Output $atlasExecutable
} finally {
    foreach ($atlasTempFile in @($atlasMetadata, $atlasManifest, $atlasCompiled)) {
        if (Test-Path -LiteralPath $atlasTempFile -PathType Leaf) { Remove-Item -LiteralPath $atlasTempFile }
    }
    [IO.Directory]::Delete($atlasScratch)
}
