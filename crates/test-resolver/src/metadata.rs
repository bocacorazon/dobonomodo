use crate::errors::MetadataError;
use dobo_core::model::metadata_store::{MetadataStore, MetadataStoreError};
use dobo_core::model::{Dataset, Project, Resolver, RunStatus};
use std::collections::HashMap;
use std::sync::Mutex;
use uuid::Uuid;

fn map_metadata_error(error: MetadataError) -> MetadataStoreError {
    match error {
        MetadataError::DatasetNotFound { id } => MetadataStoreError::DatasetNotFound { id },
        MetadataError::ProjectNotFound { id } => MetadataStoreError::ProjectNotFound { id },
        MetadataError::ResolverNotFound { id } => MetadataStoreError::ResolverNotFound { id },
        MetadataError::DatasetVersionMismatch {
            id,
            requested,
            found,
        } => MetadataStoreError::OperationFailed {
            message: format!(
                "dataset '{id}' version mismatch: requested {requested}, found {found}"
            ),
        },
        MetadataError::LockPoisoned { message } => MetadataStoreError::OperationFailed { message },
    }
}

/// In-memory metadata store for test scenarios
pub struct InMemoryMetadataStore {
    datasets: HashMap<Uuid, Dataset>,
    projects: HashMap<Uuid, Project>,
    resolvers: HashMap<String, Resolver>,
    run_statuses: Mutex<HashMap<Uuid, RunStatus>>,
}

impl InMemoryMetadataStore {
    /// Create a new in-memory metadata store
    pub fn new() -> Self {
        Self {
            datasets: HashMap::new(),
            projects: HashMap::new(),
            resolvers: HashMap::new(),
            run_statuses: Mutex::new(HashMap::new()),
        }
    }

    /// Add a dataset to the store
    pub fn add_dataset(&mut self, dataset: Dataset) {
        self.datasets.insert(dataset.id, dataset);
    }

    /// Add a project to the store
    pub fn add_project(&mut self, project: Project) {
        self.projects.insert(project.id, project);
    }

    /// Add a resolver to the store
    pub fn add_resolver(&mut self, resolver: Resolver) {
        self.resolvers.insert(resolver.id.clone(), resolver);
    }

    fn get_dataset_typed(
        &self,
        id: &Uuid,
        version: Option<i32>,
    ) -> std::result::Result<Dataset, MetadataError> {
        let dataset = self
            .datasets
            .get(id)
            .cloned()
            .ok_or(MetadataError::DatasetNotFound { id: *id })?;

        if let Some(expected_version) = version {
            if dataset.version != expected_version {
                return Err(MetadataError::DatasetVersionMismatch {
                    id: *id,
                    requested: expected_version,
                    found: dataset.version,
                });
            }
        }

        Ok(dataset)
    }

    fn get_project_typed(&self, id: &Uuid) -> std::result::Result<Project, MetadataError> {
        self.projects
            .get(id)
            .cloned()
            .ok_or(MetadataError::ProjectNotFound { id: *id })
    }

    fn get_resolver_typed(&self, id: &str) -> std::result::Result<Resolver, MetadataError> {
        self.resolvers
            .get(id)
            .cloned()
            .ok_or_else(|| MetadataError::ResolverNotFound { id: id.to_string() })
    }
}

impl Default for InMemoryMetadataStore {
    fn default() -> Self {
        Self::new()
    }
}

impl MetadataStore for InMemoryMetadataStore {
    fn get_dataset(
        &self,
        id: &Uuid,
        version: Option<i32>,
    ) -> std::result::Result<Dataset, MetadataStoreError> {
        self.get_dataset_typed(id, version).map_err(map_metadata_error)
    }

    fn get_project(&self, id: &Uuid) -> std::result::Result<Project, MetadataStoreError> {
        self.get_project_typed(id).map_err(map_metadata_error)
    }

    fn get_resolver(&self, id: &str) -> std::result::Result<Resolver, MetadataStoreError> {
        self.get_resolver_typed(id).map_err(map_metadata_error)
    }

    fn update_run_status(
        &self,
        id: &Uuid,
        status: RunStatus,
    ) -> std::result::Result<(), MetadataStoreError> {
        self.run_statuses
            .lock()
            .map_err(|poisoned| map_metadata_error(MetadataError::LockPoisoned {
                message: poisoned.to_string(),
            }))?
            .insert(*id, status);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dobo_core::model::{
        ColumnDef, ColumnType, DatasetStatus, Materialization, OperationKind, OperationInstance,
        ProjectStatus, ResolverStatus, ResolutionRule, ResolutionStrategy, TableRef, TemporalMode,
        Visibility,
    };
    use std::collections::BTreeMap;

    fn sample_dataset(version: i32) -> Dataset {
        Dataset {
            id: Uuid::now_v7(),
            name: "dataset".to_string(),
            description: None,
            owner: "owner".to_string(),
            version,
            status: DatasetStatus::Active,
            resolver_id: None,
            main_table: TableRef {
                name: "main".to_string(),
                temporal_mode: Some(TemporalMode::Period),
                columns: vec![ColumnDef {
                    name: "id".to_string(),
                    column_type: ColumnType::Integer,
                    nullable: Some(false),
                    description: None,
                }],
            },
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
            status: ProjectStatus::Active,
            visibility: Visibility::Private,
            input_dataset_id: dataset_id,
            input_dataset_version: 1,
            materialization: Materialization::Eager,
            operations: vec![OperationInstance {
                order: 1,
                kind: OperationKind::Output,
                alias: None,
                parameters: serde_json::json!({}),
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
            id: "resolver-default".to_string(),
            name: "Default Resolver".to_string(),
            description: None,
            version: 1,
            status: ResolverStatus::Active,
            is_default: Some(true),
            rules: vec![ResolutionRule {
                name: "main".to_string(),
                when_expression: None,
                data_level: "dataset".to_string(),
                strategy: ResolutionStrategy::Path {
                    datasource_id: "local".to_string(),
                    path: "data/orders.parquet".to_string(),
                },
            }],
            created_at: None,
            updated_at: None,
        }
    }

    #[test]
    fn get_dataset_respects_requested_version() {
        let mut store = InMemoryMetadataStore::new();
        let dataset = sample_dataset(3);
        let id = dataset.id;
        store.add_dataset(dataset);

        let fetched = store
            .get_dataset(&id, Some(3))
            .expect("dataset should match");
        assert_eq!(fetched.version, 3);
    }

    #[test]
    fn get_dataset_rejects_version_mismatch() {
        let mut store = InMemoryMetadataStore::new();
        let dataset = sample_dataset(2);
        let id = dataset.id;
        store.add_dataset(dataset);

        let error = store.get_dataset(&id, Some(1)).unwrap_err().to_string();
        assert!(error.contains("version mismatch"));
    }

    #[test]
    fn get_project_returns_project_when_present() {
        let mut store = InMemoryMetadataStore::new();
        let dataset = sample_dataset(1);
        let project = sample_project(dataset.id);
        let project_id = project.id;
        store.add_project(project);

        let fetched = store
            .get_project(&project_id)
            .expect("project should be returned");
        assert_eq!(fetched.id, project_id);
        assert_eq!(fetched.status, ProjectStatus::Active);
    }

    #[test]
    fn get_project_returns_not_found_for_unknown_id() {
        let store = InMemoryMetadataStore::new();
        let error = store
            .get_project(&Uuid::now_v7())
            .expect_err("unknown project should fail")
            .to_string();
        assert!(error.contains("not found"));
        assert!(error.contains("project"));
    }

    #[test]
    fn get_resolver_returns_resolver_when_present() {
        let mut store = InMemoryMetadataStore::new();
        let resolver = sample_resolver();
        let resolver_id = resolver.id.clone();
        store.add_resolver(resolver);

        let fetched = store
            .get_resolver(&resolver_id)
            .expect("resolver should be returned");
        assert_eq!(fetched.id, resolver_id);
        assert_eq!(fetched.status, ResolverStatus::Active);
    }

    #[test]
    fn get_resolver_returns_not_found_for_unknown_id() {
        let store = InMemoryMetadataStore::new();
        let error = store
            .get_resolver("missing-resolver")
            .expect_err("unknown resolver should fail")
            .to_string();
        assert!(error.contains("not found"));
        assert!(error.contains("resolver"));
    }

    #[test]
    fn update_run_status_persists_value() {
        let store = InMemoryMetadataStore::new();
        let run_id = Uuid::now_v7();

        store
            .update_run_status(&run_id, RunStatus::Running)
            .expect("status update should persist");

        let statuses = store
            .run_statuses
            .lock()
            .expect("status map lock should succeed");
        assert_eq!(statuses.get(&run_id), Some(&RunStatus::Running));
    }

    #[test]
    fn update_run_status_overwrites_previous_value() {
        let store = InMemoryMetadataStore::new();
        let run_id = Uuid::now_v7();

        store
            .update_run_status(&run_id, RunStatus::Queued)
            .expect("initial status update should persist");
        store
            .update_run_status(&run_id, RunStatus::Completed)
            .expect("second status update should persist");

        let statuses = store
            .run_statuses
            .lock()
            .expect("status map lock should succeed");
        assert_eq!(statuses.get(&run_id), Some(&RunStatus::Completed));
    }
}
