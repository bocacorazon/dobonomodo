use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::model::operation::OperationInstance;
use crate::model::project::Materialization;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RunStatus {
    Queued,
    Running,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TriggerType {
    Manual,
    Scheduled,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ResolverSnapshot {
    pub dataset_id: Uuid,
    pub resolver_id: String,
    pub resolver_version: i32,
    /// Stores one entry per runtime join resolution for reproducibility.
    #[serde(default)]
    pub join_datasets: Vec<JoinDatasetSnapshot>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct JoinDatasetSnapshot {
    pub alias: String,
    pub dataset_id: Uuid,
    pub dataset_version: i32,
    pub resolver_source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProjectSnapshot {
    pub input_dataset_id: Uuid,
    pub input_dataset_version: i32,
    pub materialization: Materialization,
    pub operations: Vec<OperationInstance>,
    #[serde(default)]
    pub resolver_snapshots: Vec<ResolverSnapshot>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ErrorDetail {
    pub operation_order: u32,
    pub message: String,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Run {
    pub id: Uuid,
    pub project_id: Uuid,
    pub project_version: i32,
    pub project_snapshot: ProjectSnapshot,
    pub period_ids: Vec<Uuid>,
    pub status: RunStatus,
    pub trigger_type: TriggerType,
    pub triggered_by: String,
    #[serde(default)]
    pub last_completed_operation: Option<u32>,
    #[serde(default)]
    pub output_dataset_id: Option<Uuid>,
    #[serde(default)]
    pub parent_run_id: Option<Uuid>,
    #[serde(default)]
    pub error: Option<ErrorDetail>,
    #[serde(default)]
    pub started_at: Option<String>,
    #[serde(default)]
    pub completed_at: Option<String>,
    #[serde(default)]
    pub created_at: Option<String>,
}
