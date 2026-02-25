use anyhow::{anyhow, Result};
use dobo_core::engine::io_traits::OutputWriter;
use dobo_core::engine::ops::output::{
    execute_output, execute_output_with_registration_store,
    execute_output_with_registration_store_and_warning_logger, execute_output_with_registry,
    execute_output_with_registry_and_warning_logger, extract_schema, ColumnType, OutputError,
    OutputOperation, OutputWarning, OutputWarningLogger, TemporalMode,
};
use dobo_core::model::{Dataset, OutputDestination, Project, Resolver, RunStatus};
use dobo_core::{DatasetRegistrationStore, MetadataStore};
use polars::prelude::*;
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;
use uuid::Uuid;

struct MockOutputWriter {
    fail_with: Option<String>,
    write_count: Mutex<usize>,
}

#[derive(Default)]
struct CapturingWarningLogger {
    warnings: Mutex<Vec<OutputWarning>>,
}

impl CapturingWarningLogger {
    fn take(&self) -> Vec<OutputWarning> {
        std::mem::take(&mut *self.warnings.lock().unwrap())
    }
}

impl OutputWarningLogger for CapturingWarningLogger {
    fn warn(&self, warning: OutputWarning) {
        self.warnings.lock().unwrap().push(warning);
    }
}

impl MockOutputWriter {
    fn ok() -> Self {
        Self {
            fail_with: None,
            write_count: Mutex::new(0),
        }
    }

    fn failing(message: &str) -> Self {
        Self {
            fail_with: Some(message.to_string()),
            write_count: Mutex::new(0),
        }
    }

    fn writes(&self) -> usize {
        *self.write_count.lock().unwrap()
    }
}

impl OutputWriter for MockOutputWriter {
    fn write(&self, _frame: &DataFrame, _destination: &OutputDestination) -> Result<()> {
        if let Some(message) = self.fail_with.as_ref() {
            return Err(anyhow!(message.clone()));
        }
        let mut count = self.write_count.lock().unwrap();
        *count += 1;
        Ok(())
    }
}

struct MockMetadataStore {
    datasets: Mutex<Vec<Dataset>>,
}

impl MockMetadataStore {
    fn new() -> Self {
        Self {
            datasets: Mutex::new(Vec::new()),
        }
    }
}

impl MetadataStore for MockMetadataStore {
    fn get_dataset(&self, _id: &Uuid, _version: Option<i32>) -> Result<Dataset> {
        Err(anyhow!("not used in tests"))
    }

    fn get_dataset_by_name(&self, name: &str) -> Result<Option<Dataset>> {
        DatasetRegistrationStore::get_dataset_by_name(self, name)
    }

    fn register_dataset(&self, dataset: Dataset) -> Result<Uuid> {
        DatasetRegistrationStore::register_dataset(self, dataset)
    }

    fn get_project(&self, _id: &Uuid) -> Result<Project> {
        Err(anyhow!("not used in tests"))
    }

    fn get_resolver(&self, _id: &str) -> Result<Resolver> {
        Err(anyhow!("not used in tests"))
    }

    fn update_run_status(&self, _id: &Uuid, _status: RunStatus) -> Result<()> {
        Ok(())
    }
}

impl DatasetRegistrationStore for MockMetadataStore {
    fn get_dataset_by_name(&self, name: &str) -> Result<Option<Dataset>> {
        Ok(self
            .datasets
            .lock()
            .unwrap()
            .iter()
            .filter(|dataset| dataset.name == name)
            .max_by_key(|dataset| dataset.version)
            .cloned())
    }

    fn register_dataset(&self, dataset: Dataset) -> Result<Uuid> {
        let id = dataset.id;
        self.datasets.lock().unwrap().push(dataset);
        Ok(id)
    }
}

struct NonRegisteringMetadataStore;

impl MetadataStore for NonRegisteringMetadataStore {
    fn get_dataset(&self, _id: &Uuid, _version: Option<i32>) -> Result<Dataset> {
        Err(anyhow!("not used in tests"))
    }

    fn get_dataset_by_name(&self, _name: &str) -> Result<Option<Dataset>> {
        Err(anyhow!("registration lookup unavailable"))
    }

    fn register_dataset(&self, _dataset: Dataset) -> Result<Uuid> {
        Err(anyhow!("registration backend unavailable"))
    }

    fn get_project(&self, _id: &Uuid) -> Result<Project> {
        Err(anyhow!("not used in tests"))
    }

    fn get_resolver(&self, _id: &str) -> Result<Resolver> {
        Err(anyhow!("not used in tests"))
    }

    fn update_run_status(&self, _id: &Uuid, _status: RunStatus) -> Result<()> {
        Ok(())
    }
}

fn destination() -> OutputDestination {
    OutputDestination::Location {
        path: "output.csv".to_string(),
    }
}

#[test]
fn test_basic_output_all_rows_all_columns() {
    let df = df! {
        "id" => &[1, 2, 3],
        "name" => &["A", "B", "C"],
    }
    .unwrap();
    let writer = MockOutputWriter::ok();
    let operation = OutputOperation {
        destination: destination(),
        selector: None,
        columns: None,
        include_deleted: false,
        register_as_dataset: None,
    };

    let result = execute_output(&df.lazy(), &operation, &writer, None).unwrap();
    assert_eq!(result.rows_written, 3);
    assert_eq!(result.columns_written, vec!["id", "name"]);
    assert_eq!(writer.writes(), 1);
}

#[test]
fn test_valid_table_destination_writes_successfully() {
    let df = df! {
        "id" => &[1, 2, 3],
        "name" => &["A", "B", "C"],
    }
    .unwrap();
    let writer = MockOutputWriter::ok();
    let operation = OutputOperation {
        destination: OutputDestination::Table {
            datasource_id: "warehouse".to_string(),
            table: "fact_output".to_string(),
            schema: Some("finance".to_string()),
        },
        selector: None,
        columns: None,
        include_deleted: false,
        register_as_dataset: None,
    };

    let result = execute_output(&df.lazy(), &operation, &writer, None).unwrap();
    assert_eq!(result.rows_written, 3);
    assert_eq!(writer.writes(), 1);
}

#[test]
fn test_column_projection_and_validation() {
    let df = df! {
        "id" => &[1, 2, 3],
        "name" => &["A", "B", "C"],
        "amount" => &[10, 20, 30],
    }
    .unwrap();
    let writer = MockOutputWriter::ok();
    let operation = OutputOperation {
        destination: destination(),
        selector: None,
        columns: Some(vec!["id".to_string(), "amount".to_string()]),
        include_deleted: false,
        register_as_dataset: None,
    };

    let result = execute_output(&df.lazy(), &operation, &writer, None).unwrap();
    assert_eq!(result.columns_written, vec!["id", "amount"]);
}

#[test]
fn test_missing_columns_reports_available() {
    let df = df! {
        "id" => &[1, 2, 3],
        "name" => &["A", "B", "C"],
    }
    .unwrap();
    let writer = MockOutputWriter::ok();
    let operation = OutputOperation {
        destination: destination(),
        selector: None,
        columns: Some(vec!["id".to_string(), "missing".to_string()]),
        include_deleted: false,
        register_as_dataset: None,
    };

    let error = execute_output(&df.lazy(), &operation, &writer, None).unwrap_err();
    match error {
        OutputError::ColumnProjectionError { missing, available } => {
            assert_eq!(missing, vec!["missing"]);
            assert!(available.contains(&"id".to_string()));
            assert!(available.contains(&"name".to_string()));
        }
        other => panic!("expected ColumnProjectionError, got {other:?}"),
    }
}

#[test]
fn test_empty_column_projection_fails_before_write() {
    let df = df! {
        "id" => &[1, 2, 3],
        "name" => &["A", "B", "C"],
    }
    .unwrap();
    let writer = MockOutputWriter::ok();
    let operation = OutputOperation {
        destination: destination(),
        selector: None,
        columns: Some(Vec::new()),
        include_deleted: false,
        register_as_dataset: None,
    };

    let error = execute_output(&df.lazy(), &operation, &writer, None).unwrap_err();
    assert!(matches!(error, OutputError::EmptyColumnProjection));
    assert_eq!(writer.writes(), 0);
}

#[test]
fn test_excludes_deleted_rows_by_default() {
    let df = df! {
        "id" => &[1, 2, 3],
        "_deleted" => &[false, true, false],
    }
    .unwrap();
    let writer = MockOutputWriter::ok();
    let operation = OutputOperation {
        destination: destination(),
        selector: None,
        columns: None,
        include_deleted: false,
        register_as_dataset: None,
    };

    let result = execute_output(&df.lazy(), &operation, &writer, None).unwrap();
    assert_eq!(result.rows_written, 2);
}

#[test]
fn test_include_deleted_rows_when_requested() {
    let df = df! {
        "id" => &[1, 2, 3],
        "_deleted" => &[false, true, false],
    }
    .unwrap();
    let writer = MockOutputWriter::ok();
    let operation = OutputOperation {
        destination: destination(),
        selector: None,
        columns: None,
        include_deleted: true,
        register_as_dataset: None,
    };

    let result = execute_output(&df.lazy(), &operation, &writer, None).unwrap();
    assert_eq!(result.rows_written, 3);
}

#[test]
fn test_selector_filtering() {
    let df = df! {
        "id" => &[1, 2, 3],
        "amount" => &[100, 200, 300],
    }
    .unwrap();
    let writer = MockOutputWriter::ok();
    let operation = OutputOperation {
        destination: destination(),
        selector: Some(col("amount").gt(lit(150))),
        columns: None,
        include_deleted: true,
        register_as_dataset: None,
    };

    let result = execute_output(&df.lazy(), &operation, &writer, None).unwrap();
    assert_eq!(result.rows_written, 2);
}

#[test]
fn test_selector_cannot_override_deleted_filter_when_include_deleted_false() {
    let df = df! {
        "id" => &[1, 2, 3],
        "_deleted" => &[false, true, true],
    }
    .unwrap();
    let writer = MockOutputWriter::ok();
    let operation = OutputOperation {
        destination: destination(),
        selector: Some(col("_deleted").eq(lit(true))),
        columns: None,
        include_deleted: false,
        register_as_dataset: None,
    };

    let result = execute_output(&df.lazy(), &operation, &writer, None).unwrap();
    assert_eq!(result.rows_written, 0);
}

#[test]
fn test_selector_error_when_non_boolean_expression() {
    let df = df! {
        "id" => &[1, 2, 3],
        "amount" => &[100, 200, 300],
    }
    .unwrap();
    let writer = MockOutputWriter::ok();
    let operation = OutputOperation {
        destination: destination(),
        selector: Some(col("amount") + lit(1)),
        columns: None,
        include_deleted: true,
        register_as_dataset: None,
    };

    let error = execute_output(&df.lazy(), &operation, &writer, None).unwrap_err();
    assert!(matches!(error, OutputError::SelectorError(_)));
}

#[test]
fn test_missing_metadata_store_when_registration_requested() {
    let df = df! { "id" => &[1, 2, 3] }.unwrap();
    let writer = MockOutputWriter::ok();
    let operation = OutputOperation {
        destination: destination(),
        selector: None,
        columns: None,
        include_deleted: true,
        register_as_dataset: Some("registered".to_string()),
    };

    let error = execute_output(&df.lazy(), &operation, &writer, None).unwrap_err();
    assert!(matches!(error, OutputError::MissingMetadataStore));
    assert_eq!(writer.writes(), 0);
}

#[test]
fn test_register_dataset_and_increment_version() {
    let df = df! { "id" => &[1, 2, 3] }.unwrap();
    let writer = MockOutputWriter::ok();
    let store = MockMetadataStore::new();
    let operation = OutputOperation {
        destination: destination(),
        selector: None,
        columns: None,
        include_deleted: true,
        register_as_dataset: Some("my_dataset".to_string()),
    };

    let first = execute_output_with_registration_store(
        &df.clone().lazy(),
        &operation,
        &writer,
        Some(&store),
        Some(&store),
    )
    .unwrap();
    let second = execute_output_with_registration_store(
        &df.clone().lazy(),
        &operation,
        &writer,
        Some(&store),
        Some(&store),
    )
    .unwrap();
    assert!(first.dataset_id.is_some());
    assert!(second.dataset_id.is_some());
    assert_eq!(writer.writes(), 2);

    let datasets = store.datasets.lock().unwrap();
    assert_eq!(datasets.len(), 2);
    assert_eq!(datasets[0].version, 1);
    assert_eq!(datasets[1].version, 2);
}

#[test]
fn test_execute_output_registers_via_metadata_store_primary_path() {
    let df = df! { "id" => &[1, 2, 3] }.unwrap();
    let writer = MockOutputWriter::ok();
    let store = MockMetadataStore::new();
    let operation = OutputOperation {
        destination: destination(),
        selector: None,
        columns: None,
        include_deleted: true,
        register_as_dataset: Some("registered".to_string()),
    };

    let result = execute_output(&df.lazy(), &operation, &writer, Some(&store)).unwrap();
    assert!(result.dataset_id.is_some());
    assert_eq!(writer.writes(), 1);
    assert_eq!(store.datasets.lock().unwrap().len(), 1);
}

#[test]
fn test_write_failed_error() {
    let df = df! { "id" => &[1, 2, 3] }.unwrap();
    let writer = MockOutputWriter::failing("disk full");
    let operation = OutputOperation {
        destination: destination(),
        selector: None,
        columns: None,
        include_deleted: true,
        register_as_dataset: None,
    };

    let error = execute_output(&df.lazy(), &operation, &writer, None).unwrap_err();
    assert!(matches!(error, OutputError::WriteFailed(_)));
}

#[test]
fn test_no_dataset_registration_on_write_failure() {
    let df = df! { "id" => &[1, 2, 3] }.unwrap();
    let writer = MockOutputWriter::failing("destination unavailable");
    let store = MockMetadataStore::new();
    let operation = OutputOperation {
        destination: destination(),
        selector: None,
        columns: None,
        include_deleted: true,
        register_as_dataset: Some("not_written".to_string()),
    };

    let _ = execute_output(&df.lazy(), &operation, &writer, Some(&store));
    assert!(store.datasets.lock().unwrap().is_empty());
}

#[test]
fn test_registration_failure_on_primary_path_is_non_fatal() {
    let df = df! { "id" => &[1, 2, 3] }.unwrap();
    let writer = MockOutputWriter::ok();
    let store = NonRegisteringMetadataStore;
    let operation = OutputOperation {
        destination: destination(),
        selector: None,
        columns: None,
        include_deleted: true,
        register_as_dataset: Some("registered".to_string()),
    };

    let result = execute_output(&df.lazy(), &operation, &writer, Some(&store)).unwrap();
    assert_eq!(result.dataset_id, None);
    assert_eq!(writer.writes(), 1);
}

#[test]
fn test_execute_output_with_registry_uses_metadata_path_without_registration_adapter() {
    let df = df! { "id" => &[1, 2, 3] }.unwrap();
    let writer = MockOutputWriter::ok();
    let store = NonRegisteringMetadataStore;
    let operation = OutputOperation {
        destination: destination(),
        selector: None,
        columns: None,
        include_deleted: true,
        register_as_dataset: Some("registered".to_string()),
    };

    let result =
        execute_output_with_registry(&df.lazy(), &operation, &writer, Some(&store)).unwrap();
    assert_eq!(result.dataset_id, None);
    assert_eq!(writer.writes(), 1);
}

#[test]
fn test_execute_output_with_registry_registration_failure_is_non_fatal() {
    let df = df! { "id" => &[1, 2, 3] }.unwrap();
    let writer = MockOutputWriter::ok();
    let store = NonRegisteringMetadataStore;
    let operation = OutputOperation {
        destination: destination(),
        selector: None,
        columns: None,
        include_deleted: true,
        register_as_dataset: Some("registered".to_string()),
    };

    let result =
        execute_output_with_registry(&df.lazy(), &operation, &writer, Some(&store)).unwrap();
    assert_eq!(result.dataset_id, None);
    assert_eq!(writer.writes(), 1);
}

#[test]
fn test_execute_output_with_registry_registration_failure_emits_warning() {
    let df = df! { "id" => &[1, 2, 3] }.unwrap();
    let writer = MockOutputWriter::ok();
    let store = NonRegisteringMetadataStore;
    let warning_logger = CapturingWarningLogger::default();
    let operation = OutputOperation {
        destination: destination(),
        selector: None,
        columns: None,
        include_deleted: true,
        register_as_dataset: Some("registered".to_string()),
    };

    let result = execute_output_with_registry_and_warning_logger(
        &df.lazy(),
        &operation,
        &writer,
        Some(&store),
        &warning_logger,
    )
    .unwrap();
    assert_eq!(result.dataset_id, None);

    let warnings = warning_logger.take();
    assert_eq!(warnings.len(), 1);
    assert_eq!(warnings[0].code, "output.registration.failed");
    assert_eq!(warnings[0].dataset_name, "registered");
    assert_eq!(warnings[0].reason, "registration_backend_error");
}

struct FailingRegistrationStore;

impl DatasetRegistrationStore for FailingRegistrationStore {
    fn get_dataset_by_name(&self, _name: &str) -> Result<Option<Dataset>> {
        Ok(None)
    }

    fn register_dataset(&self, _dataset: Dataset) -> Result<Uuid> {
        Err(anyhow!("registration backend unavailable"))
    }
}

#[test]
fn test_execute_output_with_registration_store_failure_emits_warning() {
    let df = df! { "id" => &[1, 2, 3] }.unwrap();
    let writer = MockOutputWriter::ok();
    let warning_logger = CapturingWarningLogger::default();
    let operation = OutputOperation {
        destination: destination(),
        selector: None,
        columns: None,
        include_deleted: true,
        register_as_dataset: Some("registered".to_string()),
    };

    let result = execute_output_with_registration_store_and_warning_logger(
        &df.lazy(),
        &operation,
        &writer,
        None,
        Some(&FailingRegistrationStore),
        &warning_logger,
    )
    .unwrap();
    assert_eq!(result.dataset_id, None);

    let warnings = warning_logger.take();
    assert_eq!(warnings.len(), 1);
    assert_eq!(warnings[0].code, "output.registration.failed");
    assert_eq!(warnings[0].dataset_name, "registered");
    assert_eq!(warnings[0].reason, "registration_backend_error");
}

#[test]
fn test_invalid_destination_type_fails_before_write() {
    let df = df! { "id" => &[1, 2, 3] }.unwrap();
    let writer = MockOutputWriter::ok();
    let operation = OutputOperation {
        destination: OutputDestination::Table {
            datasource_id: "   ".to_string(),
            table: "out_table".to_string(),
            schema: None,
        },
        selector: None,
        columns: None,
        include_deleted: true,
        register_as_dataset: None,
    };

    let error = execute_output(&df.lazy(), &operation, &writer, None).unwrap_err();
    assert!(matches!(error, OutputError::InvalidDestination(_)));
    assert_eq!(writer.writes(), 0);
}

#[test]
fn test_missing_destination_target_fails_before_write() {
    let df = df! { "id" => &[1, 2, 3] }.unwrap();
    let writer = MockOutputWriter::ok();
    let operation = OutputOperation {
        destination: OutputDestination::Location {
            path: "   ".to_string(),
        },
        selector: None,
        columns: None,
        include_deleted: true,
        register_as_dataset: None,
    };

    let error = execute_output(&df.lazy(), &operation, &writer, None).unwrap_err();
    assert!(matches!(error, OutputError::InvalidDestination(_)));
    assert_eq!(writer.writes(), 0);
}

#[test]
fn test_empty_table_destination_name_fails_before_write() {
    let df = df! { "id" => &[1, 2, 3] }.unwrap();
    let writer = MockOutputWriter::ok();
    let operation = OutputOperation {
        destination: OutputDestination::Table {
            datasource_id: "ds-main".to_string(),
            table: "   ".to_string(),
            schema: None,
        },
        selector: None,
        columns: None,
        include_deleted: true,
        register_as_dataset: None,
    };

    let error = execute_output(&df.lazy(), &operation, &writer, None).unwrap_err();
    assert!(matches!(error, OutputError::InvalidDestination(_)));
    assert_eq!(writer.writes(), 0);
}

#[test]
fn test_registration_store_can_register_without_metadata_store() {
    let df = df! { "id" => &[1, 2, 3] }.unwrap();
    let writer = MockOutputWriter::ok();
    let registration_store = MockMetadataStore::new();
    let operation = OutputOperation {
        destination: destination(),
        selector: None,
        columns: None,
        include_deleted: true,
        register_as_dataset: Some("registered".to_string()),
    };

    let result = execute_output_with_registration_store(
        &df.lazy(),
        &operation,
        &writer,
        None,
        Some(&registration_store),
    )
    .unwrap();
    assert!(result.dataset_id.is_some());
    assert_eq!(writer.writes(), 1);
}

#[test]
fn test_registration_store_takes_precedence_when_metadata_provided() {
    let df = df! { "id" => &[1, 2, 3] }.unwrap();
    let writer = MockOutputWriter::ok();
    let metadata_store = NonRegisteringMetadataStore;
    let registration_store = MockMetadataStore::new();
    let operation = OutputOperation {
        destination: destination(),
        selector: None,
        columns: None,
        include_deleted: true,
        register_as_dataset: Some("registered".to_string()),
    };

    let result = execute_output_with_registration_store(
        &df.lazy(),
        &operation,
        &writer,
        Some(&metadata_store),
        Some(&registration_store),
    )
    .unwrap();
    assert!(result.dataset_id.is_some());
    assert_eq!(writer.writes(), 1);
}

#[test]
fn test_registration_helper_requires_metadata_or_registration_store() {
    let df = df! { "id" => &[1, 2, 3] }.unwrap();
    let writer = MockOutputWriter::ok();
    let operation = OutputOperation {
        destination: destination(),
        selector: None,
        columns: None,
        include_deleted: true,
        register_as_dataset: Some("registered".to_string()),
    };

    let error = execute_output_with_registration_store(&df.lazy(), &operation, &writer, None, None)
        .unwrap_err();
    assert!(matches!(error, OutputError::MissingMetadataStore));
    assert_eq!(writer.writes(), 0);
}

#[test]
fn test_invalid_dataset_name_fails_before_write() {
    let df = df! { "id" => &[1, 2, 3] }.unwrap();
    let writer = MockOutputWriter::ok();
    let store = MockMetadataStore::new();
    let operation = OutputOperation {
        destination: destination(),
        selector: None,
        columns: None,
        include_deleted: true,
        register_as_dataset: Some("   ".to_string()),
    };

    let error = execute_output(&df.lazy(), &operation, &writer, Some(&store)).unwrap_err();
    assert!(matches!(error, OutputError::InvalidDatasetName(_)));
    assert_eq!(writer.writes(), 0);
}

#[test]
fn test_execute_output_collects_once_contract() {
    let output_rs = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/engine/ops/output.rs");
    let source = fs::read_to_string(output_rs).unwrap();
    let function = source
        .split("pub fn execute_output_with_registration_store_and_warning_logger")
        .nth(1)
        .unwrap();

    assert_eq!(function.matches("output_frame.collect(").count(), 1);
}

#[test]
fn test_extract_schema() {
    let df = df! {
        "id" => &[1, 2, 3],
        "name" => &["Alice", "Bob", "Charlie"],
        "amount" => &[100.5, 200.75, 300.25],
    }
    .unwrap();

    let schema = extract_schema(&df).unwrap();
    assert_eq!(schema.columns.len(), 3);
    assert_eq!(schema.columns[0].name, "id");
    assert_eq!(schema.columns[0].data_type, ColumnType::Integer);
    assert_eq!(schema.columns[1].name, "name");
    assert_eq!(schema.columns[1].data_type, ColumnType::String);
    assert_eq!(schema.temporal_mode, TemporalMode::None);
}

#[test]
fn test_extract_schema_with_period() {
    let df = df! {
        "id" => &[1, 2],
        "_period" => &["2024-01", "2024-02"],
    }
    .unwrap();

    let schema = extract_schema(&df).unwrap();
    assert_eq!(schema.temporal_mode, TemporalMode::Period);
}

#[test]
fn test_extract_schema_rejects_unsupported_dtype() {
    let df = df! {
        "payload" => &[b"a".as_ref(), b"b".as_ref()],
    }
    .unwrap();

    let result = extract_schema(&df);
    assert!(matches!(result, Err(OutputError::InvalidSchema(_))));
}

#[test]
fn test_extract_schema_empty_dataframe() {
    let df = DataFrame::empty();
    let result = extract_schema(&df);
    assert!(matches!(result, Err(OutputError::EmptyDataFrame)));
}
