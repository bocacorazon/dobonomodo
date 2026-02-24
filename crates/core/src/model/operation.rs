use serde::de::{Deserializer, Error as DeError};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::model::expression::Expression;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum OperationKind {
    Update,
    Aggregate,
    Append,
    Output,
    Delete,
}

/// RuntimeJoin configuration for a single join within an update operation
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RuntimeJoin {
    /// Logical name for referencing joined columns in expressions
    pub alias: String,
    /// ID of the Dataset to join
    pub dataset_id: Uuid,
    /// Optional pinned version; omit for latest active version
    #[serde(default)]
    pub dataset_version: Option<i32>,
    /// Join condition expression
    pub on: Expression,
}

/// Arguments for Update operations
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct UpdateArguments {
    /// Runtime joins to apply before assignments
    #[serde(default)]
    pub joins: Vec<RuntimeJoin>,
    /// Assignment expressions (placeholder - to be extended)
    #[serde(deserialize_with = "deserialize_non_empty_assignments")]
    pub assignments: Vec<serde_json::Value>,
}

fn deserialize_non_empty_assignments<'de, D>(
    deserializer: D,
) -> Result<Vec<serde_json::Value>, D::Error>
where
    D: Deserializer<'de>,
{
    let assignments = Vec::<serde_json::Value>::deserialize(deserializer)?;
    if assignments.is_empty() {
        return Err(DeError::custom("assignments must contain at least 1 item"));
    }
    Ok(assignments)
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OperationInstance {
    pub order: u32,
    #[serde(rename = "type")]
    pub kind: OperationKind,
    #[serde(default)]
    pub alias: Option<String>,
    #[serde(default)]
    pub parameters: serde_json::Value,
}
