param([switch]$IncludeModules)
$ErrorActionPreference = 'Stop'
$root = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
    $portable = Join-Path $env:TEMP 'breplot-rust'
    $env:RUSTUP_HOME = Join-Path $portable 'rustup'
    $env:CARGO_HOME = Join-Path $portable 'cargo'
    $env:PATH = (Join-Path $portable 'cargo/bin') + ';' + $env:PATH
}
$manifest = Join-Path $root 'nurbs-surface/rust/Cargo.toml'
& cargo test --release --manifest-path $manifest
if ($LASTEXITCODE -ne 0) { throw 'Pure Rust B-Rep tests failed.' }
& (Join-Path $root 'nurbs-surface/scripts/build-pure-3dm.ps1')
& typst compile --root $root (Join-Path $root 'main.typ') (Join-Path $root 'target/main.pdf')
if ($LASTEXITCODE -ne 0) { throw 'Main 3dm article failed.' }
& pdftoppm -scale-to 1200 -singlefile -png (Join-Path $root 'target/main.pdf') (Join-Path $root 'target/main')
if ($LASTEXITCODE -ne 0) { throw 'Main 3dm preview failed.' }
if ($IncludeModules) {
    & (Join-Path $root 'nurbs-surface/scripts/build.ps1')
    & (Join-Path $root 'cetz-nurbs/scripts/build.ps1')
}
Write-Output 'Built main 3dm project: target/release/breplot.exe, target/main.pdf and target/main.png.'
