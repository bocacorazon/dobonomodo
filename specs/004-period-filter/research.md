# Research: Period Filter

## 1. Technical Approach

### Polars Filtering
We will use `polars`' `LazyFrame` API for efficient filtering. This allows predicate pushdown optimization.

**Expressions**:
- **Period Mode**: `col("_period").eq(lit(period.identifier))`
- **Bitemporal Mode**:
  ```rust
  col("_period_from").lt_eq(lit(period.start_date))
  .and(
      col("_period_to").is_null()
      .or(col("_period_to").gt(lit(period.start_date)))
  )
  ```
- **Deleted Rows**: `col("_deleted").neq(lit(true))` (handles false and null safely if nullable, though schema should enforce not null)

### Date Handling
- `_period` is `String`.
- `_period_from` and `_period_to` are likely `Date` (NaiveDate) or `Datetime` (DateTime<Utc>).
- We will use `chrono` types compatible with Polars' `AnyValue::Date` or `AnyValue::Datetime`.

## 2. Integration Point
The filtering logic belongs in the `engine-worker` crate (or `core` if shared) where datasets are loaded. It should be applied immediately after scanning the source (e.g. Parquet/CSV) to minimize data loaded into memory.

## 3. Decisions
- **Library**: Use `polars` with `lazy` feature.
- **Location**: Implement as a transformation step in the data loading pipeline.
- **Error Handling**: Return empty frame on no match (inherent in filter), handle missing columns with `LazyFrame::schema()` check if necessary, or let Polars error on execution.
