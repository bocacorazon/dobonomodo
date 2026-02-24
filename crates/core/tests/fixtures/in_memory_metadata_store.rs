use anyhow::{anyhow, Result};
use dobo_core::model::metadata_store::DatasetLookupError;
use dobo_core::model::{Dataset, DatasetStatus, Project, Resolver, RunStatus};
use dobo_core::MetadataStore;
use uuid::Uuid;

#[derive(Clone, Default)]
pub struct InMemoryMetadataStore {
    datasets: Vec<Dataset>,
    resolvers: Vec<Resolver>,
    failure: Option<String>,
}

impl InMemoryMetadataStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_dataset(mut self, dataset: Dataset) -> Self {
        self.datasets.push(dataset);
        self
    }

    #[allow(dead_code)]
    pub fn with_resolver(mut self, resolver: Resolver) -> Self {
        self.resolvers.push(resolver);
        self
    }

    #[allow(dead_code)]
    pub fn with_failure(mut self, message: impl Into<String>) -> Self {
        self.failure = Some(message.into());
        self
    }
}

impl MetadataStore for InMemoryMetadataStore {
    fn get_dataset(
        &self,
        id: &Uuid,
        version: Option<i32>,
    ) -> std::result::Result<Dataset, DatasetLookupError> {
        if let Some(message) = &self.failure {
            return Err(DatasetLookupError::Other(anyhow!("{message}")));
        }

        let matching = self
            .datasets
            .iter()
            .filter(|dataset| dataset.id == *id)
            .collect::<Vec<_>>();

        if matching.is_empty() {
            return Err(DatasetLookupError::DatasetNotFound { dataset_id: *id });
        }

        if let Some(version) = version {
            return matching
                .into_iter()
                .find(|dataset| dataset.version == version)
                .cloned()
                .ok_or(DatasetLookupError::VersionNotFound {
                    dataset_id: *id,
                    version,
                });
        }

        matching
            .iter()
            .filter(|dataset| dataset.status == DatasetStatus::Active)
            .max_by_key(|dataset| dataset.version)
            .or_else(|| matching.iter().max_by_key(|dataset| dataset.version))
            .map(|dataset| (*dataset).clone())
            .ok_or(DatasetLookupError::DatasetNotFound { dataset_id: *id })
    }

    fn get_project(&self, _id: &Uuid) -> Result<Project> {
        Err(anyhow!("not implemented"))
    }

    fn get_resolver(&self, id: &str) -> Result<Resolver> {
        self.resolvers
            .iter()
            .find(|resolver| resolver.id == id)
            .cloned()
            .ok_or_else(|| anyhow!("resolver {id} not found"))
    }

    fn update_run_status(&self, _id: &Uuid, _status: RunStatus) -> Result<()> {
        Err(anyhow!("not implemented"))
    }
}
