param(
    [string]$PackagePath = $(if ($env:TYPST_PACKAGE_PATH) { $env:TYPST_PACKAGE_PATH } else { Join-Path $env:APPDATA 'typst\packages' })
)
$ErrorActionPreference = 'Stop'
$source = Join-Path $PSScriptRoot '..\package'
$files = @('typst.toml', 'lib.typ', 'evaluation.typ', 'cetz_nurbs.wasm')
foreach ($file in $files) {
    if (-not (Test-Path -LiteralPath (Join-Path $source $file) -PathType Leaf)) {
        throw "Missing $file. Run cetz-nurbs\scripts\build.ps1 first."
    }
}
$destination = Join-Path $PackagePath 'local\cetz-nurbs\0.1.0'
New-Item -ItemType Directory -Force -Path $destination | Out-Null
foreach ($file in $files) {
    Copy-Item -LiteralPath (Join-Path $source $file) -Destination (Join-Path $destination $file) -Force
    $expected = (Get-FileHash -LiteralPath (Join-Path $source $file) -Algorithm SHA256).Hash
    $actual = (Get-FileHash -LiteralPath (Join-Path $destination $file) -Algorithm SHA256).Hash
    if ($expected -ne $actual) { throw "Installed package hash mismatch: $file" }
}
Write-Output "Installed @local/cetz-nurbs:0.1.0 to $destination"
