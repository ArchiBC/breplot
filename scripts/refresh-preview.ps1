$ErrorActionPreference = 'Stop'

$root = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
$rust = Join-Path $root 'rust'
$cache = Join-Path $root 'target\preview-step.bin'
$model = Join-Path $root 'testdemo\demo.stp'
$config = Join-Path $root 'examples\step-preview.json'

if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
    $portable = Join-Path $env:TEMP 'breplot-rust'
    $cargoBin = Join-Path $portable 'cargo\bin'
    if (Test-Path (Join-Path $cargoBin 'cargo.exe')) {
        $env:RUSTUP_HOME = Join-Path $portable 'rustup'
        $env:CARGO_HOME = Join-Path $portable 'cargo'
        $env:PATH = $cargoBin + ';' + $env:PATH
    } else {
        throw 'cargo not found. Install Rust 1.96.0 first.'
    }
}

New-Item -ItemType Directory -Force -Path (Join-Path $root 'target') | Out-Null
& cargo build --manifest-path (Join-Path $rust 'Cargo.toml') --release --bin breplot-cli
if ($LASTEXITCODE -ne 0) { throw 'Native cache tool build failed.' }

$temporary = Join-Path $root 'target\preview-step.tmp'
try {
    & (Join-Path $rust 'target\release\breplot-cli.exe') cache $model $temporary $config
    if ($LASTEXITCODE -ne 0) { throw 'STEP preview cache generation failed.' }
    Move-Item -LiteralPath $temporary -Destination $cache -Force
} finally {
    if (Test-Path -LiteralPath $temporary) {
        Remove-Item -LiteralPath $temporary -Force
    }
}
Write-Output "Updated $cache"
