#!/usr/bin/env bash
# Orchestrator — manages parallel agent execution with phase-gated merges.
#
# Usage:
#   orchestrate.sh                      # Run all phases sequentially
#   orchestrate.sh --phase 0-1          # Run only phase 0-1
#   orchestrate.sh --phase 1 --phase-gate-only  # Merge completed phase from existing statuses
#   orchestrate.sh --spec S01           # Run only spec S01
#   orchestrate.sh --dry-run            # Show what would be done
#   orchestrate.sh --max-parallel 4     # Limit concurrent containers
#
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
SPEC_MAP="$SCRIPT_DIR/spec-map.toml"
WORKTREE_BASE="$(dirname "$REPO_ROOT")/.worktrees"

# Defaults
DRY_RUN=false
TARGET_PHASE=""
TARGET_SPEC=""
MAX_PARALLEL=8
POLL_INTERVAL=30
PHASE_GATE_ONLY=false

# Parse args
while [[ $# -gt 0 ]]; do
    case "$1" in
        --phase) TARGET_PHASE="$2"; shift 2 ;;
        --spec) TARGET_SPEC="$2"; shift 2 ;;
        --dry-run) DRY_RUN=true; shift ;;
        --phase-gate-only) PHASE_GATE_ONLY=true; shift ;;
        --max-parallel) MAX_PARALLEL="$2"; shift 2 ;;
        --poll-interval) POLL_INTERVAL="$2"; shift 2 ;;
        *) echo "Unknown option: $1" >&2; exit 1 ;;
    esac
done

# --- TOML parser (minimal, handles our spec-map format) ---

# Get all phase IDs in order
get_phases() {
    grep '^\[phases\.' "$SPEC_MAP" | sed 's/\[phases\.\(.*\)\]/\1/' | sort -t. -k1,1n -k2,2
}

# Get specs for a phase
get_phase_specs() {
    local phase="$1"
    local in_section=false
    while IFS= read -r line; do
        if [[ "$line" == "[phases.${phase}]" ]]; then
            in_section=true
            continue
        fi
        if $in_section; then
            if [[ "$line" == "["* ]]; then
                break
            fi
            if [[ "$line" == specs* ]]; then
                echo "$line" | sed 's/specs *= *\[//; s/\]//; s/"//g; s/,/ /g; s/^ *//'
                return
            fi
        fi
    done < "$SPEC_MAP"
}

# Get dependencies for a phase
get_phase_deps() {
    local phase="$1"
    local in_section=false
    while IFS= read -r line; do
        if [[ "$line" == "[phases.${phase}]" ]]; then
            in_section=true
            continue
        fi
        if $in_section; then
            if [[ "$line" == "["* ]]; then
                break
            fi
            if [[ "$line" == depends_on* ]]; then
                echo "$line" | sed 's/depends_on *= *\[//; s/\]//; s/"//g; s/,/ /g; s/^ *//'
                return
            fi
        fi
    done < "$SPEC_MAP"
}

# Get phase description
get_phase_desc() {
    local phase="$1"
    local in_section=false
    while IFS= read -r line; do
        if [[ "$line" == "[phases.${phase}]" ]]; then
            in_section=true
            continue
        fi
        if $in_section; then
            if [[ "$line" == "["* ]]; then
                break
            fi
            if [[ "$line" == description* ]]; then
                echo "$line" | sed 's/description *= *"\(.*\)"/\1/'
                return
            fi
        fi
    done < "$SPEC_MAP"
}

# Get branch for a spec
get_spec_branch() {
    local spec="$1"
    local in_section=false
    while IFS= read -r line; do
        if [[ "$line" == "[specs.${spec}]" ]]; then
            in_section=true
            continue
        fi
        if $in_section; then
            if [[ "$line" == "["* ]]; then
                break
            fi
            if [[ "$line" == branch* ]]; then
                echo "$line" | sed 's/branch *= *"\(.*\)"/\1/'
                return
            fi
        fi
    done < "$SPEC_MAP"
}

# --- Status display ---

print_status_header() {
    printf "\n%-8s %-25s %-18s %-10s\n" "SPEC" "BRANCH" "STATUS" "ELAPSED"
    printf "%-8s %-25s %-18s %-10s\n" "----" "------" "------" "-------"
}

print_spec_status() {
    local spec="$1" branch="$2" status="$3" elapsed="$4"
    printf "%-8s %-25s %-18s %-10s\n" "$spec" "$branch" "$status" "$elapsed"
}

# --- Core logic ---

# Track completed phases
declare -A COMPLETED_PHASES

launch_spec() {
    local spec="$1"
    local branch
    branch=$(get_spec_branch "$spec")

    if $DRY_RUN; then
        echo "[DRY-RUN] Would launch: $spec (branch: ${branch:-auto})"
        return
    fi

    echo "Launching agent for $spec..."

    # Create worktree
    "$SCRIPT_DIR/worktree-create.sh" "$spec"

    # Launch devcontainer
    "$SCRIPT_DIR/dc-launch.sh" "$spec"
}

wait_for_specs() {
    local specs=("$@")
    local start_time
    start_time=$(date +%s)

    if $DRY_RUN; then
        echo "[DRY-RUN] Would wait for: ${specs[*]}"
        return 0
    fi

    echo ""
    echo "Waiting for ${#specs[@]} specs to complete..."
    print_status_header

    while true; do
        local all_done=true
        local any_failed=false
        local any_needs_review=false
        local now
        now=$(date +%s)

        for spec in "${specs[@]}"; do
            local worktree="$WORKTREE_BASE/dobonomodo-${spec}"
            local status="RUNNING"
            local branch
            branch=$(get_spec_branch "$spec")
            local elapsed="$((now - start_time))s"

            if [ -f "$worktree/.agent-status" ]; then
                status=$(cat "$worktree/.agent-status")
            fi

            print_spec_status "$spec" "${branch:-auto}" "$status" "$elapsed"

            case "$status" in
                SUCCESS) ;;
                NEEDS_HUMAN_REVIEW) any_needs_review=true ;;
                FAILED|BLOCKED) any_failed=true ;;
                *) all_done=false ;;
            esac
        done

        if $all_done; then
            echo ""
            if $any_failed; then
                echo "❌ Some specs FAILED. Check .agent-log in their worktrees."
                return 1
            fi
            if $any_needs_review; then
                echo "⚠️  Some specs need human review. Check .agent-review in their worktrees."
                echo "Review the findings, then re-run the orchestrator to continue."
                return 2
            fi
            echo "✅ All specs in phase completed successfully."
            return 0
        fi

        sleep "$POLL_INTERVAL"
        # Clear and reprint (simple approach)
        echo "---"
    done
}

validate_phase_ready_from_status() {
    local specs=("$@")
    local all_ready=true

    echo ""
    echo "Validating existing agent statuses for phase-gate merge..."
    printf "%-8s %-25s %-22s %-s\n" "SPEC" "WORKTREE" "STATUS" "DETAIL"
    printf "%-8s %-25s %-22s %-s\n" "----" "--------" "------" "------"

    for spec in "${specs[@]}"; do
        local worktree="$WORKTREE_BASE/dobonomodo-${spec}"
        local detail="ok"
        local status=""

        if [ ! -d "$worktree" ]; then
            status="MISSING_WORKTREE"
            detail="run this spec first"
            all_ready=false
            printf "%-8s %-25s %-22s %-s\n" "$spec" "$(basename "$worktree")" "$status" "$detail"
            continue
        fi

        if [ ! -f "$worktree/.agent-status" ]; then
            status="MISSING_STATUS"
            detail="no .agent-status in worktree"
            all_ready=false
            printf "%-8s %-25s %-22s %-s\n" "$spec" "$(basename "$worktree")" "$status" "$detail"
            continue
        fi

        status="$(cat "$worktree/.agent-status")"
        case "$status" in
            SUCCESS)
                detail="ready"
                ;;
            NEEDS_HUMAN_REVIEW)
                detail="resolve .agent-review first"
                all_ready=false
                ;;
            FAILED|BLOCKED)
                detail="rerun/fix this spec first"
                all_ready=false
                ;;
            RUNNING)
                detail="wait for completion"
                all_ready=false
                ;;
            *)
                detail="unexpected status"
                all_ready=false
                ;;
        esac

        printf "%-8s %-25s %-22s %-s\n" "$spec" "$(basename "$worktree")" "$status" "$detail"
    done

    if ! $all_ready; then
        echo ""
        echo "❌ Phase is not ready for phase-gate merge."
        return 1
    fi

    echo ""
    echo "✅ All specs are SUCCESS and ready to merge."
    return 0
}

phase_gate_merge() {
    local phase="$1"
    shift
    local specs=("$@")

    if $DRY_RUN; then
        echo "[DRY-RUN] Would merge specs to develop: ${specs[*]}"
        return 0
    fi

    echo ""
    echo "=== Phase-gate merge for phase $phase ==="

    # Tag pre-merge state
    git -C "$REPO_ROOT" tag "pre-phase-${phase}-merge" 2>/dev/null || true

    for spec in "${specs[@]}"; do
        local branch
        branch=$(get_spec_branch "$spec")
        if [ -z "$branch" ]; then
            local worktree="$WORKTREE_BASE/dobonomodo-${spec}"
            if [ -f "$worktree/.env.agent" ]; then
                # shellcheck source=/dev/null
                source "$worktree/.env.agent"
                branch="${SPECIFY_FEATURE:-}"
            fi
        fi

        if [ -z "$branch" ]; then
            echo "WARNING: No branch found for $spec, skipping merge"
            continue
        fi

        echo "Merging $spec ($branch) into develop..."
        git -C "$REPO_ROOT" checkout develop
        if ! git -C "$REPO_ROOT" merge "$branch" --no-ff -m "merge($spec): phase $phase gate merge"; then
            echo "❌ MERGE CONFLICT merging $spec ($branch). Resolve manually."
            return 1
        fi
    done

    # Run quality gates on merged develop
    echo "Running quality gates on merged develop..."
    cd "$REPO_ROOT"
    if ! cargo build --workspace 2>&1; then
        echo "❌ Build failed after phase merge"
        return 1
    fi
    if ! cargo test --workspace 2>&1; then
        echo "❌ Tests failed after phase merge"
        return 1
    fi
    if ! cargo clippy --workspace -- -D warnings 2>&1; then
        echo "❌ Clippy failed after phase merge"
        return 1
    fi

    # Run phase-gate review if script exists
    if [ -f "$SCRIPT_DIR/phase-gate-review.sh" ]; then
        "$SCRIPT_DIR/phase-gate-review.sh" "$phase"
    fi

    # Tag completion
    git -C "$REPO_ROOT" tag "phase-${phase}-complete"
    echo "✅ Phase $phase merge complete. Tagged: phase-${phase}-complete"
}

run_phase() {
    local phase="$1"
    local desc
    desc=$(get_phase_desc "$phase")
    local specs_str
    specs_str=$(get_phase_specs "$phase")
    local specs=($specs_str)

    echo ""
    echo "╔══════════════════════════════════════════════════════╗"
    echo "  Phase $phase: $desc"
    echo "  Specs: ${specs[*]}"
    echo "╚══════════════════════════════════════════════════════╝"

    # Check dependencies
    local deps_str
    deps_str=$(get_phase_deps "$phase")
    if [ -n "$deps_str" ]; then
        for dep in $deps_str; do
            if [ "${COMPLETED_PHASES[$dep]:-}" != "done" ]; then
                echo "ERROR: Dependency phase $dep not yet completed" >&2
                return 1
            fi
        done
    fi

    if $PHASE_GATE_ONLY; then
        if ! $DRY_RUN; then
            if ! validate_phase_ready_from_status "${specs[@]}"; then
                return 1
            fi
        else
            echo "[DRY-RUN] Would validate existing statuses for: ${specs[*]}"
        fi
    else
        # Limit concurrency
        local batch_size=$MAX_PARALLEL
        local i=0
        local batch=()

        for spec in "${specs[@]}"; do
            batch+=("$spec")
            i=$((i + 1))

            if [ $i -ge "$batch_size" ] || [ $i -eq ${#specs[@]} ]; then
                # Launch batch
                for s in "${batch[@]}"; do
                    launch_spec "$s"
                done

                # Wait for batch
                if ! wait_for_specs "${batch[@]}"; then
                    return 1
                fi

                batch=()
            fi
        done
    fi

    # Phase-gate merge
    if ! phase_gate_merge "$phase" "${specs[@]}"; then
        return 1
    fi

    COMPLETED_PHASES[$phase]="done"
    echo "✅ Phase $phase complete"
}

# --- Main ---

echo "DobONoMoDo Orchestrator"
echo "======================="
echo "Spec map: $SPEC_MAP"
echo "Max parallel: $MAX_PARALLEL"
echo "Dry run: $DRY_RUN"
echo "Phase-gate-only: $PHASE_GATE_ONLY"

if $PHASE_GATE_ONLY && [ -n "$TARGET_SPEC" ]; then
    echo "ERROR: --phase-gate-only cannot be used with --spec." >&2
    exit 1
fi
if $PHASE_GATE_ONLY && [ -z "$TARGET_PHASE" ]; then
    echo "ERROR: --phase-gate-only requires --phase <id>." >&2
    exit 1
fi

# Handle single spec mode
if [ -n "$TARGET_SPEC" ]; then
    echo "Running single spec: $TARGET_SPEC"
    launch_spec "$TARGET_SPEC"
    if ! $DRY_RUN; then
        wait_for_specs "$TARGET_SPEC"
    fi
    exit $?
fi

# Get phases to run
PHASES=($(get_phases))

if [ -n "$TARGET_PHASE" ]; then
    # Mark all dependency phases as completed (assume they were done previously)
    deps_str=$(get_phase_deps "$TARGET_PHASE")
    for dep in $deps_str; do
        COMPLETED_PHASES[$dep]="done"
    done
    run_phase "$TARGET_PHASE"
else
    # Run all phases in order
    for phase in "${PHASES[@]}"; do
        if ! run_phase "$phase"; then
            echo "❌ Phase $phase failed. Stopping."
            exit 1
        fi
    done
    echo ""
    echo "🎉 All phases complete!"
fi
