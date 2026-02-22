# Reusable Prompt: Go Parallel Agent Environment (Auto-Detect Repo/Branch)

Use this prompt in any **Go** repository to set up the same autonomous, phase-gated, parallel agent workflow.

## Prompt to copy

```text
You are a senior platform engineer. Build an autonomous, parallel, spec-driven development environment in this Go repository.

## Auto-detect repository context (required)

Before doing anything else, detect runtime context from git metadata:

```bash
REPO_ROOT="$(git rev-parse --show-toplevel)"
REPO_NAME="$(basename "$REPO_ROOT")"

DEFAULT_BRANCH="$(git remote show origin 2>/dev/null | sed -n '/HEAD branch/s/.*: //p')"
if [ -z "$DEFAULT_BRANCH" ]; then
  DEFAULT_BRANCH="$(git symbolic-ref refs/remotes/origin/HEAD 2>/dev/null | sed 's@^refs/remotes/origin/@@')"
fi
if [ -z "$DEFAULT_BRANCH" ]; then
  DEFAULT_BRANCH="$(git rev-parse --abbrev-ref HEAD)"
fi

echo "REPO_ROOT=$REPO_ROOT"
echo "REPO_NAME=$REPO_NAME"
echo "DEFAULT_BRANCH=$DEFAULT_BRANCH"
```

Do not ask me for repo name or default branch unless detection fails completely.

## Goals

- Build a reusable framework where multiple specs are implemented in parallel by headless agents.
- Use one worktree + one container per spec for isolation.
- Enforce quality gates and automated review/fix cycles before merge.
- Use phase-gate merges into `$DEFAULT_BRANCH`.
- Escalate only on CRITICAL blockers.

## Constraints

1. Make minimal, surgical changes.
2. Follow existing conventions in this repo.
3. No destructive git commands.
4. No secret leakage or hardcoded credentials.
5. No silent failures; surface explicit errors.

## Deliverables

### A) Devcontainer for Go

Create:
- `.devcontainer/Dockerfile`
- `.devcontainer/devcontainer.json`
- `.devcontainer/post-create.sh`

Requirements:
- Go toolchain (stable, e.g. 1.24+)
- `git`, `curl`, `gh`, Node.js 20+
- `make` and common build tools
- Copilot CLI available inside container
- Shared module/build caches:
  - `GOMODCACHE=/home/agent/.cache/go-mod`
  - `GOCACHE=/home/agent/.cache/go-build`

### B) Spec map

Create:
- `scripts/spec-map.toml`

Include:
- phases
- per-phase spec IDs
- dependencies (`depends_on`)
- per-spec branch + prompt paths

### C) Worktree scripts

Create:
- `scripts/worktree-create.sh <spec-id> [--base-branch <branch>]`
- `scripts/worktree-destroy.sh <spec-id>`
- `scripts/worktree-list.sh`

Behavior:
- worktrees at `../.worktrees/${REPO_NAME}-<spec-id>/`
- branch per spec
- write `.env.agent` with:
  - `SPECIFY_FEATURE`
  - `GOMODCACHE`
  - `GOCACHE`

### D) Container lifecycle scripts

Create:
- `scripts/dc-launch.sh <spec-id> [<worktree-path>]`
- `scripts/dc-stop.sh <spec-id>`

Must:
- build image if missing
- run one container per spec
- mount worktree to `/workspace`
- forward auth robustly:
  - mount host `copilot` binary if found
  - derive token using `gh auth token` if env vars are empty
  - pass `GITHUB_TOKEN` and `GH_TOKEN`
  - mount `~/.copilot` and `~/.config/gh` when present

### E) Agent runner

Create:
- `scripts/agent-run.sh <spec-id>`

Per-spec flow:
1. Validate branch + spec preconditions
2. Run spec-kit stages:
   - `speckit.specify`
   - `speckit.plan`
   - `speckit.tasks`
   - `speckit.implement`
3. Run Go quality gates:
   - `go test ./...`
   - `go vet ./...`
   - `gofmt -l .` (must return no files)
   - if repo uses golangci-lint config, run `golangci-lint run`
4. Run code review
5. Run auto-fix cycle (max 3 rounds)
6. Commit and push on success

Copilot CLI command format (required):
- `copilot -p "<prompt>" --yolo --no-ask-user`
- `copilot -p "<prompt>" --agent "<agent-name>" --yolo --no-ask-user`

Status artifacts:
- `.agent-status` (`RUNNING|SUCCESS|FAILED|BLOCKED|NEEDS_HUMAN_REVIEW`)
- `.agent-log`
- `.agent-review`
- `.agent-review-history/`
- `.agent-escalation` (only for blockers)

### F) Orchestrator

Create:
- `scripts/orchestrate.sh`

Support:
- `--spec <id>`
- `--phase <id>`
- `--dry-run`
- `--max-parallel <n>`

Behavior:
- parse `spec-map.toml`
- enforce dependency order
- launch specs in parallel
- monitor status files
- halt on FAILED/BLOCKED
- pause on NEEDS_HUMAN_REVIEW
- phase-gate merge all completed branches into `$DEFAULT_BRANCH`
- run post-merge quality gates
- tag `phase-<id>-complete`

### G) Cross-spec integration review

Create:
- `scripts/phase-gate-review.sh <phase-id>`
- `scripts/review-prompt-template.md`

Review for:
- conflicting interfaces/contracts
- duplicated logic across specs
- missing integration points
- inconsistent error handling

If CRITICAL findings remain:
- create a phase fix branch
- apply automated fixes
- rerun gates
- merge only if green

### H) Docs and ignore rules

Create:
- `docs/development-workflow.md`

Update:
- `.gitignore` for runtime artifacts:
  - `.agent-status`, `.agent-log`, `.agent-review`, `.agent-review-history/`, `.agent-escalation`, `.env.agent`
  - temporary diff/review files
  - local worktree-generated metadata

## Execution order

1. Detect repo context (`REPO_NAME`, `DEFAULT_BRANCH`) from git.
2. Implement A + B + C.
3. Implement D + E.
4. Implement F + G.
5. Implement H.
6. Validate:
   - `bash -n scripts/*.sh`
   - `scripts/orchestrate.sh --dry-run`
   - optional smoke run: `scripts/orchestrate.sh --spec <sample-id>`
7. Summarize:
   - files created
   - commands to run
   - known limitations

## Escalation policy

Do not ask me questions unless blocked by one of:
- git context detection failed (cannot determine repo root/name/branch)
- no container runtime and no viable fallback
- spec directory cannot be inferred from repository structure
- repository policy conflicts with required automation

If blocked:
- stop
- describe exact blocker
- give one concrete next command for me to run
```

