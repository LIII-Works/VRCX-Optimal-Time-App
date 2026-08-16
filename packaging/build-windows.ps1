[CmdletBinding()]
param(
    [string]$Configuration = "release"
)

$ErrorActionPreference = "Stop"
$projectRoot = Split-Path -Parent $PSScriptRoot
Set-Location $projectRoot

$target = "x86_64-pc-windows-msvc"
$targetList = & rustup target list --installed
if ($LASTEXITCODE -ne 0 -or $targetList -notcontains $target) {
    throw "Rust target $target is not installed. Run: rustup target add $target"
}

cargo fmt --check
if ($LASTEXITCODE -ne 0) { throw "cargo fmt --check failed" }
cargo test
if ($LASTEXITCODE -ne 0) { throw "cargo test failed" }
cargo clippy --all-targets --all-features -- -D warnings
if ($LASTEXITCODE -ne 0) { throw "cargo clippy failed" }
cargo build --release --target $target
if ($LASTEXITCODE -ne 0) { throw "cargo build failed" }

$dist = Join-Path $projectRoot "dist"
New-Item -ItemType Directory -Force -Path $dist | Out-Null
$binary = Join-Path $projectRoot "target\$target\$Configuration\vrcx-optimal-time-app.exe"
$outputBinary = Join-Path $dist "VRCXOptimalTimeApp.exe"
Copy-Item -LiteralPath $binary -Destination $outputBinary -Force
Copy-Item -LiteralPath (Join-Path $projectRoot "README.md") -Destination $dist -Force
Copy-Item -LiteralPath (Join-Path $projectRoot "LICENSE") -Destination $dist -Force
Copy-Item -LiteralPath (Join-Path $projectRoot "THIRD_PARTY_NOTICES.txt") -Destination $dist -Force
Copy-Item -LiteralPath (Join-Path $projectRoot "src\analyzer\upstream_license.txt") -Destination $dist -Force

$hash = (Get-FileHash -Algorithm SHA256 -LiteralPath $outputBinary).Hash
"$hash  VRCXOptimalTimeApp.exe" | Set-Content -Encoding ASCII (Join-Path $dist "SHA256SUMS.txt")
Write-Host "Built $outputBinary"
Write-Host "Version $((Get-Item $outputBinary).VersionInfo.ProductVersion)"
