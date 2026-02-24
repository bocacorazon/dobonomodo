use dobo_core::engine::period_filter::apply_period_filter;
use dobo_core::model::TemporalMode;
use polars::prelude::{ChunkCompareEq, IntoLazy};

use crate::sample_datasets;

#[test]
fn bitemporal_asof_selects_2026_01_rates() {
    let period = sample_datasets::run_period_2026_01();

    let result = apply_period_filter(
        sample_datasets::exchange_rates_frame().lazy(),
        &TemporalMode::Bitemporal,
        &period,
    )
    .expect("filter should succeed")
    .collect()
    .expect("collect filtered rates");

    let eur_mask = result
        .column("from_currency")
        .expect("from_currency")
        .str()
        .expect("currency strings")
        .equal("EUR");

    let eur_rows = result.filter(&eur_mask).expect("filter eur");
    let eur_rate = eur_rows
        .column("rate")
        .expect("rate column")
        .f64()
        .expect("f64 rate")
        .get(0);

    assert_eq!(eur_rate, Some(1.0920));
}
