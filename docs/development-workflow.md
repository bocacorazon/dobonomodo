# DobONoMoDo — Agent-Driven Development Workflow

This document describes the automated, parallel development workflow used to implement DobONoMoDo specs. The system uses headless Copilot CLI agents running inside isolated devcontainers, orchestrated by a bash script that respects the spec dependency graph.

## Overview

```
┌──────────────┐     ┌──────────────┐     ┌──────────────┐     ┌──────────────┐
│  Spec Author │────▶│ Orchestrator │────▶│   Agents     │────▶│  Phase Gate  │
│  (human)     │     │  (bash)      │     │  (parallel)  │     │  (merge)     │
└──────────────┘     └──────────────┘     └──────────────┘     └──────────────┘
                            │                    │                     │
                     reads spec-map       one devcontainer        merges to
                     creates worktrees    per spec, runs          develop,
                     launches containers  full pipeline           quality gates
```

The workflow is divided into two halves:

1. **Human-driven** — finalize specs (prompts) and trigger the orchestrator.
2. **Agent-driven** — everything from spec elaboration through implementation, review, and fix cycles runs autonomously.

## Workflow Stages

### Stage 1: Spec Preparation (Human)

The human author writes or refines `prompt.md` files for each spec under `docs/specs/S##-<name>/prompt.md`. Each prompt defines what to build, references architecture docs, and states scope boundaries.

Once prompts are ready, the human triggers the orchestrator:

```bash
# Run all phases end-to-end
scripts/orchestrate.sh

# Run a single phase
scripts/orchestrate.sh --phase 0-1

# Run a single spec (for testing)
scripts/orchestrate.sh --spec S01

# Preview without executing
scripts/orchestrate.sh --dry-run
```

### Stage 2: Orchestration

The orchestrator (`scripts/orchestrate.sh`) reads `scripts/spec-map.toml` — a configuration file that maps every spec to a phase, a branch name, and its dependencies.

**What the orchestrator does for each phase:**

1. Verifies all dependency phases are complete.
2. For each spec in the phase (in parallel, up to `--max-parallel`):
   a. Creates a **git worktree** — an isolated copy of the repo on the spec's feature branch (`scripts/worktree-create.sh`).
   b. Launches a **devcontainer** — a Docker container with the Rust toolchain, gh CLI, Node.js, and Copilot CLI (`scripts/dc-launch.sh`).
   c. Starts the **agent runner** inside the container (`scripts/agent-run.sh`).
3. Polls for completion by watching `.agent-status` files in each worktree.
4. When all specs in the phase finish, performs a **phase-gate merge** (see Stage 6).

### Stage 3: Spec-Kit Pipeline (Agent, per spec)

Inside each devcontainer, `agent-run.sh` drives the full Spec-Kit pipeline:

```
speckit.specify  →  speckit.plan  →  speckit.tasks  →  speckit.implement
```

- **specify**: Elaborates `prompt.md` into a full `spec.md` (what to build, acceptance criteria).
- **plan**: Produces `plan.md` (technical design, architecture decisions).
- **tasks**: Produces `tasks.md` (ordered, actionable implementation steps).
- **implement**: Executes every task — writes code following TDD (red → green → refactor).

Each stage is skipped if its output file already exists. If a stage fails, it retries once before marking the spec as `FAILED`.

### Stage 4: Quality Gates (Agent, per spec)

After implementation, the agent runs four quality checks:

| Gate | Command | Failure action |
|---|---|---|
| Build | `cargo build --workspace` | Auto-fix via Copilot, retry |
| Tests | `cargo test --workspace` | Auto-fix via Copilot, retry |
| Lint | `cargo clippy --workspace -- -D warnings` | Auto-fix via Copilot, retry |
| Format | `cargo fmt --all --check` | Auto-fix via Copilot, retry |

If gates fail after an auto-fix attempt, the spec is marked `FAILED`.

### Stage 5: Code Review + Fix Cycle (Agent, per spec)

After quality gates pass, the agent runs a **code review**:

1. Generates a diff of the feature branch against `develop`.
2. Prompts Copilot with a structured review template (`scripts/review-prompt-template.md`) that checks for:
   - Spec compliance
   - Bugs, logic errors, security issues
   - Trait contract violations (IO boundary traits)
   - TDD discipline (tests exist for all functionality)
   - Correct Polars lazy API usage
3. Findings are classified as **CRITICAL**, **IMPORTANT**, or **MINOR**.

**Fix cycle** (up to 3 rounds):

```
Review finds issues → Agent fixes CRITICAL + IMPORTANT findings
                    → Re-runs quality gates
                    → Re-runs review
                    → Repeat if CRITICAL findings remain
```

**Outcomes:**
- All findings resolved → status: `SUCCESS`
- No CRITICAL findings but IMPORTANT/MINOR remain → status: `SUCCESS` (logged for reference)
- CRITICAL findings persist after 3 rounds → status: `NEEDS_HUMAN_REVIEW`

### Stage 6: Phase-Gate Merge (Orchestrator)

When all specs in a phase complete, the orchestrator merges them to `develop`:

1. Tags `develop` as `pre-phase-<N>-merge` (rollback point).
2. Merges each spec branch into `develop` with `--no-ff` (preserves branch history).
3. Runs full quality gates on the merged `develop`.
4. Runs a **cross-spec integration review** (`scripts/phase-gate-review.sh`):
   - Reviews the combined diff of all specs in the phase.
   - Checks for conflicting trait implementations, duplicated logic, inconsistent error handling, missing integration.
   - If CRITICAL issues found: creates a fix branch, auto-fixes, re-gates, merges.
5. Tags `develop` as `phase-<N>-complete`.

**Merge conflicts** or **quality gate failures** after merge halt the orchestrator and require human intervention.

### Stage 7: Human Checkpoints

The workflow pauses for human review in these situations:

| Situation | What happens |
|---|---|
| Spec status: `NEEDS_HUMAN_REVIEW` | Orchestrator pauses at phase gate, presents review findings |
| Merge conflict | Orchestrator halts, reports conflicting files |
| Quality gate failure after merge | Orchestrator halts, shows test/build errors |
| Agent escalation (`.agent-escalation`) | Agent marked `BLOCKED`, orchestrator halts |

After resolving, re-run the orchestrator to continue from where it stopped.

## Parallelism Plan

The 22 specs are organized into 7 phases based on their dependency graph:

```
Phase 0:   S00 (scaffold)
Phase 0→1: S01, S02, S16, S17          (4 parallel)
Phase 0→2: S03                          (1 sequential)
Phase 1:   S04–S09, S11                 (7 parallel — max parallelism)
Phase 2:   S10, S13                     (2 parallel)
Phase 3:   S12, S14, S15                (3 parallel)
Phase 4:   S18                          (1 sequential)
Phase 5:   S19, S20                     (2 parallel)
Phase 6:   S21                          (1 sequential)
```

Maximum parallelism: **8 containers** during Phase 1 (7 ops + carryover).

## Environment Isolation

Each spec gets its own isolated environment:

| Layer | Mechanism | Purpose |
|---|---|---|
| Git | Worktree at `../.worktrees/dobonomodo-<spec>/` | Branch isolation, no interference |
| Build | `CARGO_TARGET_DIR=<worktree>/.cargo-target` | No lock contention between builds |
| Runtime | Docker container (`dobonomodo-dev`) | Full process isolation |
| Cargo cache | Shared Docker volumes (`dobonomodo-cargo-registry`, `dobonomodo-cargo-git`) | Avoid re-downloading crates |

## Agent Output Protocol

Each agent writes status files to its worktree root:

| File | Content |
|---|---|
| `.agent-status` | `SUCCESS`, `FAILED`, `BLOCKED`, `NEEDS_HUMAN_REVIEW`, or `RUNNING` |
| `.agent-log` | Full execution log (timestamped) |
| `.agent-review` | Latest code review findings |
| `.agent-review-history/` | All review rounds (`round-1.md`, `round-2.md`, ...) |
| `.agent-escalation` | Human-readable description of the blocker (only if `BLOCKED`) |

All these files are gitignored.

## Scripts Reference

| Script | Purpose |
|---|---|
| `scripts/orchestrate.sh` | Main entry point — manages phases, agents, merges |
| `scripts/agent-run.sh` | Per-spec pipeline runner (runs inside container) |
| `scripts/worktree-create.sh` | Creates a git worktree for a spec |
| `scripts/worktree-destroy.sh` | Removes a spec's worktree |
| `scripts/worktree-list.sh` | Lists active worktrees with agent status |
| `scripts/dc-launch.sh` | Builds image and launches a devcontainer |
| `scripts/dc-stop.sh` | Stops and removes a spec's container |
| `scripts/phase-gate-review.sh` | Cross-spec integration review at phase boundaries |
| `scripts/spec-map.toml` | Spec → branch → phase → dependency mapping |
| `scripts/review-prompt-template.md` | Structured code review prompt |

## Escalation Policy

Per the project constitution's **completion bias** principle:

- Agents should resolve ambiguity themselves, making reasonable decisions consistent with the spec and architecture.
- Only **CRITICAL** blockers that genuinely cannot be resolved (e.g., contradictory spec requirements, missing upstream dependencies) trigger escalation.
- Decisions made autonomously are documented in commit messages.
- Style, naming, and formatting disagreements are never escalated — tooling (rustfmt, clippy) is authoritative.
