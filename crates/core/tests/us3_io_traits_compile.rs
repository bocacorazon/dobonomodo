use std::collections::HashMap;
use std::sync::Mutex;

use anyhow::{anyhow, bail, Result};
use dobo_core::model::{
    ColumnDef, ColumnType, Dataset, DatasetStatus, OperationInstance, OperationKind,
    OutputDestination, Project, ProjectStatus, ResolutionRule, ResolutionStrategy,
    ResolvedLocation, Resolver, ResolverStatus, RunStatus, TableRef, Visibility,
};
use dobo_core::trace::types::TraceEvent;
use dobo_core::{DataLoader, MetadataStore, OutputWriter, TraceWriter};
use polars::prelude::{DataFrame, IntoLazy, LazyFrame, NamedFrom, Series};
use uuid::Uuid;

struct InMemoryLoader {
    frame: LazyFrame,
    fail: bool,
    calls: Mutex<usize>,
}

impl DataLoader for InMemoryLoader {
    fn load(&self, _location: &ResolvedLocation, _schema: &TableRef) -> Result<LazyFrame> {
        *self.calls.lock().expect("calls lock should be available") += 1;
        if self.fail {
            bail!("loader failed")
        } else {
            Ok(self.frame.clone())
        }
    }
}

struct InMemoryOutputWriter {
    fail: bool,
    writes: Mutex<Vec<(usize, String)>>,
}

impl OutputWriter for InMemoryOutputWriter {
    fn write(&self, frame: &DataFrame, destination: &OutputDestination) -> Result<()> {
        if self.fail {
            bail!("writer failed")
        }

        self.writes
            .lock()
            .expect("writes lock should be available")
            .push((frame.height(), destination.destination_type.clone()));
        Ok(())
    }
}

struct InMemoryMetadataStore {
    datasets_by_version: HashMap<(Uuid, i32), Dataset>,
    latest_versions: HashMap<Uuid, i32>,
    projects: HashMap<Uuid, Project>,
    resolvers: HashMap<String, Resolver>,
    fail_updates: bool,
    updates: Mutex<Vec<(Uuid, RunStatus)>>,
}

impl MetadataStore for InMemoryMetadataStore {
    fn get_dataset(&self, id: &Uuid, version: Option<i32>) -> Result<Dataset> {
        let resolved_version = match version {
            Some(version) => version,
            None => *self
                .latest_versions
                .get(id)
                .ok_or_else(|| anyhow!("dataset not found: {id}"))?,
        };

        self.datasets_by_version
            .get(&(*id, resolved_version))
            .cloned()
            .ok_or_else(|| anyhow!("dataset not found: {id}"))
    }

    fn get_project(&self, id: &Uuid) -> Result<Project> {
        self.projects
            .get(id)
            .cloned()
            .ok_or_else(|| anyhow!("project not found: {id}"))
    }

    fn get_resolver(&self, id: &str) -> Result<Resolver> {
        self.resolvers
            .get(id)
            .cloned()
            .ok_or_else(|| anyhow!("resolver not found: {id}"))
    }

    fn update_run_status(&self, id: &Uuid, status: RunStatus) -> Result<()> {
        if self.fail_updates {
            bail!("status update failed")
        }
        self.updates
            .lock()
            .expect("updates lock should be available")
            .push((*id, status));
        Ok(())
    }
}

struct InMemoryTraceWriter {
    fail: bool,
    writes: Mutex<Vec<(Uuid, usize)>>,
}

impl TraceWriter for InMemoryTraceWriter {
    fn write_events(&self, run_id: &Uuid, events: &[TraceEvent]) -> Result<()> {
        if self.fail {
            bail!("trace write failed")
        }
        self.writes
            .lock()
            .expect("writes lock should be available")
            .push((*run_id, events.len()));
        Ok(())
    }
}

fn sample_dataset(id: Uuid, version: i32) -> Dataset {
    Dataset {
        id,
        name: "dataset".to_string(),
        description: None,
        owner: "owner".to_string(),
        version,
        status: DatasetStatus::Active,
        resolver_id: None,
        main_table: TableRef {
            name: "transactions".to_string(),
            temporal_mode: None,
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

fn sample_project(id: Uuid, input_dataset_id: Uuid) -> Project {
    Project {
        id,
        name: "project".to_string(),
        description: None,
        owner: "owner".to_string(),
        version: 1,
        status: ProjectStatus::Draft,
        visibility: Visibility::Private,
        input_dataset_id,
        input_dataset_version: 1,
        materialization: dobo_core::model::Materialization::Eager,
        operations: vec![OperationInstance {
            order: 1,
            kind: OperationKind::Output,
            alias: None,
            parameters: serde_json::json!({}),
        }],
        selectors: Default::default(),
        resolver_overrides: Default::default(),
        conflict_report: None,
        created_at: None,
        updated_at: None,
    }
}

fn sample_resolver(id: &str) -> Resolver {
    Resolver {
        id: id.to_string(),
        name: "resolver".to_string(),
        description: None,
        version: 1,
        status: ResolverStatus::Active,
        is_default: Some(true),
        rules: vec![ResolutionRule {
            name: "rule".to_string(),
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

#[test]
fn data_loader_loads_frames_and_propagates_errors() {
    let df = DataFrame::new(vec![Series::new("id".into(), &[1, 2]).into()]).expect("df is valid");
    let location = ResolvedLocation {
        datasource_id: "ds".to_string(),
        path: Some("path".to_string()),
        table: None,
        schema: None,
        period_identifier: None,
    };
    let table_ref = TableRef {
        name: "transactions".to_string(),
        temporal_mode: None,
        columns: vec![],
    };

    let loader = InMemoryLoader {
        frame: df.clone().lazy(),
        fail: false,
        calls: Mutex::new(0),
    };
    let loaded = loader
        .load(&location, &table_ref)
        .expect("loader should return frame")
        .collect()
        .expect("frame should collect");
    assert_eq!(loaded.height(), 2);
    assert_eq!(*loader.calls.lock().expect("calls lock"), 1);

    let failing_loader = InMemoryLoader {
        frame: df.lazy(),
        fail: true,
        calls: Mutex::new(0),
    };
    let error = match failing_loader.load(&location, &table_ref) {
        Ok(_) => panic!("loader should fail"),
        Err(err) => err,
    };
    assert!(error.to_string().contains("loader failed"));
}

#[test]
fn output_writer_writes_and_propagates_errors() {
    let writer = InMemoryOutputWriter {
        fail: false,
        writes: Mutex::new(Vec::new()),
    };
    let frame =
        DataFrame::new(vec![Series::new("amount".into(), &[10, 20]).into()]).expect("df is valid");
    let destination = OutputDestination {
        destination_type: "table".to_string(),
        target: Some("target_table".to_string()),
    };
    writer
        .write(&frame, &destination)
        .expect("writer should succeed");
    let writes = writer.writes.lock().expect("writes lock");
    assert_eq!(writes.len(), 1);
    assert_eq!(writes[0], (2, "table".to_string()));

    let failing_writer = InMemoryOutputWriter {
        fail: true,
        writes: Mutex::new(Vec::new()),
    };
    let error = failing_writer
        .write(&frame, &destination)
        .expect_err("writer should fail");
    assert!(error.to_string().contains("writer failed"));
}

#[test]
fn metadata_store_gets_entities_tracks_updates_and_propagates_errors() {
    let dataset_id = Uuid::from_u128(1);
    let project_id = Uuid::from_u128(2);
    let run_id = Uuid::from_u128(3);
    let resolver_id = "resolver-1";
    let dataset_v1 = sample_dataset(dataset_id, 1);
    let dataset_v2 = sample_dataset(dataset_id, 2);
    let project = sample_project(project_id, dataset_id);
    let resolver = sample_resolver(resolver_id);

    let mut datasets_by_version = HashMap::new();
    datasets_by_version.insert((dataset_id, 1), dataset_v1.clone());
    datasets_by_version.insert((dataset_id, 2), dataset_v2.clone());
    let mut latest_versions = HashMap::new();
    latest_versions.insert(dataset_id, 2);
    let mut projects = HashMap::new();
    projects.insert(project_id, project.clone());
    let mut resolvers = HashMap::new();
    resolvers.insert(resolver_id.to_string(), resolver.clone());

    let store = InMemoryMetadataStore {
        datasets_by_version,
        latest_versions,
        projects,
        resolvers,
        fail_updates: false,
        updates: Mutex::new(Vec::new()),
    };

    assert_eq!(
        store
            .get_dataset(&dataset_id, Some(1))
            .expect("dataset should exist"),
        dataset_v1
    );
    assert_eq!(
        store
            .get_dataset(&dataset_id, Some(2))
            .expect("dataset should exist"),
        dataset_v2.clone()
    );
    assert_eq!(
        store
            .get_dataset(&dataset_id, None)
            .expect("latest dataset should exist"),
        dataset_v2
    );
    assert_eq!(
        store
            .get_project(&project_id)
            .expect("project should exist"),
        project
    );
    assert_eq!(
        store
            .get_resolver(resolver_id)
            .expect("resolver should exist"),
        resolver
    );
    store
        .update_run_status(&run_id, RunStatus::Running)
        .expect("status update should succeed");
    store
        .update_run_status(&run_id, RunStatus::Completed)
        .expect("status update should succeed");
    let updates = store.updates.lock().expect("updates lock");
    assert_eq!(updates.len(), 2);
    assert_eq!(updates[1], (run_id, RunStatus::Completed));

    let missing = store
        .get_dataset(&Uuid::from_u128(999), None)
        .expect_err("missing dataset should fail");
    assert!(missing.to_string().contains("dataset not found"));
    let missing_version = store
        .get_dataset(&dataset_id, Some(999))
        .expect_err("missing dataset version should fail");
    assert!(missing_version.to_string().contains("dataset not found"));

    let failing_store = InMemoryMetadataStore {
        datasets_by_version: HashMap::new(),
        latest_versions: HashMap::new(),
        projects: HashMap::new(),
        resolvers: HashMap::new(),
        fail_updates: true,
        updates: Mutex::new(Vec::new()),
    };
    let update_error = failing_store
        .update_run_status(&run_id, RunStatus::Failed)
        .expect_err("status update should fail");
    assert!(update_error.to_string().contains("status update failed"));
}

#[test]
fn trace_writer_writes_events_and_propagates_errors() {
    let writer = InMemoryTraceWriter {
        fail: false,
        writes: Mutex::new(Vec::new()),
    };
    let run_id = Uuid::from_u128(4);
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
    let writes = writer.writes.lock().expect("writes lock");
    assert_eq!(writes.len(), 1);
    assert_eq!(writes[0], (run_id, 2));

    let failing_writer = InMemoryTraceWriter {
        fail: true,
        writes: Mutex::new(Vec::new()),
    };
    let error = failing_writer
        .write_events(&run_id, &events)
        .expect_err("trace write should fail");
    assert!(error.to_string().contains("trace write failed"));
}
