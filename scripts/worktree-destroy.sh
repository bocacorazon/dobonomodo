#!/usr/bin/env bash
# Remove a git worktree for a spec.
# Usage: worktree-destroy.sh <spec-id>
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
WORKTREE_BASE="$(dirname "$REPO_ROOT")/.worktrees"

SPEC_ID="${1:?Usage: worktree-destroy.sh <spec-id>}"
WORKTREE_DIR="$WORKTREE_BASE/dobonomodo-${SPEC_ID}"

if [ ! -d "$WORKTREE_DIR" ]; then
    echo "Worktree does not exist: $WORKTREE_DIR"
    exit 0
fi

echo "Removing worktree: $WORKTREE_DIR"
git -C "$REPO_ROOT" worktree remove "$WORKTREE_DIR" --force
git -C "$REPO_ROOT" worktree prune

echo "Worktree removed: $SPEC_ID"
