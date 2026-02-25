#!/usr/bin/env bash
# Post-create setup for agent devcontainer
set -euo pipefail

echo "[post-create] Installing Copilot CLI extension..."
if command -v gh &>/dev/null; then
    gh extension install github/gh-copilot 2>/dev/null || true
    echo "[post-create] Copilot CLI installed"
else
    echo "[post-create] WARNING: gh CLI not found, skipping Copilot install"
fi

echo "[post-create] Warming cargo cache..."
if [ -f Cargo.toml ]; then
    cargo fetch --quiet 2>/dev/null || true
    echo "[post-create] Cargo cache warmed"
fi

echo "[post-create] Done"
