use std::collections::BTreeMap;

use dobo_core::engine::join::{apply_runtime_joins, resolve_and_load_join};
use dobo_core::model::{Expression, ResolvedLocation, RuntimeJoin};
use polars::prelude::{col, IntoLazy};
use test_resolver::InMemoryDataLoader;
use uuid::Uuid;

use crate::in_memory_metadata_store::InMemoryMetadataStore;
use crate::sample_datasets;

#[test]
fn ts03_fx_conversion_uses_bitemporal_asof_rates() {
    let period = sample_datasets::run_period_2026_01();
    let dataset_id = Uuid::new_v4();
    let dataset = sample_datasets::exchange_rates_dataset(dataset_id, 2);

    let mut loader = InMemoryDataLoader::new();
    loader.seed_table(
        "gl://transactions",
        sample_datasets::gl_transactions_frame(),
    );
    loader.seed_table(
        "fx://exchange_rates",
        sample_datasets::exchange_rates_frame(),
    );

    let working = sample_datasets::gl_transactions_frame().lazy();
    let fx_join = RuntimeJoin {
        alias: "fx".to_string(),
        dataset_id,
        dataset_version: None,
        on: Expression {
            source: "currency = fx.from_currency AND fx.to_currency = 'USD' AND fx.rate_type = 'closing'"
                .to_string(),
        },
    };
    let metadata_store = InMemoryMetadataStore::new().with_dataset(dataset);

    let joined = apply_runtime_joins(
        working,
        std::slice::from_ref(&fx_join),
        "gl",
        &[
            "journal_id".to_string(),
            "currency".to_string(),
            "amount_local".to_string(),
            "_period".to_string(),
        ],
        |_| {
            let mut join_snapshot = Vec::new();
            let (join_lf, _) = resolve_and_load_join(
                &fx_join,
                &BTreeMap::new(),
                "system-default",
                &period,
                &metadata_store,
                |_, resolver_id, _| {
                    Ok(ResolvedLocation {
                        datasource_id: resolver_id.to_string(),
                        path: Some("fx://exchange_rates".to_string()),
                        table: None,
                        schema: None,
                        period_identifier: Some("2026-01".to_string()),
                    })
                },
                &loader,
                &mut join_snapshot,
            )?;

            Ok((
                join_lf,
                vec![
                    "from_currency".to_string(),
                    "to_currency".to_string(),
                    "rate".to_string(),
                    "rate_type".to_string(),
                    "_period_from".to_string(),
                    "_period_to".to_string(),
                ],
            ))
        },
    )
    .expect("fx join should succeed");

    let result = joined
        .with_columns([(col("amount_local") * col("rate_fx")).alias("amount_reporting")])
        .collect()
        .expect("collect conversion results");

    let ids = result
        .column("journal_id")
        .expect("journal_id")
        .str()
        .expect("string ids");
    let amounts = result
        .column("amount_reporting")
        .expect("amount_reporting")
        .f64()
        .expect("f64 amounts");

    let mut actual = std::collections::HashMap::new();
    for (id, amount) in ids.into_no_null_iter().zip(amounts.into_no_null_iter()) {
        actual.insert(id.to_string(), amount);
    }

    let assert_close = |journal_id: &str, expected: f64| {
        let value = actual.get(journal_id).copied().expect("journal id present");
        assert!(
            (value - expected).abs() < 1e-6,
            "{journal_id}: {value} != {expected}"
        );
    };

    assert_close("JE-001", 15000.0);
    assert_close("JE-002", 9282.0);
    assert_close("JE-003", 27962.0);
    assert_close("JE-005", 16800.0);
}
