use thiserror::Error;
use uuid::Uuid;

use crate::model::{Dataset, Project, Resolver, RunStatus};

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum MetadataStoreError {
    #[error("dataset '{id}' not found")]
    DatasetNotFound { id: Uuid },
    #[error("project '{id}' not found")]
    ProjectNotFound { id: Uuid },
    #[error("resolver '{id}' not found")]
    ResolverNotFound { id: String },
    #[error("metadata operation failed: {message}")]
    OperationFailed { message: String },
}

pub trait MetadataStore {
    fn get_dataset(&self, id: &Uuid, version: Option<i32>) -> Result<Dataset, MetadataStoreError>;
    fn get_dataset_by_name(&self, _name: &str) -> Result<Option<Dataset>, MetadataStoreError> {
        Err(MetadataStoreError::OperationFailed {
            message: "get_dataset_by_name is not implemented".to_string(),
        })
    }

    fn register_dataset(&self, _dataset: Dataset) -> Result<Uuid, MetadataStoreError> {
        Err(MetadataStoreError::OperationFailed {
            message: "register_dataset is not implemented".to_string(),
        })
    }

    fn get_project(&self, id: &Uuid) -> Result<Project, MetadataStoreError>;
    fn get_resolver(&self, id: &str) -> Result<Resolver, MetadataStoreError>;
    fn update_run_status(&self, id: &Uuid, status: RunStatus) -> Result<(), MetadataStoreError>;
}
