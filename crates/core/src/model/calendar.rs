use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CalendarStatus {
    Draft,
    Active,
    Deprecated,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DateRule {
    pub sequence: i32,
    pub start_month: i32,
    pub start_day: i32,
    pub end_month: i32,
    pub end_day: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LevelDef {
    pub name: String,
    #[serde(default)]
    pub parent_level: Option<String>,
    #[serde(default)]
    pub identifier_pattern: Option<String>,
    #[serde(default)]
    pub date_rules: Vec<DateRule>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Calendar {
    pub id: Uuid,
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    pub status: CalendarStatus,
    pub is_default: bool,
    #[serde(default)]
    pub levels: Vec<LevelDef>,
    #[serde(default)]
    pub created_at: Option<String>,
    #[serde(default)]
    pub updated_at: Option<String>,
}
