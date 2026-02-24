use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(from = "ExpressionRepr")]
pub struct Expression {
    pub source: String,
}

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
