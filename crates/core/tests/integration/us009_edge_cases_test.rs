use dobo_core::engine::append::{
    execute_append, execute_append_operation, validate_append_operation, AppendExecutionContext,
};
use dobo_core::engine::error::AppendError;
use dobo_core::engine::io_traits::DataLoader;
use dobo_core::model::{
    Aggregation, AppendAggregation, AppendOperation, ColumnDef, ColumnType, Dataset, DatasetRef,
    DatasetStatus, Materialization, OperationInstance, Project, ProjectStatus, ResolutionRule,
    ResolutionStrategy, ResolvedLocation, Resolver, ResolverStatus, RunStatus, TableRef,
    Visibility,
};
use dobo_core::MetadataStore;
use polars::df;
use polars::prelude::IntoLazy;
use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap};
use uuid::Uuid;

struct MissingDatasetStore;

impl MetadataStore for MissingDatasetStore {
    fn get_dataset(
        &self,
        _id: &Uuid,
        _version: Option<i32>,
    ) -> anyhow::Result<dobo_core::model::Dataset> {
        anyhow::bail!("dataset missing")
    }

    fn get_project(&self, _id: &Uuid) -> anyhow::Result<Project> {
        anyhow::bail!("not used")
    }

    fn get_resolver(&self, _id: &str) -> anyhow::Result<Resolver> {
        anyhow::bail!("not used")
    }

    fn update_run_status(&self, _id: &Uuid, _status: RunStatus) -> anyhow::Result<()> {
        anyhow::bail!("not used")
    }
}

#[derive(Default)]
struct RecordingLoader {
    locations: RefCell<Vec<ResolvedLocation>>,
}

impl DataLoader for RecordingLoader {
    fn load(
        &self,
        location: &ResolvedLocation,
        _schema: &TableRef,
    ) -> anyhow::Result<polars::prelude::LazyFrame> {
        self.locations.borrow_mut().push(location.clone());
        let frame = df!(
            "account_code" => &["4000"],
            "amount" => &[50i64],
            "_deleted" => &[false]
        )
        .expect("source frame");
        Ok(frame.lazy())
    }
}

struct RecordingStore {
    dataset: Dataset,
    resolvers: HashMap<String, Resolver>,
    default_resolver_id: String,
    requested_versions: RefCell<Vec<Option<i32>>>,
}

impl RecordingStore {
    fn new(dataset: Dataset, resolvers: Vec<Resolver>, default_resolver_id: &str) -> Self {
        Self {
            dataset,
            resolvers: resolvers
                .into_iter()
                .map(|resolver| (resolver.id.clone(), resolver))
                .collect(),
            default_resolver_id: default_resolver_id.to_owned(),
            requested_versions: RefCell::new(Vec::new()),
        }
    }
}

impl MetadataStore for RecordingStore {
    fn get_dataset(&self, _id: &Uuid, version: Option<i32>) -> anyhow::Result<Dataset> {
        self.requested_versions.borrow_mut().push(version);
        Ok(self.dataset.clone())
    }

    fn get_project(&self, _id: &Uuid) -> anyhow::Result<Project> {
        anyhow::bail!("not used")
    }

    fn get_resolver(&self, id: &str) -> anyhow::Result<Resolver> {
        self.resolvers
            .get(id)
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("resolver not found: {id}"))
    }

    fn get_default_resolver(&self) -> anyhow::Result<Resolver> {
        self.resolvers
            .get(&self.default_resolver_id)
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("resolver not found: {}", self.default_resolver_id))
    }

    fn update_run_status(&self, _id: &Uuid, _status: RunStatus) -> anyhow::Result<()> {
        anyhow::bail!("not used")
    }
}

struct MetadataFailureStore;

impl MetadataStore for MetadataFailureStore {
    fn get_dataset(&self, _id: &Uuid, _version: Option<i32>) -> anyhow::Result<Dataset> {
        anyhow::bail!("metadata backend unavailable")
    }

    fn get_project(&self, _id: &Uuid) -> anyhow::Result<Project> {
        anyhow::bail!("not used")
    }

    fn get_resolver(&self, _id: &str) -> anyhow::Result<Resolver> {
        anyhow::bail!("not used")
    }

    fn update_run_status(&self, _id: &Uuid, _status: RunStatus) -> anyhow::Result<()> {
        anyhow::bail!("not used")
    }
}

fn append_operation(version: Option<i32>) -> AppendOperation {
    AppendOperation {
        source: DatasetRef {
            dataset_id: source_id(),
            dataset_version: version,
        },
        source_selector: None,
        aggregation: None,
    }
}

fn source_dataset(resolver_id: Option<&str>) -> Dataset {
    Dataset {
        id: source_id(),
        name: "budget".to_owned(),
        description: None,
        owner: "finance".to_owned(),
        version: 3,
        status: DatasetStatus::Active,
        resolver_id: resolver_id.map(str::to_owned),
        main_table: TableRef {
            name: "budget_lines".to_owned(),
            temporal_mode: None,
            columns: vec![
                ColumnDef {
                    name: "account_code".to_owned(),
                    column_type: ColumnType::String,
                    nullable: None,
                    description: None,
                },
                ColumnDef {
                    name: "amount".to_owned(),
                    column_type: ColumnType::Integer,
                    nullable: None,
                    description: None,
                },
            ],
        },
        lookups: vec![],
        natural_key_columns: vec![],
        created_at: None,
        updated_at: None,
    }
}

fn resolver(
    id: &str,
    datasource_id: &str,
    when_expression: Option<&str>,
    is_default: bool,
) -> Resolver {
    Resolver {
        id: id.to_owned(),
        name: format!("{id}-resolver"),
        description: None,
        version: 1,
        status: ResolverStatus::Active,
        is_default: Some(is_default),
        rules: vec![ResolutionRule {
            name: format!("{id}-rule"),
            when_expression: when_expression.map(str::to_owned),
            data_level: "table".to_owned(),
            strategy: ResolutionStrategy::Path {
                datasource_id: datasource_id.to_owned(),
                path: "s3://bucket/{{table_name}}".to_owned(),
            },
        }],
        created_at: None,
        updated_at: None,
    }
}

fn project_with_overrides(overrides: BTreeMap<Uuid, String>) -> Project {
    Project {
        id: Uuid::now_v7(),
        name: "demo-project".to_owned(),
        description: None,
        owner: "owner".to_owned(),
        version: 1,
        status: ProjectStatus::Draft,
        visibility: Visibility::Private,
        input_dataset_id: source_id(),
        input_dataset_version: 1,
        materialization: Materialization::Runtime,
        operations: Vec::<OperationInstance>::new(),
        selectors: BTreeMap::new(),
        resolver_overrides: overrides,
        conflict_report: None,
        created_at: None,
        updated_at: None,
    }
}

fn source_id() -> Uuid {
    Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").expect("uuid")
}

fn working_for_errors() -> polars::prelude::DataFrame {
    df!(
        "_row_id" => &["w1"],
        "_source_dataset" => &["working"],
        "_operation_seq" => &[1i64],
        "_deleted" => &[false],
        "account_code" => &["4000"],
        "amount" => &[100i64]
    )
    .expect("working frame")
}

#[test]
fn non_existent_dataset_returns_dataset_not_found() {
    let op = append_operation(None);

    let error = validate_append_operation(&MissingDatasetStore, &op).expect_err("should fail");
    assert!(matches!(error, AppendError::DatasetNotFound { .. }));
}

#[test]
fn metadata_access_failure_returns_metadata_error() {
    let op = append_operation(None);

    let error = validate_append_operation(&MetadataFailureStore, &op).expect_err("should fail");
    assert!(matches!(error, AppendError::MetadataAccessError { .. }));
}

#[test]
fn extra_columns_in_source_return_column_mismatch() {
    let working = working_for_errors();
    let source = df!(
        "account_code" => &["4000"],
        "amount" => &[50i64],
        "budget_type" => &[Some("original")]
    )
    .expect("source frame");
    let op = append_operation(None);

    let error = execute_append(&working, &source, &op, &AppendExecutionContext::default())
        .expect_err("should fail");
    assert!(matches!(error, AppendError::ColumnMismatch { .. }));
}

#[test]
fn extra_columns_in_source_with_aggregation_succeeds_if_aggregated_away() {
    let working = working_for_errors();
    let source = df!(
        "account_code" => &["4000", "4000"],
        "amount" => &[50i64, 60],
        "budget_type" => &[Some("original"), Some("original")]
    )
    .expect("source frame");
    let op = AppendOperation {
        source: DatasetRef {
            dataset_id: source_id(),
            dataset_version: None,
        },
        source_selector: None,
        aggregation: Some(AppendAggregation {
            group_by: vec!["account_code".to_owned()],
            aggregations: vec![Aggregation {
                column: "amount".to_owned(),
                expression: "SUM(amount)".to_owned(),
            }],
        }),
    };

    let result = execute_append(&working, &source, &op, &AppendExecutionContext::default())
        .expect("should succeed");

    assert_eq!(result.rows_appended, 1);
}

#[test]
fn invalid_aggregate_function_returns_aggregation_error() {
    let working = working_for_errors();
    let source = df!(
        "account_code" => &["4000", "4000"],
        "amount" => &[10i64, 20]
    )
    .expect("source frame");
    let op = AppendOperation {
        source: DatasetRef {
            dataset_id: source_id(),
            dataset_version: None,
        },
        source_selector: None,
        aggregation: Some(AppendAggregation {
            group_by: vec!["account_code".to_owned()],
            aggregations: vec![Aggregation {
                column: "amount".to_owned(),
                expression: "MEDIAN(amount)".to_owned(),
            }],
        }),
    };

    let error = execute_append(&working, &source, &op, &AppendExecutionContext::default())
        .expect_err("should fail");
    assert!(matches!(error, AppendError::AggregationError { .. }));
}

#[test]
fn wildcard_non_count_aggregate_returns_aggregation_error() {
    let working = working_for_errors();
    let source = df!(
        "account_code" => &["4000", "4000"],
        "amount" => &[10i64, 20]
    )
    .expect("source frame");
    let op = AppendOperation {
        source: DatasetRef {
            dataset_id: source_id(),
            dataset_version: None,
        },
        source_selector: None,
        aggregation: Some(AppendAggregation {
            group_by: vec!["account_code".to_owned()],
            aggregations: vec![Aggregation {
                column: "amount".to_owned(),
                expression: "SUM(*)".to_owned(),
            }],
        }),
    };

    let error = execute_append(&working, &source, &op, &AppendExecutionContext::default())
        .expect_err("should fail");
    match error {
        AppendError::AggregationError { message } => {
            assert!(message.contains("COUNT(*)"));
        }
        other => panic!("expected AggregationError, got {other:?}"),
    }
}

#[test]
fn missing_group_by_column_returns_column_not_found() {
    let working = working_for_errors();
    let source = df!(
        "account_code" => &["4000", "4000"],
        "amount" => &[10i64, 20]
    )
    .expect("source frame");
    let op = AppendOperation {
        source: DatasetRef {
            dataset_id: source_id(),
            dataset_version: None,
        },
        source_selector: None,
        aggregation: Some(AppendAggregation {
            group_by: vec!["missing_col".to_owned()],
            aggregations: vec![Aggregation {
                column: "amount".to_owned(),
                expression: "SUM(amount)".to_owned(),
            }],
        }),
    };

    let error = execute_append(&working, &source, &op, &AppendExecutionContext::default())
        .expect_err("should fail");
    assert!(matches!(error, AppendError::ColumnNotFound { .. }));
}

#[test]
fn zero_row_selector_returns_success_with_zero_appended() {
    let working = working_for_errors();
    let source = df!(
        "account_code" => &["4000", "5000"],
        "amount" => &[10i64, 20],
        "_deleted" => &[false, false]
    )
    .expect("source frame");
    let mut op = append_operation(None);
    op.source_selector = Some("amount > 999".into());

    let result = execute_append(&working, &source, &op, &AppendExecutionContext::default())
        .expect("append should succeed");
    assert_eq!(result.rows_appended, 0);
    assert_eq!(result.source_rows_after_selector, 0);
    assert_eq!(result.frame.height(), working.height());
}

#[test]
fn soft_deleted_source_rows_are_excluded_by_default() {
    let working = working_for_errors();
    let source = df!(
        "account_code" => &["4000", "5000"],
        "amount" => &[10i64, 20],
        "_deleted" => &[true, false]
    )
    .expect("source frame");

    let result = execute_append(
        &working,
        &source,
        &append_operation(None),
        &AppendExecutionContext::default(),
    )
    .expect("append should succeed");

    assert_eq!(result.rows_appended, 1);
}

#[test]
fn resolver_precedence_uses_project_then_dataset_then_default() {
    let working = working_for_errors();
    let context = AppendExecutionContext {
        run_period: Some("2026-01".to_owned()),
        operation_seq: 2,
        ..Default::default()
    };

    let mut overrides = BTreeMap::new();
    overrides.insert(source_id(), "project-resolver".to_owned());
    let project = project_with_overrides(overrides);
    let store = RecordingStore::new(
        source_dataset(Some("dataset-resolver")),
        vec![
            resolver("project-resolver", "project-ds", None, false),
            resolver("dataset-resolver", "dataset-ds", None, false),
            resolver("system-default-id", "default-ds", None, true),
        ],
        "system-default-id",
    );
    let loader = RecordingLoader::default();
    execute_append_operation(
        &working,
        &store,
        &loader,
        &project,
        &append_operation(None),
        &context,
    )
    .expect("project resolver append should succeed");
    assert_eq!(
        loader
            .locations
            .borrow()
            .last()
            .expect("location")
            .datasource_id,
        "project-ds"
    );

    let project = project_with_overrides(BTreeMap::new());
    let store = RecordingStore::new(
        source_dataset(Some("dataset-resolver")),
        vec![
            resolver("project-resolver", "project-ds", None, false),
            resolver("dataset-resolver", "dataset-ds", None, false),
            resolver("system-default-id", "default-ds", None, true),
        ],
        "system-default-id",
    );
    let loader = RecordingLoader::default();
    execute_append_operation(
        &working,
        &store,
        &loader,
        &project,
        &append_operation(None),
        &context,
    )
    .expect("dataset resolver append should succeed");
    assert_eq!(
        loader
            .locations
            .borrow()
            .last()
            .expect("location")
            .datasource_id,
        "dataset-ds"
    );

    let project = project_with_overrides(BTreeMap::new());
    let store = RecordingStore::new(
        source_dataset(None),
        vec![
            resolver("project-resolver", "project-ds", None, false),
            resolver("dataset-resolver", "dataset-ds", None, false),
            resolver("system-default-id", "default-ds", None, true),
        ],
        "system-default-id",
    );
    let loader = RecordingLoader::default();
    execute_append_operation(
        &working,
        &store,
        &loader,
        &project,
        &append_operation(None),
        &context,
    )
    .expect("default resolver append should succeed");
    assert_eq!(
        loader
            .locations
            .borrow()
            .last()
            .expect("location")
            .datasource_id,
        "default-ds"
    );
}

#[test]
fn resolver_when_expression_is_evaluated() {
    let working = working_for_errors();
    let context = AppendExecutionContext {
        run_period: Some("2026-01".to_owned()),
        operation_seq: 2,
        ..Default::default()
    };
    let project = project_with_overrides(BTreeMap::new());

    let matching_resolver = Resolver {
        id: "dataset-resolver".to_owned(),
        name: "dataset".to_owned(),
        description: None,
        version: 1,
        status: ResolverStatus::Active,
        is_default: Some(false),
        rules: vec![
            ResolutionRule {
                name: "old-period".to_owned(),
                when_expression: Some("run_period = '2025-12'".to_owned()),
                data_level: "table".to_owned(),
                strategy: ResolutionStrategy::Path {
                    datasource_id: "old-ds".to_owned(),
                    path: "s3://bucket/{{table_name}}".to_owned(),
                },
            },
            ResolutionRule {
                name: "current-period".to_owned(),
                when_expression: Some("run_period = '2026-01'".to_owned()),
                data_level: "table".to_owned(),
                strategy: ResolutionStrategy::Path {
                    datasource_id: "current-ds".to_owned(),
                    path: "s3://bucket/{{table_name}}".to_owned(),
                },
            },
        ],
        created_at: None,
        updated_at: None,
    };

    let store = RecordingStore::new(
        source_dataset(Some("dataset-resolver")),
        vec![
            matching_resolver,
            resolver("system-default-id", "default-ds", None, true),
        ],
        "system-default-id",
    );
    let loader = RecordingLoader::default();

    execute_append_operation(
        &working,
        &store,
        &loader,
        &project,
        &append_operation(None),
        &context,
    )
    .expect("append should succeed");
    assert_eq!(
        loader
            .locations
            .borrow()
            .last()
            .expect("location")
            .datasource_id,
        "current-ds"
    );
}

#[test]
fn dataset_version_pin_is_used_during_execution() {
    let working = working_for_errors();
    let context = AppendExecutionContext {
        operation_seq: 2,
        ..Default::default()
    };
    let project = project_with_overrides(BTreeMap::new());
    let store = RecordingStore::new(
        source_dataset(Some("dataset-resolver")),
        vec![
            resolver("dataset-resolver", "dataset-ds", None, false),
            resolver("system-default-id", "default-ds", None, true),
        ],
        "system-default-id",
    );
    let loader = RecordingLoader::default();

    execute_append_operation(
        &working,
        &store,
        &loader,
        &project,
        &append_operation(Some(7)),
        &context,
    )
    .expect("append should succeed");

    let requested_versions = store.requested_versions.borrow();
    assert!(!requested_versions.is_empty());
    assert!(requested_versions.iter().all(|version| *version == Some(7)));
}
