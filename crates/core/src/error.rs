use thiserror::Error;

pub type Result<T> = std::result::Result<T, CoreError>;

#[derive(Debug, Error)]
pub enum CoreError {
    #[error("{0}")]
    Message(String),
}

impl CoreError {
    pub fn message(message: impl Into<String>) -> Self {
        Self::Message(message.into())
    }
}
