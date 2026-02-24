use anyhow::Result;
use uuid::Uuid;

use crate::model::{Dataset, Project, Resolver, RunStatus};

pub trait MetadataStore {
    fn get_dataset(&self, id: &Uuid, version: Option<i32>) -> Result<Dataset>;
    fn get_dataset_by_name(&self, name: &str) -> Result<Option<Dataset>>;
    fn register_dataset(&self, dataset: Dataset) -> Result<Uuid>;
    fn get_project(&self, id: &Uuid) -> Result<Project>;
    fn get_resolver(&self, id: &str) -> Result<Resolver>;
    fn update_run_status(&self, id: &Uuid, status: RunStatus) -> Result<()>;
}
