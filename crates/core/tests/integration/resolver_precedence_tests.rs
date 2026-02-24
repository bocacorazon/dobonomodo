use std::collections::BTreeMap;

use anyhow::{anyhow, Result};
use dobo_core::engine::join::{resolve_and_load_join, ResolverSource};
use dobo_core::model::{Expression, ResolvedLocation, RuntimeJoin, TableRef};
use dobo_core::DataLoader;
use polars::prelude::{DataFrame, IntoLazy, LazyFrame};
use uuid::Uuid;

use crate::in_memory_metadata_store::InMemoryMetadataStore;
use crate::sample_datasets;

#[derive(Default)]
struct InMemoryLoader {
    frame: Option<DataFrame>,
}

impl DataLoader for InMemoryLoader {
    fn load(&self, _location: &ResolvedLocation, _schema: &TableRef) -> Result<LazyFrame> {
        let frame = self
            .frame
            .as_ref()
            .ok_or_else(|| anyhow!("missing frame"))?
            .clone();
        Ok(frame.lazy())
    }
}

#[test]
fn project_override_takes_precedence_over_dataset_resolver() {
    let dataset_id = Uuid::new_v4();
    let dataset = sample_datasets::exchange_rates_dataset(dataset_id, 1);
    let join = RuntimeJoin {
        alias: "fx".to_string(),
        dataset_id,
        dataset_version: None,
        on: Expression {
            source: "currency = fx.from_currency".to_string(),
        },
    };

    let period = sample_datasets::run_period_2026_01();
    let mut overrides = BTreeMap::new();
    overrides.insert(dataset_id, "project-test-resolver".to_string());

    let mut join_snapshot = Vec::new();
    let store = InMemoryMetadataStore::new().with_dataset(dataset);
    let loader = InMemoryLoader {
        frame: Some(sample_datasets::exchange_rates_frame()),
    };

    let (_, resolved) = resolve_and_load_join(
        &join,
        &overrides,
        "system-default-resolver",
        &period,
        &store,
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
    )
    .expect("join resolution should succeed");

    assert_eq!(resolved.resolver_id, "project-test-resolver");
    assert_eq!(resolved.resolver_source, ResolverSource::ProjectOverride);
}
