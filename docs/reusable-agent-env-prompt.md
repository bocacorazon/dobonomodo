# Reusable Prompt: Build an Autonomous Parallel Agent Dev Environment

Use this prompt in another repository when you want the same workflow style used here: spec-first, phase-gated, parallelized implementation by headless agents with minimal human intervention.

## How to use

1. Copy the prompt block below.
2. Replace the `<PLACEHOLDERS>` with values for your target repository.
3. Paste into Copilot CLI/Claude Code (or equivalent coding agent) at the root of that repository.

---

## Prompt to copy

```text
You are a senior platform engineer. Set up an autonomous, parallel, spec-driven development environment in this repository.

## Context and goals

- Repository: <REPO_NAME>
- Default branch: <DEFAULT_BRANCH>
- Spec source directory: <SPEC_DIR>                # e.g., docs/specs
- Specs naming convention: <SPEC_PATTERN>          # e.g., S##-name
- Preferred agent runtime: GitHub Copilot CLI
- Container runtime: Docker
- Orchestration style: Bash scripts
- Merge strategy: phase-gate merge to <DEFAULT_BRANCH>
- Human intervention policy: only escalate on CRITICAL blockers

Goal: Create an environment where multiple specs can be implemented in parallel by headless agents, each running in an isolated worktree + container, with quality gates and automated code-review/fix cycles before merge.

## Non-negotiable behavior

1. Make minimal, surgical changes.
2. Follow existing repository conventions.
3. Do not ask for confirmation unless blocked by a critical ambiguity.
4. Use autonomous defaults and continue execution.
5. Preserve safety:
   - no destructive git operations
   - no secrets committed
   - explicit failures over silent fallback

## Deliverables (must create/update)

### A) Devcontainer

Create:
- `.devcontainer/Dockerfile`
- `.devcontainer/devcontainer.json`
- `.devcontainer/post-create.sh`

Requirements:
- Rust toolchain + clippy + rustfmt (if repo is Rust; if not, adapt to repo language/toolchain)
- `gh` CLI
- Node.js 20+ (for MCP/npx workflows)
- `git`, `curl`, and common build dependencies
- Shared cache volumes for dependency downloads
- Default `CARGO_TARGET_DIR=/workspace/.cargo-target` (or language-equivalent isolated build cache)

### B) Spec/phase map

Create:
- `scripts/spec-map.toml`

Include:
- phase definitions
- per-phase spec lists
- dependency edges (`depends_on`)
- per-spec branch mapping and prompt path

### C) Worktree scripts

Create:
- `scripts/worktree-create.sh <spec-id> [--base-branch <branch>]`
- `scripts/worktree-destroy.sh <spec-id>`
- `scripts/worktree-list.sh`

Behavior:
- create one worktree per spec under `../.worktrees/<repo>-<spec-id>/`
- create/check out branch per spec
- write `.env.agent` with branch + isolated target dir
- print clear machine-readable outputs where useful

### D) Container lifecycle scripts

Create:
- `scripts/dc-launch.sh <spec-id> [<worktree-path>]`
- `scripts/dc-stop.sh <spec-id>`

Behavior:
- build dev image if missing
- launch one container per spec
- mount worktree at `/workspace`
- forward auth correctly:
  - detect `copilot` binary on host and mount into container
  - extract token via `gh auth token` when env vars are absent
  - pass `GITHUB_TOKEN`/`GH_TOKEN` into container
  - mount `~/.copilot` and `~/.config/gh` when present

### E) Agent runner

Create:
- `scripts/agent-run.sh <spec-id>`

Pipeline per spec:
1. Validate branch/spec preconditions
2. Run spec workflow:
   - `speckit.specify`
   - `speckit.plan`
   - `speckit.tasks`
   - `speckit.implement`
3. Run quality gates:
   - build
   - tests
   - lint
   - format check
4. Run code review
5. Run fix cycle (max 3 rounds)
6. Commit and push if successful

Copilot CLI command format (required):
- Non-interactive prompts must use:
  - `copilot -p "<prompt>" --yolo --no-ask-user`
- For custom agent selection:
  - `copilot -p "<prompt>" --agent "<agent-name>" --yolo --no-ask-user`

Output protocol in worktree root:
- `.agent-status` (`RUNNING|SUCCESS|FAILED|BLOCKED|NEEDS_HUMAN_REVIEW`)
- `.agent-log`
- `.agent-review`
- `.agent-review-history/`
- `.agent-escalation` (only for critical blockers)

### F) Orchestrator

Create:
- `scripts/orchestrate.sh`

Support:
- `--spec <id>`
- `--phase <id>`
- `--dry-run`
- `--max-parallel <n>`

Behavior:
- parse spec-map
- enforce dependency order
- launch specs in parallel up to max limit
- monitor statuses
- halt on FAILED/BLOCKED
- pause on NEEDS_HUMAN_REVIEW
- perform phase-gate merge to `<DEFAULT_BRANCH>`
- run full quality gates after phase merge
- tag phase completion

### G) Cross-spec integration review

Create:
- `scripts/phase-gate-review.sh <phase-id>`
- `scripts/review-prompt-template.md`

Review checks:
- conflicting contracts/interfaces/traits
- duplicated logic between parallel specs
- missing integration points
- inconsistent error handling

If CRITICAL findings exist:
- create phase fix branch
- apply automated fixes
- re-run quality gates
- merge back only on green

### H) Docs + ignore rules

Create:
- `docs/development-workflow.md` describing end-to-end operation

Update:
- `.gitignore` with runtime artifacts:
  - `.agent-status`, `.agent-log`, `.agent-review`, `.agent-review-history/`, `.agent-escalation`, `.env.agent`, and temporary review files

Optionally update:
- `.github/copilot-instructions.md` with “headless mode” instructions.

## Execution plan (perform in this order)

1. Inspect repository structure and existing conventions.
2. Implement files for A+B+C in parallel where safe.
3. Implement files for D+E in parallel where safe.
4. Implement F and G.
5. Implement H.
6. Validate:
   - shell syntax checks (`bash -n scripts/*.sh`)
   - orchestrator dry run (`scripts/orchestrate.sh --dry-run`)
   - one-spec smoke run (`scripts/orchestrate.sh --spec <sample>`) if credentials available
7. Provide concise summary:
   - files created/changed
   - how to run
   - known limitations

## Escalation policy

Do not ask questions unless absolutely blocked by one of these:
- no detectable spec directory and no way to infer one
- missing default branch and cannot infer from git metadata
- container runtime unavailable and no alternative
- repository policy explicitly forbids required automation behavior

When blocked:
- stop
- explain exact blocker
- propose one concrete next command for the user
```

---

If you want, I can also provide a second variant optimized for non-Rust repos (Node/Python/Go) with language-specific quality gates and container images.
