use anyhow::{anyhow, Result};
use uuid::Uuid;

use crate::model::{Dataset, Project, Resolver, RunStatus};

pub trait MetadataStore {
    fn get_dataset(&self, id: &Uuid, version: Option<i32>) -> Result<Dataset>;
    fn get_project(&self, id: &Uuid) -> Result<Project>;
    fn get_resolver(&self, id: &str) -> Result<Resolver>;
    fn get_default_resolver(&self) -> Result<Resolver> {
        Err(anyhow!(
            "default resolver lookup is not implemented by this metadata store"
        ))
    }
    fn update_run_status(&self, id: &Uuid, status: RunStatus) -> Result<()>;
}
