param([switch]$AfterRename)
$ErrorActionPreference = 'Stop'
$atlasProject = Split-Path -Parent $PSScriptRoot
$env:CARGO_HOME = Join-Path $atlasProject 'tools\cargo'
$env:RUSTUP_HOME = Join-Path $atlasProject 'tools\rustup'
$llvmDirectory = Get-ChildItem -LiteralPath (Join-Path $atlasProject 'tools') -Directory -Filter 'llvm-mingw-*-ucrt-x86_64' | Select-Object -First 1
$env:PATH = (Join-Path $llvmDirectory.FullName 'bin') + ';' + (Join-Path $env:CARGO_HOME 'bin') + ';' + $env:PATH
$scratch = Join-Path $atlasProject ('work\native-tests-' + [guid]::NewGuid().ToString())
New-Item -ItemType Directory -Path $scratch -Force | Out-Null
$env:ATLAS_HOME = $scratch
$env:ATLAS_LAUNCHER_HOME = $scratch
$env:XDG_DATA_HOME = $scratch
$env:XDG_CONFIG_HOME = $scratch
$env:ATLAS_NO_AUDIO = '1'
$env:ATLAS_NO_HEARTBEAT = '1'
$env:ATLAS_TELEMETRY_URL = 'off'
$env:ATLAS_LAUNCHER_NO_SELF_UPDATE = '1'
$client = 'atlas-client'
$world = 'atlas-world'
Push-Location (Join-Path $atlasProject 'engine\atlas')
try {
  foreach ($filter in @('atlas_loading::tests','packs::windows_overlay_links','audio::tests','zip::tests','title::tests','mobs::shulker::tests')) {
    & cargo test --locked --profile fast -p $client --features mods --lib $filter
    if ($LASTEXITCODE -ne 0) { throw "Native library test failed: $filter" }
  }
  foreach ($filter in @('reload_recovery_tests','game::entity_culling::tests','startup_menu::tests','mouse_position_after')) {
    & cargo test --locked --profile fast -p $client --features mods --bin $client $filter
    if ($LASTEXITCODE -ne 0) { throw "Native client test failed: $filter" }
  }
  & cargo test --locked --profile fast -p $world decoration_is_shared_by_biome_source
  if ($LASTEXITCODE -ne 0) { throw 'Worldgen cache regression failed' }
  & cargo test --locked --profile fast -p atlas-mods --lib api_level::tests
  if ($LASTEXITCODE -ne 0) { throw 'Mod API marker regression failed' }
  & cargo test --locked --profile fast -p atlas-memory
  if ($LASTEXITCODE -ne 0) { throw 'Native memory limit tests failed' }
} finally { Pop-Location }
