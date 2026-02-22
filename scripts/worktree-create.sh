#!/usr/bin/env bash
# Create a git worktree for a spec, isolated from the main repo.
# Usage: worktree-create.sh <spec-id> [--base-branch <branch>]
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
WORKTREE_BASE="$(dirname "$REPO_ROOT")/.worktrees"
SPEC_MAP="$SCRIPT_DIR/spec-map.toml"

SPEC_ID="${1:?Usage: worktree-create.sh <spec-id>}"
BASE_BRANCH="${2:-develop}"

# Parse branch name from spec-map.toml
get_spec_branch() {
    local spec_id="$1"
    local branch
    branch=$(grep -A2 "^\[specs\.${spec_id}\]" "$SPEC_MAP" | grep '^branch' | sed 's/branch *= *"\(.*\)"/\1/')
    echo "$branch"
}

BRANCH=$(get_spec_branch "$SPEC_ID")
if [ -z "$BRANCH" ]; then
    # Auto-generate branch name from spec directory name
    SPEC_DIR=$(ls -d "$REPO_ROOT/docs/specs/${SPEC_ID}-"* 2>/dev/null | head -1)
    if [ -z "$SPEC_DIR" ]; then
        echo "ERROR: No spec directory found for $SPEC_ID" >&2
        exit 1
    fi
    SPEC_NAME=$(basename "$SPEC_DIR")
    # Extract number from spec name (e.g., S01 -> 002, using spec number + 1)
    SPEC_NUM=$(echo "$SPEC_ID" | sed 's/S//' | sed 's/^0*//')
    BRANCH=$(printf "%03d-%s" "$((SPEC_NUM + 1))" "$(echo "$SPEC_NAME" | sed "s/^${SPEC_ID}-//")")
fi

WORKTREE_DIR="$WORKTREE_BASE/dobonomodo-${SPEC_ID}"

if [ -d "$WORKTREE_DIR" ]; then
    echo "Worktree already exists: $WORKTREE_DIR"
    echo "WORKTREE_DIR=$WORKTREE_DIR"
    echo "BRANCH=$BRANCH"
    exit 0
fi

mkdir -p "$WORKTREE_BASE"

# Create branch if it doesn't exist
if ! git -C "$REPO_ROOT" rev-parse --verify "$BRANCH" &>/dev/null; then
    echo "Creating branch $BRANCH from $BASE_BRANCH..."
    git -C "$REPO_ROOT" branch "$BRANCH" "$BASE_BRANCH"
fi

# Create worktree
echo "Creating worktree at $WORKTREE_DIR on branch $BRANCH..."
git -C "$REPO_ROOT" worktree add "$WORKTREE_DIR" "$BRANCH"

# Set isolated cargo target dir
echo "CARGO_TARGET_DIR=$WORKTREE_DIR/.cargo-target" > "$WORKTREE_DIR/.env.agent"
echo "SPECIFY_FEATURE=$BRANCH" >> "$WORKTREE_DIR/.env.agent"

echo ""
echo "WORKTREE_DIR=$WORKTREE_DIR"
echo "BRANCH=$BRANCH"
echo "CARGO_TARGET_DIR=$WORKTREE_DIR/.cargo-target"
