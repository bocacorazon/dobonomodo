#!/usr/bin/env bash
# List active git worktrees with their spec status.
# Usage: worktree-list.sh
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

printf "%-12s %-30s %-20s %-15s\n" "SPEC" "WORKTREE" "BRANCH" "AGENT STATUS"
printf "%-12s %-30s %-20s %-15s\n" "----" "--------" "------" "------------"

git -C "$REPO_ROOT" worktree list --porcelain | while read -r line; do
    case "$line" in
        "worktree "*)
            WORKTREE_PATH="${line#worktree }"
            ;;
        "branch "*)
            BRANCH="${line#branch refs/heads/}"
            ;;
        "")
            if [[ "$WORKTREE_PATH" == *".worktrees/dobonomodo-"* ]]; then
                SPEC_ID=$(basename "$WORKTREE_PATH" | sed 's/dobonomodo-//')
                STATUS="—"
                if [ -f "$WORKTREE_PATH/.agent-status" ]; then
                    STATUS=$(cat "$WORKTREE_PATH/.agent-status")
                fi
                printf "%-12s %-30s %-20s %-15s\n" "$SPEC_ID" "$WORKTREE_PATH" "$BRANCH" "$STATUS"
            fi
            WORKTREE_PATH=""
            BRANCH=""
            ;;
    esac
done
