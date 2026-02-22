use serde::{Deserialize, Serialize};

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
