# Data Model: Resolver Rule Evaluation Engine

**Feature**: Resolver Rule Evaluation Engine  
**Branch**: 012-resolver-engine  
**Date**: 2026-02-22

## Overview

This document defines the entities, relationships, and data structures for the Resolver Rule Evaluation Engine. Most entities already exist in the codebase (`crates/core/src/model/`); this feature adds new internal types for resolution execution.

---

## Existing Entities (Reference)

These entities are already defined in `crates/core/src/model/` and used by the resolver engine.

### Resolver
**Location**: `crates/core/src/model/resolver.rs`

| Field | Type | Description | Validation |
|-------|------|-------------|------------|
| id | String | Unique resolver identifier | Non-empty |
| name | String | Human-readable name | Non-empty |
| description | Option\<String\> | Optional description | - |
| version | i32 | Version number | > 0 |
| status | ResolverStatus | Active \| Disabled | Enum |
| is_default | Option\<bool\> | System default flag | - |
| rules | Vec\<ResolutionRule\> | Ordered list of rules | Non-empty |
| created_at | Option\<String\> | Creation timestamp | ISO 8601 |
| updated_at | Option\<String\> | Last update timestamp | ISO 8601 |

**Relationships**:
- Referenced by Dataset (dataset.resolver_id)
- Referenced by Project override (project.resolver_override_id)
- Contains multiple ResolutionRule (ordered list)

---

### ResolutionRule
**Location**: `crates/core/src/model/resolver.rs`

| Field | Type | Description | Validation |
|-------|------|-------------|------------|
| name | String | Rule identifier | Non-empty |
| when_expression | Option\<String\> | Boolean condition (None = always match) | Valid expression syntax |
| data_level | String | Target period level or "any" | Valid level name or "any" |
| strategy | ResolutionStrategy | Location template | Enum variant |

**Relationships**:
- Owned by Resolver (in rules list)
- References Calendar level via data_level

---

### ResolutionStrategy
**Location**: `crates/core/src/model/resolver.rs`

Tagged enum with three variants:

#### Path Variant
| Field | Type | Description |
|-------|------|-------------|
| datasource_id | String | DataSource identifier |
| path | String | File path template (e.g., `/data/{period_id}/{table_name}.parquet`) |

#### Table Variant
| Field | Type | Description |
|-------|------|-------------|
| datasource_id | String | DataSource identifier |
| table | String | Table name template (e.g., `{table_name}_{period_id}`) |
| schema | Option\<String\> | Optional schema name template |

#### Catalog Variant
| Field | Type | Description |
|-------|------|-------------|
| endpoint | String | Catalog API endpoint template |
| method | String | HTTP method |
| auth | Option\<String\> | Authentication config |
| params | serde_json::Value | Query parameters (JSON) |
| headers | serde_json::Value | HTTP headers (JSON) |

---

### Calendar
**Location**: `crates/core/src/model/calendar.rs`

| Field | Type | Description | Validation |
|-------|------|-------------|------------|
| id | Uuid | Unique calendar identifier | - |
| name | String | Calendar name | Non-empty |
| description | Option\<String\> | Optional description | - |
| status | CalendarStatus | Draft \| Active \| Deprecated | Enum |
| is_default | bool | System default flag | - |
| levels | Vec\<LevelDef\> | Hierarchy level definitions | Non-empty |
| created_at | Option\<String\> | Creation timestamp | ISO 8601 |
| updated_at | Option\<String\> | Last update timestamp | ISO 8601 |

**Relationships**:
- Contains multiple LevelDef (hierarchy definition)
- Referenced by Period (period.calendar_id)

---

### LevelDef
**Location**: `crates/core/src/model/calendar.rs`

| Field | Type | Description | Validation |
|-------|------|-------------|------------|
| name | String | Level name (e.g., "year", "quarter", "month") | Non-empty, unique in calendar |
| parent_level | Option\<String\> | Parent level name (None = root) | Valid level name |
| identifier_pattern | Option\<String\> | Regex pattern for identifiers | Valid regex |
| date_rules | Vec\<DateRule\> | Date calculation rules | - |

**Relationships**:
- Owned by Calendar (in levels list)
- Forms hierarchy via parent_level references

---

### Period
**Location**: `crates/core/src/model/period.rs`

| Field | Type | Description | Validation |
|-------|------|-------------|------------|
| id | Uuid | Unique period identifier | - |
| identifier | String | Period identifier (e.g., "2024-Q1") | Matches level pattern |
| name | String | Display name | Non-empty |
| description | Option\<String\> | Optional description | - |
| calendar_id | Uuid | Calendar reference | Valid calendar |
| year | i32 | Year number | - |
| sequence | i32 | Ordering within parent | > 0 |
| start_date | String | Start date | ISO 8601 |
| end_date | String | End date | ISO 8601 |
| status | PeriodStatus | Open \| Closed \| Locked | Enum |
| parent_id | Option\<Uuid\> | Parent period reference | Valid period |
| created_at | Option\<String\> | Creation timestamp | ISO 8601 |
| updated_at | Option\<String\> | Last update timestamp | ISO 8601 |

**Relationships**:
- Belongs to Calendar (calendar_id)
- Has parent Period (parent_id) - forms tree
- Referenced in ResolutionRequest

---

### ResolvedLocation
**Location**: `crates/core/src/model/resolver.rs`

| Field | Type | Description | Validation |
|-------|------|-------------|------------|
| datasource_id | String | DataSource identifier | Non-empty |
| path | Option\<String\> | File path (for Path strategy) | - |
| table | Option\<String\> | Table name (for Table strategy) | - |
| schema | Option\<String\> | Schema name (for Table strategy) | - |
| period_identifier | Option\<String\> | Period identifier | - |

**Note**: This entity exists but needs extension. See "New Fields" below.

---

## New Entities (To Be Created)

These entities are internal to the resolver engine implementation.

### ResolutionRequest
**Location**: `crates/core/src/resolver/context.rs` (new file)

| Field | Type | Description | Validation |
|-------|------|-------------|------------|
| dataset_id | String | Dataset identifier | Non-empty |
| table_name | String | Target table name | Non-empty |
| period_id | Uuid | Requested period | Valid period |
| project_id | Option\<String\> | Project context (for precedence) | - |

**Purpose**: Input to resolution engine, carries context for rule evaluation and template rendering.

---

### ResolutionContext
**Location**: `crates/core/src/resolver/context.rs` (new file)

| Field | Type | Description |
|-------|------|-------------|
| dataset_id | String | Dataset identifier |
| table_name | String | Table name |
| period | Period | Requested period (full object) |
| period_level | String | Period's level name |
| additional_context | HashMap\<String, String\> | Extensible key-value pairs |

**Purpose**: Enriched context used for expression evaluation and template rendering.

**Derivation**: Built from ResolutionRequest + loaded metadata (Period, Calendar).

---

### ResolutionResult
**Location**: `crates/core/src/resolver/engine.rs` (new file)

| Field | Type | Description |
|-------|------|-------------|
| locations | Vec\<ResolvedLocation\> | Resolved locations (one per expanded period) |
| diagnostic | ResolutionDiagnostic | Evaluation trace |

**Purpose**: Output from resolution engine containing both data (locations) and metadata (diagnostics).

---

### ResolutionDiagnostic
**Location**: `crates/core/src/resolver/diagnostics.rs` (new file)

| Field | Type | Description |
|-------|------|-------------|
| resolver_id | String | Selected resolver ID |
| resolver_source | ResolverSource | How resolver was selected |
| evaluated_rules | Vec\<RuleDiagnostic\> | Rule evaluation trace |
| outcome | DiagnosticOutcome | Final outcome status |
| expanded_periods | Vec\<String\> | Period identifiers after expansion |

**Purpose**: Troubleshooting information for operators and developers.

---

### RuleDiagnostic
**Location**: `crates/core/src/resolver/diagnostics.rs` (new file)

| Field | Type | Description |
|-------|------|-------------|
| rule_name | String | Rule identifier |
| matched | bool | Whether rule matched |
| reason | String | Explanation (why matched or why not) |
| evaluated_expression | Option\<String\> | The when_expression evaluated |

**Purpose**: Per-rule evaluation details for diagnostic output.

---

### ResolverSource (Enum)
**Location**: `crates/core/src/resolver/diagnostics.rs` (new file)

Variants:
- `ProjectOverride` - Selected from project.resolver_override_id
- `DatasetReference` - Selected from dataset.resolver_id
- `SystemDefault` - Selected as system default resolver

**Purpose**: Tracks resolver precedence for diagnostics.

---

### DiagnosticOutcome (Enum)
**Location**: `crates/core/src/resolver/diagnostics.rs` (new file)

Variants:
- `Success` - Resolution succeeded
- `NoMatchingRule` - No rules matched
- `PeriodExpansionFailure` - Calendar hierarchy traversal failed
- `TemplateRenderError` - Template contained invalid tokens

**Purpose**: Categorizes resolution outcome for diagnostics and error handling.

---

## Extensions to Existing Entities

### ResolvedLocation (Extended)

**New fields to add**:
| Field | Type | Description |
|-------|------|-------------|
| resolver_id | String | Source resolver ID (for traceability) |
| rule_name | String | Source rule name (for traceability) |

**Rationale**: FR-012 requires resolver and rule identity in results for traceability.

**Migration**: Backward-compatible - new fields are added, no existing fields removed.

---

## Entity Relationships Diagram

```
┌─────────────────┐
│ ResolutionRequest│
│ (input)         │
└────────┬────────┘
         │
         ▼
┌─────────────────┐     ┌──────────────┐
│ Resolver        │────▶│ ResolutionRule│
│ (precedence     │     │ (ordered)     │
│  selection)     │     └──────┬────────┘
└────────┬────────┘            │
         │                     │ data_level
         │                     ▼
         │            ┌─────────────────┐
         │            │ Calendar        │
         │            │ ├── LevelDef    │
         │            │ └── hierarchy   │
         │            └────────┬────────┘
         │                     │
         │                     ▼
         │            ┌─────────────────┐
         │            │ Period          │
         │            │ (tree structure)│
         │            └────────┬────────┘
         │                     │
         ▼                     ▼
┌─────────────────────────────────┐
│ ResolutionResult                │
│ ├── Vec<ResolvedLocation>       │
│ │   └── (extended with metadata)│
│ └── ResolutionDiagnostic        │
│     ├── ResolverSource           │
│     ├── Vec<RuleDiagnostic>      │
│     └── DiagnosticOutcome        │
└─────────────────────────────────┘
```

---

## State Transitions

### Resolver Status
```
Draft ──(validate)──▶ Active
Active ──(deprecate)──▶ Deprecated
```

### Period Status
```
Open ──(close)──▶ Closed ──(lock)──▶ Locked
```

**Note**: Resolver engine reads Period status but does not modify it.

---

## Validation Rules

### Rule Evaluation
1. If `when_expression` is None, rule matches unconditionally
2. If `when_expression` is Some(expr), expr must evaluate to boolean
3. Rules are evaluated in list order (first match wins)
4. At least one rule must match, or resolution fails with NoMatchingRule

### Period Expansion
1. If `data_level == "any"`, return single period (no expansion)
2. If requested period level == `data_level`, return single period
3. If `data_level` is finer than requested level, expand via hierarchy:
   - Traverse parent_id chain to find common ancestor
   - Collect all descendants at `data_level`
   - Sort by `sequence` for deterministic order
4. If no path exists in hierarchy, fail with PeriodExpansionFailure

### Template Rendering
1. Templates may contain tokens: `{period_id}`, `{period_name}`, `{table_name}`, `{dataset_id}`, `{datasource_id}`
2. All tokens must be resolvable from ResolutionContext
3. Unknown tokens cause TemplateRenderError
4. Rendered values are URL-encoded if used in path/endpoint contexts

---

## Indexing Requirements

### For Performance
No new database indexes needed. This is an in-memory computation feature.

### For Queries (Caller Responsibility)
Engine-worker must efficiently load:
- Resolver by ID (primary key)
- Calendar by ID (primary key)
- Periods by calendar_id and level (filtered query)
- Period hierarchy (parent_id traversal)

**Assumption**: These queries are already optimized by existing metadata store implementation.

---

## Summary

**Existing entities**: Resolver, ResolutionRule, ResolutionStrategy, Calendar, LevelDef, Period, ResolvedLocation (minor extension)

**New entities**: ResolutionRequest, ResolutionContext, ResolutionResult, ResolutionDiagnostic, RuleDiagnostic, ResolverSource, DiagnosticOutcome

**Key relationships**:
- Resolver contains ordered ResolutionRules
- ResolutionRule references Calendar level via data_level
- Period forms tree via parent_id, belongs to Calendar
- ResolutionResult contains ResolvedLocations + diagnostics for traceability

**Validation**: Rules enforce first-match semantics, period expansion follows hierarchy, template tokens must be valid.
