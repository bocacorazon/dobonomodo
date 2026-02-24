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
    fn get_dataset(
        &self,
        id: &Uuid,
        version: Option<i32>,
    ) -> std::result::Result<Dataset, MetadataStoreError>;
    fn get_project(&self, id: &Uuid) -> std::result::Result<Project, MetadataStoreError>;
    fn get_resolver(&self, id: &str) -> std::result::Result<Resolver, MetadataStoreError>;
    fn get_default_resolver(&self) -> std::result::Result<Resolver, MetadataStoreError> {
        Err(MetadataStoreError::OperationFailed {
            message: "default resolver lookup is not implemented by this metadata store"
                .to_string(),
        })
    }
    fn update_run_status(
        &self,
        id: &Uuid,
        status: RunStatus,
    ) -> std::result::Result<(), MetadataStoreError>;
}
