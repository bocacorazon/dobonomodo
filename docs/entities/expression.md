# Entity: Expression

**Status**: Draft  
**Created**: 2026-02-21  
**Domain**: Computation / DSL

## Definition

An Expression is an inline, composable formula written in the DobONoMoDo DSL that evaluates to a typed value. Expressions are **not** named or stored independently — they exist only as parameters embedded inside Operation instances within a Project. Expressions combine column references, literal values, function calls, and infix operators to express boolean conditions, arithmetic computations, string transformations, date/time calculations, and aggregations. The engine compiles each Expression into executable instructions at Run time.

---

## Attributes

| Attribute | Type | Required | Notes |
|---|---|---|---|
| `source` | String | Yes | Raw expression text as authored by the user |
| `return_type` | Enum | Inferred | `boolean \| number \| string \| date \| null` — inferred by the engine at compile time |

Expressions carry no `id` or `version` — they are value objects owned by their enclosing OperationInstance.

---

## Syntax

### Column References

Columns are referenced using the **Dataset's logical table name** followed by the column name, separated by a dot:

```
logical_table_name.column_name
```

Examples:
- `orders.amount`
- `customer.country_code`
- `products.unit_price`

The logical table name matches the alias assigned to each `TableRef` in the Dataset definition.

### Literals

| Type | Syntax Examples |
|---|---|
| Number | `42`, `3.14`, `-7` |
| String | `"active"`, `"USD"` |
| Boolean | `TRUE`, `FALSE` |
| Date | `DATE("2026-01-01")` |
| Null | `NULL` |

### Infix Operators

| Category | Operators |
|---|---|
| Arithmetic | `+`, `-`, `*`, `/` |
| Comparison | `=`, `<>`, `<`, `<=`, `>`, `>=` |
| Logical | `AND`, `OR`, `NOT` |

Operator precedence follows standard mathematical convention. Parentheses override precedence.

### Functions

Functions use Excel-style call syntax: `FUNCTION_NAME(arg1, arg2, ...)`.

#### Boolean / Conditional
| Function | Description |
|---|---|
| `IF(condition, value_if_true, value_if_false)` | Conditional branch |
| `AND(expr1, expr2, ...)` | Logical AND of all arguments |
| `OR(expr1, expr2, ...)` | Logical OR of all arguments |
| `NOT(expr)` | Logical negation |
| `ISNULL(expr)` | Returns TRUE if expr is NULL |
| `COALESCE(expr1, expr2, ...)` | Returns first non-NULL argument |

#### Arithmetic / Math
| Function | Description |
|---|---|
| `ABS(number)` | Absolute value |
| `ROUND(number, decimals)` | Round to N decimal places |
| `FLOOR(number)` | Round down to nearest integer |
| `CEIL(number)` | Round up to nearest integer |
| `MOD(number, divisor)` | Remainder after division |
| `MIN(a, b)` | Smaller of two values |
| `MAX(a, b)` | Larger of two values |

#### String
| Function | Description |
|---|---|
| `CONCAT(str1, str2, ...)` | Concatenate strings |
| `UPPER(str)` | Convert to uppercase |
| `LOWER(str)` | Convert to lowercase |
| `TRIM(str)` | Remove leading/trailing whitespace |
| `LEFT(str, n)` | First N characters |
| `RIGHT(str, n)` | Last N characters |
| `LEN(str)` | Character count |
| `CONTAINS(str, substr)` | Returns TRUE if substr is found |
| `REPLACE(str, old, new)` | Replace all occurrences |

#### Date / Time
| Function | Description |
|---|---|
| `DATE(iso_string)` | Parse ISO 8601 date literal |
| `TODAY()` | Current date at Run start (captured in snapshot) |
| `YEAR(date)` | Extract year as number |
| `MONTH(date)` | Extract month (1–12) as number |
| `DAY(date)` | Extract day of month as number |
| `DATEDIFF(end_date, start_date)` | Difference in days (end − start) |
| `DATEADD(date, n_days)` | Add N days to a date |

#### Aggregate
| Function | Description |
|---|---|
| `SUM(column_ref)` | Sum of all values in column |
| `COUNT(column_ref)` | Count of non-NULL values |
| `COUNT_ALL()` | Count of all rows (including NULLs) |
| `AVG(column_ref)` | Arithmetic mean |
| `MIN_AGG(column_ref)` | Minimum value in column |
| `MAX_AGG(column_ref)` | Maximum value in column |

> Aggregate functions are valid only inside Operations that define a grouping context (e.g., a Rollup or Summarise operation). Using an aggregate outside such a context is a compile-time error.

---

## NULL Handling

NULLs propagate explicitly:

- Any arithmetic involving `NULL` yields `NULL` (e.g., `orders.amount + NULL → NULL`)
- Any comparison involving `NULL` yields `NULL` (e.g., `orders.amount = NULL → NULL`, not `TRUE`)
- Boolean operators treat `NULL` as unknown: `TRUE AND NULL → NULL`, `FALSE OR NULL → NULL`
- Use `ISNULL(expr)` to test for NULL; use `COALESCE(expr, fallback)` to substitute a default

---

## Behaviors / Rules

| ID | Rule |
|---|---|
| BR-001 | Column references MUST match a logical table name defined in the enclosing Operation's Dataset. Unknown references are a compile-time error. |
| BR-002 | The engine infers `return_type` at compile time. A type mismatch between an Expression's return type and its expected parameter type is a compile-time error. |
| BR-003 | Aggregate functions are only valid within an Operation that defines a grouping context. Using them outside such a context is a compile-time error. |
| BR-004 | Division by zero (`x / 0`) produces NULL at runtime (not an error). |
| BR-005 | `TODAY()` is resolved to the Run's `started_at` timestamp, ensuring reproducible execution. It is NOT re-evaluated live. |
| BR-006 | Expressions are always inline. They cannot be extracted, named, or shared across Operations. |
| BR-007 | Expressions are compiled and type-checked when a Project transitions from `draft` to `active`. A Project with expression errors cannot be activated. |

---

## Lifecycle

Expressions have no independent lifecycle. They are created and destroyed with the OperationInstance that contains them. When a Project is snapshotted into a Run's `ProjectSnapshot`, all expression `source` strings are captured verbatim.

---

## Relationships

| Entity | Relationship |
|---|---|
| OperationInstance | Expression is embedded as a parameter value within an OperationInstance |
| Dataset | Column references resolve against the logical table names of the Project's input Dataset |
| Run | At execution, expressions are compiled from the ProjectSnapshot and evaluated by the engine |

---

## Boundaries (What This Is Not)

- An Expression is **not** a stored formula library — there is no named formula registry.
- An Expression is **not** a template — it cannot be parameterised with variables beyond the column references it contains.
- An Expression is **not** a script — arbitrary control flow (loops, variable assignment) is out of scope.
- Aggregate functions are **not** window functions — there is no `OVER(PARTITION BY ...)` syntax in v1.

---

## Open Questions

| # | Question | Status |
|---|---|---|
| OQ-001 | Should window/ranking functions (e.g., `ROW_NUMBER()`, `RANK()`) be added in a future version? | Deferred |
| OQ-002 | Should cross-table aggregates be supported (e.g., `SUM` over a lookup table's column)? | Deferred |
| OQ-003 | Should string-to-number and number-to-string coercions be implicit or require explicit cast functions? | Deferred |

---

## Serialization (YAML DSL)

### Schema

```yaml
# Expressions appear inline as string values inside operation parameters.
# They are not serialized as standalone objects.

expression: <string>
# A quoted formula string. May use:
#   - Column refs:     logical_table.column_name
#   - Literals:        42, "text", TRUE, NULL, DATE("2026-01-01")
#   - Infix ops:       +  -  *  /  =  <>  <  <=  >  >=  AND  OR  NOT
#   - Functions:       IF, SUM, CONCAT, DATEDIFF, COALESCE, etc.
```

### Annotated Example

```yaml
# Inside an operation that filters rows:
operation: filter
parameters:
  condition: "orders.status = \"active\" AND NOT ISNULL(orders.amount)"

# Inside an operation that adds a computed column:
operation: add_column
parameters:
  name: "discounted_price"
  expression: "ROUND(products.unit_price * (1 - orders.discount_rate), 2)"

# Inside a rollup operation with an aggregate:
operation: rollup
parameters:
  group_by:
    - "orders.region"
    - "orders.period"
  aggregations:
    - column: "total_sales"
      expression: "SUM(orders.amount)"
    - column: "order_count"
      expression: "COUNT(orders.order_id)"

# NULL handling example:
operation: add_column
parameters:
  name: "effective_price"
  expression: "COALESCE(orders.override_price, products.unit_price)"

# Date arithmetic example:
operation: add_column
parameters:
  name: "days_overdue"
  expression: "IF(ISNULL(orders.paid_at), DATEDIFF(TODAY(), orders.due_at), 0)"
```
