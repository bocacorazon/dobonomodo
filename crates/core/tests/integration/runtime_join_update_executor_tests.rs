use std::collections::BTreeMap;

use dobo_core::engine::update::execute_update_operation;
use dobo_core::model::{
    Materialization, OperationInstance, OperationKind, ProjectSnapshot, ResolutionStrategy,
    ResolvedLocation, Resolver, ResolverStatus, Run, RunStatus, TriggerType,
};
use polars::prelude::IntoLazy;
use serde_json::json;
use test_resolver::InMemoryDataLoader;
use uuid::Uuid;

use crate::in_memory_metadata_store::InMemoryMetadataStore;
use crate::sample_datasets;

fn test_run() -> Run {
    Run {
        id: Uuid::new_v4(),
        project_id: Uuid::new_v4(),
        project_version: 1,
        project_snapshot: ProjectSnapshot {
            input_dataset_id: Uuid::new_v4(),
            input_dataset_version: 1,
            materialization: Materialization::Runtime,
            operations: vec![],
            resolver_snapshots: vec![],
        },
        period_ids: vec![Uuid::new_v4()],
        status: RunStatus::Queued,
        trigger_type: TriggerType::Manual,
        triggered_by: "tests".to_string(),
        last_completed_operation: None,
        output_dataset_id: None,
        parent_run_id: None,
        error: None,
        started_at: None,
        completed_at: None,
        created_at: None,
    }
}

fn test_resolver(id: &str, version: i32) -> Resolver {
    Resolver {
        id: id.to_string(),
        name: id.to_string(),
        description: None,
        version,
        status: ResolverStatus::Active,
        is_default: None,
        rules: vec![dobo_core::model::ResolutionRule {
            name: "fallback".to_string(),
            when_expression: None,
            data_level: "any".to_string(),
            strategy: ResolutionStrategy::Path {
                datasource_id: "fx".to_string(),
                path: "fx://exchange_rates".to_string(),
            },
        }],
        created_at: None,
        updated_at: None,
    }
}

#[test]
fn runtime_join_executes_via_update_operation_entrypoint_with_object_on_shape() {
    assert_runtime_join_executes(json!({
        "source": "currency = fx.from_currency AND fx.to_currency = 'USD'"
    }));
}

#[test]
fn runtime_join_executes_via_update_operation_entrypoint_with_string_on_shape() {
    assert_runtime_join_executes(json!(
        "currency = fx.from_currency AND fx.to_currency = 'USD'"
    ));
}

fn assert_runtime_join_executes(on: serde_json::Value) {
    let dataset_id = Uuid::new_v4();
    let period = sample_datasets::run_period_2026_01();
    let operation = OperationInstance {
        order: 1,
        kind: OperationKind::Update,
        alias: None,
        parameters: json!({
            "joins": [{
                "alias": "fx",
                "dataset_id": dataset_id,
                "on": on
            }],
            "assignments": [{
                "column": "amount_reporting",
                "expression": "amount_local * fx.rate"
            }]
        }),
    };

    let metadata_store = InMemoryMetadataStore::new()
        .with_dataset(sample_datasets::exchange_rates_dataset(dataset_id, 2))
        .with_resolver(test_resolver("fx-resolver", 9));

    let mut loader = InMemoryDataLoader::new();
    loader.seed_table(
        "fx://exchange_rates",
        sample_datasets::exchange_rates_frame(),
    );

    let mut run = test_run();
    let joined = execute_update_operation(
        &mut run,
        &operation,
        sample_datasets::gl_transactions_frame().lazy(),
        "gl",
        &[
            "journal_id".to_string(),
            "currency".to_string(),
            "amount_local".to_string(),
            "_period".to_string(),
        ],
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
    )
    .expect("update executor should apply runtime join")
    .collect()
    .expect("collect joined output");

    assert!(joined.column("rate_fx").is_err());
    let journal_ids = joined
        .column("journal_id")
        .expect("journal_id column")
        .str()
        .expect("journal ids should be strings");
    let amounts = joined
        .column("amount_reporting")
        .expect("amount_reporting column")
        .f64()
        .expect("amount_reporting should be f64");
    let mut actual = BTreeMap::new();
    for (journal_id, amount) in journal_ids
        .into_no_null_iter()
        .zip(amounts.into_no_null_iter())
    {
        actual.insert(journal_id.to_string(), amount);
    }
    let assert_close = |journal_id: &str, expected: f64| {
        let value = actual
            .get(journal_id)
            .copied()
            .expect("journal id should be present");
        assert!(
            (value - expected).abs() < 1e-6,
            "{journal_id}: expected {expected}, got {value}"
        );
    };
    assert_close("JE-001", 15000.0);
    assert_close("JE-002", 9282.0);
    assert_close("JE-003", 27962.0);
    assert_close("JE-005", 16800.0);
    assert_eq!(run.project_snapshot.resolver_snapshots.len(), 1);
    assert_eq!(
        run.project_snapshot.resolver_snapshots[0].resolver_id,
        "fx-resolver"
    );
    assert_eq!(
        run.project_snapshot.resolver_snapshots[0].join_datasets[0].alias,
        "fx"
    );
}

#[test]
fn update_operation_requires_assignments_field() {
    let dataset_id = Uuid::new_v4();
    let period = sample_datasets::run_period_2026_01();
    let operation = OperationInstance {
        order: 1,
        kind: OperationKind::Update,
        alias: None,
        parameters: json!({
            "joins": [{
                "alias": "fx",
                "dataset_id": dataset_id,
                "on": "currency = fx.from_currency AND fx.to_currency = 'USD'"
            }]
        }),
    };
    let metadata_store = InMemoryMetadataStore::new()
        .with_dataset(sample_datasets::exchange_rates_dataset(dataset_id, 2))
        .with_resolver(test_resolver("fx-resolver", 9));
    let loader = InMemoryDataLoader::new();
    let mut run = test_run();
    let result = execute_update_operation(
        &mut run,
        &operation,
        sample_datasets::gl_transactions_frame().lazy(),
        "gl",
        &[
            "journal_id".to_string(),
            "currency".to_string(),
            "amount_local".to_string(),
            "_period".to_string(),
        ],
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
    );
    let error = match result {
        Ok(_) => panic!("missing assignments should fail"),
        Err(error) => error,
    };

    assert!(error
        .to_string()
        .contains("invalid update arguments: missing field `assignments`"));
}

#[test]
fn update_operation_rejects_empty_assignments() {
    let dataset_id = Uuid::new_v4();
    let period = sample_datasets::run_period_2026_01();
    let operation = OperationInstance {
        order: 1,
        kind: OperationKind::Update,
        alias: None,
        parameters: json!({
            "joins": [{
                "alias": "fx",
                "dataset_id": dataset_id,
                "on": "currency = fx.from_currency AND fx.to_currency = 'USD'"
            }],
            "assignments": []
        }),
    };
    let metadata_store = InMemoryMetadataStore::new()
        .with_dataset(sample_datasets::exchange_rates_dataset(dataset_id, 2))
        .with_resolver(test_resolver("fx-resolver", 9));
    let loader = InMemoryDataLoader::new();
    let mut run = test_run();
    let result = execute_update_operation(
        &mut run,
        &operation,
        sample_datasets::gl_transactions_frame().lazy(),
        "gl",
        &[
            "journal_id".to_string(),
            "currency".to_string(),
            "amount_local".to_string(),
            "_period".to_string(),
        ],
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
    );
    let error = match result {
        Ok(_) => panic!("empty assignments should fail"),
        Err(error) => error,
    };

    assert!(error
        .to_string()
        .contains("invalid update arguments: assignments must contain at least 1 item"));
}
