use crate::errors::MetadataError;
use anyhow::Result;
use dobo_core::model::metadata_store::MetadataStore;
use dobo_core::model::{Dataset, Project, Resolver, RunStatus};
use std::collections::HashMap;
use std::sync::Mutex;
use uuid::Uuid;

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
    fn get_dataset(&self, id: &Uuid, version: Option<i32>) -> Result<Dataset> {
        self.get_dataset_typed(id, version).map_err(Into::into)
    }

    fn get_project(&self, id: &Uuid) -> Result<Project> {
        self.get_project_typed(id).map_err(Into::into)
    }

    fn get_resolver(&self, id: &str) -> Result<Resolver> {
        self.get_resolver_typed(id).map_err(Into::into)
    }

    fn update_run_status(&self, id: &Uuid, status: RunStatus) -> Result<()> {
        self.run_statuses
            .lock()
            .map_err(|poisoned| MetadataError::LockPoisoned {
                message: poisoned.to_string(),
            })?
            .insert(*id, status);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dobo_core::model::{ColumnDef, ColumnType, DatasetStatus, TableRef, TemporalMode};

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
