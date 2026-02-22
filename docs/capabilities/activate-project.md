# Capability: Activate Project

**Status**: Draft  
**Created**: 2026-02-22  
**Domain**: Computation / Orchestration

## Summary

Activate Project is the manual transition that moves a Project from `draft` to `active`, enabling it to write Run outputs to production destinations. The capability validates the Project's full definition — expressions, column references, named selectors, and Resolver availability — and prompts the user to confirm the Dataset version to pin before committing the transition. Validation failures block activation and return structured errors. A successful activation makes the Project ready for scheduled and manual production Runs.

---

## Trigger

Explicit user action. The system does not automatically activate a Project. The user initiates activation at any point while the Project is in `draft` status.

---

## Preconditions

| # | Condition |
|---|---|
| PC-001 | The Project exists and has `status: draft` |
| PC-002 | The Project has at least one operation |
| PC-003 | The `input_dataset_id` references an existing Dataset with `status: active` |

---

## Activation Flow

1. **User initiates activation** on a Project in `draft`.
2. **System presents available Dataset versions** for the configured `input_dataset_id` and prompts the user to confirm which version to pin.
3. **System runs validation** (see Validation Rules below) against the selected Dataset version.
4. **On validation failure**: activation is aborted; the Project remains in `draft`; a structured `ActivationError` is returned listing every failure with its location in the operation pipeline.
5. **On validation success**: `input_dataset_version` is set to the user-confirmed version, `status` is set to `active`, and `version` is auto-incremented.

---

## Validation Rules

All validation is performed against the selected Dataset version and the Project's current operation sequence.

| ID | Rule | Failure Type |
|---|---|---|
| VAL-001 | All expressions in every operation MUST parse without syntax errors | `ExpressionSyntaxError` |
| VAL-002 | All column references in expressions (`table.column_name`) MUST resolve to columns declared in the Dataset schema or a RuntimeJoin alias in scope | `UnresolvedColumnRef` |
| VAL-003 | Expression types MUST be compatible with their context (e.g., `selector` expressions must be boolean; assignment target types must match expression output type) | `TypeMismatch` |
| VAL-004 | All `{{NAME}}` references in `selector` fields MUST resolve to a key in `project.selectors` | `UnresolvedSelectorRef` |
| VAL-005 | All named selectors in `project.selectors` MUST parse as valid boolean expressions | `ExpressionSyntaxError` |
| VAL-006 | A Resolver MUST be reachable for the Dataset: either `project.resolver_overrides` contains an entry for the Dataset, or `dataset.resolver_id` is set, or a system default Resolver with `is_default: true` exists | `ResolverNotFound` |
| VAL-007 | The resolved Resolver MUST have `status: active` | `ResolverDisabled` |
| VAL-008 | All RuntimeJoin `source` references in `update` operations MUST resolve to a Dataset or TableRef that exists and is active | `UnresolvedJoinSource` |
| VAL-009 | Operation `order` values MUST be unique within the Project | `DuplicateOperationOrder` |

---

## Status Transitions

### `draft → active`
Triggered by successful activation. `input_dataset_version` is pinned; `version` is incremented.

### `active → draft`
Triggered automatically when a **structural change** is made to an active Project. Structural changes include:
- Any modification to the `operations` list (add, remove, reorder, change parameters)
- Changing `input_dataset_id`
- Changing `materialization`

The following changes do NOT trigger reversion to `draft`:
- `name`, `description`, `visibility`
- Adding/updating/removing entries in `selectors` (named selectors are re-validated at next activation)
- Adding/updating/removing entries in `resolver_overrides`

### `active → inactive`
Triggered by explicit user deactivation. An `inactive` Project cannot be Run and does not receive scheduled triggers. It can be re-activated (which restarts the activation flow) or left dormant.

### `active → conflict`
Triggered automatically when the pinned Dataset version is superseded by a new version that introduces breaking changes (column removals, type changes) detected by the Dataset conflict model. See `project.md` `ConflictReport`.

### `inactive → draft`
Triggered when the user re-opens an inactive Project for editing.

### `conflict → draft`
Triggered when the user acknowledges and resolves the conflict (e.g., updates operations to accommodate Dataset changes).

---

## Draft Mode Behaviour

A Project in `draft` is a **development sandbox**. It is fully executable — Runs can be triggered manually — but all `output` operation destinations are transparently redirected to the deployment-level **sandbox DataSource** rather than the configured production destinations.

- The sandbox DataSource is configured once at deployment level. All draft Run outputs go there regardless of individual `output` operation configuration.
- The `output` operation executes normally in all other respects (selector, column projection, `register_as_dataset`).
- Draft Runs are indistinguishable from production Runs in terms of the Run entity itself — they carry the same `id`, `status`, trace events, etc. Their sandbox nature is determined solely by the Project's `status: draft` at the time of execution.
- Sandbox outputs are not guaranteed to be retained long-term; retention policy is deployment-specific.

---

## ActivationError (output structure)

Returned when validation fails. Activation is not committed.

| Field | Type | Description |
|---|---|---|
| `project_id` | `UUID` | The Project that failed activation |
| `failures` | `List<ValidationFailure>` | One entry per validation failure |

### ValidationFailure

| Field | Type | Description |
|---|---|---|
| `rule` | `String` | The VAL-xxx rule ID that was violated |
| `type` | `String` | Failure type (e.g., `ExpressionSyntaxError`, `UnresolvedColumnRef`) |
| `operation_order` | `Integer` | The operation where the failure occurred; `null` for Project-level failures |
| `detail` | `String` | Human-readable description of the specific failure |

---

## Behaviors / Rules

| ID | Rule |
|---|---|
| BR-001 | Activation MUST be manually triggered by a user. The system MUST NOT auto-activate a Project. |
| BR-002 | The user MUST confirm the Dataset version to pin before validation proceeds. The system MUST present all available versions of the `input_dataset_id` Dataset for selection. |
| BR-003 | Activation is atomic: either all validations pass and the Project becomes `active`, or all fail and the Project remains `draft`. There is no partial activation. |
| BR-004 | On successful activation, `input_dataset_version` MUST be set to the user-confirmed version and `project.version` MUST be auto-incremented. |
| BR-005 | On activation failure, the Project MUST remain in `draft` with its current `input_dataset_version` unchanged. A full `ActivationError` MUST be returned. |
| BR-006 | Structural changes to an `active` Project MUST automatically revert its status to `draft`. The `input_dataset_version` pin is preserved but the Project must be re-activated before production Runs resume. |
| BR-007 | An `inactive` Project MUST NOT execute Runs (manual or scheduled). |
| BR-008 | Draft Runs MUST redirect all `output` operation writes to the deployment-level sandbox DataSource. The production destination configured on the `output` operation MUST NOT be written to. |
| BR-009 | A Project that reverts from `active` to `draft` due to a structural edit does NOT invalidate past Runs — they retain their ProjectSnapshot and remain valid records. |
| BR-010 | Re-activation of a previously `active` Project follows the same flow as first activation — the user is prompted to confirm a Dataset version, and all validation rules apply. |

---

## Boundaries

- This capability does NOT execute the Project — activation is a validation and status transition only.
- This capability does NOT permanently alter Dataset entities — it only pins a reference on the Project.
- This capability does NOT clean up sandbox outputs from previous draft Runs.
- This capability does NOT manage scheduled trigger configuration — scheduling is a separate concern.

---

## Open Questions

| # | Question | Status |
|---|---|---|
| OQ-001 | Should activation also validate that `output` operation destinations (DataSource references) are reachable, or is that deferred to Run time? | Open |
| OQ-002 | When a structural edit reverts an `active` Project to `draft`, should in-flight Runs be allowed to complete or be cancelled? | Open |
| OQ-003 | Should `inactive` be reachable from `draft` directly (i.e., can you "park" a draft without activating)? Or is `inactive` only reachable from `active`? | Open |
