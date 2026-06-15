#!/usr/bin/env bash
# Smoke test: build the server binary without Tauri system dependencies.
# This script verifies that the server target compiles cleanly using only
# the "server" feature flag (no desktop/Tauri crates required).
#
# Usage:
#   ./src-tauri/scripts/build-server.sh
#
# In CI, run from the repository root:
#   bash src-tauri/scripts/build-server.sh

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SRC_TAURI_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"

echo "[build-server] Building server binary (no Tauri dependencies)..."
echo "[build-server] Working directory: $SRC_TAURI_DIR"

cd "$SRC_TAURI_DIR"

cargo build --bin server --no-default-features --features server

echo "[build-server] ✓ Server binary compiled successfully."
