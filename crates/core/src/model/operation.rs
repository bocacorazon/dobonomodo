use serde::{Deserialize, Serialize};

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
