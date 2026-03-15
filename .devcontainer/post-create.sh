#!/usr/bin/env bash
# Post-create setup for agent devcontainer
set -euo pipefail

echo "[post-create] Installing standalone Copilot CLI..."
if command -v npm &>/dev/null; then
    npm install -g @github/copilot >/dev/null 2>&1 || true
    if command -v copilot &>/dev/null; then
        echo "[post-create] Copilot CLI installed"
    else
        echo "[post-create] WARNING: Could not install standalone Copilot CLI"
    fi
else
    echo "[post-create] WARNING: npm not found, skipping Copilot CLI install"
fi

echo "[post-create] Warming cargo cache..."
if [ -f Cargo.toml ]; then
    cargo fetch --quiet 2>/dev/null || true
    echo "[post-create] Cargo cache warmed"
fi

echo "[post-create] Done"
