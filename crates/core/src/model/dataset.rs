use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TemporalMode {
    Period,
    Bitemporal,
    Snapshot,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ColumnType {
    String,
    Integer,
    Decimal,
    Boolean,
    Date,
    Timestamp,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DatasetStatus {
    Active,
    Disabled,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ColumnDef {
    pub name: String,
    #[serde(rename = "type")]
    pub column_type: ColumnType,
    #[serde(default)]
    pub nullable: Option<bool>,
    #[serde(default)]
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TableRef {
    pub name: String,
    #[serde(default)]
    pub temporal_mode: Option<TemporalMode>,
    pub columns: Vec<ColumnDef>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct JoinCondition {
    pub source_column: String,
    pub target_column: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum LookupTarget {
    Table {
        name: String,
        #[serde(default)]
        temporal_mode: Option<TemporalMode>,
        columns: Vec<ColumnDef>,
    },
    Dataset {
        id: Uuid,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LookupDef {
    #[serde(default)]
    pub alias: Option<String>,
    pub target: LookupTarget,
    pub join_conditions: Vec<JoinCondition>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Dataset {
    pub id: Uuid,
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    pub owner: String,
    pub version: i32,
    pub status: DatasetStatus,
    #[serde(default)]
    pub resolver_id: Option<String>,
    pub main_table: TableRef,
    #[serde(default)]
    pub lookups: Vec<LookupDef>,
    #[serde(default)]
    pub natural_key_columns: Vec<String>,
    #[serde(default)]
    pub created_at: Option<String>,
    #[serde(default)]
    pub updated_at: Option<String>,
}
