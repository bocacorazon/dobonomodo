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

docker run -d \
    --name "$CONTAINER_NAME" \
    --network host \
    -v "$WORKTREE_DIR:/workspace" \
    -v dobonomodo-cargo-registry:/home/agent/.cargo/registry \
    -v dobonomodo-cargo-git:/home/agent/.cargo/git \
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
