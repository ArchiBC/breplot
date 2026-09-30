param()
$ErrorActionPreference = 'Stop'
$root = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
    $portable = Join-Path $env:TEMP 'breplot-rust'
    $env:RUSTUP_HOME = Join-Path $portable 'rustup'
    $env:CARGO_HOME = Join-Path $portable 'cargo'
    $env:PATH = (Join-Path $portable 'cargo/bin') + ';' + $env:PATH
}
$out = Join-Path $root 'target/pure-3dm'
New-Item -ItemType Directory -Force $out | Out-Null
$fixture = Join-Path $root '3dm/fixtures/logo.3dm'
if ((Get-FileHash -LiteralPath $fixture -Algorithm SHA256).Hash -ne '854730269B0FCAEEC41CA29122936C05ADCC2B94BAD678C4C72E7480CF865FCC') { throw 'Official Logo fixture hash mismatch.' }
& cargo build --release --manifest-path (Join-Path $root '../Cargo.toml') --bin breplot
if ($LASTEXITCODE -ne 0) { throw 'Pure Rust reader build failed.' }
$exe = Join-Path $root '../target/release/breplot.exe'
& $exe $fixture (Join-Path $out 'logo.pdf') shaded 2 2 0 0.15 0.01
if ($LASTEXITCODE -ne 0) { throw 'Logo shading failed.' }
& $exe $fixture (Join-Path $out 'logo-hidden.svg') hidden 2 2 0 0.15 0.01
if ($LASTEXITCODE -ne 0) { throw 'Logo hidden-line view failed.' }
# The compact article embeds a raster preview; the full Type 4 PDF is separate.
& pdftoppm -scale-to 1600 -singlefile -png (Join-Path $out 'logo.pdf') (Join-Path $out 'logo')
if ($LASTEXITCODE -ne 0) { throw 'PDF preview rendering failed (Poppler required).' }
& typst compile --root $root (Join-Path $root '3dm/pure/main.typ') (Join-Path $out 'main.pdf')
if ($LASTEXITCODE -ne 0) { throw 'Pure Rust 3dm article failed.' }
Write-Output "Built $out/main.pdf and $out/logo.pdf without Python, .NET or native 3dm DLL."
