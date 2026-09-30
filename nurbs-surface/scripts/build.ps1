param([switch]$VectorSurfaces)
$ErrorActionPreference = 'Stop'
$moduleRoot = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
    $portable = Join-Path $env:TEMP 'breplot-rust'
    $env:RUSTUP_HOME = Join-Path $portable 'rustup'
    $env:CARGO_HOME = Join-Path $portable 'cargo'
    $env:PATH = (Join-Path $portable 'cargo\bin') + ';' + $env:PATH
}
$manifest = Join-Path $moduleRoot 'rust\Cargo.toml'
& cargo test --manifest-path $manifest --lib
if ($LASTEXITCODE -ne 0) { throw 'Surface data tests failed.' }
& cargo test --manifest-path $manifest --test cases
if ($LASTEXITCODE -ne 0) { throw 'Surface case tests failed.' }
& cargo test --manifest-path $manifest --test trimming
if ($LASTEXITCODE -ne 0) { throw 'Surface trimming tests failed.' }
& cargo run --manifest-path $manifest --example data
if ($LASTEXITCODE -ne 0) { throw 'Surface data example failed.' }
& cargo build --manifest-path $manifest --target wasm32-unknown-unknown --lib
if ($LASTEXITCODE -ne 0) { throw 'Surface data WASM compilation failed.' }
New-Item -ItemType Directory -Force -Path (Join-Path $moduleRoot 'target') | Out-Null
& cargo run --manifest-path $manifest --example mesh -- (Join-Path $moduleRoot 'target\meshes.json')
if ($LASTEXITCODE -ne 0) { throw 'Surface mesh demonstration failed.' }
& cargo test --manifest-path $manifest --test display
if ($LASTEXITCODE -ne 0) { throw 'B-Rep display tests failed.' }
& cargo run --manifest-path $manifest --example display -- (Join-Path $moduleRoot 'target')
if ($LASTEXITCODE -ne 0) { throw 'B-Rep display demonstration failed.' }
$displayMode = if ($VectorSurfaces) { 'vector' } else { 'hybrid' }
if (-not $VectorSurfaces) { & (Join-Path $PSScriptRoot 'build-display-assets.ps1') }
& typst compile --input "surface-output=$displayMode" --root $moduleRoot (Join-Path $moduleRoot 'main.typ') (Join-Path $moduleRoot 'main.pdf')
if ($LASTEXITCODE -ne 0) { throw 'Surface article PDF failed.' }
& typst compile --input "surface-output=$displayMode" --root $moduleRoot (Join-Path $moduleRoot 'main.typ') (Join-Path $moduleRoot 'target\main-{p}.png')
if ($LASTEXITCODE -ne 0) { throw 'Surface article PNG failed.' }
Write-Output 'Built independent NURBS surface data and article.'
