use serde::de::Deserializer;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct Expression {
    pub source: String,
}

impl<'de> Deserialize<'de> for Expression {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum ExpressionWireFormat {
            Inline(String),
            Object { source: String },
        }

        let wire = ExpressionWireFormat::deserialize(deserializer)?;
        Ok(match wire {
            ExpressionWireFormat::Inline(source) => Self { source },
            ExpressionWireFormat::Object { source } => Self { source },
        })
    }
}
