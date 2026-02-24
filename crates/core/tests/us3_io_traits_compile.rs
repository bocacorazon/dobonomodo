use dobo_core::{
    DataLoader, DataLoaderError, MetadataStore, MetadataStoreError, OutputWriter,
    OutputWriterError, TraceWriteError, TraceWriter,
};

#[test]
fn io_traits_are_publicly_importable() {
    fn assert_loader<T: DataLoader>() {}
    fn assert_output<T: OutputWriter>() {}
    fn assert_metadata<T: MetadataStore>() {}
    fn assert_trace<T: TraceWriter>() {}

    let _ = assert_loader::<NoopLoader>;
    let _ = assert_output::<NoopOutputWriter>;
    let _ = assert_metadata::<NoopMetadataStore>;
    let _ = assert_trace::<NoopTraceWriter>;
}

struct NoopLoader;
struct NoopOutputWriter;
struct NoopMetadataStore;
struct NoopTraceWriter;

impl DataLoader for NoopLoader {
    fn load(
        &self,
        _location: &dobo_core::model::ResolvedLocation,
        _schema: &dobo_core::model::TableRef,
    ) -> std::result::Result<polars::prelude::LazyFrame, DataLoaderError> {
        Err(DataLoaderError::LoadFailed {
            message: "not implemented".to_string(),
        })
    }
}

impl OutputWriter for NoopOutputWriter {
    fn write(
        &self,
        _frame: &polars::prelude::DataFrame,
        _destination: &dobo_core::model::OutputDestination,
    ) -> std::result::Result<(), OutputWriterError> {
        Err(OutputWriterError::WriteFailed {
            message: "not implemented".to_string(),
        })
    }
}

impl MetadataStore for NoopMetadataStore {
    fn get_dataset(
        &self,
        _id: &uuid::Uuid,
        _version: Option<i32>,
    ) -> std::result::Result<dobo_core::model::Dataset, MetadataStoreError> {
        Err(MetadataStoreError::OperationFailed {
            message: "not implemented".to_string(),
        })
    }

    fn get_project(
        &self,
        _id: &uuid::Uuid,
    ) -> std::result::Result<dobo_core::model::Project, MetadataStoreError> {
        Err(MetadataStoreError::OperationFailed {
            message: "not implemented".to_string(),
        })
    }

    fn get_resolver(
        &self,
        _id: &str,
    ) -> std::result::Result<dobo_core::model::Resolver, MetadataStoreError> {
        Err(MetadataStoreError::OperationFailed {
            message: "not implemented".to_string(),
        })
    }

    fn update_run_status(
        &self,
        _id: &uuid::Uuid,
        _status: dobo_core::model::RunStatus,
    ) -> std::result::Result<(), MetadataStoreError> {
        Err(MetadataStoreError::OperationFailed {
            message: "not implemented".to_string(),
        })
fn sample_project(dataset_id: Uuid) -> Project {
    Project {
        id: Uuid::now_v7(),
        name: "project".to_string(),
        description: None,
        owner: "owner".to_string(),
        version: 1,
        status: ProjectStatus::Draft,
        visibility: Visibility::Private,
        input_dataset_id: dataset_id,
        input_dataset_version: 1,
        materialization: Materialization::Runtime,
        operations: vec![OperationInstance {
            order: 1,
            kind: dobo_core::model::OperationKind::Output,
            alias: None,
            parameters: serde_json::json!({
                "destination": {
                    "destination_type": "memory"
                }
            }),
        }],
        selectors: BTreeMap::new(),
        resolver_overrides: BTreeMap::new(),
        conflict_report: None,
        created_at: None,
        updated_at: None,
    }
}

impl TraceWriter for NoopTraceWriter {
    fn write_events(
        &self,
        _run_id: &uuid::Uuid,
        _events: &[dobo_core::trace::types::TraceEvent],
    ) -> std::result::Result<(), TraceWriteError> {
        Err(TraceWriteError::WriteFailed {
            message: "not implemented".to_string(),
        })
    }
}