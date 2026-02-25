use serde::{Deserialize, Deserializer, Serialize};
use uuid::Uuid;

use crate::model::Expression;

use super::resolver::OutputDestination;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum OperationKind {
    Update,
    Aggregate,
    Append,
    Output,
    Delete,
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

impl OperationInstance {
    pub fn append_parameters(&self) -> Result<AppendOperation, serde_json::Error> {
        serde_json::from_value(self.parameters.clone())
    }
}

/// Reference to a source dataset with optional version pinning
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DatasetRef {
    pub dataset_id: Uuid,
    #[serde(default)]
    pub dataset_version: Option<i32>,
}

/// Single aggregation computation with output column name and aggregate expression
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Aggregation {
    /// Output column name for the aggregated value
    pub column: String,
    /// Aggregate function expression (e.g., "SUM(amount)", "COUNT(budget_id)")
    pub expression: String,
}

/// Configuration for aggregating source rows before appending
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AppendAggregation {
    /// Columns to group by
    pub group_by: Vec<String>,
    /// Aggregate computations to perform
    pub aggregations: Vec<Aggregation>,
}

/// Parameters for an append operation
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AppendOperation {
    /// Reference to the source dataset to append from
    pub source: DatasetRef,
    /// Optional filter expression for source rows
    #[serde(default, deserialize_with = "deserialize_optional_expression")]
    pub source_selector: Option<Expression>,
    /// Optional aggregation to apply before appending
    #[serde(default)]
    pub aggregation: Option<AppendAggregation>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum ExpressionInput {
    String(String),
    Structured { source: String },
}

fn deserialize_optional_expression<'de, D>(deserializer: D) -> Result<Option<Expression>, D::Error>
where
    D: Deserializer<'de>,
{
    let input = Option::<ExpressionInput>::deserialize(deserializer)?;
    Ok(input.map(|value| match value {
        ExpressionInput::String(source) => Expression::from(source),
        ExpressionInput::Structured { source } => Expression::from(source),
    }))
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct DeleteOperationParams {
    #[serde(default)]
    pub selector: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OutputOperationParams {
    pub destination: OutputDestination,
    #[serde(default)]
    pub include_deleted: bool,
    #[serde(default)]
    pub selector: Option<String>,
}

impl Default for OutputOperationParams {
    fn default() -> Self {
        Self {
            destination: OutputDestination {
                destination_type: "memory".to_string(),
                target: None,
            },
            include_deleted: false,
            selector: None,
        }
    }
}
