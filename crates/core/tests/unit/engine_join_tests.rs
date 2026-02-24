use std::collections::{BTreeMap, HashMap};

use anyhow::{anyhow, Result};
use dobo_core::engine::join::{
    apply_runtime_joins, apply_runtime_joins_for_operation_pipeline,
    apply_update_operation_runtime_joins, resolve_dataset_version, resolve_resolver_id,
    resolve_resolver_with_source, validate_assignment_alias_references, validate_join_aliases,
    JoinError, ResolverSource,
};
use dobo_core::model::{
    ColumnDef, ColumnType, Dataset, DatasetStatus, Expression, Materialization, OperationInstance,
    OperationKind, ProjectSnapshot, ResolutionStrategy, ResolvedLocation, Resolver, ResolverStatus,
    Run, RunStatus, RuntimeJoin, TableRef, TemporalMode, TriggerType,
};
use dobo_core::DataLoader;
use polars::prelude::{df, DataFrame, IntoLazy, LazyFrame};
use serde_json::json;
use uuid::Uuid;

use crate::in_memory_metadata_store::InMemoryMetadataStore;
use crate::sample_datasets;

#[derive(Default)]
struct InMemoryLoader {
    frames: HashMap<String, DataFrame>,
}

impl InMemoryLoader {
    fn with_frame(mut self, key: &str, frame: DataFrame) -> Self {
        self.frames.insert(key.to_string(), frame);
        self
    }
}

impl DataLoader for InMemoryLoader {
    fn load(&self, location: &ResolvedLocation, _schema: &TableRef) -> Result<LazyFrame> {
        let key = location.path.clone().unwrap_or_default();
        let frame = self
            .frames
            .get(&key)
            .ok_or_else(|| anyhow!("missing frame for key '{key}'"))?;
        Ok(frame.clone().lazy())
    }
}

fn runtime_join(alias: &str, dataset_id: Uuid, on: &str) -> RuntimeJoin {
    RuntimeJoin {
        alias: alias.to_string(),
        dataset_id,
        dataset_version: None,
        on: Expression {
            source: on.to_string(),
        },
    }
}

fn test_dataset(
    id: Uuid,
    version: i32,
    status: DatasetStatus,
    resolver_id: Option<&str>,
) -> Dataset {
    Dataset {
        id,
        name: "test".to_string(),
        description: None,
        owner: "tests".to_string(),
        version,
        status,
        resolver_id: resolver_id.map(ToString::to_string),
        main_table: TableRef {
            name: "table".to_string(),
            temporal_mode: Some(TemporalMode::Period),
            columns: vec![ColumnDef {
                name: "id".to_string(),
                column_type: ColumnType::String,
                nullable: Some(false),
                description: None,
            }],
        },
        lookups: vec![],
        natural_key_columns: vec![],
        created_at: None,
        updated_at: None,
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
                datasource_id: "ds".to_string(),
                path: "data/{{table_name}}.parquet".to_string(),
            },
        }],
        created_at: None,
        updated_at: None,
    }
}

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

#[test]
fn resolves_pinned_version() {
    let dataset_id = Uuid::new_v4();
    let pinned_dataset = test_dataset(dataset_id, 2, DatasetStatus::Active, Some("dataset"));
    let store = InMemoryMetadataStore::new().with_dataset(pinned_dataset);

    let resolved = resolve_dataset_version(&dataset_id, Some(2), &store)
        .expect("pinned version should resolve");

    assert_eq!(resolved.1, 2);
}

#[test]
fn resolves_latest_active_version() {
    let dataset_id = Uuid::new_v4();
    let latest_dataset = test_dataset(dataset_id, 7, DatasetStatus::Active, Some("dataset"));
    let store = InMemoryMetadataStore::new().with_dataset(latest_dataset);

    let resolved =
        resolve_dataset_version(&dataset_id, None, &store).expect("latest version should resolve");

    assert_eq!(resolved.1, 7);
}

#[test]
fn validates_alias_uniqueness() {
    let dataset_id = Uuid::new_v4();
    let joins = vec![
        runtime_join("fx", dataset_id, "currency = fx.from_currency"),
        runtime_join("fx", dataset_id, "currency = fx.from_currency"),
    ];

    let error = validate_join_aliases(&joins, "gl").expect_err("duplicate alias should fail");
    assert!(matches!(error, JoinError::AliasNotUnique(alias) if alias == "fx"));
}

#[test]
fn validates_alias_conflict_with_working_table() {
    let dataset_id = Uuid::new_v4();
    let joins = vec![runtime_join(
        "gl",
        dataset_id,
        "currency = gl.from_currency",
    )];

    let error =
        validate_join_aliases(&joins, "gl").expect_err("working table conflict should fail");
    assert!(matches!(
        error,
        JoinError::AliasConflictsWithWorkingTable { alias, table }
        if alias == "gl" && table == "gl"
    ));
}

#[test]
fn applies_multiple_joins_sequentially() {
    let working = sample_datasets::gl_transactions_frame().lazy();
    let joins = vec![
        runtime_join("customers", Uuid::new_v4(), "customer_id = customers.id"),
        runtime_join("products", Uuid::new_v4(), "product_id = products.id"),
    ];

    let joined = apply_runtime_joins(
        working,
        &joins,
        "gl",
        &[
            "journal_id".to_string(),
            "customer_id".to_string(),
            "product_id".to_string(),
            "currency".to_string(),
            "amount_local".to_string(),
            "_period".to_string(),
        ],
        |join| {
            if join.alias == "customers" {
                Ok((
                    sample_datasets::customers_frame().lazy(),
                    vec!["id".to_string(), "tier".to_string(), "_period".to_string()],
                ))
            } else {
                Ok((
                    sample_datasets::products_frame().lazy(),
                    vec![
                        "id".to_string(),
                        "category".to_string(),
                        "_period".to_string(),
                    ],
                ))
            }
        },
    )
    .expect("multi-join should succeed");

    let result = joined.collect().expect("collect joined frame");
    assert!(result.column("tier_customers").is_ok());
    assert!(result.column("category_products").is_ok());
}

#[test]
fn produces_nulls_when_join_has_no_matching_period_rows() {
    let working = df! {
        "currency" => &["CAD"],
        "amount_local" => &[100.0_f64],
    }
    .expect("working frame")
    .lazy();

    let joins = vec![runtime_join(
        "fx",
        Uuid::new_v4(),
        "currency = fx.from_currency",
    )];

    let joined = apply_runtime_joins(
        working,
        &joins,
        "gl",
        &["currency".to_string(), "amount_local".to_string()],
        |_| {
            Ok((
                sample_datasets::exchange_rates_frame().lazy(),
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
    .expect("join should succeed");

    let result = joined.collect().expect("collect joined frame");
    let rates = result
        .column("rate_fx")
        .expect("rate column")
        .f64()
        .expect("f64 rate column");
    assert_eq!(rates.get(0), None);
}

#[test]
fn supports_grouped_or_and_numeric_join_predicates() {
    let working = sample_datasets::gl_transactions_frame().lazy();
    let joins = vec![runtime_join(
        "fx",
        Uuid::new_v4(),
        "currency = fx.from_currency AND (fx.to_currency = 'USD' OR fx.to_currency = 'EUR') AND fx.rate > 1.0",
    )];

    let joined = apply_runtime_joins(
        working,
        &joins,
        "gl",
        &[
            "journal_id".to_string(),
            "currency".to_string(),
            "amount_local".to_string(),
            "_period".to_string(),
        ],
        |_| {
            Ok((
                sample_datasets::exchange_rates_frame().lazy(),
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
    .expect("join should succeed")
    .collect()
    .expect("collect joined frame");

    let journal_ids = joined
        .column("journal_id")
        .expect("journal_id")
        .str()
        .expect("journal_id strings");
    let rates = joined
        .column("rate_fx")
        .expect("rate column")
        .f64()
        .expect("f64 rate");

    let mut eur_has_match = false;
    let mut jpy_rate = Some(0.0_f64);
    for (journal_id, rate) in journal_ids.into_no_null_iter().zip(rates.into_iter()) {
        if journal_id == "JE-002" {
            eur_has_match = eur_has_match || rate.is_some_and(|value| value > 1.0);
        }
        if journal_id == "JE-005" {
            jpy_rate = rate;
        }
    }

    assert!(eur_has_match);
    assert_eq!(jpy_rate, None);
}

#[test]
fn resolver_precedence_project_override_wins() {
    let dataset_id = Uuid::new_v4();
    let mut overrides = BTreeMap::new();
    overrides.insert(dataset_id, "test-resolver".to_string());

    let (resolver, source) =
        resolve_resolver_with_source(&dataset_id, Some("dataset-resolver"), &overrides, "default");

    assert_eq!(resolver, "test-resolver");
    assert_eq!(source, ResolverSource::ProjectOverride);
}

#[test]
fn resolver_precedence_dataset_fallback() {
    let dataset_id = Uuid::new_v4();
    let overrides = BTreeMap::new();

    let resolver =
        resolve_resolver_id(&dataset_id, Some("dataset-resolver"), &overrides, "default");
    assert_eq!(resolver, "dataset-resolver");
}

#[test]
fn resolver_precedence_system_default_fallback() {
    let dataset_id = Uuid::new_v4();
    let overrides = BTreeMap::new();

    let resolver = resolve_resolver_id(&dataset_id, None, &overrides, "default");
    assert_eq!(resolver, "default");
}

#[test]
fn errors_for_nonexistent_dataset() {
    let dataset_id = Uuid::new_v4();
    let store = InMemoryMetadataStore::new();
    let error = resolve_dataset_version(&dataset_id, None, &store)
        .expect_err("missing dataset should fail");
    assert!(matches!(error, JoinError::DatasetNotFound(id) if id == dataset_id));
}

#[test]
fn pinned_lookup_errors_for_nonexistent_dataset() {
    let dataset_id = Uuid::new_v4();
    let store = InMemoryMetadataStore::new();

    let error = resolve_dataset_version(&dataset_id, Some(3), &store)
        .expect_err("missing pinned dataset should fail");
    assert!(matches!(error, JoinError::DatasetNotFound(id) if id == dataset_id));
}

#[test]
fn surfaces_metadata_lookup_failures() {
    let dataset_id = Uuid::new_v4();
    let store = InMemoryMetadataStore::new().with_failure("metadata backend unavailable");

    let error = resolve_dataset_version(&dataset_id, None, &store)
        .expect_err("metadata failures should be surfaced");
    assert!(matches!(
        error,
        JoinError::MetadataLookupFailed {
            dataset_id: id,
            version: None,
            ..
        } if id == dataset_id
    ));
}

#[test]
fn metadata_errors_with_not_found_text_are_not_misclassified() {
    let dataset_id = Uuid::new_v4();
    let store = InMemoryMetadataStore::new().with_failure("backend not found while connecting");

    let error = resolve_dataset_version(&dataset_id, None, &store)
        .expect_err("backend failures should not be mapped as missing datasets");
    assert!(matches!(
        error,
        JoinError::MetadataLookupFailed {
            dataset_id: id,
            version: None,
            ..
        } if id == dataset_id
    ));
}

#[test]
fn errors_for_disabled_dataset() {
    let dataset_id = Uuid::new_v4();
    let dataset = test_dataset(dataset_id, 1, DatasetStatus::Disabled, Some("resolver"));
    let store = InMemoryMetadataStore::new().with_dataset(dataset);

    let error = resolve_dataset_version(&dataset_id, None, &store)
        .expect_err("disabled dataset should fail");
    assert!(matches!(error, JoinError::DatasetDisabled(id) if id == dataset_id));
}

#[test]
fn errors_for_unknown_join_column_in_condition() {
    let working = sample_datasets::gl_transactions_frame().lazy();
    let joins = vec![runtime_join("fx", Uuid::new_v4(), "currency = fx.unknown")];

    let error = match apply_runtime_joins(working, &joins, "gl", &["currency".to_string()], |_| {
        Ok((
            sample_datasets::exchange_rates_frame().lazy(),
            vec!["from_currency".to_string()],
        ))
    }) {
        Ok(_) => panic!("unknown join column should fail"),
        Err(error) => error,
    };

    assert!(matches!(error, JoinError::UnknownColumn(column) if column == "unknown"));
}

#[test]
fn errors_for_unknown_assignment_alias() {
    let mut alias_columns = HashMap::new();
    alias_columns.insert("fx".to_string(), vec!["rate".to_string()]);

    let error = validate_assignment_alias_references(
        &["amount_local * unknown.rate".to_string()],
        &alias_columns,
    )
    .expect_err("unknown alias should fail");

    assert!(matches!(error, JoinError::UnknownJoinAlias(alias) if alias == "unknown"));
}

#[test]
fn assignment_alias_validation_ignores_quoted_literals() {
    let mut alias_columns = HashMap::new();
    alias_columns.insert("fx".to_string(), vec!["rate".to_string()]);

    validate_assignment_alias_references(
        &[r#"IF(amount_local > 0, fx.rate, "unknown.rate")"#.to_string()],
        &alias_columns,
    )
    .expect("quoted dotted literals must not be treated as alias references");
}

#[test]
fn errors_for_unknown_assignment_column() {
    let mut alias_columns = HashMap::new();
    alias_columns.insert("fx".to_string(), vec!["rate".to_string()]);

    let error = validate_assignment_alias_references(
        &["amount_local * fx.missing".to_string()],
        &alias_columns,
    )
    .expect_err("unknown column should fail");

    assert!(matches!(
        error,
        JoinError::UnknownJoinColumn { alias, column }
        if alias == "fx" && column == "missing"
    ));
}

#[test]
fn zero_runtime_joins_is_valid_baseline() {
    let working = sample_datasets::gl_transactions_frame().lazy();

    let result = apply_runtime_joins(
        working,
        &[],
        "gl",
        &["journal_id".to_string(), "currency".to_string()],
        |_| unreachable!("no joins should not invoke loader"),
    )
    .expect("empty joins should succeed")
    .collect()
    .expect("collect baseline frame");

    assert_eq!(result.height(), 4);
}

#[test]
fn resolves_and_loads_join_dataset_via_dataloader() {
    use dobo_core::engine::join::resolve_and_load_join;

    let dataset_id = Uuid::new_v4();
    let period = sample_datasets::run_period_2026_01();
    let dataset = sample_datasets::exchange_rates_dataset(dataset_id, 2);
    let join = runtime_join(
        "fx",
        dataset_id,
        "currency = fx.from_currency AND fx.to_currency = 'USD'",
    );

    let loader = InMemoryLoader::default().with_frame(
        "fx://exchange_rates",
        sample_datasets::exchange_rates_frame(),
    );

    let store = InMemoryMetadataStore::new().with_dataset(dataset);
    let mut snapshot = Vec::new();
    let (join_lf, resolved) = resolve_and_load_join(
        &join,
        &BTreeMap::new(),
        "system-default",
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
        &mut snapshot,
    )
    .expect("resolve_and_load_join should succeed");

    let result = join_lf.collect().expect("collect filtered join data");
    assert_eq!(resolved.dataset_version, 2);
    assert_eq!(snapshot.len(), 1);
    assert_eq!(snapshot[0].dataset_id, dataset_id);
    assert_eq!(snapshot[0].dataset_version, 2);
    assert_eq!(snapshot[0].alias, "fx");
    assert_eq!(result.height(), 4);
}

#[test]
fn omitted_temporal_mode_defaults_to_period_filtering() {
    use dobo_core::engine::join::resolve_and_load_join;

    let dataset_id = Uuid::new_v4();
    let period = sample_datasets::run_period_2026_01();
    let mut dataset = sample_datasets::customers_dataset(dataset_id);
    dataset.main_table.temporal_mode = None;
    let join = runtime_join("customers", dataset_id, "customer_id = customers.id");

    let customers_frame = df! {
        "id" => &["C1", "C2", "C3"],
        "tier" => &["silver", "gold", "bronze"],
        "_period" => &["2026-01", "2025-12", "2026-01"],
    }
    .expect("valid customers frame");

    let loader = InMemoryLoader::default().with_frame("customers://main", customers_frame);
    let store = InMemoryMetadataStore::new().with_dataset(dataset);
    let mut snapshot = Vec::new();

    let (join_lf, resolved) = resolve_and_load_join(
        &join,
        &BTreeMap::new(),
        "system-default",
        &period,
        &store,
        |_, resolver_id, _| {
            Ok(ResolvedLocation {
                datasource_id: resolver_id.to_string(),
                path: Some("customers://main".to_string()),
                table: None,
                schema: None,
                period_identifier: Some("2026-01".to_string()),
            })
        },
        &loader,
        &mut snapshot,
    )
    .expect("resolve_and_load_join should apply default period filter");

    let result = join_lf
        .collect()
        .expect("collect period-filtered join data");
    let periods = result
        .column("_period")
        .expect("_period column present")
        .str()
        .expect("_period should be utf8")
        .into_no_null_iter()
        .collect::<Vec<_>>();

    assert_eq!(resolved.temporal_mode, Some(TemporalMode::Period));
    assert_eq!(snapshot.len(), 1);
    assert_eq!(result.height(), 2);
    assert!(periods.iter().all(|value| *value == "2026-01"));
}

#[test]
fn rejects_or_predicates_with_clear_contract_error() {
    let working = sample_datasets::gl_transactions_frame().lazy();
    let joins = vec![runtime_join(
        "fx",
        Uuid::new_v4(),
        "currency = fx.from_currency OR amount_local > fx.rate",
    )];

    let error = match apply_runtime_joins(
        working,
        &joins,
        "gl",
        &[
            "journal_id".to_string(),
            "currency".to_string(),
            "amount_local".to_string(),
            "_period".to_string(),
        ],
        |_| {
            Ok((
                sample_datasets::exchange_rates_frame().lazy(),
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
    ) {
        Ok(_) => panic!("OR predicates should be rejected"),
        Err(error) => error,
    };
    assert!(matches!(
        error,
        JoinError::InvalidJoinCondition(message)
        if message.contains("only AND-connected equality predicates are supported")
    ));
}

#[test]
fn rejects_left_side_only_predicates_with_clear_contract_error() {
    let working = sample_datasets::gl_transactions_frame().lazy();
    let joins = vec![runtime_join("fx", Uuid::new_v4(), "amount_local > 0")];

    let error =
        match apply_runtime_joins(working, &joins, "gl", &["amount_local".to_string()], |_| {
            Ok((
                sample_datasets::exchange_rates_frame().lazy(),
                vec![
                    "from_currency".to_string(),
                    "to_currency".to_string(),
                    "rate".to_string(),
                    "rate_type".to_string(),
                    "_period_from".to_string(),
                    "_period_to".to_string(),
                ],
            ))
        }) {
            Ok(_) => panic!("left-only predicates should be rejected"),
            Err(error) => error,
        };
    assert!(matches!(
        error,
        JoinError::InvalidJoinCondition(message)
        if message.contains("must reference the join alias")
    ));
}

#[test]
fn update_operation_path_applies_assignments_and_persists_snapshots() {
    let dataset_id = Uuid::new_v4();
    let resolver_id = "fx-resolver";
    let dataset = sample_datasets::exchange_rates_dataset(dataset_id, 2);
    let resolver = test_resolver(resolver_id, 9);
    let store = InMemoryMetadataStore::new()
        .with_dataset(dataset)
        .with_resolver(resolver);
    let period = sample_datasets::run_period_2026_01();
    let loader = InMemoryLoader::default().with_frame(
        "fx://exchange_rates",
        sample_datasets::exchange_rates_frame(),
    );
    let mut run = test_run();
    let operation = OperationInstance {
        order: 1,
        kind: OperationKind::Update,
        alias: None,
        parameters: json!({
            "joins": [{
                "alias": "fx",
                "dataset_id": dataset_id,
                "on": { "source": "currency = fx.from_currency AND fx.to_currency = 'USD'" }
            }],
            "assignments": [{
                "column": "amount_reporting",
                "expression": "amount_local * fx.rate"
            }]
        }),
    };

    let joined = apply_update_operation_runtime_joins(
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
    )
    .expect("update path should apply runtime joins")
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
    let mut actual = HashMap::new();
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
    let snapshot = &run.project_snapshot.resolver_snapshots[0];
    assert_eq!(snapshot.dataset_id, dataset_id);
    assert_eq!(snapshot.resolver_id, resolver_id);
    assert_eq!(snapshot.resolver_version, 9);
    assert_eq!(snapshot.join_datasets.len(), 1);
    assert_eq!(snapshot.join_datasets[0].alias, "fx");
    assert_eq!(snapshot.join_datasets[0].dataset_version, 2);
}

#[test]
fn operation_pipeline_path_scopes_runtime_join_aliases_per_operation() {
    let dataset_id = Uuid::new_v4();
    let resolver_id = "fx-resolver";
    let dataset = sample_datasets::exchange_rates_dataset(dataset_id, 2);
    let resolver = test_resolver(resolver_id, 9);
    let store = InMemoryMetadataStore::new()
        .with_dataset(dataset)
        .with_resolver(resolver);
    let period = sample_datasets::run_period_2026_01();
    let loader = InMemoryLoader::default().with_frame(
        "fx://exchange_rates",
        sample_datasets::exchange_rates_frame(),
    );
    let mut run = test_run();
    let operations = vec![
        OperationInstance {
            order: 1,
            kind: OperationKind::Update,
            alias: None,
            parameters: json!({
                "joins": [{
                    "alias": "fx",
                    "dataset_id": dataset_id,
                    "on": { "source": "currency = fx.from_currency AND fx.to_currency = 'USD'" }
                }],
                "assignments": [{
                    "column": "amount_reporting",
                    "expression": "amount_local * fx.rate"
                }]
            }),
        },
        OperationInstance {
            order: 2,
            kind: OperationKind::Update,
            alias: None,
            parameters: json!({
                "joins": [{
                    "alias": "fx",
                    "dataset_id": dataset_id,
                    "on": { "source": "currency = fx.from_currency AND fx.to_currency = 'USD'" }
                }],
                "assignments": [{
                    "column": "double_converted",
                    "expression": "amount_reporting * fx.rate"
                }]
            }),
        },
    ];

    let joined = apply_runtime_joins_for_operation_pipeline(
        &mut run,
        &operations,
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
    )
    .expect("pipeline path should apply runtime joins")
    .collect()
    .expect("collect joined output");

    assert!(joined.column("rate_fx").is_err());
    let journal_ids = joined
        .column("journal_id")
        .expect("journal_id column")
        .str()
        .expect("journal ids should be strings");
    let amounts = joined
        .column("double_converted")
        .expect("double_converted column")
        .f64()
        .expect("double_converted should be f64");
    let mut actual = HashMap::new();
    for (journal_id, amount) in journal_ids
        .into_no_null_iter()
        .zip(amounts.into_no_null_iter())
    {
        actual.insert(journal_id.to_string(), amount);
    }

    let expected = [
        ("JE-001", 15000.0),
        ("JE-002", 10135.944),
        ("JE-003", 35539.702),
        ("JE-005", 112.896),
    ];
    for (journal_id, expected_value) in expected {
        let value = actual
            .get(journal_id)
            .copied()
            .expect("journal id should be present");
        assert!(
            (value - expected_value).abs() < 1e-6,
            "{journal_id}: expected {expected_value}, got {value}"
        );
    }

    assert_eq!(run.project_snapshot.resolver_snapshots.len(), 2);
}

#[test]
fn update_operation_path_validates_assignment_aliases() {
    let store = InMemoryMetadataStore::new();
    let period = sample_datasets::run_period_2026_01();
    let mut run = test_run();
    let operation = OperationInstance {
        order: 1,
        kind: OperationKind::Update,
        alias: None,
        parameters: json!({
            "joins": [],
            "assignments": [{
                "column": "amount_reporting",
                "expression": "amount_local * fx.rate"
            }]
        }),
    };

    let error = match apply_update_operation_runtime_joins(
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
        &store,
        |_, _, _| unreachable!("no joins should not resolve locations"),
        &InMemoryLoader::default(),
    ) {
        Ok(_) => panic!("unknown join alias in assignments should fail"),
        Err(error) => error,
    };

    assert!(matches!(error, JoinError::UnknownJoinAlias(alias) if alias == "fx"));
}

#[test]
fn validates_alias_column_conflict() {
    let dataset_id = Uuid::new_v4();
    // Working frame has 'rate_fx' already
    let working = df! {
        "currency" => &["USD"],
        "rate_fx" => &[10.0],
    }
    .unwrap()
    .lazy();

    let joins = vec![runtime_join(
        "fx",
        dataset_id,
        "currency = fx.from_currency",
    )];

    let result = apply_runtime_joins(
        working,
        &joins,
        "gl",
        &["currency".to_string(), "rate_fx".to_string()],
        |_| {
            Ok((
                df! {
                    "from_currency" => &["USD"],
                    "rate" => &[1.5],
                }
                .unwrap()
                .lazy(),
                vec!["from_currency".to_string(), "rate".to_string()],
            ))
        },
    );

    match result {
        Ok(_) => panic!("Collision check passed unexpectedly"),
        Err(JoinError::AliasColumnConflict(col)) => assert_eq!(col, "rate_fx"),
        Err(e) => panic!("Collision check failed with unexpected error: {:?}", e),
    }
}

#[test]
fn supports_quoted_identifiers_in_tokenization() {
    let mut alias_columns = HashMap::new();
    alias_columns.insert("fx".to_string(), vec!["My Rate".to_string()]);

    validate_assignment_alias_references(
        &["amount_local * fx.\"My Rate\"".to_string()],
        &alias_columns,
    )
    .expect("should support quoted identifiers");
}
