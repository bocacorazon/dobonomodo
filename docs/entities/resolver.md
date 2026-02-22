# Entity: Resolver

**Status**: Draft  
**Created**: 2026-02-22  
**Domain**: Data Access / Infrastructure

## Definition

A Resolver is a versioned, configuration-driven entity that defines the rules by which the system locates physical data for a given Dataset table and Period. It contains a prioritised list of resolution rules — each with a declarative `when` condition and a typed resolution strategy — and is invoked by the Resolve Dataset capability at Run time. Because data storage layouts can vary by Period (e.g., different formats or locations before and after a migration date), a Resolver returns a **list** of resolved data locations, automatically expanding coarser-grained Periods into the constituent child Periods at which data actually exists.

---

## Purpose & Role

The Resolver is the bridge between the Dataset's purely logical schema and the physical data that backs it. Without a Resolver, the engine has no way to find data — the Dataset intentionally holds no location information. By making location resolution declarative and rule-based, the system can transparently serve data from heterogeneous environments (legacy CSV volumes, parquet on S3, databases, external catalogs) without requiring code changes or Dataset modifications. Because Resolvers are versioned and their identity is captured in the Run snapshot, every execution is fully reproducible.

---

## Attributes

| Attribute | Type | Required | Constraints | Description |
|---|---|---|---|---|
| `id` | `String` | Yes | Immutable, user-defined; unique across the system | Stable identifier used in Dataset `resolver_id` and Project overrides |
| `name` | `String` | Yes | Non-empty | Human-readable name |
| `description` | `String` | No | — | Optional narrative description |
| `version` | `Integer` | Yes | Auto-incremented on every change; starts at 1 | Enables reproducibility — Runs capture both `id` and `version` |
| `status` | `Enum` | Yes | `active` \| `disabled` | Only `active` Resolvers may be used in new Runs |
| `is_default` | `Boolean` | No | At most one Resolver may have `is_default: true` per deployment | When true, this Resolver is used when neither the Dataset nor the Project specifies one |
| `rules` | `List<ResolutionRule>` | Yes | At least one rule; evaluated in declaration order; first match wins | Prioritised list of resolution rules |
| `created_at` | `Timestamp` | Yes | System-set on creation; immutable | Creation time |
| `updated_at` | `Timestamp` | Yes | System-set on every change | Last modification time |

---

### ResolutionRule (embedded structure)

| Attribute | Type | Required | Constraints | Description |
|---|---|---|---|---|
| `name` | `String` | Yes | Non-empty; unique within the Resolver | Human-readable rule label; included in diagnostics on no-match |
| `when` | `Expression` | No | Must be a boolean Expression. When omitted, the rule always matches (use as the final fallback) | Condition evaluated against resolution context variables (see below) |
| `data_level` | `String` | Yes | Must match a level name defined in the relevant Calendar; use `"any"` for non-period-partitioned data | The Calendar level at which physical data exists for this rule. Used for automatic period expansion |
| `strategy` | `ResolutionStrategy` | Yes | Discriminated union by `type` | How to resolve the physical location once the rule matches |

#### Resolution Context Variables (available in `when` expressions)

| Variable | Type | Description |
|---|---|---|
| `period.identifier` | `String` | The Period's string identifier (e.g., `"2026-01"`, `"FY2026-Q1"`) |
| `period.level` | `String` | The Calendar level name of the requested Period (e.g., `"month"`, `"quarter"`) |
| `period.start_date` | `Date` | The Period's start date |
| `period.end_date` | `Date` | The Period's end date |
| `table.name` | `String` | The logical table name from the Dataset's `TableRef` |

---

### ResolutionStrategy (discriminated union by `type`)

#### `type: path` — file-based sources (parquet, csv, etc.)

| Field | Type | Required | Description |
|---|---|---|---|
| `datasource_id` | `String` | Yes | References an active DataSource of file type |
| `path` | `String` | Yes | Path template; supports period tokens (`{{YYYY}}`, `{{MM}}`, `{{QQ}}`, `{{identifier}}`) and `{{table_name}}` |

#### `type: table` — database sources

| Field | Type | Required | Description |
|---|---|---|---|
| `datasource_id` | `String` | Yes | References an active DataSource of database type |
| `table` | `String` | Yes | Table name template; supports `{{table_name}}` and period tokens |
| `schema` | `String` | No | Schema/namespace within the database; supports `{{table_name}}` and period tokens |

#### `type: catalog` — external data catalog API

| Field | Type | Required | Description |
|---|---|---|---|
| `endpoint` | `String` | Yes | URL of the catalog API |
| `method` | `Enum` | Yes | `GET \| POST` |
| `auth` | `String` | No | Credential reference key (e.g., `vault://catalog-token`) |
| `params` | `Map<String, String>` | No | Query parameters; values support `{{table_name}}` and period tokens |
| `headers` | `Map<String, String>` | No | HTTP headers; values support credential reference keys |

> Path, table, and parameter templates support all Calendar `identifier_pattern` tokens relevant to the resolved Period's level, plus `{{table_name}}` (the `TableRef` logical name).

---

## Resolution Algorithm

When the Resolve Dataset capability invokes a Resolver for a given `(table, period)`:

1. **Rule evaluation**: Evaluate each `ResolutionRule.when` condition against the resolution context, in declaration order. Stop at the first matching rule.
2. **No match**: If no rule matches, return `status: error` with diagnostics listing every rule name and why its `when` condition evaluated to false.
3. **Period expansion**: If the matched rule's `data_level` is finer than the requested Period's level (e.g., rule says `data_level: "month"` but a quarter was requested), traverse the Calendar hierarchy downward to enumerate all child Periods at `data_level`. Apply the strategy's template to each child Period.
4. **Template rendering**: Substitute period tokens and `{{table_name}}` in the `path`, `table`, or `params` templates for each resolved Period.
5. **Return**: A list of `ResolvedLocation` entries — one per rendered template result.

If `data_level: "any"`, no expansion occurs — a single location is returned regardless of the requested Period's granularity.

---

### ResolvedLocation (output structure)

| Field | Type | Description |
|---|---|---|
| `datasource_id` | `String` | The DataSource to use |
| `path` | `String` | Rendered path (for `path` strategy) |
| `table` | `String` | Rendered table name (for `table` strategy) |
| `schema` | `String` | Rendered schema (for `table` strategy, if applicable) |
| `catalog_response` | `Map` | Raw catalog API response (for `catalog` strategy) |
| `period_identifier` | `String` | The Period identifier this location corresponds to |

---

## Resolver Selection Precedence

When the engine needs a Resolver for a Dataset table within a Run, the following order applies:

1. **Project-level override** — if the Project defines a `resolver_override` for this Dataset, use that Resolver.
2. **Dataset-level** — if the Dataset has a `resolver_id`, use that Resolver.
3. **System default** — if a Resolver with `is_default: true` exists, use it.
4. **Error** — if none of the above resolve to an active Resolver, the Run fails with `ResolverNotFound`.

---

## Behaviors / Rules

| ID | Rule |
|---|---|
| BR-001 | `version` MUST be auto-incremented on every change to the Resolver definition (any rule added, removed, or modified). |
| BR-002 | At most one Resolver per deployment may have `is_default: true`. Attempting to set a second default is a validation error. |
| BR-003 | Rules are evaluated in declaration order. The first rule whose `when` condition evaluates to `true` is used. Subsequent rules are not evaluated. |
| BR-004 | A rule with no `when` condition always matches. It SHOULD be placed last to act as a fallback. Having it anywhere other than last is a validation warning. |
| BR-005 | If no rule matches, the Resolver MUST return `status: error` with diagnostics that include each rule's `name` and the evaluated result of its `when` condition. |
| BR-006 | `data_level` MUST match a level name defined in the Calendar associated with the requested Period's hierarchy. The value `"any"` is always valid. |
| BR-007 | When `data_level` is finer than the requested Period's level, the Resolver automatically expands to all child Periods at `data_level` using the Calendar hierarchy. It MUST NOT return locations for Periods outside the requested Period's range. |
| BR-008 | Template tokens (`{{YYYY}}`, `{{MM}}`, etc.) are resolved using the Calendar's `identifier_pattern` definition for the target `data_level`. Unknown tokens are a render-time error. |
| BR-009 | A disabled Resolver MUST NOT be used in new Runs. Existing Runs that captured a disabled Resolver's `id + version` remain valid. |
| BR-010 | The Run's ProjectSnapshot MUST record the `resolver_id` and `resolver_version` for each Dataset resolved during the Run. |

---

## Lifecycle

| State | Description | Transitions To |
|---|---|---|
| `active` | Resolver is available for use in new Runs | `disabled` |
| `disabled` | Resolver cannot be used in new Runs; existing Run snapshots remain valid | `active` (re-enabled) |

Every modification auto-increments `version`. There is no hard delete while any Run snapshot references the Resolver.

---

## Relationships

| Entity | Relationship |
|---|---|
| Dataset | A Dataset optionally references a Resolver via `resolver_id`; the Resolver locates physical data for each of the Dataset's tables |
| Project | A Project may override the Resolver for a specific Dataset via `resolver_override` |
| Run | The Run's ProjectSnapshot captures `resolver_id + resolver_version` for each resolved Dataset |
| DataSource | Each `path` and `table` strategy references an active DataSource by `datasource_id` |
| Period | The resolution context receives the requested Period's attributes; Calendar hierarchy drives period expansion |
| Calendar | Calendar level definitions provide the `data_level` validation and period expansion hierarchy |

---

## Boundaries

- A Resolver does **NOT** store or cache data — it only produces location references.
- A Resolver does **NOT** know the Dataset's schema — schema validation against the resolved data is performed by the Resolve Dataset capability after loading.
- A Resolver does **NOT** execute Operations — it is only invoked during data loading.
- The `catalog` strategy returns a raw API response — interpreting that response into a `ResolvedLocation` is the responsibility of the Resolve Dataset capability implementation.

---

## Open Questions

| # | Question | Status |
|---|---|---|
| OQ-001 | Should `when` conditions support environment variables (e.g., `env.DEPLOYMENT_ENV = "prod"`) as context variables in addition to period and table attributes? | Deferred |
| OQ-002 | How does the `catalog` strategy's API response get mapped to a `ResolvedLocation`? Is there a response mapping config on the `catalog` strategy? | Deferred |
| OQ-003 | When period expansion produces child Periods that have no data (e.g., a month in a quarter where no transactions occurred), should the Resolver return an empty location, skip it, or surface it as a warning? | Deferred |

---

## Serialization (YAML DSL)

### Schema

```yaml
resolver:
  id: string                    # required; user-defined; stable across versions
  name: string                  # required; human-readable
  description: string           # optional
  version: integer              # auto-incremented; starts at 1
  status: active | disabled     # required; default: active
  is_default: boolean           # optional; default false; at most one per deployment
  rules:                        # required; at least one; evaluated in order
    - name: string              # required; unique within resolver
      when: <expression>        # optional; boolean; omit for catch-all fallback
      data_level: string        # required; Calendar level name or "any"
      strategy:
        type: path | table | catalog
        # --- path ---
        datasource_id: string
        path: string            # template; supports {{YYYY}}, {{MM}}, {{table_name}}, etc.
        # --- table ---
        datasource_id: string
        table: string           # template
        schema: string          # optional template
        # --- catalog ---
        endpoint: string
        method: GET | POST
        auth: string            # optional; credential ref key
        params:                 # optional; template values
          <key>: <value>
        headers:                # optional
          <key>: <value>
```

### Annotated Example

```yaml
resolver:
  id: "orders-resolver"
  name: "Orders Data Resolver"
  description: "Handles legacy CSV (pre-2025), S3 parquet (2025+), and reference tables from DB"
  version: 3
  status: active
  is_default: false
  rules:

    # Reference table — not period-partitioned; always read from DB
    - name: "Reference tables from database"
      when: "table.name = \"customers\" OR table.name = \"products\""
      data_level: "any"
      strategy:
        type: table
        datasource_id: "ds-postgres"
        table: "{{table_name}}"
        schema: "reference"

    # Legacy CSV on network volume — monthly files before 2025
    - name: "Pre-2025 monthly CSV on volume"
      when: "period.start_date < DATE(\"2025-01-01\")"
      data_level: "month"
      strategy:
        type: path
        datasource_id: "ds-legacy-volume"
        path: "/mnt/data/{{YYYY}}/{{MM}}/{{table_name}}.csv"

    # S3 Parquet — monthly files from 2025 onwards
    - name: "2025+ monthly Parquet on S3"
      when: "period.start_date >= DATE(\"2025-01-01\")"
      data_level: "month"
      strategy:
        type: path
        datasource_id: "ds-s3"
        path: "data/{{YYYY}}/{{table_name}}/{{MM}}.parquet"
```
