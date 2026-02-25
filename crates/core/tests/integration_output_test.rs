use dobo_core::engine::io_traits::{OutputWriter, OutputWriterError};
use dobo_core::engine::ops::output::{
    execute_output, execute_output_with_registration_store,
    execute_output_with_registration_store_and_warning_logger, OutputError, OutputOperation,
    OutputWarning, OutputWarningLogger,
};
use dobo_core::model::{Dataset, OutputDestination, Project, Resolver, RunStatus};
use dobo_core::{DatasetRegistrationStore, MetadataStore, MetadataStoreError};
use polars::prelude::*;
use std::sync::Mutex;
use uuid::Uuid;

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

struct CapturingWriter {
    written_data: Mutex<Vec<DataFrame>>,
}

impl CapturingWriter {
    fn new() -> Self {
        Self {
            written_data: Mutex::new(Vec::new()),
        }
    }

    fn written(&self) -> Vec<DataFrame> {
        self.written_data.lock().unwrap().clone()
    }
}

impl OutputWriter for CapturingWriter {
    fn write(
        &self,
        frame: &DataFrame,
        _destination: &OutputDestination,
    ) -> std::result::Result<(), OutputWriterError> {
        self.written_data.lock().unwrap().push(frame.clone());
        Ok(())
    }
}

struct IntegrationMetadataStore {
    datasets: Mutex<Vec<Dataset>>,
    fail_registration: bool,
}

impl IntegrationMetadataStore {
    fn new() -> Self {
        Self {
            datasets: Mutex::new(Vec::new()),
            fail_registration: false,
        }
    }

    fn failing_registration() -> Self {
        Self {
            datasets: Mutex::new(Vec::new()),
            fail_registration: true,
        }
    }
}

impl MetadataStore for IntegrationMetadataStore {
    fn get_dataset(
        &self,
        _id: &Uuid,
        _version: Option<i32>,
    ) -> std::result::Result<Dataset, MetadataStoreError> {
        Err(MetadataStoreError::OperationFailed {
            message: "not used in integration tests".to_string(),
        })
    }

    fn get_dataset_by_name(
        &self,
        name: &str,
    ) -> std::result::Result<Option<Dataset>, MetadataStoreError> {
        DatasetRegistrationStore::get_dataset_by_name(self, name)
            .map_err(|e| MetadataStoreError::OperationFailed {
                message: e.to_string(),
            })
    }

    fn register_dataset(
        &self,
        dataset: Dataset,
    ) -> std::result::Result<Uuid, MetadataStoreError> {
        DatasetRegistrationStore::register_dataset(self, dataset)
            .map_err(|e| MetadataStoreError::OperationFailed {
                message: e.to_string(),
            })
    }

    fn get_project(
        &self,
        _id: &Uuid,
    ) -> std::result::Result<Project, MetadataStoreError> {
        Err(MetadataStoreError::OperationFailed {
            message: "not used in integration tests".to_string(),
        })
    }

    fn get_resolver(
        &self,
        _id: &str,
    ) -> std::result::Result<Resolver, MetadataStoreError> {
        Err(MetadataStoreError::OperationFailed {
            message: "not used in integration tests".to_string(),
        })
    }

    fn update_run_status(
        &self,
        _id: &Uuid,
        _status: RunStatus,
    ) -> std::result::Result<(), MetadataStoreError> {
        Ok(())
    }
}

impl DatasetRegistrationStore for IntegrationMetadataStore {
    fn get_dataset_by_name(&self, name: &str) -> anyhow::Result<Option<Dataset>> {
        Ok(self
            .datasets
            .lock()
            .unwrap()
            .iter()
            .filter(|dataset| dataset.name == name)
            .max_by_key(|dataset| dataset.version)
            .cloned())
    }

    fn register_dataset(&self, dataset: Dataset) -> anyhow::Result<Uuid> {
        if self.fail_registration {
            anyhow::bail!("registration backend unavailable");
        }

        let id = dataset.id;
        self.datasets.lock().unwrap().push(dataset);
        Ok(id)
    }
}

fn destination() -> OutputDestination {
    OutputDestination::Location {
        path: "output.csv".to_string(),
    }
}

#[test]
fn test_e2e_basic_output() {
    let df = df! {
        "journal_id" => &["J001", "J002", "J003"],
        "account_code" => &["1000", "2000", "3000"],
        "amount" => &[1000.0, 2000.0, 3000.0],
    }
    .unwrap();
    let writer = CapturingWriter::new();
    let operation = OutputOperation {
        destination: destination(),
        selector: None,
        columns: None,
        include_deleted: false,
        register_as_dataset: None,
    };

    let result = execute_output(&df.lazy(), &operation, &writer, None).unwrap();
    assert_eq!(result.rows_written, 3);
    assert_eq!(writer.written().len(), 1);
}

#[test]
fn test_working_dataset_immutability() {
    let df = df! {
        "id" => &[1, 2, 3],
        "value" => &[10, 20, 30],
    }
    .unwrap();

    let working_dataset = df.lazy();
    let before = working_dataset.clone().collect().unwrap();
    let writer = CapturingWriter::new();
    let operation = OutputOperation {
        destination: destination(),
        selector: Some(col("value").gt(lit(10))),
        columns: Some(vec!["id".to_string()]),
        include_deleted: false,
        register_as_dataset: None,
    };

    execute_output(&working_dataset, &operation, &writer, None).unwrap();
    let after = working_dataset.collect().unwrap();
    assert_eq!(before.shape(), after.shape());
    assert_eq!(before.get_column_names(), after.get_column_names());
}

#[test]
fn test_complex_selector_filtering() {
    let df = df! {
        "region" => &["EMEA", "NA", "EMEA", "APAC"],
        "amount" => &[15000, 5000, 12000, 7000],
    }
    .unwrap();
    let writer = CapturingWriter::new();
    let operation = OutputOperation {
        destination: destination(),
        selector: Some(
            col("amount")
                .gt(lit(10000))
                .and(col("region").eq(lit("EMEA"))),
        ),
        columns: None,
        include_deleted: false,
        register_as_dataset: None,
    };

    let result = execute_output(&df.lazy(), &operation, &writer, None).unwrap();
    assert_eq!(result.rows_written, 2);
}

#[test]
fn test_invalid_column_returns_projection_error() {
    let df = df! {
        "id" => &[1, 2, 3],
        "name" => &["Alice", "Bob", "Charlie"],
    }
    .unwrap();
    let writer = CapturingWriter::new();
    let operation = OutputOperation {
        destination: destination(),
        selector: None,
        columns: Some(vec!["id".to_string(), "missing".to_string()]),
        include_deleted: false,
        register_as_dataset: None,
    };

    let error = execute_output(&df.lazy(), &operation, &writer, None).unwrap_err();
    assert!(matches!(error, OutputError::ColumnProjectionError { .. }));
}

#[test]
fn test_register_as_dataset_returns_dataset_id() {
    let df = df! {
        "journal_id" => &["J001", "J002"],
        "amount" => &[1000.0, 2000.0],
    }
    .unwrap();
    let writer = CapturingWriter::new();
    let metadata_store = IntegrationMetadataStore::new();
    let operation = OutputOperation {
        destination: destination(),
        selector: None,
        columns: None,
        include_deleted: false,
        register_as_dataset: Some("monthly_summary".to_string()),
    };

    let result = execute_output_with_registration_store(
        &df.lazy(),
        &operation,
        &writer,
        Some(&metadata_store),
        Some(&metadata_store),
    )
    .unwrap();

    assert!(result.dataset_id.is_some());
    assert_eq!(writer.written().len(), 1);
}

#[test]
fn test_registered_dataset_main_table_matches_destination_target() {
    let df = df! {
        "journal_id" => &["J001", "J002"],
        "amount" => &[1000.0, 2000.0],
    }
    .unwrap();
    let writer = CapturingWriter::new();
    let metadata_store = IntegrationMetadataStore::new();
    let operation = OutputOperation {
        destination: OutputDestination::Table {
            datasource_id: "warehouse".to_string(),
            table: "monthly_summary_out".to_string(),
            schema: Some("finance".to_string()),
        },
        selector: None,
        columns: None,
        include_deleted: false,
        register_as_dataset: Some("monthly_summary".to_string()),
    };

    let _ = execute_output_with_registration_store(
        &df.lazy(),
        &operation,
        &writer,
        Some(&metadata_store),
        Some(&metadata_store),
    )
    .unwrap();

    let datasets = metadata_store.datasets.lock().unwrap();
    assert_eq!(datasets.len(), 1);
    assert_eq!(datasets[0].main_table.name, "monthly_summary_out");
}

#[test]
fn test_register_as_dataset_via_execute_output_primary_path() {
    let df = df! {
        "journal_id" => &["J001", "J002"],
        "amount" => &[1000.0, 2000.0],
    }
    .unwrap();
    let writer = CapturingWriter::new();
    let metadata_store = IntegrationMetadataStore::new();
    let operation = OutputOperation {
        destination: destination(),
        selector: None,
        columns: None,
        include_deleted: false,
        register_as_dataset: Some("monthly_summary".to_string()),
    };

    let result = execute_output(&df.lazy(), &operation, &writer, Some(&metadata_store)).unwrap();

    assert!(result.dataset_id.is_some());
    assert_eq!(writer.written().len(), 1);
}

#[test]
fn test_register_as_dataset_increments_version() {
    let df = df! {
        "journal_id" => &["J001", "J002"],
        "amount" => &[1000.0, 2000.0],
    }
    .unwrap();
    let writer = CapturingWriter::new();
    let metadata_store = IntegrationMetadataStore::new();
    let operation = OutputOperation {
        destination: destination(),
        selector: None,
        columns: None,
        include_deleted: false,
        register_as_dataset: Some("monthly_summary".to_string()),
    };

    let _ = execute_output_with_registration_store(
        &df.clone().lazy(),
        &operation,
        &writer,
        Some(&metadata_store),
        Some(&metadata_store),
    )
    .unwrap();
    let _ = execute_output_with_registration_store(
        &df.lazy(),
        &operation,
        &writer,
        Some(&metadata_store),
        Some(&metadata_store),
    )
    .unwrap();

    let datasets = metadata_store.datasets.lock().unwrap();
    assert_eq!(datasets.len(), 2);
    assert_eq!(datasets[0].version, 1);
    assert_eq!(datasets[1].version, 2);
}

#[test]
fn test_registration_failure_is_non_fatal_after_successful_write() {
    let df = df! {
        "journal_id" => &["J001", "J002"],
        "amount" => &[1000.0, 2000.0],
    }
    .unwrap();
    let writer = CapturingWriter::new();
    let metadata_store = IntegrationMetadataStore::failing_registration();
    let operation = OutputOperation {
        destination: destination(),
        selector: None,
        columns: None,
        include_deleted: false,
        register_as_dataset: Some("monthly_summary".to_string()),
    };

    let result = execute_output_with_registration_store(
        &df.lazy(),
        &operation,
        &writer,
        Some(&metadata_store),
        Some(&metadata_store),
    )
    .unwrap();

    assert_eq!(result.dataset_id, None);
    assert_eq!(result.rows_written, 2);
    assert_eq!(writer.written().len(), 1);
    assert!(metadata_store.datasets.lock().unwrap().is_empty());
}

#[test]
fn test_registration_failure_is_non_fatal_on_primary_execute_output_path() {
    let df = df! {
        "journal_id" => &["J001", "J002"],
        "amount" => &[1000.0, 2000.0],
    }
    .unwrap();
    let writer = CapturingWriter::new();
    let metadata_store = IntegrationMetadataStore::failing_registration();
    let operation = OutputOperation {
        destination: destination(),
        selector: None,
        columns: None,
        include_deleted: false,
        register_as_dataset: Some("monthly_summary".to_string()),
    };

    let result = execute_output(&df.lazy(), &operation, &writer, Some(&metadata_store)).unwrap();

    assert_eq!(result.dataset_id, None);
    assert_eq!(result.rows_written, 2);
    assert_eq!(writer.written().len(), 1);
    assert!(metadata_store.datasets.lock().unwrap().is_empty());
}

#[test]
fn test_registration_failure_emits_warning_event() {
    let df = df! {
        "journal_id" => &["J001", "J002"],
        "amount" => &[1000.0, 2000.0],
    }
    .unwrap();
    let writer = CapturingWriter::new();
    let metadata_store = IntegrationMetadataStore::failing_registration();
    let warning_logger = CapturingWarningLogger::default();
    let operation = OutputOperation {
        destination: destination(),
        selector: None,
        columns: None,
        include_deleted: false,
        register_as_dataset: Some("monthly_summary".to_string()),
    };

    let result = execute_output_with_registration_store_and_warning_logger(
        &df.lazy(),
        &operation,
        &writer,
        Some(&metadata_store),
        Some(&metadata_store),
        &warning_logger,
    )
    .unwrap();

    assert_eq!(result.dataset_id, None);
    assert_eq!(writer.written().len(), 1);

    let warnings = warning_logger.take();
    assert_eq!(warnings.len(), 1);
    assert_eq!(warnings[0].code, "output.registration.failed");
    assert_eq!(warnings[0].dataset_name, "monthly_summary");
    assert_eq!(warnings[0].reason, "registration_backend_error");
}
