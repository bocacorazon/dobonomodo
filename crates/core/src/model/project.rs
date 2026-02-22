use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use uuid::Uuid;

use crate::model::operation::OperationInstance;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ProjectStatus {
    Draft,
    Active,
    Inactive,
    Conflict,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Visibility {
    Private,
    Public,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Materialization {
    Eager,
    Runtime,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ChangeType {
    Removed,
    Renamed,
    TypeChanged,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ConflictResolution {
    Adapted,
    Pinned,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BreakingChange {
    pub column: String,
    pub change_type: ChangeType,
    pub affected_operations: Vec<u32>,
    #[serde(default)]
    pub resolution: Option<ConflictResolution>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ConflictReport {
    pub dataset_version_from: i32,
    pub dataset_version_to: i32,
    pub breaking_changes: Vec<BreakingChange>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Project {
    pub id: Uuid,
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    pub owner: String,
    pub version: i32,
    pub status: ProjectStatus,
    pub visibility: Visibility,
    pub input_dataset_id: Uuid,
    pub input_dataset_version: i32,
    pub materialization: Materialization,
    pub operations: Vec<OperationInstance>,
    #[serde(default)]
    pub selectors: BTreeMap<String, String>,
    #[serde(default)]
    pub resolver_overrides: BTreeMap<Uuid, String>,
    #[serde(default)]
    pub conflict_report: Option<ConflictReport>,
    #[serde(default)]
    pub created_at: Option<String>,
    #[serde(default)]
    pub updated_at: Option<String>,
}
