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
    /// Look up a dataset by its logical name.
    ///
    /// Implement this method (and [`register_dataset`]) to support dataset
    /// registration and lookup by name as described in the API contracts.
    ///
    /// The default implementation returns an [`OperationFailed`] error to
    /// indicate that this `MetadataStore` implementation does not support
    /// dataset lookup by name.
    fn get_dataset_by_name(&self, _name: &str) -> Result<Option<Dataset>, MetadataStoreError> {
        Err(MetadataStoreError::OperationFailed {
            message: "dataset lookup by name is not supported by this MetadataStore implementation"
                .to_string(),
        })
    }

    /// Register a new dataset and return its assigned identifier.
    ///
    /// Implement this method (and [`get_dataset_by_name`]) to support dataset
    /// registration functionality as described in the API contracts.
    ///
    /// The default implementation returns an [`OperationFailed`] error to
    /// indicate that this `MetadataStore` implementation does not support
    /// dataset registration.
    fn register_dataset(&self, _dataset: Dataset) -> Result<Uuid, MetadataStoreError> {
        Err(MetadataStoreError::OperationFailed {
            message:
                "dataset registration is not supported by this MetadataStore implementation"
                    .to_string(),
        })
    }

    fn get_project(&self, id: &Uuid) -> Result<Project, MetadataStoreError>;
    fn get_resolver(&self, id: &str) -> Result<Resolver, MetadataStoreError>;
    fn get_default_resolver(&self) -> Result<Resolver, MetadataStoreError> {
        Err(MetadataStoreError::OperationFailed {
            message: "default resolver lookup is not implemented by this metadata store"
                .to_string(),
        })
    }
    fn update_run_status(&self, id: &Uuid, status: RunStatus) -> Result<(), MetadataStoreError>;
}
