#!/usr/bin/env bash
# Agent execution wrapper — runs the full spec-kit pipeline for a single spec
# inside a devcontainer. Includes implementation, quality gates, code review,
# and automated fix cycles.
#
# Usage: agent-run.sh <spec-id>
# Env:   SPECIFY_FEATURE (branch name), CARGO_TARGET_DIR
set -euo pipefail

SPEC_ID="${1:?Usage: agent-run.sh <spec-id>}"
MAX_REVIEW_ROUNDS="${MAX_REVIEW_ROUNDS:-3}"

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

LOG_FILE="$REPO_ROOT/.agent-log"
STATUS_FILE="$REPO_ROOT/.agent-status"
REVIEW_FILE="$REPO_ROOT/.agent-review"
REVIEW_HISTORY="$REPO_ROOT/.agent-review-history"
ESCALATION_FILE="$REPO_ROOT/.agent-escalation"

mkdir -p "$REVIEW_HISTORY"

# Logging helpers
log() { echo "[$(date +%H:%M:%S)] $*" | tee -a "$LOG_FILE"; }
set_status() { echo "$1" > "$STATUS_FILE"; log "STATUS: $1"; }

set_status "RUNNING"
log "=== Agent run for $SPEC_ID ==="
log "Branch: ${SPECIFY_FEATURE:-unknown}"
log "Repo root: $REPO_ROOT"

# --- Step 1: Validate preconditions ---
log "Step 1: Validating preconditions..."

BRANCH="${SPECIFY_FEATURE:-$(git rev-parse --abbrev-ref HEAD)}"
if [[ ! "$BRANCH" =~ ^[0-9]{3}- ]]; then
    log "ERROR: Not on a feature branch: $BRANCH"
    set_status "FAILED"
    exit 1
fi

DOC_SPEC_DIR=""
if [ -d "$REPO_ROOT/docs/specs" ]; then
    DOC_SPEC_DIR="$(find "$REPO_ROOT/docs/specs" -maxdepth 1 -name "${SPEC_ID}-*" -type d 2>/dev/null | head -1 || true)"
fi

find_feature_dir() {
    local dir=""
    local prefix="${BRANCH%%-*}"

    if [ -d "$REPO_ROOT/specs" ]; then
        # Prefer exact branch match first.
        dir="$(find "$REPO_ROOT/specs" -maxdepth 1 -name "${BRANCH}" -type d 2>/dev/null | head -1 || true)"
        if [ -z "$dir" ]; then
            # Fallback to numeric prefix match (supports 004-fix-* style branches).
            dir="$(find "$REPO_ROOT/specs" -maxdepth 1 -name "${prefix}-*" -type d 2>/dev/null | head -1 || true)"
        fi
    fi

    echo "$dir"
}

refresh_artifact_paths() {
    ARTIFACT_DIR="$(find_feature_dir)"
    if [ -z "$ARTIFACT_DIR" ]; then
        ARTIFACT_DIR="$DOC_SPEC_DIR"
    fi
    if [ -z "$ARTIFACT_DIR" ]; then
        ARTIFACT_DIR="$REPO_ROOT/specs/$BRANCH"
    fi

    FEATURE_SPEC="$ARTIFACT_DIR/spec.md"
    IMPL_PLAN="$ARTIFACT_DIR/plan.md"
    TASKS="$ARTIFACT_DIR/tasks.md"
}

refresh_artifact_paths
log "Artifact directory: $ARTIFACT_DIR"

# --- Step 2: Run spec-kit pipeline ---
log "Step 2: Running spec-kit pipeline..."

run_speckit_stage() {
    local stage="$1"
    local check_file="$2"

    if [ -n "$check_file" ] && [ -f "$check_file" ]; then
        log "  $stage: already exists ($check_file), skipping"
        return 0
    fi

    log "  $stage: running..."
    if copilot -p "Run the $stage workflow for this feature." --agent "$stage" --yolo --no-ask-user 2>&1 | tee -a "$LOG_FILE"; then
        log "  $stage: complete"
        return 0
    else
        log "  $stage: FAILED, retrying once..."
        if copilot -p "Run the $stage workflow for this feature." --agent "$stage" --yolo --no-ask-user 2>&1 | tee -a "$LOG_FILE"; then
            log "  $stage: complete (retry)"
            return 0
        else
            log "  $stage: FAILED after retry"
            return 1
        fi
    fi
}

if ! run_speckit_stage "speckit.specify" "$FEATURE_SPEC"; then
    set_status "FAILED"
    exit 1
fi
refresh_artifact_paths

if ! run_speckit_stage "speckit.plan" "$IMPL_PLAN"; then
    set_status "FAILED"
    exit 1
fi
refresh_artifact_paths

if ! run_speckit_stage "speckit.tasks" "$TASKS"; then
    set_status "FAILED"
    exit 1
fi

log "  speckit.implement: running..."
if ! copilot -p "Execute the implementation plan by processing all tasks." --agent "speckit.implement" --yolo --no-ask-user 2>&1 | tee -a "$LOG_FILE"; then
    log "  speckit.implement: FAILED"
    set_status "FAILED"
    exit 1
fi
log "  speckit.implement: complete"

# --- Step 3: Quality gates ---
run_quality_gates() {
    log "Running quality gates..."
    local failed=0

    log "  cargo build..."
    if ! cargo build --workspace 2>&1 | tee -a "$LOG_FILE"; then
        log "  cargo build: FAILED"
        failed=1
    fi

    log "  cargo test..."
    if ! cargo test --workspace 2>&1 | tee -a "$LOG_FILE"; then
        log "  cargo test: FAILED"
        failed=1
    fi

    log "  cargo clippy..."
    if ! cargo clippy --workspace -- -D warnings 2>&1 | tee -a "$LOG_FILE"; then
        log "  cargo clippy: FAILED"
        failed=1
    fi

    log "  cargo fmt --check..."
    if ! cargo fmt --all --check 2>&1 | tee -a "$LOG_FILE"; then
        log "  cargo fmt: FAILED"
        failed=1
    fi

    return $failed
}

if ! run_quality_gates; then
    log "Quality gates failed, attempting auto-fix..."
    copilot -p "Fix all cargo build errors, test failures, clippy warnings, and formatting issues in this workspace. Run cargo fmt --all first." --yolo --no-ask-user 2>&1 | tee -a "$LOG_FILE" || true
    if ! run_quality_gates; then
        log "Quality gates still failing after auto-fix"
        set_status "FAILED"
        exit 1
    fi
fi

# --- Step 4: Code review ---
log "Step 4: Running code review..."

run_code_review() {
    local round="$1"
    log "  Code review round $round..."

    # Generate diff against develop
    local diff_file="$REPO_ROOT/.agent-diff"
    git diff develop..HEAD > "$diff_file" 2>/dev/null || git diff HEAD~10..HEAD > "$diff_file" 2>/dev/null || true

    # Run code review via Copilot
    local review_prompt
    review_prompt=$(cat "$SCRIPT_DIR/review-prompt-template.md" 2>/dev/null || echo "Review the code changes for bugs, security issues, and spec compliance.")
    review_prompt="${review_prompt//\{\{SPEC_ID\}\}/$SPEC_ID}"
    review_prompt="${review_prompt//\{\{BRANCH_NAME\}\}/$BRANCH}"

    copilot -p "$review_prompt" --yolo --no-ask-user 2>&1 | tee "$REVIEW_FILE" | tee -a "$LOG_FILE" || true
    cp "$REVIEW_FILE" "$REVIEW_HISTORY/round-${round}.md"

    rm -f "$diff_file"

    # Check for CRITICAL findings
    if grep -qi "CRITICAL" "$REVIEW_FILE" 2>/dev/null; then
        return 1  # has critical findings
    fi
    return 0  # clean or only minor/important
}

REVIEW_ROUND=1
REVIEW_CLEAN=false

while [ "$REVIEW_ROUND" -le "$MAX_REVIEW_ROUNDS" ]; do
    if run_code_review "$REVIEW_ROUND"; then
        REVIEW_CLEAN=true
        log "  Code review clean (round $REVIEW_ROUND)"
        break
    fi

    log "  Code review found CRITICAL issues (round $REVIEW_ROUND)"

    if [ "$REVIEW_ROUND" -lt "$MAX_REVIEW_ROUNDS" ]; then
        # --- Step 5: Fix cycle ---
        log "  Entering fix cycle..."
        copilot -p "Fix all CRITICAL and IMPORTANT issues identified in the code review. The review findings are in .agent-review. Address each finding and ensure quality gates still pass." --yolo --no-ask-user 2>&1 | tee -a "$LOG_FILE" || true

        # Re-run quality gates after fixes
        if ! run_quality_gates; then
            log "Quality gates failed after review fixes, attempting auto-fix..."
            copilot -p "Fix all cargo build errors, test failures, clippy warnings, and formatting issues." --yolo --no-ask-user 2>&1 | tee -a "$LOG_FILE" || true
            if ! run_quality_gates; then
                log "Quality gates still failing"
                set_status "FAILED"
                exit 1
            fi
        fi
    fi

    REVIEW_ROUND=$((REVIEW_ROUND + 1))
done

if [ "$REVIEW_CLEAN" = false ]; then
    log "CRITICAL findings persist after $MAX_REVIEW_ROUNDS review rounds"
    echo "CRITICAL code review findings persist after $MAX_REVIEW_ROUNDS automated fix rounds. Human review required." > "$ESCALATION_FILE"
    set_status "NEEDS_HUMAN_REVIEW"
else
    set_status "SUCCESS"
fi

# --- Step 6: Commit and push ---
log "Step 6: Committing and pushing..."

git add -A
if git diff --cached --quiet; then
    log "No changes to commit"
else
    git commit -m "feat($SPEC_ID): implement spec via agent

Automated implementation by agent-run.sh.
Spec: $SPEC_ID
Branch: $BRANCH

Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>"

    git push origin "$BRANCH" 2>&1 | tee -a "$LOG_FILE" || log "WARNING: push failed (may need manual push)"
fi

log "=== Agent run complete for $SPEC_ID ==="
