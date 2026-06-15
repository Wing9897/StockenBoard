# Smoke test: build the server binary without Tauri system dependencies.
# This script verifies that the server target compiles cleanly using only
# the "server" feature flag (no desktop/Tauri crates required).
#
# Usage (from repo root):
#   powershell -ExecutionPolicy Bypass -File src-tauri/scripts/build-server.ps1

$ErrorActionPreference = "Stop"

$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$SrcTauriDir = Split-Path -Parent $ScriptDir

Write-Host "[build-server] Building server binary (no Tauri dependencies)..."
Write-Host "[build-server] Working directory: $SrcTauriDir"

Push-Location $SrcTauriDir
try {
    cargo build --bin server --no-default-features --features server
    if ($LASTEXITCODE -ne 0) {
        Write-Error "[build-server] ✗ Build failed with exit code $LASTEXITCODE"
        exit $LASTEXITCODE
    }
    Write-Host "[build-server] ✓ Server binary compiled successfully."
} finally {
    Pop-Location
}
