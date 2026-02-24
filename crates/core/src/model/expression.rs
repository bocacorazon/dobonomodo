use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
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
        }
    }
}
