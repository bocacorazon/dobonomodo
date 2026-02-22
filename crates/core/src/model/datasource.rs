use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DataSourceStatus {
    Active,
    Disabled,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DataSourceType {
    Database,
    Parquet,
    Csv,
    Api,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DataSource {
    pub id: Uuid,
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    pub owner: String,
    pub status: DataSourceStatus,
    #[serde(rename = "type")]
    pub source_type: DataSourceType,
    pub options: serde_json::Value,
    #[serde(default)]
    pub credential_ref: Option<String>,
    #[serde(default)]
    pub created_at: Option<String>,
    #[serde(default)]
    pub updated_at: Option<String>,
}
