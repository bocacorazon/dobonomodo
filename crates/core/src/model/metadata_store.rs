use anyhow::{Error, Result};
use thiserror::Error;
use uuid::Uuid;

use crate::model::{Dataset, Project, Resolver, RunStatus};

#[derive(Debug, Error)]
pub enum DatasetLookupError {
    #[error("Dataset {dataset_id} not found")]
    DatasetNotFound { dataset_id: Uuid },

    #[error("Dataset {dataset_id} version {version} not found")]
    VersionNotFound { dataset_id: Uuid, version: i32 },

    #[error("{0}")]
    Other(Error),
}

pub trait MetadataStore {
    /// Returns the resolved dataset or a precise lookup failure.
    ///
    /// Implementations must return `DatasetNotFound` when no dataset exists for the ID,
    /// `VersionNotFound` when the dataset exists but the requested version is missing,
    /// and `Other` for backend/transport failures.
    fn get_dataset(
        &self,
        id: &Uuid,
        version: Option<i32>,
    ) -> std::result::Result<Dataset, DatasetLookupError>;
    fn get_project(&self, id: &Uuid) -> Result<Project>;
    fn get_resolver(&self, id: &str) -> Result<Resolver>;
    fn update_run_status(&self, id: &Uuid, status: RunStatus) -> Result<()>;
}
