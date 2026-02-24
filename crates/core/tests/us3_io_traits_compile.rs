use std::collections::{BTreeMap, HashMap};
use std::sync::{Arc, Mutex};

use dobo_core::model::{
    ColumnDef, ColumnType, Dataset, DatasetStatus, Materialization, OperationInstance,
    OutputDestination, Project, ProjectStatus, ResolutionRule, ResolutionStrategy,
    ResolvedLocation, Resolver, ResolverStatus, RunStatus, TableRef, Visibility,
};
use dobo_core::trace::types::TraceEvent;
use dobo_core::{
    DataLoader, DataLoaderError, MetadataStore, MetadataStoreError, OutputWriter,
    OutputWriterError, TraceWriteError, TraceWriter,
};
use polars::df;
use polars::prelude::{DataFrame, IntoLazy};
use uuid::Uuid;

#[test]
fn io_traits_are_publicly_importable() {
    fn assert_loader<T: DataLoader>() {}
    fn assert_output<T: OutputWriter>() {}
    fn assert_metadata<T: MetadataStore>() {}
    fn assert_trace<T: TraceWriter>() {}

    let _ = assert_loader::<InMemoryLoader>;
    let _ = assert_output::<InMemoryOutputWriter>;
    let _ = assert_metadata::<InMemoryMetadataStore>;
    let _ = assert_trace::<InMemoryTraceWriter>;
}

#[derive(Default)]
struct InMemoryLoader {
    frames_by_location: HashMap<String, DataFrame>,
}

impl DataLoader for InMemoryLoader {
    fn load(
        &self,
        location: &ResolvedLocation,
        _schema: &TableRef,
    ) -> Result<polars::prelude::LazyFrame, DataLoaderError> {
        let key = location
            .path
            .as_deref()
            .or(location.table.as_deref())
            .ok_or_else(|| DataLoaderError::LoadFailed {
                message: "location must provide path or table".to_string(),
            })?;
        let frame =
            self.frames_by_location
                .get(key)
                .ok_or_else(|| DataLoaderError::LoadFailed {
                    message: format!("no frame for location '{key}'"),
                })?;
        Ok(frame.clone().lazy())
    }
}

#[derive(Default, Clone)]
struct InMemoryOutputWriter {
    writes: Arc<Mutex<Vec<(usize, OutputDestination)>>>,
}

impl OutputWriter for InMemoryOutputWriter {
    fn write(
        &self,
        frame: &DataFrame,
        destination: &OutputDestination,
    ) -> Result<(), OutputWriterError> {
        self.writes
            .lock()
            .expect("lock should succeed")
            .push((frame.height(), destination.clone()));
        Ok(())
    }
}

#[derive(Default, Clone)]
struct InMemoryMetadataStore {
    datasets: Arc<Mutex<HashMap<Uuid, Dataset>>>,
    projects: Arc<Mutex<HashMap<Uuid, Project>>>,
    resolvers: Arc<Mutex<HashMap<String, Resolver>>>,
    run_status_updates: Arc<Mutex<Vec<(Uuid, RunStatus)>>>,
}

impl MetadataStore for InMemoryMetadataStore {
    fn get_dataset(&self, id: &Uuid, _version: Option<i32>) -> Result<Dataset, MetadataStoreError> {
        self.datasets
            .lock()
            .expect("lock should succeed")
            .get(id)
            .cloned()
            .ok_or(MetadataStoreError::DatasetNotFound { id: *id })
    }

    fn get_project(&self, id: &Uuid) -> Result<Project, MetadataStoreError> {
        self.projects
            .lock()
            .expect("lock should succeed")
            .get(id)
            .cloned()
            .ok_or(MetadataStoreError::ProjectNotFound { id: *id })
    }

    fn get_resolver(&self, id: &str) -> Result<Resolver, MetadataStoreError> {
        self.resolvers
            .lock()
            .expect("lock should succeed")
            .get(id)
            .cloned()
            .ok_or_else(|| MetadataStoreError::ResolverNotFound { id: id.to_string() })
    }

    fn update_run_status(&self, id: &Uuid, status: RunStatus) -> Result<(), MetadataStoreError> {
        self.run_status_updates
            .lock()
            .expect("lock should succeed")
            .push((*id, status));
        Ok(())
    }
}

#[derive(Default, Clone)]
struct InMemoryTraceWriter {
    events_by_run: Arc<Mutex<HashMap<Uuid, Vec<TraceEvent>>>>,
}

impl TraceWriter for InMemoryTraceWriter {
    fn write_events(&self, run_id: &Uuid, events: &[TraceEvent]) -> Result<(), TraceWriteError> {
        self.events_by_run
            .lock()
            .expect("lock should succeed")
            .entry(*run_id)
            .or_default()
            .extend_from_slice(events);
        Ok(())
    }
}

#[test]
fn data_loader_loads_expected_frame() {
    let mut loader = InMemoryLoader::default();
    loader.frames_by_location.insert(
        "transactions.parquet".to_string(),
        df! {"transactions.amount" => [1.0f64, 2.0f64]}.expect("frame should build"),
    );
    let location = ResolvedLocation {
        datasource_id: "ds".to_string(),
        path: Some("transactions.parquet".to_string()),
        table: None,
        schema: None,
        period_identifier: None,
        catalog_response: None,
    };

    let out = loader
        .load(&location, &sample_table_ref())
        .expect("load should succeed")
        .collect()
        .expect("collect should succeed");
    assert_eq!(out.height(), 2);
}

#[test]
fn data_loader_propagates_missing_location_error() {
    let loader = InMemoryLoader::default();
    let location = ResolvedLocation {
        datasource_id: "ds".to_string(),
        path: Some("missing.parquet".to_string()),
        table: None,
        schema: None,
        period_identifier: None,
        catalog_response: None,
    };
    let err = match loader.load(&location, &sample_table_ref()) {
        Err(err) => err,
        Ok(_) => panic!("missing location should fail"),
    };
    assert!(err.to_string().contains("no frame for location"));
}

#[test]
fn output_writer_records_writes() {
    let writer = InMemoryOutputWriter::default();
    let frame = df! {"value" => [1i64, 2i64, 3i64]}.expect("frame should build");
    let destination = OutputDestination {
        destination_type: "table".to_string(),
        target: Some("analytics.results".to_string()),
    };

    writer
        .write(&frame, &destination)
        .expect("write should succeed");
    let writes = writer.writes.lock().expect("lock should succeed");
    assert_eq!(writes.len(), 1);
    assert_eq!(writes[0].0, 3);
    assert_eq!(writes[0].1.target.as_deref(), Some("analytics.results"));
}

#[test]
fn metadata_store_returns_entities_and_updates_status() {
    let store = InMemoryMetadataStore::default();
    let dataset = sample_dataset();
    let project = sample_project(dataset.id);
    let resolver = sample_resolver();
    let run_id = Uuid::now_v7();

    store
        .datasets
        .lock()
        .expect("lock should succeed")
        .insert(dataset.id, dataset.clone());
    store
        .projects
        .lock()
        .expect("lock should succeed")
        .insert(project.id, project.clone());
    store
        .resolvers
        .lock()
        .expect("lock should succeed")
        .insert(resolver.id.clone(), resolver.clone());

    assert_eq!(
        store
            .get_dataset(&dataset.id, None)
            .expect("dataset should exist")
            .name,
        dataset.name
    );
    assert_eq!(
        store
            .get_project(&project.id)
            .expect("project should exist")
            .name,
        project.name
    );
    assert_eq!(
        store
            .get_resolver(&resolver.id)
            .expect("resolver should exist")
            .name,
        resolver.name
    );

    store
        .update_run_status(&run_id, RunStatus::Running)
        .expect("status update should succeed");
    let updates = store
        .run_status_updates
        .lock()
        .expect("lock should succeed");
    assert_eq!(updates.as_slice(), &[(run_id, RunStatus::Running)]);
}

#[test]
fn metadata_store_propagates_missing_error() {
    let store = InMemoryMetadataStore::default();
    let err = store
        .get_resolver("missing")
        .expect_err("missing resolver should fail");
    assert!(err.to_string().contains("resolver 'missing' not found"));
}

#[test]
fn trace_writer_persists_events_per_run() {
    let writer = InMemoryTraceWriter::default();
    let run_id = Uuid::now_v7();
    let events = vec![
        TraceEvent {
            operation_order: 1,
            message: "started".to_string(),
        },
        TraceEvent {
            operation_order: 2,
            message: "completed".to_string(),
        },
    ];

    writer
        .write_events(&run_id, &events)
        .expect("trace write should succeed");
    let stored = writer
        .events_by_run
        .lock()
        .expect("lock should succeed")
        .get(&run_id)
        .cloned()
        .expect("events should be stored");
    assert_eq!(stored, events);
}

fn sample_table_ref() -> TableRef {
    TableRef {
        name: "transactions".to_string(),
        temporal_mode: None,
        columns: vec![ColumnDef {
            name: "amount".to_string(),
            column_type: ColumnType::Decimal,
            nullable: Some(true),
            description: None,
        }],
    }
}

fn sample_dataset() -> Dataset {
    Dataset {
        id: Uuid::now_v7(),
        name: "transactions".to_string(),
        description: None,
        owner: "owner".to_string(),
        version: 1,
        status: DatasetStatus::Active,
        resolver_id: Some("resolver.default".to_string()),
        main_table: sample_table_ref(),
        lookups: vec![],
        natural_key_columns: vec!["id".to_string()],
        created_at: None,
        updated_at: None,
    }
}

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

fn sample_resolver() -> Resolver {
    Resolver {
        id: "resolver.default".to_string(),
        name: "Default Resolver".to_string(),
        description: None,
        version: 1,
        status: ResolverStatus::Active,
        is_default: Some(true),
        rules: vec![ResolutionRule {
            name: "default".to_string(),
            when_expression: None,
            data_level: "dataset".to_string(),
            strategy: ResolutionStrategy::Path {
                datasource_id: "lake".to_string(),
                path: "/data/input".to_string(),
            },
        }],
        created_at: None,
        updated_at: None,
    }
}
