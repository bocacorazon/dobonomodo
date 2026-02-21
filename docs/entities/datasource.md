# Entity: DataSource

**Status**: Draft  
**Created**: 2026-02-21  
**Domain**: Data Layer / Connectivity

## Definition

A DataSource is a named, reusable connection definition that describes how to reach an external data store. It captures the connection parameters and optional scope (database schema or file path prefix) for a given source type, and references credentials via an external secrets manager — never storing them directly. A DataSource is the preferred way to define data connections in the system; individual Dataset table references may use it by ID, with only the table-specific detail (table name or relative path) supplied at the reference site. An inline Location remains available for one-off connections.

## Purpose & Role

A DataSource decouples connection configuration from Dataset definitions. Without it, every Dataset would embed full connection details — duplicating credentials references, making connection changes require edits across many Datasets, and creating audit surface for sensitive configuration. By centralising the connection definition, a DataSource enables many Datasets to share a single managed connection point, and allows credential rotation or host changes to be made in one place.

## Attributes

| Attribute | Type | Required | Constraints | Description |
|---|---|---|---|---|
| `id` | `UUID` | Yes | Immutable, system-generated | Unique identifier |
| `name` | `String` | Yes | Non-empty; unique within deployment | Human-readable name (e.g., `"Postgres Warehouse"`, `"S3 Data Lake"`) |
| `description` | `String` | No | — | Optional narrative description |
| `owner` | `User` | Yes | Must reference a valid user | The user responsible for this DataSource |
| `status` | `Enum` | Yes | `active` \| `disabled` | Controls availability for use in Dataset table references |
| `type` | `Enum` | Yes | `database` \| `parquet` \| `csv` \| `api` \| extensible | Source type discriminator; determines which `options` keys are valid |
| `options` | `Map<String, Any>` | Yes | Keys are type-specific; MUST NOT include credentials | Connection parameters excluding credentials (see type-specific options below) |
| `credential_ref` | `String` | No | A key resolvable by the configured external secrets manager | Reference to the external secret containing credentials; resolved at runtime |
| `created_at` | `Timestamp` | Yes | System-set on creation; immutable | Creation time |
| `updated_at` | `Timestamp` | Yes | System-set on every change | Last modification time |

**Type-specific `options`:**

| `type` | Required options | Optional options |
|---|---|---|
| `database` | `connection_string` | `schema` (default schema for all tables in this source) |
| `parquet` | `path_prefix` | — |
| `csv` | `path_prefix` | `delimiter` (default `,`), `has_header` (default `true`) |
| `api` | `endpoint` | `method` (default `GET`), `headers`, `params` |

> `options` MUST NOT contain passwords, tokens, or any credential material. All sensitive values MUST be stored in the external secrets manager and referenced via `credential_ref`.

## How DataSource Integrates with Dataset TableRef

When a Dataset table reference uses a DataSource, only the **table-specific** detail is supplied at the reference site. The DataSource provides the connection context:

| Source type | DataSource provides | TableRef adds |
|---|---|---|
| `database` | `connection_string`, optional `schema` | `table` name (overrides DataSource `schema` if needed) |
| `parquet` | `path_prefix` | relative `path` (appended to prefix) |
| `csv` | `path_prefix`, `delimiter`, `has_header` | relative `path` (appended to prefix) |
| `api` | `endpoint`, `method` | relative `path` (appended to endpoint), per-call `params` |

An inline `location` on a TableRef (no `datasource_id`) remains valid for one-off connections.

## Relationships

| Relationship | Related Entity | Cardinality | Description |
|---|---|---|---|
| referenced by | Dataset (via TableRef) | 1:N | Many Dataset table references may share one DataSource |
| owned by | User | N:1 | Each DataSource has one owner |

## Behaviors & Rules

- **BR-001**: A DataSource MUST NOT store credential material (passwords, tokens, API keys) directly in `options`. All credentials MUST be externalised via `credential_ref`.
- **BR-002**: `credential_ref` is a lookup key only — the DataSource entity is never responsible for resolving or caching the credential value.
- **BR-003**: A `disabled` DataSource MUST NOT be used in new Dataset table references. Existing Dataset definitions that reference a disabled DataSource remain valid but MUST fail at execution time with a clear error.
- **BR-004**: The `type` field is the authoritative discriminator. Any component resolving a DataSource reference MUST use it to determine the connection strategy. Unknown types MUST be treated as an error.
- **BR-005**: When a Dataset TableRef supplies a `datasource_id`, the DataSource's `type` and `options` are merged with the TableRef's table-specific detail at resolution time. The TableRef MAY override `schema`, `delimiter`, `has_header`, or per-call `params` locally.
- **BR-006**: A DataSource with `status: active` referenced by one or more Datasets SHOULD NOT be deleted — disabling is preferred.

## Lifecycle

| State | Description | Transitions To |
|---|---|---|
| `active` | Available for use in new Dataset table references | `disabled` |
| `disabled` | Cannot be used in new Dataset definitions; existing references fail at execution time | `active` (re-enabled) |

**What creates a DataSource**: A user creates it explicitly, providing type, options, and an optional credential reference.  
**What modifies a DataSource**: Changes to name, description, status, options, or credential_ref.  
**What destroys a DataSource**: Open question — see below.

## Boundaries

- A DataSource does NOT catalog tables, schemas, or files within the source — it only defines how to connect.
- A DataSource does NOT resolve or validate credentials — it stores only a reference key.
- A DataSource does NOT own data — it is a connection definition only.
- A DataSource does NOT manage access control to the underlying system — that is governed by the external source's own permissions.

## Open Questions

- [ ] What are the deletion rules? Can a DataSource be hard-deleted if no Dataset table references it?
- [ ] Should the system validate that `credential_ref` resolves to a valid secret at DataSource creation time, or only at execution time?
- [ ] Should DataSource support connection testing (e.g., a "test connection" action to verify reachability before use)?

## Serialization (YAML DSL)

Schema for serializing a DataSource for inter-component communication.

```yaml
# datasource.schema.yaml
datasource:
  id: uuid                        # system-generated, immutable
  name: string                    # required; unique within deployment
  description: string             # optional
  owner: string                   # required; user identifier
  status: active | disabled       # required
  type: database | parquet | csv | api  # required; extensible
  options:                        # required; type-specific; NO credentials
    # database:  connection_string, [schema]
    # parquet:   path_prefix
    # csv:       path_prefix, [delimiter], [has_header]
    # api:       endpoint, [method], [headers], [params]
    <key>: <value>
  credential_ref: string          # optional; secret lookup key resolved at runtime
  created_at: timestamp           # system-set on creation; ISO 8601; immutable
  updated_at: timestamp           # system-set on every change; ISO 8601
```

```yaml
# Example: PostgreSQL warehouse DataSource
datasource:
  id: "ds-0000-0000-0000-000000000001"
  name: "Postgres Warehouse"
  description: "Primary OLAP warehouse on AWS RDS"
  owner: "user-marcos"
  status: active
  type: database
  options:
    connection_string: "postgresql://warehouse.internal:5432/sales"
    schema: public
  credential_ref: "vault://secret/prod/postgres-warehouse"
  created_at: "2026-01-01T00:00:00Z"
  updated_at: "2026-01-01T00:00:00Z"

---

# Example: S3 data lake DataSource (Parquet)
datasource:
  id: "ds-0000-0000-0000-000000000002"
  name: "S3 Data Lake"
  description: "Parquet files on S3 — raw and processed zones"
  owner: "user-marcos"
  status: active
  type: parquet
  options:
    path_prefix: "s3://my-bucket/data/"
  credential_ref: "env://AWS_S3_CREDENTIALS"
  created_at: "2026-01-01T00:00:00Z"
  updated_at: "2026-01-01T00:00:00Z"
```

**Using a DataSource in a Dataset TableRef:**

```yaml
# Instead of inline location:
main_table:
  name: orders
  datasource_id: "ds-0000-0000-0000-000000000001"   # references Postgres Warehouse
  table: orders                                       # table-specific detail only

# Lookup using S3 DataSource:
lookups:
  - alias: region_codes
    target:
      type: table
      name: region_codes
      datasource_id: "ds-0000-0000-0000-000000000002"  # references S3 Data Lake
      path: "reference/region_codes.parquet"            # relative to path_prefix
    join_conditions:
      - source_column: region_id
        target_column: code
```

## Related Entities

- [[Dataset]] — Dataset table references (main table and lookups) use a DataSource by ID as the preferred connection method, with only table-specific detail supplied at the reference site.
