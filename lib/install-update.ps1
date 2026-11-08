param([Parameter(Mandatory=$true)][string]$PlanPath, [switch]$Bootstrap)
$ErrorActionPreference = 'Stop'
if ($Bootstrap) {
    # ShellExecute starts a hidden, independent console host. A PowerShell host
    # inherited directly from a closing launcher can lose its console and exit.
    $atlasArguments = @('-NoProfile','-NonInteractive','-ExecutionPolicy','Bypass','-File',('"' + $PSCommandPath + '"'),'-PlanPath',('"' + [IO.Path]::GetFullPath($PlanPath) + '"'))
    Start-Process -FilePath (Join-Path $PSHOME 'powershell.exe') -ArgumentList $atlasArguments -WorkingDirectory (Split-Path -Parent $PSCommandPath) -WindowStyle Hidden | Out-Null
    exit 0
}
$atlasPlan = $null
$atlasApplied = [Collections.Generic.List[object]]::new()

function Atlas-Guard([string]$atlasRoot, [string]$atlasTarget) {
    $atlasRoot = [IO.Path]::GetFullPath($atlasRoot).TrimEnd('\')
    $atlasTarget = [IO.Path]::GetFullPath($atlasTarget)
    if ($atlasTarget -ne $atlasRoot -and -not $atlasTarget.StartsWith($atlasRoot + '\', [StringComparison]::OrdinalIgnoreCase)) { throw 'Update destination escapes its installation folder.' }
    $atlasProbe = $atlasTarget
    while ($atlasProbe) {
        if (Test-Path -LiteralPath $atlasProbe) {
            if ((Get-Item -LiteralPath $atlasProbe -Force).Attributes -band [IO.FileAttributes]::ReparsePoint) { throw 'Update paths must not contain linked folders.' }
        }
        $atlasParent = Split-Path -Parent $atlasProbe
        if ($atlasParent -eq $atlasProbe) { break }
        $atlasProbe = $atlasParent
    }
    return $atlasTarget
}
function Atlas-Relative([string]$atlasValue) {
    if (-not $atlasValue -or $atlasValue.Contains('\') -or $atlasValue -match '[:\x00-\x1f]') { throw 'Invalid update relative path.' }
    foreach ($atlasComponent in $atlasValue.Split('/')) {
        if (-not $atlasComponent -or $atlasComponent -in @('.', '..') -or $atlasComponent -match '[. ]$' -or $atlasComponent -match '^(con|prn|aux|nul|com[1-9]|lpt[1-9])(?:\.|$)') { throw 'Invalid update relative path.' }
    }
}
function Atlas-Destination($atlasFile) {
    Atlas-Relative $atlasFile.relative
    switch ($atlasFile.kind) {
        'app' {
            $atlasAllowed = @('Atlas Launcher.exe','chrome_100_percent.pak','chrome_200_percent.pak','d3dcompiler_47.dll','dxcompiler.dll','dxil.dll','electron_wer.dll','ffmpeg.dll','icudtl.dat','LICENSE','LICENSES.chromium.html','resources.pak','snapshot_blob.bin','v8_context_snapshot.bin','version','vk_swiftshader_icd.json','vk_swiftshader.dll','vulkan-1.dll','resources/app.asar','atlas-portable.json','START-HERE.txt')
            if ($atlasFile.relative -cnotin $atlasAllowed -and $atlasFile.relative -cnotmatch '^locales/[a-zA-Z0-9_-]+\.pak$') { throw 'Unexpected launcher update file.' }
            return Atlas-Guard $atlasPlan.appRoot (Join-Path $atlasPlan.appRoot $atlasFile.relative)
        }
        'runtime' {
            if ($atlasFile.relative -cnotin @('atlas-client.exe','atlas-ctl.exe','atlas-prepare.exe','atlas-server.exe','libc++.dll','libunwind.dll','libwinpthread-1.dll','atlas-runtime.json') -and $atlasFile.relative -cnotmatch '^LICENSE[^/]*\.txt$') { throw 'Unexpected runtime update file.' }
            $atlasRuntimeRoot = Join-Path $atlasPlan.projectRoot 'runtime'
            return Atlas-Guard $atlasRuntimeRoot (Join-Path $atlasRuntimeRoot $atlasFile.relative)
        }
        'bundle' {
            $atlasAllowed = $atlasFile.relative -cin @('atlas-client.exe','atlas-ctl.exe','atlas-prepare.exe','atlas-server.exe','libc++.dll','libunwind.dll','libwinpthread-1.dll','atlas-runtime.json','game-root/research-lock.json','game-root/ATLAS-UPSTREAM.json','game-root/crates/atlas-prepare/src/exporters.rs') -or $atlasFile.relative -cmatch '^LICENSE[^/]*\.txt$' -or $atlasFile.relative -cmatch '^game-root/exporters/(?:manifest\.json|(?:client|server)/dev/atlaslab/parity/[A-Za-z0-9_$]+\.class)$' -or $atlasFile.relative -cmatch '^game-root/mods/atlas/src/test/java/dev/atlaslab/parity/[A-Za-z0-9_$]+\.java$'
            if (-not $atlasAllowed) { throw 'Unexpected bundled game update file.' }
            $atlasBundleRoot = Join-Path $atlasPlan.projectRoot 'bundle/game'
            return Atlas-Guard $atlasBundleRoot (Join-Path $atlasBundleRoot $atlasFile.relative)
        }
        default { throw 'Unknown update file category.' }
    }
}
function Atlas-Result([string]$atlasStatus, [string]$atlasError) {
    $atlasResultPath = Atlas-Guard (Split-Path -Parent (Split-Path -Parent $PlanPath)) $atlasPlan.result
    @{ status=$atlasStatus; version=$atlasPlan.version; error=$atlasError; time=[DateTime]::UtcNow.ToString('o') } | ConvertTo-Json | Set-Content -LiteralPath $atlasResultPath -Encoding UTF8
}
function Atlas-SHA256([string]$atlasFile) {
    $atlasStream = [IO.File]::OpenRead($atlasFile)
    $atlasHasher = [Security.Cryptography.SHA256]::Create()
    try { return [BitConverter]::ToString($atlasHasher.ComputeHash($atlasStream)).Replace('-', '').ToLowerInvariant() }
    finally { $atlasHasher.Dispose(); $atlasStream.Dispose() }
}

try {
    $atlasPlan = Get-Content -LiteralPath $PlanPath -Raw | ConvertFrom-Json
    if ($atlasPlan.schema -ne 1 -or $atlasPlan.version -notmatch '^\d+\.\d+\.\d+$' -or $atlasPlan.files.Count -lt 1 -or $atlasPlan.files.Count -gt 24000) { throw 'Invalid Atlas update plan.' }
    $atlasStage = Split-Path -Parent ([IO.Path]::GetFullPath($PlanPath))
    Atlas-Guard $atlasStage $atlasPlan.payload | Out-Null
    Atlas-Guard $atlasStage $atlasPlan.backup | Out-Null
    if ($atlasPlan.started) {
        $atlasStarted = Atlas-Guard $atlasStage $atlasPlan.started
        [IO.File]::WriteAllText($atlasStarted, (@{ version=$atlasPlan.version; parentPid=$atlasPlan.parentPid } | ConvertTo-Json), [Text.UTF8Encoding]::new($false))
    }
    $atlasBlockedPaths = @($atlasPlan.executable, (Join-Path $atlasPlan.projectRoot 'runtime/atlas-client.exe'), (Join-Path $atlasPlan.projectRoot 'runtime/atlas-prepare.exe'), (Join-Path $atlasPlan.projectRoot 'bundle/game/atlas-client.exe'), (Join-Path $atlasPlan.projectRoot 'bundle/game/atlas-prepare.exe'))
    # A game open in another launcher window is protected too. Never force-close a game.
    $atlasDeadline = [DateTime]::UtcNow.AddMinutes(10)
    do {
        $atlasParent = Get-Process -Id $atlasPlan.parentPid -ErrorAction SilentlyContinue
        $atlasBusy = @(Get-CimInstance Win32_Process | Where-Object { $_.ExecutablePath -and $_.ExecutablePath -in $atlasBlockedPaths })
        if (-not $atlasParent -and $atlasBusy.Count -eq 0) { break }
        if ([DateTime]::UtcNow -gt $atlasDeadline) { throw 'Close Atlas and Minecraft to finish the downloaded update.' }
        Start-Sleep -Milliseconds 500
    } while ($true)

    # Verify the complete payload and every destination before replacing the first file.
    $atlasTargets = @{}
    foreach ($atlasFile in $atlasPlan.files) {
        Atlas-Relative $atlasFile.path
        $atlasSource = Atlas-Guard $atlasPlan.payload (Join-Path $atlasPlan.payload $atlasFile.path)
        $atlasTarget = Atlas-Destination $atlasFile
        if ($atlasTargets.ContainsKey($atlasTarget)) { throw 'Duplicate update destination.' }
        $atlasTargets[$atlasTarget] = $true
        if ($atlasFile.sha256 -notmatch '^[a-f0-9]{64}$' -or (Get-Item -LiteralPath $atlasSource).Length -ne $atlasFile.size -or (Atlas-SHA256 $atlasSource) -ine $atlasFile.sha256) { throw 'Atlas update checksum verification failed.' }
    }
    foreach ($atlasFile in $atlasPlan.files) {
        $atlasSource = Atlas-Guard $atlasPlan.payload (Join-Path $atlasPlan.payload $atlasFile.path)
        $atlasTarget = Atlas-Destination $atlasFile
        $atlasBackup = Atlas-Guard $atlasPlan.backup (Join-Path $atlasPlan.backup ($atlasFile.kind + '/' + $atlasFile.relative))
        $atlasExisted = Test-Path -LiteralPath $atlasTarget -PathType Leaf
        if (Test-Path -LiteralPath $atlasTarget -PathType Container) { throw 'A folder occupies an Atlas update file destination.' }
        if ($atlasExisted) {
            New-Item -ItemType Directory -Path (Split-Path -Parent $atlasBackup) -Force | Out-Null
            Copy-Item -LiteralPath $atlasTarget -Destination $atlasBackup
        }
        New-Item -ItemType Directory -Path (Split-Path -Parent $atlasTarget) -Force | Out-Null
        # Record before mutation so rollback also covers a failed replacement.
        $atlasApplied.Add(@{ target=$atlasTarget; backup=$atlasBackup; existed=$atlasExisted })
        $atlasTemporary = $atlasTarget + '.atlas-update-' + [guid]::NewGuid().ToString('N')
        try {
            Copy-Item -LiteralPath $atlasSource -Destination $atlasTemporary
            Move-Item -LiteralPath $atlasTemporary -Destination $atlasTarget -Force
        } finally {
            if (Test-Path -LiteralPath $atlasTemporary) { Remove-Item -LiteralPath $atlasTemporary -Force }
        }
    }
    Atlas-Result 'installed' ''
    if ($atlasPlan.restart) {
        Start-Process -FilePath $atlasPlan.executable -WorkingDirectory $atlasPlan.projectRoot -WindowStyle Hidden | Out-Null
    }
} catch {
    $atlasFailure = $_.Exception.Message
    $atlasRollbackFailure = ''
    for ($atlasIndex = $atlasApplied.Count - 1; $atlasIndex -ge 0; $atlasIndex--) {
        $atlasPrevious = $atlasApplied[$atlasIndex]
        try {
            if ($atlasPrevious.existed) { Copy-Item -LiteralPath $atlasPrevious.backup -Destination $atlasPrevious.target -Force }
            elseif (Test-Path -LiteralPath $atlasPrevious.target -PathType Leaf) { Remove-Item -LiteralPath $atlasPrevious.target -Force }
        } catch { $atlasRollbackFailure += $_.Exception.Message + ' ' }
    }
    if ($atlasRollbackFailure) { $atlasFailure += ' Rollback: ' + $atlasRollbackFailure }
    if ($atlasPlan) {
        try { Atlas-Result 'error' $atlasFailure } catch { }
        if ($atlasPlan.restart -and (Test-Path -LiteralPath $atlasPlan.executable -PathType Leaf)) {
            Start-Process -FilePath $atlasPlan.executable -WorkingDirectory $atlasPlan.projectRoot -WindowStyle Hidden | Out-Null
        }
    }
    exit 1
}
