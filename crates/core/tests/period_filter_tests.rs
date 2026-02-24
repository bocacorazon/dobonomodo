//! Unit tests for period-based filtering
//! Tests cover both period mode (exact match) and bitemporal mode (asOf query)

use dobo_core::engine::period_filter::*;
use dobo_core::model::{Period, PeriodStatus, TemporalMode};
use polars::prelude::*;
use uuid::Uuid;

fn make_period(identifier: &str, start_date: &str, end_date: &str) -> Period {
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
    // T015: Period filter with period mode (exact match on _period)
    let df = df! {
        "_period" => &["2026-01", "2026-02", "2026-01", "2026-03"],
        "amount" => &[100, 200, 300, 400]
    }
    .unwrap();

    let lf = df.lazy();
    let period = make_period("2026-01", "2026-01-01", "2026-01-31");

    let filtered = apply_period_filter(lf, &TemporalMode::Period, &period).unwrap();
    let result = filtered.collect().unwrap();

    assert_eq!(result.height(), 2); // Should match 2 rows with "2026-01"

    // Verify amounts
    let amounts = result.column("amount").unwrap().i32().unwrap();
    let sum: i32 = amounts.into_no_null_iter().sum();
    assert_eq!(sum, 400); // 100 + 300
}

#[test]
fn test_bitemporal_mode_asof_query() {
    // T014: Period filter with bitemporal mode (asOf query)
    // Testing: _period_from <= start_date AND (_period_to IS NULL OR _period_to > start_date)
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

    // Expected matches:
    // Row 0: _period_from=2025-12-01, _period_to=2026-01-01 -> NO (_period_to NOT > 2026-01-01)
    // Row 1: _period_from=2026-01-01, _period_to=NULL -> YES
    // Row 2: _period_from=2026-01-15, _period_to=2026-01-20 -> NO (_period_from > 2026-01-01)
    // Row 3: _period_from=2026-02-01, _period_to=NULL -> NO (_period_from > 2026-01-01)
    assert_eq!(result.height(), 1);

    // Verify the correct rate
    let rates = result.column("rate").unwrap().f64().unwrap();
    assert_eq!(rates.get(0), Some(1.09));
}

#[test]
fn test_bitemporal_null_period_to() {
    // Bitemporal with NULL _period_to (ongoing validity)
    let df = df! {
        "_period_from" => &["2025-01-01", "2026-01-01", "2026-02-01"],
        "_period_to" => &[None::<&str>, None::<&str>, None::<&str>],
        "value" => &[100, 200, 300]
    }
    .unwrap();

    let lf = df.lazy();
    let period = make_period("2026-01", "2026-01-01", "2026-01-31");

    let filtered = apply_period_filter(lf, &TemporalMode::Bitemporal, &period).unwrap();
    let result = filtered.collect().unwrap();

    // Rows with _period_from <= 2026-01-01 and _period_to IS NULL
    // Row 0: 2025-01-01 <= 2026-01-01 -> YES
    // Row 1: 2026-01-01 <= 2026-01-01 -> YES
    // Row 2: 2026-02-01 > 2026-01-01 -> NO
    assert_eq!(result.height(), 2);
}

#[test]
fn test_bitemporal_historical_rate_selection() {
    // Test correct rate selection for historical period with multiple versions
    // Scenario: Exchange rates updated on 2026-01-01, selecting for asOf 2026-01-01
    let df = df! {
        "currency" => &["EUR", "EUR", "GBP", "GBP"],
        "_period_from" => &["2025-01-01", "2026-01-01", "2025-01-01", "2026-01-01"],
        "_period_to" => &[Some("2026-01-01"), None, Some("2026-01-01"), None],
        "rate" => &[1.0850, 1.0920, 1.2650, 1.2710]
    }
    .unwrap();

    let lf = df.lazy();
    let period = make_period("2026-01", "2026-01-01", "2026-01-31");

    let filtered = apply_period_filter(lf, &TemporalMode::Bitemporal, &period).unwrap();
    let result = filtered.collect().unwrap();

    // Should select current rates (from 2026-01-01 with NULL _period_to)
    assert_eq!(result.height(), 2);

    // Verify EUR rate is 1.0920 (new rate)
    let eur_mask = result
        .column("currency")
        .unwrap()
        .str()
        .unwrap()
        .equal("EUR");
    let eur_row = result.filter(&eur_mask).unwrap();
    let eur_rate = eur_row.column("rate").unwrap().f64().unwrap();
    assert_eq!(eur_rate.get(0), Some(1.0920));

    // Verify GBP rate is 1.2710 (new rate)
    let gbp_mask = result
        .column("currency")
        .unwrap()
        .str()
        .unwrap()
        .equal("GBP");
    let gbp_row = result.filter(&gbp_mask).unwrap();
    let gbp_rate = gbp_row.column("rate").unwrap().f64().unwrap();
    assert_eq!(gbp_rate.get(0), Some(1.2710));
}

#[test]
fn test_period_mode_no_matches() {
    // Period mode with no matching rows
    let df = df! {
        "_period" => &["2026-02", "2026-03"],
        "amount" => &[100, 200]
    }
    .unwrap();

    let lf = df.lazy();
    let period = make_period("2026-01", "2026-01-01", "2026-01-31");

    let filtered = apply_period_filter(lf, &TemporalMode::Period, &period).unwrap();
    let result = filtered.collect().unwrap();

    assert_eq!(result.height(), 0);
}

#[test]
fn test_bitemporal_asof_boundary_conditions() {
    // Test boundary conditions for asOf query
    // _period_from exactly equals period.start_date
    let df = df! {
        "_period_from" => &["2026-01-01", "2026-01-02"],
        "_period_to" => &[None::<&str>, None::<&str>],
        "value" => &[100, 200]
    }
    .unwrap();

    let lf = df.lazy();
    let period = make_period("2026-01", "2026-01-01", "2026-01-31");

    let filtered = apply_period_filter(lf, &TemporalMode::Bitemporal, &period).unwrap();
    let result = filtered.collect().unwrap();

    // Row with _period_from = 2026-01-01 should match (_period_from <= 2026-01-01)
    assert_eq!(result.height(), 1);
    let values = result.column("value").unwrap().i32().unwrap();
    assert_eq!(values.get(0), Some(100));
}
