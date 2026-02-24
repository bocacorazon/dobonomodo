use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum StrategyType {
    Path,
    Table,
    Catalog,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ResolutionStrategy {
    Path {
        datasource_id: String,
        path: String,
    },
    Table {
        datasource_id: String,
        table: String,
        #[serde(default)]
        schema: Option<String>,
    },
    Catalog {
        endpoint: String,
        method: String,
        #[serde(default)]
        auth: Option<String>,
        #[serde(default)]
        params: serde_json::Value,
        #[serde(default)]
        headers: serde_json::Value,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ResolutionRule {
    pub name: String,
    #[serde(default, rename = "when")]
    pub when_expression: Option<String>,
    pub data_level: String,
    pub strategy: ResolutionStrategy,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ResolverStatus {
    Active,
    Disabled,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Resolver {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    pub version: i32,
    pub status: ResolverStatus,
    #[serde(default)]
    pub is_default: Option<bool>,
    pub rules: Vec<ResolutionRule>,
    #[serde(default)]
    pub created_at: Option<String>,
    #[serde(default)]
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ResolvedLocation {
    pub datasource_id: String,
    #[serde(default)]
    pub path: Option<String>,
    #[serde(default)]
    pub table: Option<String>,
    #[serde(default)]
    pub schema: Option<String>,
    #[serde(default)]
    pub period_identifier: Option<String>,
    #[serde(default)]
    pub resolver_id: Option<String>,
    #[serde(default)]
    pub rule_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OutputDestination {
    pub destination_type: String,
    #[serde(default)]
    pub target: Option<String>,
}
