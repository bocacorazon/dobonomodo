#!/usr/bin/env bash
# Launch a devcontainer for a spec agent.
# Usage: dc-launch.sh <spec-id> [<worktree-path>]
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
WORKTREE_BASE="$(dirname "$REPO_ROOT")/.worktrees"

SPEC_ID="${1:?Usage: dc-launch.sh <spec-id> [<worktree-path>]}"
WORKTREE_DIR="${2:-$WORKTREE_BASE/dobonomodo-${SPEC_ID}}"

IMAGE_NAME="dobonomodo-dev"
CONTAINER_NAME="dobonomodo-${SPEC_ID}"

if [ ! -d "$WORKTREE_DIR" ]; then
    echo "ERROR: Worktree does not exist: $WORKTREE_DIR" >&2
    echo "Run: scripts/worktree-create.sh $SPEC_ID" >&2
    exit 1
fi

# Read env vars from worktree
BRANCH=""
CARGO_TARGET=""
if [ -f "$WORKTREE_DIR/.env.agent" ]; then
    # shellcheck source=/dev/null
    source "$WORKTREE_DIR/.env.agent"
    BRANCH="${SPECIFY_FEATURE:-}"
    CARGO_TARGET="${CARGO_TARGET_DIR:-/workspace/.cargo-target}"
fi

# Build image if not cached
if ! docker image inspect "$IMAGE_NAME" &>/dev/null; then
    echo "Building devcontainer image: $IMAGE_NAME..."
    docker build -t "$IMAGE_NAME" -f "$REPO_ROOT/.devcontainer/Dockerfile" "$REPO_ROOT"
fi

# Stop existing container if running
if docker ps -a --format '{{.Names}}' | grep -q "^${CONTAINER_NAME}$"; then
    echo "Stopping existing container: $CONTAINER_NAME"
    docker rm -f "$CONTAINER_NAME" &>/dev/null || true
fi

echo "Launching container: $CONTAINER_NAME"
echo "  Worktree: $WORKTREE_DIR"
echo "  Branch: $BRANCH"
echo "  Spec: $SPEC_ID"

# Locate host copilot binary
COPILOT_BIN="$(command -v copilot 2>/dev/null || echo "")"
COPILOT_MOUNT=""
if [ -n "$COPILOT_BIN" ] && [ -f "$COPILOT_BIN" ]; then
    COPILOT_MOUNT="-v $COPILOT_BIN:/usr/local/bin/copilot:ro"
    echo "  Copilot binary: $COPILOT_BIN (mounted into container)"
else
    echo "  WARNING: copilot binary not found on host"
fi

# Forward host copilot config if it exists
COPILOT_CONFIG_MOUNT=""
COPILOT_CONFIG_DIR="${HOME}/.config/github-copilot"
if [ -d "$COPILOT_CONFIG_DIR" ]; then
    COPILOT_CONFIG_MOUNT="-v $COPILOT_CONFIG_DIR:/home/agent/.config/github-copilot:ro"
fi

# Forward gh auth if it exists
GH_CONFIG_MOUNT=""
GH_CONFIG_DIR="${HOME}/.config/gh"
if [ -d "$GH_CONFIG_DIR" ]; then
    GH_CONFIG_MOUNT="-v $GH_CONFIG_DIR:/home/agent/.config/gh:ro"
fi

# shellcheck disable=SC2086
docker run -d \
    --name "$CONTAINER_NAME" \
    --network host \
    -v "$WORKTREE_DIR:/workspace" \
    -v dobonomodo-cargo-registry:/home/agent/.cargo/registry \
    -v dobonomodo-cargo-git:/home/agent/.cargo/git \
    $COPILOT_MOUNT \
    $COPILOT_CONFIG_MOUNT \
    $GH_CONFIG_MOUNT \
    -e "SPECIFY_FEATURE=${BRANCH}" \
    -e "CARGO_TARGET_DIR=${CARGO_TARGET}" \
    -e "GITHUB_TOKEN=${GITHUB_TOKEN:-}" \
    -e "GH_TOKEN=${GH_TOKEN:-}" \
    -e "COPILOT_TOKEN=${COPILOT_TOKEN:-}" \
    "$IMAGE_NAME" \
    bash -c "cd /workspace && bash scripts/agent-run.sh $SPEC_ID"

echo "Container $CONTAINER_NAME started"
echo "  Logs: docker logs -f $CONTAINER_NAME"
echo "  Status: cat $WORKTREE_DIR/.agent-status"
