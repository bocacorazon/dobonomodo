use crate::model::{dataset::TemporalMode, Period};
use chrono::{DateTime, NaiveDate, NaiveDateTime};
use polars::prelude::*;

#[derive(Debug, Clone)]
pub struct FilterContext {
    pub period: Period,
    pub mode: TemporalMode,
}

impl FilterContext {
    pub fn new(period: Period, mode: TemporalMode) -> Self {
        Self { period, mode }
    }
}

pub fn apply_filter(df: LazyFrame, context: &FilterContext) -> PolarsResult<LazyFrame> {
    ensure_columns_exist(&df, &["_deleted"])?;

    // Apply soft-delete filter: _deleted != true (handles false and null)
    // We use strict matching: col("_deleted").neq(lit(true))
    // If _deleted is null, neq(true) is true? No, null != true is null.
    // So we need to handle nulls if _deleted can be null.
    // If _deleted is boolean, null is not true.
    // We want to KEEP rows where _deleted is NOT true.
    // So: _deleted IS NULL OR _deleted = FALSE.
    // Or: NOT (_deleted == TRUE).
    // Polars: col("_deleted").eq(lit(true)).not()
    // If _deleted is null, eq(true) is null. not(null) is null.
    // Filter(null) drops the row.
    // So we need: col("_deleted").eq(lit(true)).fill_null(lit(false)).not()
    // OR: col("_deleted").neq(lit(true)) ? neq propagates null.

    // Safer approach: keep if _deleted is not true.
    // col("_deleted").neq(lit(true)) returns null for nulls.
    // We want nulls to be KEPT (not deleted).
    // So fill_null(false) on the comparison result?
    // col("_deleted").eq(lit(true)).fill_null(lit(false)) -> true if deleted, false if not or null.
    // Then .not() -> false if deleted, true if not or null.

    // But wait, "deleted" usually implies explicit true.
    // Let's assume standard behavior:
    let not_deleted = col("_deleted").eq(lit(true)).fill_null(lit(false)).not();

    let df = df.filter(not_deleted);

    match context.mode {
        TemporalMode::Period => {
            ensure_columns_exist(&df, &["_period"])?;
            apply_period_filter(df, &context.period)
        }
        TemporalMode::Bitemporal => apply_bitemporal_filter(df, &context.period),
        TemporalMode::Snapshot => Ok(df),
    }
}

fn apply_period_filter(df: LazyFrame, period: &Period) -> PolarsResult<LazyFrame> {
    let expr = col("_period").eq(lit(period.identifier.as_str()));
    Ok(df.filter(expr))
}

fn apply_bitemporal_filter(df: LazyFrame, period: &Period) -> PolarsResult<LazyFrame> {
    ensure_columns_exist(&df, &["_period_from", "_period_to"])?;

    // Logic: _period_from <= target_start AND (_period_to IS NULL OR _period_to > target_start)
    let target_date = parse_period_start_date(period.start_date.as_str())?;

    let schema = df.clone().collect_schema()?;
    let from_dtype = schema.get("_period_from").ok_or_else(|| {
        PolarsError::ComputeError("Missing required column '_period_from'".into())
    })?;
    let to_dtype = schema
        .get("_period_to")
        .ok_or_else(|| PolarsError::ComputeError("Missing required column '_period_to'".into()))?;

    let comparison_dtype = resolve_bitemporal_dtype(from_dtype, to_dtype)?;
    let target = target_expr_for_dtype(target_date, &comparison_dtype)?;

    let period_from = col("_period_from").cast(comparison_dtype.clone());
    let period_to = col("_period_to").cast(comparison_dtype);

    let expr = period_from
        .lt_eq(target.clone())
        .and(period_to.clone().is_null().or(period_to.gt(target)));

    Ok(df.filter(expr))
}

fn ensure_columns_exist(df: &LazyFrame, required: &[&str]) -> PolarsResult<()> {
    let schema = df.clone().collect_schema()?;

    for column_name in required {
        if schema.get(column_name).is_none() {
            return Err(PolarsError::ComputeError(
                format!("Missing required column '{column_name}' for filter operation").into(),
            ));
        }
    }

    Ok(())
}

fn parse_period_start_date(value: &str) -> PolarsResult<NaiveDate> {
    if let Ok(date) = NaiveDate::parse_from_str(value, "%Y-%m-%d") {
        return Ok(date);
    }

    if let Ok(datetime) = DateTime::parse_from_rfc3339(value) {
        return Ok(datetime.date_naive());
    }

    if let Ok(datetime) = NaiveDateTime::parse_from_str(value, "%Y-%m-%dT%H:%M:%S") {
        return Ok(datetime.date());
    }

    Err(PolarsError::ComputeError(
        format!(
            "Invalid period start_date '{}' . Expected 'YYYY-MM-DD' or ISO-8601 datetime",
            value
        )
        .into(),
    ))
}

fn resolve_bitemporal_dtype(from_dtype: &DataType, to_dtype: &DataType) -> PolarsResult<DataType> {
    match (from_dtype, to_dtype) {
        (DataType::Date, DataType::Date) => Ok(DataType::Date),
        (DataType::Date, DataType::Datetime(unit, tz))
        | (DataType::Datetime(unit, tz), DataType::Date) => {
            Ok(DataType::Datetime(*unit, tz.clone()))
        }
        (DataType::Datetime(from_unit, from_tz), DataType::Datetime(to_unit, to_tz)) => {
            let unit = higher_precision_time_unit(*from_unit, *to_unit);
            let tz = merge_timezones(from_tz, to_tz)?;
            Ok(DataType::Datetime(unit, tz))
        }
        _ => Err(PolarsError::ComputeError(
            format!(
                "Bitemporal columns must be Date/Datetime. Found _period_from={from_dtype:?}, _period_to={to_dtype:?}"
            )
            .into(),
        )),
    }
}

fn higher_precision_time_unit(left: TimeUnit, right: TimeUnit) -> TimeUnit {
    match (left, right) {
        (TimeUnit::Nanoseconds, _) | (_, TimeUnit::Nanoseconds) => TimeUnit::Nanoseconds,
        (TimeUnit::Microseconds, _) | (_, TimeUnit::Microseconds) => TimeUnit::Microseconds,
        _ => TimeUnit::Milliseconds,
    }
}

fn merge_timezones(
    left: &Option<TimeZone>,
    right: &Option<TimeZone>,
) -> PolarsResult<Option<TimeZone>> {
    match (left, right) {
        (Some(left_tz), Some(right_tz)) if left_tz != right_tz => Err(PolarsError::ComputeError(
            format!(
                "Mismatched datetime timezones in bitemporal columns: '{left_tz}' vs '{right_tz}'"
            )
            .into(),
        )),
        (Some(left_tz), _) => Ok(Some(left_tz.clone())),
        (_, Some(right_tz)) => Ok(Some(right_tz.clone())),
        _ => Ok(None),
    }
}

fn target_expr_for_dtype(target_date: NaiveDate, dtype: &DataType) -> PolarsResult<Expr> {
    match dtype {
        DataType::Date => {
            let epoch = NaiveDate::from_ymd_opt(1970, 1, 1).ok_or_else(|| {
                PolarsError::ComputeError("Failed to construct epoch date".into())
            })?;
            let days_since_epoch = (target_date - epoch).num_days() as i32;
            Ok(lit(days_since_epoch).cast(DataType::Date))
        }
        DataType::Datetime(unit, tz) => {
            let midnight = target_date.and_hms_opt(0, 0, 0).ok_or_else(|| {
                PolarsError::ComputeError("Failed to construct midnight datetime".into())
            })?;

            let timestamp = match unit {
                TimeUnit::Milliseconds => midnight.and_utc().timestamp_millis(),
                TimeUnit::Microseconds => midnight.and_utc().timestamp_micros(),
                TimeUnit::Nanoseconds => {
                    midnight.and_utc().timestamp_nanos_opt().ok_or_else(|| {
                        PolarsError::ComputeError(
                            "Datetime nanosecond conversion overflowed".into(),
                        )
                    })?
                }
            };

            Ok(lit(timestamp).cast(DataType::Datetime(*unit, tz.clone())))
        }
        _ => Err(PolarsError::ComputeError(
            format!("Unsupported target dtype for bitemporal filtering: {dtype:?}").into(),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::period::PeriodStatus;
    use uuid::Uuid;

    fn create_dummy_period(identifier: &str) -> Period {
        Period {
            id: Uuid::now_v7(),
            identifier: identifier.to_string(),
            name: "Test Period".to_string(),
            description: None,
            calendar_id: Uuid::now_v7(),
            year: 2024,
            sequence: 1,
            start_date: "2024-01-01".to_string(),
            end_date: "2024-02-01".to_string(),
            status: PeriodStatus::Open,
            parent_id: None,
            created_at: None,
            updated_at: None,
        }
    }

    #[test]
    fn test_apply_period_filter_match() {
        let period = create_dummy_period("2024-01");

        let df = df!(
            "id" => &[1, 2, 3],
            "_period" => &["2024-01", "2024-02", "2024-01"]
        )
        .unwrap()
        .lazy();

        let filtered = apply_period_filter(df, &period).unwrap().collect().unwrap();

        assert_eq!(filtered.height(), 2);
        let periods: Vec<String> = filtered
            .column("_period")
            .unwrap()
            .str()
            .unwrap()
            .into_no_null_iter()
            .map(|s| s.to_string())
            .collect();
        assert_eq!(periods, vec!["2024-01", "2024-01"]);
    }

    #[test]
    fn test_apply_period_filter_no_match() {
        let period = create_dummy_period("2024-03");

        let df = df!(
            "id" => &[1, 2, 3],
            "_period" => &["2024-01", "2024-02", "2024-01"]
        )
        .unwrap()
        .lazy();

        let filtered = apply_period_filter(df, &period).unwrap().collect().unwrap();

        assert_eq!(filtered.height(), 0);
    }

    #[test]
    fn test_apply_bitemporal_filter() {
        use chrono::NaiveDate;

        fn date_to_days(y: i32, m: u32, d: u32) -> i32 {
            let date = NaiveDate::from_ymd_opt(y, m, d).unwrap();
            let epoch = NaiveDate::from_ymd_opt(1970, 1, 1).unwrap();
            (date - epoch).num_days() as i32
        }

        // Case 1: _period_from <= target < _period_to
        // Case 2: _period_from <= target AND _period_to IS NULL
        // Case 3: _period_from > target (invalid)
        // Case 4: _period_to <= target (invalid - expired)

        let period = create_dummy_period("2024-01"); // Start date: "2024-01-01"

        // _period_from
        let days0 = vec![
            date_to_days(2023, 1, 1),
            date_to_days(2023, 1, 1),
            date_to_days(2024, 2, 1),
            date_to_days(2023, 1, 1),
        ];
        let s_from = Series::new("_period_from".into(), days0)
            .cast(&DataType::Date)
            .unwrap();

        // _period_to
        let days1 = vec![
            Some(date_to_days(2024, 2, 1)),
            None,
            None,
            Some(date_to_days(2024, 1, 1)),
        ];
        let s_to = Series::new("_period_to".into(), days1)
            .cast(&DataType::Date)
            .unwrap();

        let s_id = Series::new("id".into(), &[1, 2, 3, 4]);

        let df = DataFrame::new(vec![s_id.into(), s_from.into(), s_to.into()])
            .unwrap()
            .lazy();

        // Target: 2024-01-01
        // Row 1: 2023-01-01 <= 2024-01-01 < 2024-02-01 -> MATCH
        // Row 2: 2023-01-01 <= 2024-01-01 AND NULL -> MATCH
        // Row 3: 2024-02-01 > 2024-01-01 -> NO MATCH (Future)
        // Row 4: 2023-01-01 <= 2024-01-01 BUT 2024-01-01 !> 2024-01-01 -> NO MATCH

        let filtered = apply_bitemporal_filter(df, &period)
            .unwrap()
            .collect()
            .unwrap();

        let ids: Vec<i32> = filtered
            .column("id")
            .unwrap()
            .i32()
            .unwrap()
            .into_no_null_iter()
            .collect();
        let mut ids = ids;
        ids.sort();

        assert_eq!(ids, vec![1, 2]);
    }

    #[test]
    fn test_apply_filter_deleted_rows() {
        let period = create_dummy_period("2024-01");

        let df = df!(
            "id" => &[1, 2, 3, 4],
            "_period" => &["2024-01", "2024-01", "2024-01", "2024-01"],
            "_deleted" => &[false, true, false, true]
        )
        .unwrap()
        .lazy();

        let context = FilterContext::new(period, TemporalMode::Period);

        let filtered = apply_filter(df, &context).unwrap().collect().unwrap();

        let ids: Vec<i32> = filtered
            .column("id")
            .unwrap()
            .i32()
            .unwrap()
            .into_no_null_iter()
            .collect();
        let mut ids = ids;
        ids.sort();

        assert_eq!(ids, vec![1, 3]);
    }

    #[test]
    fn test_apply_filter_deleted_rows_null() {
        let period = create_dummy_period("2024-01");

        let df = df!(
            "id" => &[1, 2],
            "_period" => &["2024-01", "2024-01"],
            "_deleted" => &[None::<bool>, Some(false)]
        )
        .unwrap()
        .lazy();

        let context = FilterContext::new(period, TemporalMode::Period);

        let filtered = apply_filter(df, &context).unwrap().collect().unwrap();

        assert_eq!(filtered.height(), 2);
    }

    #[test]
    fn test_apply_bitemporal_filter_datetime_columns() {
        fn date_to_millis(y: i32, m: u32, d: u32) -> i64 {
            NaiveDate::from_ymd_opt(y, m, d)
                .unwrap()
                .and_hms_opt(0, 0, 0)
                .unwrap()
                .and_utc()
                .timestamp_millis()
        }

        let period = create_dummy_period("2024-01");

        let s_from = Series::new(
            "_period_from".into(),
            vec![
                date_to_millis(2023, 1, 1),
                date_to_millis(2023, 1, 1),
                date_to_millis(2024, 2, 1),
                date_to_millis(2023, 1, 1),
            ],
        )
        .cast(&DataType::Datetime(TimeUnit::Milliseconds, None))
        .unwrap();

        let s_to = Series::new(
            "_period_to".into(),
            vec![
                Some(date_to_millis(2024, 2, 1)),
                None,
                None,
                Some(date_to_millis(2024, 1, 1)),
            ],
        )
        .cast(&DataType::Datetime(TimeUnit::Milliseconds, None))
        .unwrap();

        let s_id = Series::new("id".into(), &[1, 2, 3, 4]);

        let df = DataFrame::new(vec![s_id.into(), s_from.into(), s_to.into()])
            .unwrap()
            .lazy();

        let filtered = apply_bitemporal_filter(df, &period)
            .unwrap()
            .collect()
            .unwrap();

        let ids: Vec<i32> = filtered
            .column("id")
            .unwrap()
            .i32()
            .unwrap()
            .into_no_null_iter()
            .collect();
        let mut ids = ids;
        ids.sort();

        assert_eq!(ids, vec![1, 2]);
    }

    #[test]
    fn test_apply_bitemporal_filter_accepts_rfc3339_period_start() {
        use chrono::NaiveDate;

        fn date_to_days(y: i32, m: u32, d: u32) -> i32 {
            let date = NaiveDate::from_ymd_opt(y, m, d).unwrap();
            let epoch = NaiveDate::from_ymd_opt(1970, 1, 1).unwrap();
            (date - epoch).num_days() as i32
        }

        let mut period = create_dummy_period("2024-01");
        period.start_date = "2024-01-01T00:00:00Z".to_string();

        let s_from = Series::new(
            "_period_from".into(),
            vec![date_to_days(2023, 1, 1), date_to_days(2024, 2, 1)],
        )
        .cast(&DataType::Date)
        .unwrap();

        let s_to = Series::new("_period_to".into(), vec![None::<i32>, None::<i32>])
            .cast(&DataType::Date)
            .unwrap();

        let s_id = Series::new("id".into(), &[1, 2]);

        let df = DataFrame::new(vec![s_id.into(), s_from.into(), s_to.into()])
            .unwrap()
            .lazy();

        let filtered = apply_bitemporal_filter(df, &period)
            .unwrap()
            .collect()
            .unwrap();

        let ids: Vec<i32> = filtered
            .column("id")
            .unwrap()
            .i32()
            .unwrap()
            .into_no_null_iter()
            .collect();

        assert_eq!(ids, vec![1]);
    }

    #[test]
    fn test_apply_filter_missing_deleted_column_returns_descriptive_error() {
        let period = create_dummy_period("2024-01");
        let context = FilterContext::new(period, TemporalMode::Period);

        let df = df!(
            "id" => &[1, 2],
            "_period" => &["2024-01", "2024-01"]
        )
        .unwrap()
        .lazy();

        let error = match apply_filter(df, &context) {
            Ok(_) => panic!("Expected missing _deleted column to return an error"),
            Err(error) => error,
        };

        assert!(error
            .to_string()
            .contains("Missing required column '_deleted' for filter operation"));
    }
}
