$ErrorActionPreference = "Stop"

$Root = Split-Path -Parent $PSScriptRoot
Set-Location $Root

if (Test-Path Env:CARGO_TARGET_DIR) {
    Remove-Item Env:CARGO_TARGET_DIR
}

Write-Host "Running cargo test..."
cargo test
if ($LASTEXITCODE -ne 0) {
    throw "cargo test failed"
}

Write-Host "Building release launcher..."
cargo build --release --bin launcher
if ($LASTEXITCODE -ne 0) {
    throw "cargo build --release --bin launcher failed"
}

$ExeSource = Join-Path $Root "target\release\launcher.exe"
if (-not (Test-Path $ExeSource)) {
    throw "missing $ExeSource"
}

$ReleaseDir = Join-Path $Root "release"
New-Item -ItemType Directory -Force -Path $ReleaseDir | Out-Null

Get-ChildItem -Force $ReleaseDir | Remove-Item -Recurse -Force

Copy-Item $ExeSource (Join-Path $ReleaseDir "Launcher.exe")
Copy-Item (Join-Path $Root "packaging\.env.example") (Join-Path $ReleaseDir ".env.example")
Copy-Item (Join-Path $Root "packaging\README.txt") (Join-Path $ReleaseDir "README.txt")

$Forbidden = @(
    (Join-Path $Root ".env"),
    (Join-Path $Root "launcher.json")
)
foreach ($Path in $Forbidden) {
    $Name = Split-Path $Path -Leaf
    if (Test-Path (Join-Path $ReleaseDir $Name)) {
        throw "refusing to ship $Name in release/"
    }
}

$SecretHits = Get-ChildItem -File $ReleaseDir | Select-String -Pattern "sk-[A-Za-z0-9]{8,}|LLM_API_KEY=.+"
foreach ($Hit in $SecretHits) {
    if ($Hit.Line -match "LLM_API_KEY=\s*$") {
        continue
    }
    if ($Hit.Line -match "LLM_API_KEY=$") {
        continue
    }
    throw "possible secret in release file: $($Hit.Path):$($Hit.LineNumber)"
}

Write-Host "Portable release ready: $ReleaseDir"
Get-ChildItem $ReleaseDir | ForEach-Object { Write-Host ("  " + $_.Name) }
