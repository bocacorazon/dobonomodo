use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(from = "ExpressionRepr")]
pub struct Expression {
    pub source: String,
}

impl From<String> for Expression {
    fn from(source: String) -> Self {
        Self { source }
    }
}

impl From<&str> for Expression {
    fn from(source: &str) -> Self {
        Self {
            source: source.to_owned(),
#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
enum ExpressionRepr {
    Structured { source: String },
    Raw(String),
}

impl From<ExpressionRepr> for Expression {
    fn from(value: ExpressionRepr) -> Self {
        match value {
            ExpressionRepr::Structured { source } => Self { source },
            ExpressionRepr::Raw(source) => Self { source },
        }
    }
}
