param([int]$Ppi = 192)
$ErrorActionPreference = 'Stop'
$moduleRoot = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
$target = Join-Path $moduleRoot 'target'
$display = Join-Path $target 'display'
New-Item -ItemType Directory -Force $display | Out-Null
# Only presentation changes: reuse the native SVG lighting, keep stroke geometry vector.
foreach ($file in Get-ChildItem -LiteralPath $target -Filter '*.svg') {
    if ($file.BaseName -eq 'mesh') { continue }
    [xml]$surface = Get-Content -LiteralPath $file.FullName -Raw
    $lines = $surface.Clone()
    $lineRoot = $lines.DocumentElement
    while ($lineRoot.HasChildNodes) { [void]$lineRoot.RemoveChild($lineRoot.FirstChild) }
    foreach ($node in @($surface.SelectNodes('//*[@stroke]'))) {
        [void]$lineRoot.AppendChild($lines.ImportNode($node, $true))
        [void]$node.ParentNode.RemoveChild($node)
    }
    $id = $file.BaseName
    $surface.Save((Join-Path $display "$id-surface.svg"))
    $lines.Save((Join-Path $display "$id-lines.svg"))
    $width = $surface.DocumentElement.GetAttribute('width')
    $height = $surface.DocumentElement.GetAttribute('height')
    $source = "#set page(width: ${width}pt, height: ${height}pt, margin: 0pt, fill: none)`n#image(`"$id-surface.svg`", width: 100%)"
    $typ = Join-Path $display "$id.typ"
    Set-Content -LiteralPath $typ -Value $source -Encoding utf8
    & typst compile --root $moduleRoot --ppi $Ppi $typ (Join-Path $display "$id.png")
    if ($LASTEXITCODE -ne 0) { throw "Display asset failed: $id" }
}
