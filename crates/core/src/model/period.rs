use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PeriodStatus {
    Open,
    Closed,
    Locked,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Period {
    pub id: Uuid,
    pub identifier: String,
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    pub calendar_id: Uuid,
    pub year: i32,
    pub sequence: i32,
    pub start_date: String,
    pub end_date: String,
    pub status: PeriodStatus,
    #[serde(default)]
    pub parent_id: Option<Uuid>,
    #[serde(default)]
    pub created_at: Option<String>,
    #[serde(default)]
    pub updated_at: Option<String>,
}
