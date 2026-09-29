param()
$ErrorActionPreference = 'Stop'
$moduleRoot = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
& python -c "import pymupdf"
if ($LASTEXITCODE -ne 0) { throw 'Type 4 export requires Python with PyMuPDF: python -m pip install PyMuPDF' }
if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
    $portable = Join-Path $env:TEMP 'breplot-rust'
    $env:RUSTUP_HOME = Join-Path $portable 'rustup'
    $env:CARGO_HOME = Join-Path $portable 'cargo'
    $env:PATH = (Join-Path $portable 'cargo\bin') + ';' + $env:PATH
}
& cargo build --release --manifest-path (Join-Path $moduleRoot 'rust/Cargo.toml') --target wasm32-unknown-unknown --lib
if ($LASTEXITCODE -ne 0) { throw 'Type 4 plugin build failed.' }
New-Item -ItemType Directory -Force (Join-Path $moduleRoot 'package') | Out-Null
Copy-Item -LiteralPath (Join-Path $moduleRoot 'rust/target/wasm32-unknown-unknown/release/nurbs_surface.wasm') -Destination (Join-Path $moduleRoot 'package/surface.wasm')
# No mesh JSON, SVG, PNG, or per-illustration PDF is generated or read.
& typst compile --input surface-output=pdf --root $moduleRoot (Join-Path $moduleRoot 'main.typ') (Join-Path $moduleRoot 'main-type4.pdf')
if ($LASTEXITCODE -ne 0) { throw 'Type 4 article compilation failed.' }
& python (Join-Path $PSScriptRoot 'normalize-pdf-groups.py') (Join-Path $moduleRoot 'main-type4.pdf')
if ($LASTEXITCODE -ne 0) { throw 'PDF RGB transparency group normalization failed.' }
Get-Item (Join-Path $moduleRoot 'main-type4.pdf') | Select-Object FullName,Length
