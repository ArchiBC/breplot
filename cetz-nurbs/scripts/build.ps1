$ErrorActionPreference = 'Stop'
$root = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
    $portable = Join-Path $env:TEMP 'breplot-rust'
    $env:RUSTUP_HOME = Join-Path $portable 'rustup'
    $env:CARGO_HOME = Join-Path $portable 'cargo'
    $env:PATH = (Join-Path $portable 'cargo\bin') + ';' + $env:PATH
}
Push-Location -LiteralPath (Join-Path $root 'rust')
try {
    & cargo test --lib
    if ($LASTEXITCODE -ne 0) { throw 'NURBS tests failed.' }
    & cargo build --release --target wasm32-unknown-unknown --lib
    if ($LASTEXITCODE -ne 0) { throw 'NURBS WASM build failed.' }
} finally { Pop-Location }
Copy-Item -LiteralPath (Join-Path $root 'rust\target\wasm32-unknown-unknown\release\cetz_nurbs.wasm') -Destination (Join-Path $root 'package\cetz_nurbs.wasm') -Force
New-Item -ItemType Directory -Force -Path (Join-Path $root 'target') | Out-Null
& typst compile --root $root (Join-Path $root 'main.typ') (Join-Path $root 'main.pdf')
if ($LASTEXITCODE -ne 0) { throw 'CeTZ NURBS PDF failed.' }
& typst compile --root $root (Join-Path $root 'main.typ') (Join-Path $root 'target\main-{p}.png')
if ($LASTEXITCODE -ne 0) { throw 'CeTZ NURBS demo failed.' }
$packageRoot = Join-Path $root 'target\typst-packages'
$localPackage = Join-Path $packageRoot 'local\cetz-nurbs\0.1.0'
New-Item -ItemType Directory -Force -Path $localPackage | Out-Null
Copy-Item -Path (Join-Path $root 'package\*') -Destination $localPackage -Force
& typst compile --root $root --package-path $packageRoot (Join-Path $root 'examples\package-import.typ') (Join-Path $root 'target\package-import.png')
if ($LASTEXITCODE -ne 0) { throw 'Standalone NURBS package import failed.' }
Write-Output 'Built standalone cetz-nurbs WASM and CeTZ examples.'
