//! Period filtering for join datasets based on temporal_mode
//!
//! This module applies period-aware filtering to join datasets:
//! - Period mode: exact match on _period column
//! - Bitemporal mode: asOf query on _period_from/_period_to

use anyhow::Result;
use polars::prelude::*;

use crate::model::{Period, TemporalMode};

fn log_filter_stats(_filtered: &LazyFrame, temporal_mode: &str, period: &str) {
    eprintln!(
        "period filter applied: mode={} period={} lazy=true",
        temporal_mode, period
    );
}

/// Apply period-based filtering to a LazyFrame based on temporal mode
///
/// # Arguments
/// * `lf` - The LazyFrame to filter
/// * `temporal_mode` - The temporal mode of the dataset (Period or Bitemporal)
/// * `period` - The Period to filter for
///
/// # Returns
/// * Filtered LazyFrame with rows matching the period criteria
///
/// # Period Mode Filtering
/// Filters where `_period == period.identifier`
///
/// # Bitemporal Mode Filtering
/// AsOf query: `_period_from <= period.start_date AND (_period_to IS NULL OR _period_to > period.start_date)`
pub fn apply_period_filter(
    lf: LazyFrame,
    temporal_mode: &TemporalMode,
    period: &Period,
) -> Result<LazyFrame> {
    match temporal_mode {
        TemporalMode::Period => {
            let filtered = lf.filter(col("_period").eq(lit(period.identifier.clone())));
            log_filter_stats(&filtered, "period", &period.identifier);
            Ok(filtered)
        }
        TemporalMode::Bitemporal => {
            let start_date = period.start_date.clone();

            let from_condition = col("_period_from").lt_eq(lit(start_date.clone()));
            let to_condition = col("_period_to")
                .is_null()
                .or(col("_period_to").gt(lit(start_date)));

            let filtered = lf.filter(from_condition.and(to_condition));
            log_filter_stats(&filtered, "bitemporal", &period.start_date);
            Ok(filtered)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    fn make_period(identifier: &str, start_date: &str, end_date: &str) -> Period {
        use crate::model::PeriodStatus;
        Period {
            id: Uuid::new_v4(),
            identifier: identifier.to_string(),
            name: identifier.to_string(),
            description: None,
            calendar_id: Uuid::new_v4(),
            year: 2026,
            sequence: 1,
            start_date: start_date.to_string(),
            end_date: end_date.to_string(),
            status: PeriodStatus::Open,
            parent_id: None,
            created_at: None,
            updated_at: None,
        }
    }

    #[test]
    fn test_period_mode_exact_match() {
        let df = df! {
            "_period" => &["2026-01", "2026-02", "2026-01", "2026-03"],
            "amount" => &[100, 200, 300, 400]
        }
        .unwrap();

        let lf = df.lazy();
        let period = make_period("2026-01", "2026-01-01", "2026-01-31");

        let filtered = apply_period_filter(lf, &TemporalMode::Period, &period).unwrap();
        let result = filtered.collect().unwrap();

        assert_eq!(result.height(), 2);
    }

    #[test]
    fn test_bitemporal_mode_asof() {
        let df = df! {
            "_period_from" => &["2025-12-01", "2026-01-01", "2026-01-15", "2026-02-01"],
            "_period_to" => &[Some("2026-01-01"), None, Some("2026-01-20"), None],
            "rate" => &[1.08, 1.09, 1.10, 1.11]
        }
        .unwrap();

        let lf = df.lazy();
        let period = make_period("2026-01", "2026-01-01", "2026-01-31");

        let filtered = apply_period_filter(lf, &TemporalMode::Bitemporal, &period).unwrap();
        let result = filtered.collect().unwrap();

        assert_eq!(result.height(), 1);
    }

    #[test]
    fn test_bitemporal_null_period_to() {
        let df = df! {
            "_period_from" => &["2025-01-01", "2026-01-01"],
            "_period_to" => &[None::<&str>, None::<&str>],
            "value" => &[100, 200]
        }
        .unwrap();

        let lf = df.lazy();
        let period = make_period("2026-01", "2026-01-01", "2026-01-31");

        let filtered = apply_period_filter(lf, &TemporalMode::Bitemporal, &period).unwrap();
        let result = filtered.collect().unwrap();

        assert_eq!(result.height(), 2);
    }
}
