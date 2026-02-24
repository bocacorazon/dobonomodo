use dobo_core::engine::period_filter::apply_period_filter;
use dobo_core::model::{Period, PeriodStatus, TemporalMode};
use polars::prelude::*;
use uuid::Uuid;

fn period_2026_01() -> Period {
    Period {
        id: Uuid::new_v4(),
        identifier: "2026-01".to_string(),
        name: "2026-01".to_string(),
        description: None,
        calendar_id: Uuid::new_v4(),
        year: 2026,
        sequence: 1,
        start_date: "2026-01-01".to_string(),
        end_date: "2026-01-31".to_string(),
        status: PeriodStatus::Open,
        parent_id: None,
        created_at: None,
        updated_at: None,
    }
}

#[test]
fn bitemporal_filter_selects_current_versions() {
    let df = df! {
        "currency" => &["EUR", "EUR", "GBP", "GBP"],
        "_period_from" => &["2025-01-01", "2026-01-01", "2025-01-01", "2026-01-01"],
        "_period_to" => &[Some("2026-01-01"), None, Some("2026-01-01"), None],
        "rate" => &[1.0850_f64, 1.0920, 1.2650, 1.2710],
    }
    .expect("valid bitemporal data");

    let result = apply_period_filter(df.lazy(), &TemporalMode::Bitemporal, &period_2026_01())
        .expect("filter should succeed")
        .collect()
        .expect("collect filtered data");

    assert_eq!(result.height(), 2);

    let eur_mask = result
        .column("currency")
        .expect("currency column")
        .str()
        .expect("string currency")
        .equal("EUR");
    let eur = result.filter(&eur_mask).expect("filter eur");
    let eur_rate = eur
        .column("rate")
        .expect("rate col")
        .f64()
        .expect("f64 rate")
        .get(0);
    assert_eq!(eur_rate, Some(1.0920));
}

#[test]
fn period_filter_exact_match_only() {
    let df = df! {
        "_period" => &["2026-01", "2026-02", "2025-12"],
        "value" => &[1_i32, 2, 3],
    }
    .expect("valid period data");

    let result = apply_period_filter(df.lazy(), &TemporalMode::Period, &period_2026_01())
        .expect("filter should succeed")
        .collect()
        .expect("collect filtered data");

    assert_eq!(result.height(), 1);
    let values = result
        .column("value")
        .expect("value column")
        .i32()
        .expect("int values");
    assert_eq!(values.get(0), Some(1));
}
