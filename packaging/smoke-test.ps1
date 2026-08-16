[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [string]$FixturePath,
    [string]$DistPath
)

$ErrorActionPreference = "Stop"
if ([string]::IsNullOrWhiteSpace($DistPath)) {
    $DistPath = Join-Path (Split-Path -Parent $PSScriptRoot) "dist"
}
$executable = Join-Path $DistPath "VRCXOptimalTimeApp.exe"
$manifest = Join-Path $DistPath "SHA256SUMS.txt"

if (-not (Test-Path -LiteralPath $executable -PathType Leaf)) {
    throw "Missing packaged executable: $executable"
}
if ([IO.Path]::GetFileName($executable) -cne "VRCXOptimalTimeApp.exe") {
    throw "Packaged executable has the wrong name"
}
if (-not (Test-Path -LiteralPath $manifest -PathType Leaf)) {
    throw "Missing SHA-256 manifest: $manifest"
}
$hash = (Get-FileHash -Algorithm SHA256 -LiteralPath $executable).Hash
$manifestEntry = Get-Content -LiteralPath $manifest | Where-Object { $_ -match "\sVRCXOptimalTimeApp\.exe$" }
if ($manifestEntry -ne "$hash  VRCXOptimalTimeApp.exe") {
    throw "SHA-256 manifest does not match VRCXOptimalTimeApp.exe"
}

& $executable --self-test (Resolve-Path -LiteralPath $FixturePath)
if ($LASTEXITCODE -ne 0) {
    throw "Packaged self-test failed with exit code $LASTEXITCODE"
}
Write-Host "Smoke test passed: $executable"
