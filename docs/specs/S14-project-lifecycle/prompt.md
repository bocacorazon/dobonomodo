# S14: Project Lifecycle

## Feature
Implement Project status transitions: `draft → active` (on successful activation), `active → draft` (on structural edit), `active → inactive` (manual deactivation), `active → conflict` (breaking Dataset change), `conflict → draft` (resolution), `inactive → draft` (re-open).

## Context
- Read: `docs/entities/project.md` (full lifecycle, ConflictReport, BreakingChange, BR-012a structural vs metadata changes)
- Read: `docs/capabilities/activate-project.md` (activation flow, Dataset version confirmation)

## Scope

### In Scope
- `core::model::project` — status transition logic
- Structural change detection: changes to operations, `input_dataset_id`, `materialization` → revert to `draft`
- Metadata change detection: `name`, `description`, `visibility`, `selectors`, `resolver_overrides` → no status change
- Dataset version pinning on activation (user confirms version)
- Breaking change detection: when Dataset version increments, compare schemas for column removal/rename/type change
- `ConflictReport` generation with `BreakingChange` entries
- Conflict resolution: `adapted` (update ops + advance version) or `pinned` (keep old version)
- Auto-return to `draft` when all `BreakingChange` entries resolved

### Out of Scope
- Validation logic (S13)
- Sandbox redirect (API server concern — S21)

## Dependencies
- **S13** (Activation Validation)

## Success Criteria
- All 6 status transitions work correctly
- Structural edits revert active → draft; metadata edits don't
- Breaking Dataset changes produce ConflictReport with specific broken operations
- Both resolution modes (adapted/pinned) work
