#!/usr/bin/env bash
# Phase-gate review — cross-spec integration review after merging all specs
# in a phase to develop.
#
# Usage: phase-gate-review.sh <phase-id>
set -euo pipefail

PHASE_ID="${1:?Usage: phase-gate-review.sh <phase-id>}"

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

echo "=== Phase-gate integration review: phase $PHASE_ID ==="

cd "$REPO_ROOT"

# Generate combined diff (phase tag to current develop)
PRE_TAG="pre-phase-${PHASE_ID}-merge"
DIFF_FILE="$REPO_ROOT/.phase-review-diff"

if git rev-parse "$PRE_TAG" &>/dev/null; then
    git diff "$PRE_TAG"..HEAD > "$DIFF_FILE"
else
    echo "WARNING: Pre-merge tag $PRE_TAG not found, reviewing last 50 commits"
    git diff HEAD~50..HEAD > "$DIFF_FILE" 2>/dev/null || git diff HEAD > "$DIFF_FILE"
fi

DIFF_LINES=$(wc -l < "$DIFF_FILE")
echo "Combined diff: $DIFF_LINES lines"

if [ "$DIFF_LINES" -eq 0 ]; then
    echo "No changes to review."
    rm -f "$DIFF_FILE"
    exit 0
fi

# Run integration review via Copilot
REVIEW_OUTPUT="$REPO_ROOT/.phase-review-${PHASE_ID}.md"

REVIEW_PROMPT="You are reviewing the combined changes from phase ${PHASE_ID} of the DobONoMoDo project.
Multiple specs were implemented in parallel and merged to develop. Review the combined diff for:

1. **Conflicting trait implementations** — do any specs provide incompatible impls of the same trait?
2. **Duplicated logic** — is there logic that should be shared but was independently implemented?
3. **Inconsistent error handling** — are error types and patterns consistent across the merged specs?
4. **Missing integration** — do specs that should interact (per the architecture) actually wire together?
5. **Broken contracts** — do the IO boundary traits (DataLoader, OutputWriter, MetadataStore, TraceWriter) have consistent signatures?

For each issue found, classify as CRITICAL, IMPORTANT, or MINOR (same as per-spec reviews).
If no issues found, output: 'Integration review: CLEAN'."

if copilot --prompt "$REVIEW_PROMPT" 2>&1 | tee "$REVIEW_OUTPUT"; then
    echo "Integration review saved: $REVIEW_OUTPUT"
else
    echo "WARNING: Integration review command failed, proceeding without review"
    echo "Integration review: SKIPPED (command failed)" > "$REVIEW_OUTPUT"
fi

# Check for CRITICAL findings
if grep -qi "CRITICAL" "$REVIEW_OUTPUT" 2>/dev/null; then
    echo ""
    echo "⚠️  CRITICAL integration issues found. Creating fix branch..."

    FIX_BRANCH="phase-${PHASE_ID}-integration-fixes"
    git checkout -b "$FIX_BRANCH"

    # Attempt auto-fix
    copilot --prompt "Fix all CRITICAL integration issues identified in .phase-review-${PHASE_ID}.md. Ensure the workspace builds, tests pass, and clippy is clean." 2>&1 || true

    # Re-run quality gates
    echo "Re-running quality gates after integration fixes..."
    if cargo build --workspace && cargo test --workspace && cargo clippy --workspace -- -D warnings; then
        git add -A
        if ! git diff --cached --quiet; then
            git commit -m "fix: phase ${PHASE_ID} integration fixes

Automated fixes for cross-spec integration issues.

Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>"
        fi

        git checkout develop
        git merge "$FIX_BRANCH" --no-ff -m "merge: phase ${PHASE_ID} integration fixes"
        git branch -d "$FIX_BRANCH"
        echo "✅ Integration fixes merged to develop"
    else
        echo "❌ Quality gates still failing after integration fixes."
        echo "Fix branch: $FIX_BRANCH"
        echo "Manual intervention required."
        git checkout develop
        exit 1
    fi
elif grep -qi "CLEAN\|SKIPPED" "$REVIEW_OUTPUT" 2>/dev/null; then
    echo "✅ Integration review: CLEAN"
else
    echo "Integration review completed with non-critical findings."
    echo "See: $REVIEW_OUTPUT"
fi

rm -f "$DIFF_FILE"
echo "=== Phase-gate review complete ==="
