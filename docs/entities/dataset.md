# Entity: Dataset

**Status**: Draft  
**Created**: 2026-02-21  
**Domain**: Data Layer / Computation Inputs

## Definition

A Dataset is a user-assembled structural definition consisting of one designated main table and zero or more lookups — each lookup being either a raw table or a nested Dataset — joined to the main table via explicit foreign key conditions. A Dataset is agnostic of how its relationships are materialised; it describes only the shape and structure of the data available to the computation engine. It does not own or store data.

## Purpose & Role

A Dataset is the primary input definition for the computation engine. It specifies what data is available for computation and how the parts relate to one another, without prescribing how those relationships are resolved (that is a Project-level concern). Without a Dataset, the computation engine has no data to operate on. Its reusability across multiple Projects makes it a shared, versioned contract between data structure and computation.

## Attributes

| Attribute | Type | Required | Constraints | Description |
|---|---|---|---|---|
| `id` | `UUID` | Yes | Immutable, system-generated | Unique identifier |
| `name` | `String` | Yes | Non-empty | Human-readable name |
| `description` | `String` | No | — | Optional narrative description |
| `owner` | `User` | Yes | Must reference a valid user | The user who assembled this Dataset |
| `version` | `Integer` | Yes | Auto-incremented on every change, starts at 1 | Tracks evolution of the definition |
| `status` | `Enum` | Yes | `active` \| `disabled` | Controls availability for use in new Projects |
| `main_table` | `TableRef` | Yes | Must reference a valid Table with a defined data source | The primary table of the Dataset |
| `lookups` | `List<LookupDef>` | No | Each entry must include target (Table or Dataset) + join conditions | Ordered list of lookup joins |
| `created_at` | `Timestamp` | Yes | System-set on creation, immutable | Creation time |
| `updated_at` | `Timestamp` | Yes | System-set on every change | Last modification time |

### LookupDef (embedded structure)

| Attribute | Type | Required | Constraints | Description |
|---|---|---|---|---|
| `target` | `TableRef \| DatasetRef` | Yes | Must reference an existing Table or Dataset | The lookup target |
| `join_conditions` | `List<JoinCondition>` | Yes | At least one condition; each specifies FK column on both sides | Explicit foreign key join conditions |
| `alias` | `String` | No | Unique within the Dataset | Optional alias for the lookup in DSL expressions |

### JoinCondition (embedded structure)

| Attribute | Type | Required | Constraints | Description |
|---|---|---|---|---|
| `source_column` | `ColumnRef` | Yes | Column on the parent table/dataset side | FK source column |
| `target_column` | `ColumnRef` | Yes | Column on the lookup target side | FK target column |

## Relationships

| Relationship | Related Entity | Cardinality | Description |
|---|---|---|---|
| has main table | Table | 1:1 | Every Dataset has exactly one designated main table |
| has lookups | Table or Dataset | 1:N | A Dataset has zero or more lookup definitions, each pointing to a Table or nested Dataset |
| sourced from | DataSource | 1:N | Tables in a Dataset may come from multiple, different data sources (cross-source joins permitted) |
| used by | Project | N:M | A Dataset is independent of any single Project and may be referenced by many Projects |

## Behaviors & Rules

- **BR-001**: A Dataset MUST have exactly one main table.
- **BR-002**: Pre-defined lookups (joins specified at assembly time) are ALWAYS included in every computation that uses the Dataset — they cannot be conditionally skipped.
- **BR-003**: Each lookup join MUST have at least one explicit foreign key condition defined at assembly time.
- **BR-004**: Cross-source joins are permitted — the main table and any lookup may reside in different data sources.
- **BR-005**: A lookup target may be either a raw Table or a nested Dataset (recursive composition). From the parent Dataset's perspective, a nested Dataset is just another lookup.
- **BR-006**: Dataset version MUST be auto-incremented on every structural change (change to main table, addition/removal/modification of any lookup or join condition).
- **BR-007**: A Dataset that is referenced by one or more Projects MUST NOT be deleted. The preferred action is to disable it.
- **BR-008**: A disabled Dataset MUST NOT be selectable for use in new Projects. Existing Project references to a disabled Dataset remain valid and unaffected.
- **BR-009**: A Dataset is agnostic of how its relationships are materialised (eagerly flattened vs. resolved at runtime) — that decision belongs to the Project.

## Lifecycle

A Dataset is mutable and evolves over time. Version is tracked automatically. Deletion is guarded by active references.

| State | Description | Transitions To |
|---|---|---|
| `active` | Dataset is fully operational; can be referenced by new Projects | `disabled` |
| `disabled` | Dataset cannot be used in new Projects; existing references remain valid | `active` (re-enabled), or hard deletion only if zero Project references exist |

**What creates a Dataset**: A user explicitly assembles it by designating a main table and defining lookup joins.  
**What modifies a Dataset**: Any change to the main table, or addition, removal, or modification of a lookup or join condition. Each modification auto-increments the version.  
**What destroys a Dataset**: Hard deletion is only permitted when there are no Project references. The preferred alternative to deletion is disabling.

## Boundaries

- This entity does NOT store or own actual data — it is a structural definition only.
- This entity does NOT determine how joins are materialised (flattened/denormalised vs. runtime-resolved) — that is a Project-level concern.
- This entity is NOT tied to a specific Project — it is a reusable, independently versioned definition.
- This entity does NOT represent a query, computation, or result — it defines the input shape available to the computation engine.
- This entity does NOT define what calculations are performed — that is the responsibility of the DSL and Computation Engine.

## Related Entities

- [[Project]] — A Project references a Dataset to determine what data its computations operate on; multiple Projects may share the same Dataset.
- [[Table]] — The primitive building block; a Dataset's main table and raw-table lookups are Tables.
- [[DataSource]] — Tables in a Dataset are sourced from one or more DataSources; cross-source joins are permitted.
- [[ComputationEngine]] — Consumes a Dataset as its primary input definition when executing DSL programs.
- [[DSL]] — May request dynamic (runtime) joins on top of the Dataset's pre-defined structure.
