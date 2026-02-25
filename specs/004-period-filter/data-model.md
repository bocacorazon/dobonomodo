# Data Model: Period Filter

## Entities

### TemporalMode
Enum defining how a table is time-partitioned.
- `Period`: Rows match exactly on `_period` column.
- `Bitemporal`: Rows match on `_period_from` <= start < `_period_to`.

### Period
Struct representing a time period.
- `identifier`: String (e.g., "2024-01")
- `start_date`: Date/Datetime (Start of the period)
- `end_date`: Date/Datetime (End of the period, exclusive)

### Dataset
Existing entity (see `docs/entities/dataset.md`).
- `main_table` has `temporal_mode`.
- `lookups` targets have `temporal_mode`.

## System Columns

### Period Mode
- `_period`: String

### Bitemporal Mode
- `_period_from`: Date/Datetime
- `_period_to`: Date/Datetime (nullable)

### General
- `_deleted`: Boolean
