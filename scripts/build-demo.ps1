$ErrorActionPreference = 'Stop'

$root = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
$rust = Join-Path $root 'rust'
Set-Location -LiteralPath $root

if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
    $portable = Join-Path $env:TEMP 'breplot-rust'
    $cargoBin = Join-Path $portable 'cargo\bin'
    if (Test-Path (Join-Path $cargoBin 'cargo.exe')) {
        $env:RUSTUP_HOME = Join-Path $portable 'rustup'
        $env:CARGO_HOME = Join-Path $portable 'cargo'
        $env:PATH = $cargoBin + ';' + $env:PATH
    } else {
        throw 'cargo not found. Install Rust 1.96.0 with wasm32-unknown-unknown first.'
    }
}
if (-not (Get-Command typst -ErrorAction SilentlyContinue)) {
    throw 'typst not found. Install the Typst CLI first.'
}

Push-Location -LiteralPath $rust
try {
    & cargo test --lib
    if ($LASTEXITCODE -ne 0) { throw 'Rust tests failed.' }
    & cargo build --release --target wasm32-unknown-unknown --lib
    if ($LASTEXITCODE -ne 0) { throw 'WASM build failed.' }
} finally {
    Pop-Location
}

Copy-Item -LiteralPath (Join-Path $rust 'target\wasm32-unknown-unknown\release\breplot.wasm') `
    -Destination (Join-Path $root 'package\breplot.wasm') -Force
New-Item -ItemType Directory -Force -Path (Join-Path $root 'target') | Out-Null
& (Join-Path $PSScriptRoot 'refresh-preview.ps1')
if ($LASTEXITCODE -ne 0) { throw 'STEP preview cache build failed.' }

& typst compile --root $root 'main.typ' 'target\main.png'
if ($LASTEXITCODE -ne 0) { throw 'Typst main preview build failed.' }
& typst compile --root $root 'examples\demo.typ' 'target\demo.png'
if ($LASTEXITCODE -ne 0) { throw 'Typst PNG preview build failed.' }
& typst compile --root $root 'examples\hidden.typ' 'target\hidden.png'
if ($LASTEXITCODE -ne 0) { throw 'Typst hidden-line preview build failed.' }
& typst compile --root $root 'examples\flat.typ' 'target\flat.png'
if ($LASTEXITCODE -ne 0) { throw 'Typst flat surface preview build failed.' }
& typst compile --root $root 'examples\nurbs.typ' 'target\nurbs.png'
if ($LASTEXITCODE -ne 0) { throw 'Typst NURBS preview build failed.' }

$packageRoot = Join-Path $root 'target\typst-packages'
$localPackage = Join-Path $packageRoot 'local\breplot\0.1.0'
New-Item -ItemType Directory -Force -Path $localPackage | Out-Null
Copy-Item -LiteralPath (Join-Path $root 'package\typst.toml') -Destination $localPackage -Force
Copy-Item -LiteralPath (Join-Path $root 'package\lib.typ') -Destination $localPackage -Force
Copy-Item -LiteralPath (Join-Path $root 'package\step.typ') -Destination $localPackage -Force
Copy-Item -LiteralPath (Join-Path $root 'package\curve.typ') -Destination $localPackage -Force
Copy-Item -LiteralPath (Join-Path $root 'package\breplot.wasm') -Destination $localPackage -Force
& typst compile --root $root --package-path $packageRoot 'examples\package-import.typ' 'target\package-import.png'
if ($LASTEXITCODE -ne 0) { throw 'Typst local package import failed.' }

Write-Output 'Built target\main.png, target\demo.png, target\hidden.png, target\flat.png, target\nurbs.png, and target\package-import.png'
